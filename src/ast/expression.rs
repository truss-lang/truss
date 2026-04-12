use super::statement::Statement;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Block { statements: Vec<Statement> },
}
