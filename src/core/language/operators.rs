#[derive(Debug)]
pub struct AssignmentStatement {
    pub variable_name: String,
    pub expression: Expression,
}

#[derive(Debug)]
pub enum Expression {
    Float(f64),
    Number(i64),
    String(String),
    Boolean(bool),
    Identifier(String),
    FunctionCall(FunctionCall),
    BinaryOperator(BinaryOperatorExpression),
    UnaryOperator(UnaryOperatorExpression),
    FieldAccess(FieldAccessExpression),
    ArrayIndex(ArrayIndexExpression),
}

#[derive(Debug)]
pub enum Operator {
    Add,                // +
    Subtract,           // -
    Multiply,           // *
    Divide,             // /
    Power,              // ^
    Equals,             // ==
    NotEquals,          // !=
    GreaterThan,        // >
    LessThan,           // <
    And,                // &&
    Or,                 // ||
    Not,                // !
}

#[derive(Debug)]
pub struct ArrayIndexExpression {
    pub target: Box<Expression>,        // Массив
    pub index: Box<Expression>,         // Индекс
}

#[derive(Debug)]
pub struct FieldAccessExpression {
    pub target: Box<Expression>,    // Объект
    pub field_name: String,         // Имя поля
}

#[derive(Debug)]
pub struct BinaryOperatorExpression {
    pub operator: Operator, // Тип оператора
    pub left: Box<Expression>, // Левый операнд
    pub right: Box<Expression>, // Правый операнд
}

#[derive(Debug)]
pub struct UnaryOperatorExpression {
    pub operator: Operator, // Тип оператора
    pub expression: Box<Expression>, // Операнд
}

#[derive(Debug)]
pub struct FunctionCall {
    pub function_name: String,
    pub arguments: Vec<Expression>,
}