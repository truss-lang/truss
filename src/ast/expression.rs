use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{
    lexer::token::{OperatorType, Token},
    symbol::Symbol,
    types::Type,
};

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
    Binary {
        left: Rc<RefCell<Expression>>,
        operator: BinaryOperator,
        right: Rc<RefCell<Expression>>,
    },
    Unary {
        expression: Rc<RefCell<Expression>>,
        operator: UnaryOperator,
        is_prefix: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOperator {
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulus,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Inc,
    Dec,
    NotNullAssertation,
}
impl BinaryOperator {
    pub fn from_operator(operator: OperatorType) -> Result<BinaryOperator> {
        match operator {
            OperatorType::Plus => Ok(BinaryOperator::Plus),
            OperatorType::Minus => Ok(BinaryOperator::Minus),
            OperatorType::Multiply => Ok(BinaryOperator::Multiply),
            OperatorType::Divide => Ok(BinaryOperator::Divide),
            OperatorType::Modulus => Ok(BinaryOperator::Modulus),
            OperatorType::Equal => Ok(BinaryOperator::Equal),
            OperatorType::NotEqual => Ok(BinaryOperator::NotEqual),
            OperatorType::Less => Ok(BinaryOperator::Less),
            OperatorType::LessEqual => Ok(BinaryOperator::LessEqual),
            OperatorType::Greater => Ok(BinaryOperator::Greater),
            OperatorType::GreaterEqual => Ok(BinaryOperator::GreaterEqual),
            OperatorType::And => Ok(BinaryOperator::And),
            OperatorType::Or => Ok(BinaryOperator::Or),
            _ => Err(anyhow!("Not a binary operator")),
        }
    }
}
impl UnaryOperator {
    pub fn from_operator(operator: OperatorType) -> Result<UnaryOperator> {
        match operator {
            OperatorType::Plus => Ok(UnaryOperator::Plus),
            OperatorType::Minus => Ok(UnaryOperator::Minus),
            OperatorType::Inc => Ok(UnaryOperator::Inc),
            OperatorType::Dec => Ok(UnaryOperator::Dec),
            _ => Err(anyhow!("Not a unary operator")),
        }
    }
}
