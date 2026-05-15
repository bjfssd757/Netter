use log::{debug, error, trace, warn};
use crate::language::ast::AstNode;
use crate::language::error::{Result, Error, ErrorKind};
use crate::language::interpreter::evaluator::Evaluator;
use crate::language::rdl_types::RDLTypes;
use crate::runtime_error;
use super::context::ExecutionContext;
use super::builtin::request::Request;
use super::builtin::response::Response;
use super::builtin::plugin::PluginManager;
use super::ErrorHandler;

#[derive(Debug, Clone)]
pub struct RouteHandler {
    pub(crate) actions: Vec<Box<AstNode>>,
    pub(crate) error_handler: Option<ErrorHandler>,
}

impl RouteHandler {
    pub fn new(actions: Vec<Box<AstNode>>, error_handler: Option<ErrorHandler>) -> Self {
        RouteHandler {
            actions,
            error_handler,
        }
    }

    pub fn execute(
        &self,
        request: &mut Request,
        response: &mut Response,
        plugin_manager: &PluginManager,
        global_error_handler: Option<&ErrorHandler>,
    // ) -> Response {
    ) {
        let mut context = ExecutionContext::new();
        let mut error: Option<String> = None;

        debug!("Начало выполнения маршрута. Действий: {}", self.actions.len());

        for (index, action) in self.actions.iter().enumerate() {
            trace!("Выполнение действия {}: {:?}", index, action);

            if let Err(err) = self.execute_action(action, request, response, &mut context, plugin_manager) {
                error = Some(err.message);
                debug!("Ошибка при выполнении действия {}: {}", index, error.as_ref().unwrap());
                break;
            }

            if response.is_sent() {
                debug!("Ответ отправлен действием {}, прерывание.", index);
                break;
            }
        }

        if let Some(err_msg) = error {
            warn!("Произошла ошибка при выполнении маршрута: {}", err_msg);
            let mut error_handled = false;

            if let Some(handler) = &self.error_handler {
                debug!("Использование локального обработчика ошибок для переменной '{}'", handler.error_var);
                let mut err_context = context.clone();
                err_context.set_variable(&handler.clone().error_var.into(), err_msg.clone().into());

                for (index, action) in handler.actions.iter().enumerate() {
                    trace!("Выполнение действия обработчика {}: {:?}", index, action);

                    if let Err(e) = self.execute_action(action, request, response, &mut err_context, plugin_manager) {
                        error!("Ошибка внутри локального обработчика ошибок: {}", e.message);
                    }

                    if response.is_sent() {
                        debug!("Ответ отправлен действием обработчика {}, прерывание.", index);
                        error_handled = true;
                        break;
                    }
                }

                if !response.is_sent() {
                    error_handled = true;
                    debug!("Локальный обработчик ошибок завершился без отправки ответа.");
                }
            }

            if !error_handled && global_error_handler.is_some() {
                let handler = global_error_handler.unwrap();
                debug!("Использование глобального обработчика ошибок для переменной '{}'", handler.error_var);
                let mut err_context = context.clone();
                err_context.set_variable(&handler.clone().error_var.into(), err_msg.clone().into());

                for (index, action) in handler.actions.iter().enumerate() {
                    trace!("Выполнение действия глоб. обработчика {}: {:?}", index, action);

                    if let Err(e) = self.execute_action(action, request, response, &mut err_context, plugin_manager) {
                        error!("Ошибка внутри глобального обработчика ошибок: {}", e.message);
                    }

                    if response.is_sent() {
                        debug!("Ответ отправлен действием глоб. обработчика {}, прерывание.", index);
                        error_handled = true;
                        break;
                    }
                }

                if !response.is_sent() {
                    error_handled = true;
                    debug!("Глобальный обработчик ошибок завершился без отправки ответа.");
                }
            }

            if !error_handled && !response.is_sent() {
                error!("Ошибка не обработана ни локальным, ни глобальным обработчиком. Отправка 500.");
                response.status(500);
                response.body(&format!("Internal Server Error: {}", err_msg));
                response.send();
            }
        } else {
            debug!("Выполнение маршрута завершено успешно.");
        }

        // response.clone()
    }

