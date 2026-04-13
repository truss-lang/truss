use super::{expression::Expression, node::GenericParameter};
use crate::lexer::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        token: Box<Token>,
        name: Box<Token>,
        parameters: Vec<Expression>,
        generic_parameters: Vec<GenericParameter>,
        body: Box<Expression>,
    },
    ExpressionStatement {
        expression: Box<Expression>,
    },
}
