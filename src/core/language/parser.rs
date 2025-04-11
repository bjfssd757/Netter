use pest::Parser;
use pest_derive::Parser;
use crate::core::language::{ operators, operators::{
    Operator,
    Expression,
    BinaryOperatorExpression,
} };

#[derive(Parser)]
#[grammar = "src/core/language/grammar.pest"]
pub struct RouteParser;


#[derive(Debug)]
struct RouteDefinition {
    path: String,
    method: String,
    block: Block,
}

#[derive(Debug)]
struct Block {
    statements: Vec<Statement>,
}

#[derive(Debug)]
enum Statement {
    // ... другие типы операторов
    Assignment(operators::AssignmentStatement),
    Empty,
}

fn process_route_definition(route_def: pest::iterators::Pair<Rule>) -> RouteDefinition {
    let mut path = String::new();
    let mut method = String::new();
    let mut block = Block { statements: Vec::new() };

    for inner_pair in route_def.into_inner() {
        match inner_pair.as_rule() {
            Rule::path => path = inner_pair.as_str().to_string(),
            Rule::method => method = inner_pair.as_str().to_string(),
            Rule::block => block = process_block(inner_pair),
            _ => {}
        }
    }

    RouteDefinition { path, method, block }
}

fn process_block(block: pest::iterators::Pair<Rule>) -> Block {
    let mut statements = Vec::new();
    for statement in block.into_inner() {
        match statement.as_rule() {
            Rule::statements => {
                for s in statement.into_inner() {
                    statements.push(process_statement(s));
                }
            }
            _ => {}
        }
    }
    Block { statements }
}

fn process_expression(expression: pest::iterators::Pair<Rule>) -> operators::Expression {
    match expression.as_rule() {
        Rule::number => Expression::Number(expression.as_str().parse().unwrap()),
        Rule::string => Expression::String(expression.as_str().to_string()),
        Rule::identifier => Expression::Identifier(expression.as_str().to_string()),
        Rule::function_call => {
            let mut inner = expression.into_inner();
            let function_name = inner.next().unwrap().as_str().to_string();
            let mut arguments = Vec::new();
            for arg in inner {
                arguments.push(process_expression(arg));
            }
            Expression::FunctionCall(operators::FunctionCall {
                function_name,
                arguments,
            })
        }
        Rule::binary_operator => {
            let mut inner = expression.into_inner();
            let left = Box::new(process_expression(inner.next().unwrap()));
            let operator = process_operator(inner.next().unwrap());
            let right = Box::new(process_expression(inner.next().unwrap()));
            Expression::BinaryOperator(BinaryOperatorExpression {
                operator,
                left,
                right,
            })
        }
        // ...
        _ => panic!("Unknown expression type"),
    }
}

fn process_operator(operator: pest::iterators::Pair<Rule>) -> Operator {
    match operator.as_rule() {
        Rule::add => Operator::Add,
        Rule::subtract => Operator::Subtract,
        Rule::multiply => Operator::Multiply,
        Rule::divide => Operator::Divide,
        Rule::equals => Operator::Equals,
        // ...
        _ => panic!("Unknown operator type"),
    }
}

fn process_statement(statement: pest::iterators::Pair<Rule>) -> Statement {
    match statement.as_rule() {
        Rule::assign_statement => { // Если правило - оператор присваивания
            let mut inner = statement.into_inner(); // Получаем дочерние элементы
            let identifier = inner.next().unwrap().as_str().to_string(); // Получаем имя переменной
            let expression = process_expression(inner.next().unwrap()); // Получаем выражение
            Statement::Assignment(operators::AssignmentStatement { // Создаем структуру AssignmentStatement
                variable_name: identifier, // Заполняем поле variable_name
                expression, // Заполняем поле expression
            })
        }
        // ... другие типы операторов
        _ => Statement::Empty, // Если правило не соответствует ни одному из известных, возвращаем пустой оператор
    }
}

fn main() {
    let input = "route \"/users/{id}\" GET { }"; // Пример строки роута

    match RouteParser::parse(Rule::route_definition, input) {
        Ok(mut pairs) => {
            let route_def = process_route_definition(pairs.next().unwrap());
            println!("AST: {:?}", route_def);
        }
        Err(e) => {
            eprintln!("Ошибка парсинга: {:?}", e);
        }
    }
}