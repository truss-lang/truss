use std::rc::Rc;

use super::{expression::Expression, node::GenericParameter};
use crate::lexer::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Rc<Expression>>,
        return_type: Option<Rc<Expression>>,
        body: Rc<Expression>,
    },
    ExpressionStatement {
        expression: Rc<Expression>,
    },
}