    fn execute_action(
        &self,
        action: &AstNode,
        request: &mut Request,
        response: &mut Response,
        context: &mut ExecutionContext,
        plugin_manager: &PluginManager,
    ) -> Result<()> {
        if response.is_sent() {
            return Ok(());
        }

        let mut req = request.clone();
        let mut res = response.clone();
        let mut con = context.clone();

        let mut evaluator = Evaluator::new(&mut con, &mut req, &mut res, plugin_manager);

        let result = match action {
            AstNode::VarDeclaration { name, value } => {
                trace!("Объявление переменной '{}'", name);
                let value_str = evaluator.evaluate(value)?;
                debug!("Установка переменной '{}' = '{}'", name, value_str);
                con.set_variable(&RDLTypes::String(name.clone()), value_str);
                Ok(())
            },
            AstNode::FunctionCall { object, name, args, try_operator, unwrap_operator } => {
                trace!("Вызов функции/метода: {}.{}",
                    object.as_ref().map_or("<global>", |o| match &**o {
                        AstNode::Identifier(id) => id,
                        _ => "<expression>"
                    }),
                    name
                );

                let result = evaluator.evaluate(action);

                match result {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        if *try_operator {
                            debug!("Оператор '?' перехватил ошибку: {}", e.message);
                            Err(e)
                        } else if *unwrap_operator {
                            error!("Оператор '!!' вызвал панику: {}", e.message);
                            panic!("Ошибка выполнения (unwrap !!): {}", e.message);
                        } else {
                            error!("Неперехваченная ошибка: {}", e.message);
                            Err(e)
                        }
                    }
                }
            },
            AstNode::IfStatement { condition, then_branch, else_branch } => {
                trace!("Проверка условия: {:?}", condition);
                let condition_value = evaluator.evaluate(condition)?;
                debug!("Результат условия: {}", condition_value);

                if condition_value == true.into() {
                    debug!("Выполнение ветки 'then'");

                    match &**then_branch {
                        AstNode::Block(statements) => {
                            for stmt in statements {
                                if res.is_sent() {
                                    return Ok(());
                                }

                                if let Err(e) = self.execute_action(stmt, &mut req, &mut res, &mut con, plugin_manager) {
                                    return Err(e);
                                }
                            }
                        },
                        _ => {
                            if let Err(e) = self.execute_action(then_branch, &mut req, &mut res, &mut con, plugin_manager) {
                                return Err(e);
                            }
                        }
                    }
                } else if let Some(else_actions) = else_branch {
                    debug!("Выполнение ветки 'else'");

                    match &**else_actions {
                        AstNode::Block(statements) => {
                            for stmt in statements {
                                if res.is_sent() {
                                    return Ok(());
                                }

                                if let Err(e) = self.execute_action(stmt, &mut req, &mut res, &mut con, plugin_manager) {
                                    return Err(e);
                                }
                            }
                        },
                        _ => {
                            if let Err(e) = self.execute_action(else_actions, &mut req, &mut res, &mut con, plugin_manager) {
                                return Err(e);
                            }
                        }
                    }
                }

                Ok(())
            },
            // FOR +=
            AstNode::BinaryOp { left, operator, right } if operator == "+=" => {
                if let AstNode::Identifier(var_name) = &**left {
                    let right_value = evaluator.evaluate(right)?;

                    if let Some(current_value) = con.get_variable(&var_name.clone().into()) {
                        if let (Ok(left_num), Ok(right_num)) = (current_value.clone().try_into() as Result<i64>, right_value.clone().try_into() as Result<i64>) {
                            let new_value = (left_num + right_num).to_string();
                            trace!("Обновление переменной '{}' += {} -> {}", var_name, right_value, new_value);
                            con.set_variable(&var_name.clone().into(), new_value.into());
                        } else {
                            let new_value = format!("{}{}", current_value, right_value);
                            trace!("Обновление переменной '{}' += '{}' -> '{}'", var_name, right_value, new_value);
                            con.set_variable(&var_name.clone().into(), new_value.into());
                        }
                        Ok(())
                    } else {
                        runtime_error!(format!("Переменная '{}' для '+=' не найдена", var_name))
                    }
                } else {
                    runtime_error!("Оператор '+=' может использоваться только с идентификатором переменной слева")
                }
            },

            // FOR -=
            AstNode::BinaryOp { left, operator, right } if operator == "-=" => {
                if let AstNode::Identifier(var_name) = &**left {
                    let right_value = evaluator.evaluate(right)?;

                    if let Some(current_value) = con.get_variable(&var_name.clone().into()) {
                        let left_num = (current_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для вычитания", current_value),
                                line: None,
                                column: None,
                            })?;
                        let right_num = (right_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для вычитания", right_value),
                                line: None,
                                column: None,
                            })?;

                        let new_value = (left_num - right_num).to_string();
                        trace!("Обновление переменной '{}' -= {} -> {}", var_name, right_value, new_value);
                        con.set_variable(&var_name.clone().into(), new_value.into());
                        Ok(())
                    } else {
                        runtime_error!(format!("Переменная '{}' для '-=' не найдена", var_name))
                    }
                } else {
                    runtime_error!("Оператор '-=' может использоваться только с идентификатором переменной слева")
                }
            },

            // FOR *=
            AstNode::BinaryOp { left, operator, right } if operator == "*=" => {
                if let AstNode::Identifier(var_name) = &**left {
                    let right_value = evaluator.evaluate(right)?;

                    if let Some(current_value) = con.get_variable(&var_name.clone().into()) {
                        let left_num = (current_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для умножения", current_value),
                                line: None,
                                column: None,
                            })?;
                        let right_num = (right_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для умножения", right_value),
                                line: None,
                                column: None,
                            })?;

                        let new_value = (left_num * right_num).to_string();
                        trace!("Обновление переменной '{}' *= {} -> {}", var_name, right_value, new_value);
                        con.set_variable(&var_name.clone().into(), new_value.into());
                        Ok(())
                    } else {
                        runtime_error!(format!("Переменная '{}' для '*=' не найдена", var_name))
                    }
                } else {
                    runtime_error!("Оператор '*=' может использоваться только с идентификатором переменной слева")
                }
            },

            // FOR /=
            AstNode::BinaryOp { left, operator, right } if operator == "/=" => {
                if let AstNode::Identifier(var_name) = &**left {
                    let right_value = evaluator.evaluate(right)?;

                    if let Some(current_value) = con.get_variable(&var_name.clone().into()) {
                        let left_num = (current_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для деления", current_value),
                                line: None,
                                column: None,
                            })?;
                        let right_num = (right_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для деления", right_value),
                                line: None,
                                column: None,
                            })?;

                        if right_num == (RDLTypes::Number(0).try_into() as Result<i64>)? {
                            return runtime_error!("Деление на ноль");
                        }

                        let new_value = (left_num / right_num).to_string();
                        trace!("Обновление переменной '{}' /= {} -> {}", var_name, right_value, new_value);
                        con.set_variable(&var_name.clone().into(), new_value.into());
                        Ok(())
                    } else {
                        runtime_error!(format!("Переменная '{}' для '/=' не найдена", var_name))
                    }
                } else {
                    runtime_error!("Оператор '/=' может использоваться только с идентификатором переменной слева")
                }
            },

            // FOR ^=
            AstNode::BinaryOp { left, operator, right } if operator == "^=" => {
                if let AstNode::Identifier(var_name) = &**left {
                    let right_value = evaluator.evaluate(right)?;

                    if let Some(current_value) = con.get_variable(&var_name.clone().into()) {
                        let left_num = (current_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для возведения в степень", current_value),
                                line: None,
                                column: None,
                            })?;
                        let right_num = (right_value.clone().try_into() as Result<i64>)
                            .map_err(|_| Error {
                                kind: ErrorKind::Runtime,
                                message: format!("Невозможно преобразовать '{}' в число для возведения в степень", right_value),
                                line: None,
                                column: None,
                            })?;

                        let new_value = crate::utils::powi(left_num, right_num);
                        trace!("Обновление переменной '{}' ^= {} -> {}", var_name, right_value, new_value);
                        con.set_variable(&var_name.clone().into(), new_value.into());
                        Ok(())
                    } else {
                        runtime_error!(format!("Переменная '{}' для '^=' не найдена", var_name))
                    }
                } else {
                    runtime_error!("Оператор '^=' может использоваться только с идентификатором переменной слева")
                }
            },
            AstNode::WhileLoop { condition, body } => {
                trace!("Начало выполнения цикла while");

                loop {
                    let mut temp_evaluator = Evaluator::new(
                        &mut con, &mut req, &mut res, plugin_manager
                    );
                    let condition_value = temp_evaluator.evaluate(condition)?;

                    if condition_value == RDLTypes::Boolean(true) {
                        match &**body {
                            AstNode::Block(statements) => {
                                for stmt in statements {
                                    if res.is_sent() {
                                        return Ok(());
                                    }

                                    if let Err(e) = self.execute_action(stmt, &mut req, &mut res, &mut con, plugin_manager) {
                                        return Err(e);
                                    }
                                }
                            },
                            _ => {
                                if let Err(e) = self.execute_action(body, &mut req, &mut res, &mut con, plugin_manager) {
                                    return Err(e);
                                }
                            }
                        }
                    } else {
                        break;
                    }
                }

                Ok(())
            },

            AstNode::ForLoop { var_name, iterable, body } => {
                trace!("Начало выполнения цикла for по {}", var_name);

                let iterable_value = evaluator.evaluate(iterable).inspect_err(
                    |e| error!("Failed on evaluate: {e}")
                )?;

                trace!("iterable_value = {}", iterable_value);
                
                let items: Vec<RDLTypes> = match serde_json::from_str::<Vec<serde_json::Value>>(iterable_value.to_string().as_str()) {
                    Ok(json_array) => {
                        json_array.iter().map(|v| match v {
                            serde_json::Value::String(s) => RDLTypes::String(s.clone()),
                            serde_json::Value::Number(n) => {
                                let i = n.as_i64();
                                if let Some(num) = i {
                                    trace!("SUCCESS: {num}");
                                    RDLTypes::Number(num)
                                } else {
                                    error!("Failed to convert to i64!\n\nn = {n}\n\n");
                                    panic!();
                                }
                            },
                            serde_json::Value::Bool(b) => RDLTypes::Boolean(*b),
                            serde_json::Value::Null => RDLTypes::String("null".to_string()),
                            _ => RDLTypes::String(v.to_string()),
                        }).collect()
                    },
                    Err(_) => {
                        if iterable_value.to_string().contains(',') {
                            iterable_value.to_string().split(',').map(RDLTypes::from).collect()
                        } else {
                            vec![iterable_value]
                        }
                    }
                };

                trace!("Цикл for будет выполнен {} раз", items.len());

                for (index, item) in items.iter().enumerate() {
                    trace!("Итерация for #{}: {}", index, item);

                    con.set_variable(&var_name.clone().into(), item.clone());

                    match &**body {
                        AstNode::Block(statements) => {
                            for stmt in statements {
                                if res.is_sent() {
                                    return Ok(());
                                }

                                if let Err(e) = self.execute_action(stmt, &mut req, &mut res, &mut con, plugin_manager) {
                                    return Err(e);
                                }
                            }
                        },
                        _ => {
                            if let Err(e) = self.execute_action(body, &mut req, &mut res, &mut con, plugin_manager) {
                                return Err(e);
                            }
                        }
                    }
                }

                Ok(())
            },
            _ => {
                match evaluator.evaluate(action) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
        };

        *request = req;
        *response = res;
        *context = con;

        result
    }
}