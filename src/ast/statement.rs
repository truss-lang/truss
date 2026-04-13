use super::{expression::Expression, node::GenericParameter};
use crate::lexer::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Expression>,
        return_type: Option<Expression>,
        body: Box<Expression>,
    },
    ExpressionStatement {
        expression: Box<Expression>,
    },
}
