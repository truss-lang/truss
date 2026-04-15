use std::{cell::RefCell, rc::Rc};

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
    Assignment {
        left: Rc<RefCell<Expression>>,
        operator: AssignmentOperator,
        right: Rc<RefCell<Expression>>,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssignmentOperator {
    Assign,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
    BitAndAssign,
    BitOrAssign,
    LeftShiftAssign,
    RightShiftAssign,
}
impl BinaryOperator {
    pub fn from_operator(operator: OperatorType) -> Option<BinaryOperator> {
        match operator {
            OperatorType::Plus => Some(BinaryOperator::Plus),
            OperatorType::Minus => Some(BinaryOperator::Minus),
            OperatorType::Multiply => Some(BinaryOperator::Multiply),
            OperatorType::Divide => Some(BinaryOperator::Divide),
            OperatorType::Modulus => Some(BinaryOperator::Modulus),
            OperatorType::Equal => Some(BinaryOperator::Equal),
            OperatorType::NotEqual => Some(BinaryOperator::NotEqual),
            OperatorType::Less => Some(BinaryOperator::Less),
            OperatorType::LessEqual => Some(BinaryOperator::LessEqual),
            OperatorType::Greater => Some(BinaryOperator::Greater),
            OperatorType::GreaterEqual => Some(BinaryOperator::GreaterEqual),
            OperatorType::And => Some(BinaryOperator::And),
            OperatorType::Or => Some(BinaryOperator::Or),
            _ => None,
        }
    }
}
impl UnaryOperator {
    pub fn from_operator(operator: OperatorType) -> Option<UnaryOperator> {
        match operator {
            OperatorType::Plus => Some(UnaryOperator::Plus),
            OperatorType::Minus => Some(UnaryOperator::Minus),
            OperatorType::Inc => Some(UnaryOperator::Inc),
            OperatorType::Dec => Some(UnaryOperator::Dec),
            _ => None,
        }
    }
}
impl AssignmentOperator {
    pub fn from_operator(operator: OperatorType) -> Option<AssignmentOperator> {
        match operator {
            OperatorType::Assign => Some(AssignmentOperator::Assign),
            OperatorType::PlusAssign => Some(AssignmentOperator::PlusAssign),
            OperatorType::MinusAssign => Some(AssignmentOperator::MinusAssign),
            OperatorType::MultiplyAssign => Some(AssignmentOperator::MultiplyAssign),
            OperatorType::DivideAssign => Some(AssignmentOperator::DivideAssign),
            OperatorType::ModulusAssign => Some(AssignmentOperator::ModulusAssign),
            OperatorType::BitAndAssign => Some(AssignmentOperator::BitAndAssign),
            OperatorType::BitOrAssign => Some(AssignmentOperator::BitOrAssign),
            OperatorType::LeftShiftAssign => Some(AssignmentOperator::LeftShiftAssign),
            OperatorType::RightShiftAssign => Some(AssignmentOperator::RightShiftAssign),
            _ => None,
        }
    }
}
