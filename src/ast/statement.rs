use super::{expression::Expression, node::GenericParameter};
use crate::lexer::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        token: Token,
        name: Token,
        parameters: Vec<Expression>,
        generic_parameters: Vec<GenericParameter>,
    },
}
