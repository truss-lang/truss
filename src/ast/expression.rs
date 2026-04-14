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
    Variable {
        name: Option<Box<Token>>,
        expression: Option<Rc<RefCell<Expression>>>,
        ty: Option<Rc<RefCell<Type>>>,
        symbol: Option<Rc<Symbol>>,
    },
    Type {
        name: Box<Token>,
        generic_parameters: Vec<Rc<RefCell<Expression>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
}
