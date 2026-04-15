use std::{cell::RefCell, rc::Rc};

use crate::{lexer::token::Token, symbol::Symbol, types::Type};

use super::statement::Statement;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Block {
        statements: Vec<Rc<RefCell<Statement>>>,
    },
    IntegerLiteral {
        token: Box<Token>,
    },
    DecimalLiteral {
        token: Box<Token>,
    },
    BooleanLiteral {
        token: Box<Token>,
    },
    NullLiteral {
        token: Box<Token>,
    },
    NullptrLiteral {
        token: Box<Token>,
    },
    UnitLiteral {
        left: Box<Token>,
        right: Box<Token>,
    },
    Variable {
        name: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
        symbol: Option<Rc<Symbol>>,
    },
    Type {
        name: Box<Token>,
        type_parameters: Option<Vec<Rc<RefCell<Expression>>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Call {
        callee: Rc<RefCell<Expression>>,
        type_parameters: Option<Vec<Rc<RefCell<Expression>>>>,
        parameters: Vec<Rc<RefCell<Expression>>>,
    },
}
