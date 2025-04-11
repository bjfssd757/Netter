use pest::Parser;
use pest_derive::Parser;
use crate::core::language::{ operators, operators::{
    Operator,
    Expression,
    BinaryOperatorExpression,
    FunctionCall,
    AssignmentStatement,
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
    Assignment(operators::AssignmentStatement),
    ExpressionStmt(operators::Expression), // Добавлено для обработки expression_statement
    Empty,
}

fn process_route_definition(route_def: pest::iterators::Pair<Rule>) -> RouteDefinition {
    println!("Enter in process_route_definition");
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
    println!("Exit from process_route_definition");

    RouteDefinition { path, method, block }
}

fn process_block(block: pest::iterators::Pair<Rule>) -> Block {
    println!("Enter in process_block");
    let mut statements = Vec::new();
    for statement in block.into_inner() {
        match statement.as_rule() {
            Rule::statements => {
                println!("Enter in process_block -> statements");
                for s in statement.into_inner() {
                    statements.push(process_statement(s));
                }
            }
            _ => {}
        }
    }
    println!("Exit from process_block");
    Block { statements }
}

fn process_expression(expression: pest::iterators::Pair<Rule>) -> operators::Expression {
    println!("Enter in process_expression");
    let mut inner = expression.into_inner();
    let mut current_expression = process_term(inner.next().unwrap());

    while let Some(pair) = inner.next() {
        match pair.as_rule() {
            Rule::add => {
                println!("Enter in process_expression -> add");
                let next_term = process_term(inner.next().unwrap());
                current_expression = Expression::BinaryOperator(BinaryOperatorExpression {
                    operator: Operator::Add,
                    left: Box::new(current_expression),
                    right: Box::new(next_term),
                });
            }
            Rule::subtract => {
                println!("Enter in process_expression -> subtract");
                let next_term = process_term(inner.next().unwrap());
                current_expression = Expression::BinaryOperator(BinaryOperatorExpression {
                    operator: Operator::Subtract,
                    left: Box::new(current_expression),
                    right: Box::new(next_term),
                });
            }
            _ => panic!("Unexpected rule within expression: {:?}", pair.as_rule()),
        }
    }

    println!("Exit from process_expression");

    current_expression
}

fn process_term(term: pest::iterators::Pair<Rule>) -> operators::Expression {
    println!("Enter in process_term");
    let mut inner = term.into_inner();
    let mut current_expression = process_power(inner.next().unwrap());

    while let Some(pair) = inner.next() {
        match pair.as_rule() {
            Rule::multiply => {
                println!("Enter in process_term -> multiply");
                let next_power = process_power(inner.next().unwrap());
                current_expression = Expression::BinaryOperator(BinaryOperatorExpression {
                    operator: Operator::Multiply,
                    left: Box::new(current_expression),
                    right: Box::new(next_power),
                });
            }
            Rule::divide => {
                println!("Enter in process_term -> divide");
                let next_power = process_power(inner.next().unwrap());
                current_expression = Expression::BinaryOperator(BinaryOperatorExpression {
                    operator: Operator::Divide,
                    left: Box::new(current_expression),
                    right: Box::new(next_power),
                });
            }
            _ => panic!("Unexpected rule within term: {:?}", pair.as_rule()),
        }
    }

    println!("Exit from process_term");

    current_expression
}

fn process_power(power: pest::iterators::Pair<Rule>) -> operators::Expression {
    println!("Enter in process_power");
    let mut inner = power.into_inner();
    let mut current_expression = process_factor(inner.next().unwrap());

    while let Some(pair) = inner.next() {
        match pair.as_rule() {
            Rule::power_op => {
                println!("Enter in process_power -> power_op");
                let next_factor = process_factor(inner.next().unwrap());
                current_expression = Expression::BinaryOperator(BinaryOperatorExpression {
                    operator: Operator::Power,
                    left: Box::new(current_expression),
                    right: Box::new(next_factor),
                });
            }
            _ => panic!("Unexpected rule within power: {:?}", pair.as_rule()),
        }
    }
    println!("Exit from process_power");

    current_expression
}

fn process_factor(factor: pest::iterators::Pair<Rule>) -> operators::Expression {
    println!("Enter in process_factor");
    let inner = factor.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::number => Expression::Number(inner.as_str().parse().unwrap()),
        Rule::string => Expression::String(inner.as_str().to_string()),
        Rule::identifier => Expression::Identifier(inner.as_str().to_string()),
        Rule::function_call => {
            let mut inner_pairs = inner.into_inner();
            let function_name = inner_pairs.next().unwrap().as_str().to_string();
            let mut arguments = Vec::new();
            for arg in inner_pairs {
                arguments.push(process_expression(arg));
            }
            Expression::FunctionCall(FunctionCall {
                function_name,
                arguments,
            })
        }
        Rule::expression => process_expression(inner),
        _ => panic!("Unexpected rule within factor: {:?}", inner.as_rule()),
    }
}

fn process_statement(statement: pest::iterators::Pair<Rule>) -> Statement {
    println!("Enter in process_statement: {:?}", statement.as_rule());
    match statement.as_rule() {
        Rule::assign_statement => {
            println!("Enter in process_statement -> assign_statement");
            let mut inner = statement.into_inner();
            let identifier = inner.next().unwrap().as_str().to_string();
            let expression = process_expression(inner.next().unwrap());
            Statement::Assignment(AssignmentStatement {
                variable_name: identifier,
                expression,
            })
        }
        Rule::expression_statement => {
            println!("Enter in process_statement -> expression_statement");
            let expression = process_expression(statement.into_inner().next().unwrap());
            Statement::ExpressionStmt(expression)
        }
        _ => {
            println!("Enter in process_statement -> Empty");
            Statement::Empty
        }
    }
}

pub fn start(path: String) {
    let file_content = match std::fs::read_to_string(path) {
        Ok(content) => {
            println!("File content: {}", content);
            content
        },
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            return;
        }
    };

    match RouteParser::parse(Rule::route_definition, &file_content) {
        Ok(mut pairs) => {
            println!("Enter in Ok() block of RouteParser::parse");
            let route_def = process_route_definition(pairs.next().unwrap());
            println!("AST: {:?}", route_def);
        }
        Err(e) => {
            eprintln!("Ошибка парсинга: {:?}", e);
        }
    }
}