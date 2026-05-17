use log::{trace, error, info};
use netter_sdk::RDLTypes;
use crate::language::ast::AstNode;
use crate::language::error::{Result, Error, ErrorKind};
use crate::language::interpreter::OBJECT_REGISTRY;
use crate::runtime_error;
use super::context::ExecutionContext;
use super::builtin::request::Request;
use super::builtin::response::Response;
use super::builtin::plugin::PluginManager;

pub struct Evaluator<'a> {
    context: &'a mut ExecutionContext,
    request: &'a mut Request,
    response: &'a mut Response,
    plugin_manager: &'a PluginManager,
}

impl<'a> Evaluator<'a> {
    pub fn new(
        context: &'a mut ExecutionContext,
        request: &'a mut Request,
        response: &'a mut Response,
        plugin_manager: &'a PluginManager,
    ) -> Self {
        Evaluator {
            context,
            request,
            response,
            plugin_manager,
        }
    }

    pub fn evaluate(&mut self, expr: &AstNode) -> Result<RDLTypes> {
        trace!("Evaluating expression: {:?}", expr);
        match expr {
            AstNode::StringLiteral(value) => Ok(RDLTypes::String(value.clone())),
            AstNode::FormattedString(parts) => {
                let mut result = String::new();
                for part in parts {
                    let val = self.evaluate(part)?;
                    result.push_str(&val.to_string())
                }
                Ok(RDLTypes::String(result))
            }
            AstNode::NumberLiteral(value) => Ok(RDLTypes::Number(*value)),
            AstNode::Identifier(name) => self.evaluate_identifier(&name.clone().into()).map(|s| s.into()),
            AstNode::BinaryOp { left, operator, right } =>
                self.evaluate_binary_op(left, operator, right),
            AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } =>
                self.evaluate_function_call(object, name, args, *try_operator, *unwrap_operator),
            AstNode::PropertyAccess { object, property } =>
                self.evaluate_property_access(object, property),
            AstNode::ArrayLiteral(elements) => self.evaluate_array_literal(elements),
            AstNode::ArrayAccess { array, index } => self.evaluate_array_access(array, index),
            _ => runtime_error!(format!("Unsupported expression: {:?}", expr)),
        }
    }

    fn evaluate_array_literal(&mut self, elements: &[Box<AstNode>]) -> Result<RDLTypes> {
        let mut values = Vec::new();

        for element in elements {
            let value = self.evaluate(element)?;
            match serde_json::from_str::<serde_json::Value>(&format!("\"{}\"", value)) {
                Ok(json_val) => {
                    if let serde_json::Value::String(s) = json_val {
                        if let Ok(num) = s.parse::<i64>() {
                            values.push(serde_json::Value::Number(serde_json::Number::from(num)));
                        } else {
                            values.push(serde_json::Value::String(s));
                        }
                    } else {
                        values.push(json_val);
                    }
                }
                Err(_) => {
                    values.push(serde_json::Value::String(value.to_string()));
                }
            }
        }

        Ok(serde_json::to_string(&values).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации массива: {}", e),
            line: None,
            column: None,
        })?.into())
    }

    fn evaluate_array_access(&mut self, array: &Box<AstNode>, index: &Box<AstNode>) -> Result<RDLTypes> {
        let array_value = self.evaluate(array)?;
        let index_value = self.evaluate(index)?;

        let array_json: Vec<serde_json::Value> = serde_json::from_str(array_value.to_string().as_str()).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: format!("Значение '{}' не является массивом", array_value),
            line: None,
            column: None,
        })?;

        let i = index_value.as_u64().map(|n| n as usize);
        let index_num = i.map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: format!("Индекс '{}' должен быть числом", index_value),
            line: None,
            column: None,
        })?;

        if index_num >= array_json.len() {
            return runtime_error!(format!("Индекс {} выходит за границы массива (размер: {})", index_num, array_json.len()));
        }

        let element = &array_json[index_num];
        match element {
            serde_json::Value::String(s) => Ok(s.clone().into()),
            serde_json::Value::Number(n) => Ok(n.as_i64().unwrap().into()),
            serde_json::Value::Bool(b) => Ok(b.clone().into()),
            serde_json::Value::Null => Ok("null".into()),
            _ => Ok(element.as_str().unwrap().into()),
        }
    }

    fn evaluate_identifier(&self, name: &RDLTypes) -> Result<RDLTypes> {
        if let Some(value) = self.context.get_variable(name) {
            return Ok(value);
        }

        match name.to_string().as_str() {
            "Request" | "Response" | "Database" | "FileSystem" => Ok(name.to_string().into()),
            _ if self.plugin_manager.has_plugin(name.to_string().as_str()) => Ok(name.to_string().into()),
            _ => runtime_error!(format!("Variable or object '{}' not found", name)),
        }
    }

    fn evaluate_binary_op(&mut self, left: &Box<AstNode>, operator: &str, right: &Box<AstNode>) -> Result<RDLTypes> {
        let left_value = self.evaluate(left)?;
        let right_value = self.evaluate(right)?;

        trace!("Binary operation: '{}' {} '{}'", left_value, operator, right_value);

        match operator {
            "==" => Ok(RDLTypes::String((left_value == right_value).to_string())),
            "!=" => Ok(RDLTypes::String((left_value != right_value).to_string())),
            "+" => {
                if let (Ok(left_num), Ok(right_num)) = (left_value.clone().as_i64(), right_value.clone().as_i64()) {
                    let result = left_num + right_num;
                    Ok(RDLTypes::Number(result))
                } else {
                    Ok(RDLTypes::String(format!("{}{}", left_value, right_value)))
                }
            },
            "-" => {
                let left_num: i64 = left_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for subtraction", left_value),
                        }
                    )?;
                let right_num: i64 = right_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for subtraction", right_value),
                        }
                    )?;
                let result = left_num - right_num;
                Ok(RDLTypes::Number(result))
            },
            "*" => {
                let left_num: i64 = left_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for multiplication", left_value)
                        }
                    )?;
                let right_num: i64 = right_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for multiplication", right_value)
                        }
                    )?;
                let result = left_num * right_num;
                Ok(RDLTypes::Number(result))
            },
            "/" => {
                let left_num: i64 = left_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for division", left_value)
                        }
                    )?;
                let right_num: i64 = right_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for division", right_value)
                        }
                    )?;
                if right_num == 0 {
                    return runtime_error!("Division by zero".to_string());
                }
                let result = left_num / right_num;

                Ok(RDLTypes::Number(result))
            },
            "^" => {
                let left_num: i64 = left_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for raising to a power", left_value)
                        }
                    )?;
                let right_num: i64 = right_value.as_i64()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Can't convert '{}' into a number for raising to a power", right_value)
                        }
                    )?;
                if right_num < 0 {

                }
                let result = crate::utils::powi(left_num, right_num);

                Ok(RDLTypes::Number(result))
            },
            "&&" => {

                if left_value == false.into() {
                    return Ok(RDLTypes::Boolean(false));
                }

                Ok(if right_value != false.into() {
                    RDLTypes::Boolean(true)
                } else {
                    RDLTypes::Boolean(false)
                })
            },
            "||" => {
                if left_value != false.into() {
                    return Ok(RDLTypes::Boolean(true));
                }

                Ok(if right_value != false.into() {
                    RDLTypes::Boolean(true)
                } else {
                    RDLTypes::Boolean(false)
                })
            },
            _ => runtime_error!(format!("Not supported binary operator: {}", operator)),
        }
    }

    fn evaluate_property_access(&mut self, object: &Box<AstNode>, property: &str) -> Result<RDLTypes> {
        let obj_value = self.evaluate(object)?;
        runtime_error!(format!("Property access '{}.{}' not implemented", obj_value, property))
    }

    fn evaluate_function_call(
        &mut self,
        object: &Option<Box<AstNode>>,
        name: &str,
        args: &[Box<AstNode>],
        try_operator: bool,
        unwrap_operator: bool
    ) -> Result<RDLTypes> {
        let mut evaluated_args = Vec::new();
        for arg in args {
            evaluated_args.push(self.evaluate(arg)?);
        }

        let object_name = if let Some(obj) = object {
            match &**obj {
                AstNode::Identifier(name) => Some(name.clone()),
                _ => {
                    let obj_value = self.evaluate(obj)?;
                    return runtime_error!(format!(
                        "Calling a method on a non-identifier ('{}') is not supported",
                        obj_value
                    ));
                }
            }
        } else {
            None
        };

        let obj_names = self.get_objects_names()?;

        let result = match object_name.as_deref() {
            Some(n) if obj_names.contains(&n) => {
                let mut lock = if let Ok(l) = OBJECT_REGISTRY.lock() {
                    l
                } else {
                    return runtime_error!("OBJECT_REGISTRY mutex is poisoned!");
                };

                if let Some(object) = lock.get_object_mut(name) {
                    object.call_method(name, evaluated_args)
                } else {
                    runtime_error!(format!("Object '{}' not found", n))
                }
            }
            Some(plugin_name) if self.plugin_manager.has_plugin(plugin_name) => {
                self.plugin_manager.call_plugin_function(plugin_name, name, &evaluated_args)
            },
            None => self.call_global_function(name, &evaluated_args),
            Some(unknown) => runtime_error!(format!("Object '{}' not found", unknown)),
        };

        match result {
            Ok(value) => Ok(value),
            Err(err) => {
                if try_operator {
                    Err(err)
                } else if unwrap_operator {
                    error!("Operator '!!' caused panic: {}", err);
                    panic!("Execution error (unwrap !!): {}", err);
                } else {
                    Err(err)
                }
            }
        }
    }

    fn get_objects_names(&self) -> Result<Vec<&'static str>> {
        if let Ok(lock) = OBJECT_REGISTRY.lock() {
            let mut names = Vec::new();
            for obj in lock.objects() {
                let name: &'static str = obj.name();
                names.push(name);
            }
            Ok(names)
        } else {
            return runtime_error!("OBJECT_REGISTRY mutex is poisoned!");
        }
    }

    fn call_global_function(&self, name: &str, args: &[RDLTypes]) -> Result<RDLTypes> {
        match name {
            "log_error" => {
                if args.len() == 1 {
                    error!("{}", args[0]);
                    Ok("".into())
                } else {
                    runtime_error!("Функция log_error требует 1 аргумент")
                }
            },
            "log_info" => {
                if args.len() == 1 {
                    info!("{}", args[0]);
                    Ok("".into())
                } else {
                    runtime_error!("Функция log_info требует 1 аргумент")
                }
            },
            "log_trace" => {
                if args.len() == 1 {
                    trace!("{}", args[0]);
                    Ok("".into())
                } else {
                    runtime_error!("Функция log_trace требует 1 аргумент")
                }
            },
            "array_length" => {
                if args.len() == 1 {
                    self.array_length(&args[0])
                } else {
                    runtime_error!("Функция array_length требует 1 аргумент")
                }
            },
            "array_push" => {
                if args.len() == 2 {
                    self.array_push(&args[0], &args[1])
                } else {
                    runtime_error!("Функция array_push требует 2 аргумента")
                }
            },
            "array_pop" => {
                if args.len() == 1 {
                    self.array_pop(&args[0])
                } else {
                    runtime_error!("Функция array_pop требует 1 аргумент")
                }
            },
            "array_contains" => {
                if args.len() == 2 {
                    self.array_contains(&args[0], &args[1])
                } else {
                    runtime_error!("Функция array_contains требует 2 аргумента")
                }
            },
            "array_join" => {
                if args.len() == 2 {
                    self.array_join(&args[0], &args[1])
                } else {
                    runtime_error!("Функция array_join требует 2 аргумента")
                }
            },
            _ => runtime_error!(format!("Глобальная функция не найдена: {}", name)),
        }
    }

    fn array_length(&self, array_name: &RDLTypes) -> Result<RDLTypes> {
        let array: Vec<serde_json::Value> = serde_json::from_str(array_name.to_string().as_str()).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;
        Ok(array.len().into())
    }

    fn array_push(&self, array_name: &RDLTypes, element: &RDLTypes) -> Result<RDLTypes> {
        let mut array: Vec<serde_json::Value> = serde_json::from_str(array_name.to_string().as_str()).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Первый аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;


        let e = element.clone();
        let n = e.clone().try_into();
        if let Ok(num) = n {
            array.push(serde_json::Value::Number(serde_json::Number::from_i128(num).unwrap_or_else(|| serde_json::Number::from(0))));
        } else if e == true.into() {
            array.push(serde_json::Value::Bool(true));
        } else if e == false.into() {
            array.push(serde_json::Value::Bool(false));
        } else {
            array.push(serde_json::Value::String(e.to_string()));
        }

        Ok(serde_json::to_string(&array).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации массива: {}", e),
            line: None,
            column: None,
        })?.into())
    }

    fn array_pop(&self, array_name: &RDLTypes) -> Result<RDLTypes> {
        let mut array: Vec<serde_json::Value> = serde_json::from_str(array_name.to_string().as_str()).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;

        if array.is_empty() {
            return runtime_error!("Невозможно удалить элемент из пустого массива");
        }

        array.pop();
        Ok(serde_json::to_string(&array).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации массива: {}", e),
            line: None,
            column: None,
        })?.into())
    }

    fn array_contains(&self, array_name: &RDLTypes, element: &RDLTypes) -> Result<RDLTypes> {
        let array: Vec<serde_json::Value> = serde_json::from_str(array_name.to_string().as_str()).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Первый аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;

        for item in &array {
            let item_str = match item {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                _ => continue,
            };

            let element_str: String = element.clone().try_into()?;
            if item_str == element_str {
                return Ok(true.into());
            }
        }

        Ok(false.into())
    }

    fn array_join(&self, array_name: &RDLTypes, separator: &RDLTypes) -> Result<RDLTypes> {
        let array: Vec<serde_json::Value> = serde_json::from_str(array_name.to_string().as_str()).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Первый аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;

        let string_items: Vec<String> = array.iter().map(|item| {
            match item {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Number(n) => n.to_string(),
                serde_json::Value::Bool(b) => b.to_string(),
                serde_json::Value::Null => "null".to_string(),
                _ => item.to_string(),
            }
        }).collect();

        Ok(string_items.join(separator.to_string().as_str()).into())
    }
}