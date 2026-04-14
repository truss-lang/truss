use std::{cell::RefCell, rc::Rc};

use super::{expression::Expression, node::GenericParameter};
use crate::{lexer::token::Token, types::Type};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Rc<RefCell<Parameter>>>,
        return_type: Option<Rc<RefCell<Expression>>>,
        body: Rc<RefCell<Expression>>,
    },
    VariableDecl {
        token: Box<Token>,
        name: Box<Token>,
        type_expression: Option<Rc<RefCell<Expression>>>,
        initializer: Option<Rc<RefCell<Expression>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    ExpressionStatement {
        expression: Rc<RefCell<Expression>>,
    },
    EmptyStatement {
        token: Box<Token>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: Box<Token>,
    pub type_expression: Option<Rc<RefCell<Expression>>>,
    pub ty: Option<Rc<RefCell<Type>>>,
}
