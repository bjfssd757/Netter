use log::{trace, debug, error, info};
use crate::language::ast::AstNode;
use crate::language::error::{Result, Error, ErrorKind};
use crate::language::interpreter::builtin::filesystem::FileSystem;
use crate::runtime_error;
use super::context::ExecutionContext;
use super::builtin::request::Request;
use super::builtin::response::Response;
use super::builtin::database::Database;
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

    pub fn evaluate(&mut self, expr: &AstNode) -> Result<String> {
        trace!("Вычисление выражения: {:?}", expr);
        match expr {
            AstNode::StringLiteral(value) => Ok(value.clone()),
            AstNode::NumberLiteral(value) => Ok(value.to_string()),
            AstNode::Identifier(name) => self.evaluate_identifier(name),
            AstNode::BinaryOp { left, operator, right } =>
                self.evaluate_binary_op(left, operator, right),
            AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } =>
                self.evaluate_function_call(object, name, args, *try_operator, *unwrap_operator),
            AstNode::PropertyAccess { object, property } =>
                self.evaluate_property_access(object, property),
            AstNode::ArrayLiteral(elements) => self.evaluate_array_literal(elements),
            AstNode::ArrayAccess { array, index } => self.evaluate_array_access(array, index),
            _ => runtime_error!(format!("Неподдерживаемый тип выражения: {:?}", expr)),
        }
    }

    fn evaluate_array_literal(&mut self, elements: &[Box<AstNode>]) -> Result<String> {
        let mut values = Vec::new();

        for element in elements {
            let value = self.evaluate(element)?;
            match serde_json::from_str::<serde_json::Value>(&format!("\"{}\"", value)) {
                Ok(json_val) => {
                    if let serde_json::Value::String(s) = json_val {
                        if let Ok(num) = s.parse::<f64>() {
                            values.push(serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0))));
                        } else {
                            values.push(serde_json::Value::String(s));
                        }
                    } else {
                        values.push(json_val);
                    }
                }
                Err(_) => {
                    values.push(serde_json::Value::String(value));
                }
            }
        }

        serde_json::to_string(&values).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации массива: {}", e),
            line: None,
            column: None,
        })
    }

    fn evaluate_array_access(&mut self, array: &Box<AstNode>, index: &Box<AstNode>) -> Result<String> {
        let array_value = self.evaluate(array)?;
        let index_value = self.evaluate(index)?;

        let array_json: Vec<serde_json::Value> = serde_json::from_str(&array_value).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: format!("Значение '{}' не является массивом", array_value),
            line: None,
            column: None,
        })?;

        let index_num = index_value.parse::<usize>().map_err(|_| Error {
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
            serde_json::Value::String(s) => Ok(s.clone()),
            serde_json::Value::Number(n) => Ok(n.to_string()),
            serde_json::Value::Bool(b) => Ok(b.to_string()),
            serde_json::Value::Null => Ok("null".to_string()),
            _ => Ok(element.to_string()),
        }
    }

    fn evaluate_identifier(&self, name: &str) -> Result<String> {
        if let Some(value) = self.context.get_variable(name) {
            return Ok(value);
        }

        match name {
            "Request" | "Response" | "Database" | "FileSystem" => Ok(name.to_string()),
            _ if self.plugin_manager.has_plugin(name) => Ok(name.to_string()),
            _ => runtime_error!(format!("Переменная или объект '{}' не найден", name)),
        }
    }

    fn evaluate_binary_op(&mut self, left: &Box<AstNode>, operator: &str, right: &Box<AstNode>) -> Result<String> {
        let left_value = self.evaluate(left)?;
        let right_value = self.evaluate(right)?;

        trace!("Бинарная операция: '{}' {} '{}'", left_value, operator, right_value);

        match operator {
            "==" => Ok((left_value == right_value).to_string()),
            "!=" => Ok((left_value != right_value).to_string()),
            "+" => {
                // let left_value = self.evaluate(left)?;
                // let right_value = self.evaluate(right)?;

                if let (Ok(left_num), Ok(right_num)) = (left_value.parse::<f64>(), right_value.parse::<f64>()) {
                    let result = (left_num + right_num);
                    if result.is_nan() || result.is_infinite() {
                        panic!("Arithmetic overflow or invalid operation")
                    }
                    Ok(result.to_string())
                } else {
                    Ok(format!("{}{}", left_value, right_value))
                }
            },
            "-" => {
                // let left_value = self.evaluate(left)?;
                // let right_value = self.evaluate(right)?;

                let left_num = left_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для вычитания", left_value),
                        }.into()
                    )?;
                let right_num = right_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для вычитания", right_value),
                        }.into()
                    )?;
                let result = left_num - right_num;
                if result.is_nan() || result.is_infinite() {
                    panic!("Arithmetic overflow or invalid operation")
                }
                Ok(result.to_string())
            },
            "*" => {
                // let left_value = self.evaluate(left)?;
                // let right_value = self.evaluate(right)?;

                let left_num = left_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для умножения", left_value)
                        }.into()
                    )?;
                let right_num = right_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для умножения", right_value)
                        }.into()
                    )?;
                let result = left_num * right_num;
                if result.is_nan() || result.is_infinite() {
                    panic!("Arithmetic overflow or invalid operation")
                }
                Ok(result.to_string())
            },
            "/" => {
                // let left_value = self.evaluate(left)?;
                // let right_value = self.evaluate(right)?;

                let left_num = left_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для деления", left_value)
                        }.into()
                    )?;
                let right_num = right_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для деления", right_value)
                        }.into()
                    )?;
                if right_num == 0.0 {
                    return runtime_error!("Деление на ноль".to_string());
                }
                let result = left_num / right_num;
                if result.is_nan() || result.is_infinite() {
                    panic!("Arithmetic overflow or invalid operation")
                }

                Ok(result.to_string())
            },
            "^" => {
                // let left_value = self.evaluate(left)?;
                // let right_value = self.evaluate(right)?;

                let left_num = left_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для возведения в степень", left_value)
                        }.into()
                    )?;
                let right_num = right_value.parse::<f64>()
                    .map_err(|_|
                        Error {
                            kind: ErrorKind::Runtime,
                            line: None,
                            column: None,
                            message: format!("Невозможно преобразовать '{}' в число для возведения в степень", right_value)
                        }.into()
                    )?;
                let result = left_num.powf(right_num);
                if result.is_nan() || result.is_infinite() {
                    panic!("Arithmetic overflow or invalid operation")
                }

                Ok(result.to_string())
            },
            "&&" => {
                // let left_value = self.evaluate(left)?;

                if left_value == "false" || left_value == "0" || left_value == "" {
                    return Ok("false".to_string());
                }

                // let right_value = self.evaluate(right)?;

                Ok(if right_value != "false" && right_value != "0" && right_value != "" {
                    "true".to_string()
                } else {
                    "false".to_string()
                })
            },
            "||" => {
                // let left_value = self.evaluate(left)?;

                if left_value != "false" && left_value != "0" && left_value != "" {
                    return Ok("true".to_string());
                }

                // let right_value = self.evaluate(right)?;

                Ok(if right_value != "false" && right_value != "0" && right_value != "" {
                    "true".to_string()
                } else {
                    "false".to_string()
                })
            },
            _ => runtime_error!(format!("Неподдерживаемый бинарный оператор: {}", operator)),
        }
    }

    fn evaluate_property_access(&mut self, object: &Box<AstNode>, property: &str) -> Result<String> {
        let obj_value = self.evaluate(object)?;
        runtime_error!(format!("Доступ к свойству '{}.{}' не реализован", obj_value, property))
    }

    fn evaluate_function_call(
        &mut self,
        object: &Option<Box<AstNode>>,
        name: &str,
        args: &[Box<AstNode>],
        try_operator: bool,
        unwrap_operator: bool
    ) -> Result<String> {
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
                        "Вызов методов у не-идентификаторов ('{}') не поддерживается",
                        obj_value
                    ));
                }
            }
        } else {
            None
        };

        let result = match object_name.as_deref() {
            Some("Database") => self.call_database_method(name, &evaluated_args),
            Some("Response") => self.call_response_method(name, &evaluated_args),
            Some("Request") => self.call_request_method(name, &evaluated_args),
            Some("FileSystem") => self.call_filesystem_method(name, &evaluated_args),
            Some(plugin_name) if self.plugin_manager.has_plugin(plugin_name) => {
                self.plugin_manager.call_plugin_function(plugin_name, name, &evaluated_args)
            },
            None => self.call_global_function(name, &evaluated_args),
            Some(unknown) => runtime_error!(format!("Объект '{}' не найден", unknown)),
        };

        match result {
            Ok(value) => Ok(value),
            Err(err) => {
                if try_operator {
                    Err(err)
                } else if unwrap_operator {
                    error!("Оператор '!!' вызвал панику: {}", err);
                    panic!("Ошибка выполнения (unwrap !!): {}", err);
                } else {
                    Err(err)
                }
            }
        }
    }

    fn call_filesystem_method(&self, method: &str, args: &[String]) -> Result<String> {
        match method {
            "exists" => {
                if args.len() == 1 {
                    FileSystem::exists(&args[0]).map(|v| v.to_string())
                } else {
                    runtime_error!("Метод FileSystem.exists требует 1 аргумент")
                }
            },
            "read_text" => {
                if args.len() == 1 {
                    FileSystem::read_text(&args[0])
                } else {
                    runtime_error!("Метод FileSystem.read_text требует 1 аргумент")
                }
            },
            "write_text" => {
                if args.len() == 2 {
                    FileSystem::write_text(&args[0], &args[1]).map(|_| "OK".to_string())
                } else {
                    runtime_error!("Метод FileSystem.write_text требует 2 аргумента")
                }
            },
            "is_directory" => {
                if args.len() == 1 {
                    FileSystem::is_directory(&args[0]).map(|v| v.to_string())
                } else {
                    runtime_error!("Метод FileSystem.is_directory требует 1 аргумент")
                }
            },
            "list_files" => {
                if args.len() == 1 {
                    FileSystem::list_files(&args[0])
                } else {
                    runtime_error!("Метод FileSystem.list_files требует 1 аргумент")
                }
            },
            _ => runtime_error!(format!("Метод не найден: FileSystem.{}", method)),
        }
    }

    fn call_database_method(&self, method: &str, args: &[String]) -> Result<String> {
        match method {
            "get_all" => Database::get_all(),
            "check" => Database::check().map(|v| v.to_string()),
            "get" => {
                if args.len() == 1 {
                    Database::get(&args[0])
                } else {
                    runtime_error!("Метод Database.get требует 1 аргумент")
                }
            },
            "add" => {
                if args.len() >= 3 {
                    Database::add(&args[0], &args[1], &args[2]).map(|_| "OK".to_string())
                } else {
                    runtime_error!("Метод Database.add требует 3 аргумента")
                }
            },
            _ => runtime_error!(format!("Метод не найден: Database.{}", method)),
        }
    }

    fn call_response_method(&mut self, method: &str, args: &[String]) -> Result<String> {
        match method {
            "body" => {
                if args.len() == 1 {
                    self.response.body(&args[0]);
                    debug!("body in call_response_methods.body():\n{:?}\n", self.response.body.clone());
                    Ok(args[0].clone())
                } else {
                    runtime_error!("Метод Response.body требует 1 аргумент")
                }
            },
            "send" => {
                debug!("Enter in Response.send() method in call_response_method()\n{:?}\n", self.response.clone());
                self.response.send();
                Ok("".to_string())
            },
            "status" => {
                if args.len() == 1 {
                    if let Ok(status_code) = args[0].parse::<u16>() {
                        self.response.status(status_code);
                        Ok(status_code.to_string())
                    } else {
                        runtime_error!(format!("Неверный статус код: {}", args[0]))
                    }
                } else {
                    runtime_error!("Метод Response.status требует 1 аргумент")
                }
            },
            "headers" | "set_header" => {
                if args.len() == 2 {
                    self.response.set_header(&args[0], &args[1]);
                    Ok(format!("{}: {}", args[0], args[1]))
                } else {
                    runtime_error!("Метод Response.headers требует 2 аргумента")
                }
            },
            _ => runtime_error!(format!("Метод не найден: Response.{}", method)),
        }
    }

    fn call_request_method(&self, method: &str, args: &[String]) -> Result<String> {
        match method {
            "get_params" | "get_param" => {
                if args.len() == 1 {
                    Ok(self.request.get_param(&args[0]))
                } else {
                    runtime_error!("Метод Request.get_params требует 1 аргумент")
                }
            },
            "get_header" => {
                if args.len() == 1 {
                    Ok(self.request.get_header(&args[0]))
                } else {
                    runtime_error!("Метод Request.get_header требует 1 аргумент")
                }
            },
            "body" | "text_body" => {
                if args.is_empty() {
                    Ok(self.request.get_body())
                } else {
                    runtime_error!(format!("Метод Request.{} не принимает аргументы", method))
                }
            },
            "body_base64" => {
                if args.is_empty() {
                    Ok(self.request.get_body_as_base64())
                } else {
                    runtime_error!("Метод Request.body_base64 не принимает аргументы")
                }
            },
            "is_binary" => {
                if args.is_empty() {
                    Ok(self.request.is_body_binary().to_string())
                } else {
                    runtime_error!("Метод Request.is_binary не принимает аргументов")
                }
            },
            _ => runtime_error!(format!("Метод не найден: Request.{}", method)),
        }
    }

    fn call_global_function(&self, name: &str, args: &[String]) -> Result<String> {
        match name {
            "log_error" => {
                if args.len() == 1 {
                    error!("{}", args[0]);
                    Ok("".to_string())
                } else {
                    runtime_error!("Функция log_error требует 1 аргумент")
                }
            },
            "log_info" => {
                if args.len() == 1 {
                    info!("{}", args[0]);
                    Ok("".to_string())
                } else {
                    runtime_error!("Функция log_info требует 1 аргумент")
                }
            },
            "log_trace" => {
                if args.len() == 1 {
                    trace!("{}", args[0]);
                    Ok("".to_string())
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

    fn array_length(&self, array_str: &str) -> Result<String> {
        let array: Vec<serde_json::Value> = serde_json::from_str(array_str).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;
        Ok(array.len().to_string())
    }

    fn array_push(&self, array_str: &str, element: &str) -> Result<String> {
        let mut array: Vec<serde_json::Value> = serde_json::from_str(array_str).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Первый аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;

        if let Ok(num) = element.parse::<f64>() {
            array.push(serde_json::Value::Number(serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0))));
        } else if element == "true" {
            array.push(serde_json::Value::Bool(true));
        } else if element == "false" {
            array.push(serde_json::Value::Bool(false));
        } else {
            array.push(serde_json::Value::String(element.to_string()));
        }

        serde_json::to_string(&array).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации массива: {}", e),
            line: None,
            column: None,
        })
    }

    fn array_pop(&self, array_str: &str) -> Result<String> {
        let mut array: Vec<serde_json::Value> = serde_json::from_str(array_str).map_err(|_| Error {
            kind: ErrorKind::Runtime,
            message: "Аргумент не является массивом".to_string(),
            line: None,
            column: None,
        })?;

        if array.is_empty() {
            return runtime_error!("Невозможно удалить элемент из пустого массива");
        }

        array.pop();
        serde_json::to_string(&array).map_err(|e| Error {
            kind: ErrorKind::Runtime,
            message: format!("Ошибка сериализации массива: {}", e),
            line: None,
            column: None,
        })
    }

    fn array_contains(&self, array_str: &str, element: &str) -> Result<String> {
        let array: Vec<serde_json::Value> = serde_json::from_str(array_str).map_err(|_| Error {
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

            if item_str == element {
                return Ok("true".to_string());
            }
        }

        Ok("false".to_string())
    }

    fn array_join(&self, array_str: &str, separator: &str) -> Result<String> {
        let array: Vec<serde_json::Value> = serde_json::from_str(array_str).map_err(|_| Error {
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

        Ok(string_items.join(separator))
    }
}