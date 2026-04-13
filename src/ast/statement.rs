use std::{cell::RefCell, rc::Rc};

use super::{expression::Expression, node::GenericParameter};
use crate::lexer::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Rc<RefCell<Expression>>>,
        return_type: Option<Rc<RefCell<Expression>>>,
        body: Rc<RefCell<Expression>>,
    },
    ExpressionStatement {
        expression: Rc<RefCell<Expression>>,
    },
}
