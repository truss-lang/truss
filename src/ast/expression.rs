use std::{cell::RefCell, rc::Rc};

use crate::{lexer::token::Token, types::Type};

use super::statement::Statement;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Block {
        statements: Vec<Rc<RefCell<Statement>>>,
    },
    IntegerLiteral {
        token: Token,
    },
    DecimalLiteral {
        token: Token,
    },
    BooleanLiteral {
        token: Token,
    },
    NullLiteral {
        token: Token,
    },
    NullptrLiteral {
        token: Token,
    },
    Type {
        name: Token,
        generic_parameters: Vec<Rc<RefCell<Expression>>>,
        ty: Option<Type>,
    },
}
