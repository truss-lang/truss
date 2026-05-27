use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{
    lexer::token::{OperatorType, Position, Token, TokenType},
    scope::Scope,
    symbol::WeakSymbol,
    types::Type,
};

use super::statement::Statement;

#[derive(Debug, Clone, PartialEq)]
pub struct CallParameter {
    pub label: Option<Box<Token>>,
    pub expression: Rc<RefCell<Expression>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Block {
        statements: Vec<Rc<RefCell<Statement>>>,
        scope: Option<Rc<RefCell<Scope>>>,
    },
    IntegerLiteral {
        token: Box<Token>,
        value: i128,
        ty: Option<Rc<RefCell<Type>>>,
    },
    DecimalLiteral {
        token: Box<Token>,
        value: f64,
        ty: Option<Rc<RefCell<Type>>>,
    },
    BooleanLiteral {
        token: Box<Token>,
    },
    NullLiteral {
        token: Box<Token>,
    },
    NullptrLiteral {
        token: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    CharLiteral {
        token: Box<Token>,
    },
    VoidLiteral {
        left: Box<Token>,
        right: Box<Token>,
    },
    Variable {
        name: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
        symbol: Option<WeakSymbol>,
    },
    Type {
        name: Box<Token>,
        type_parameters: Option<Vec<Rc<RefCell<Expression>>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    PointerType {
        base: Box<Rc<RefCell<Expression>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Call {
        callee: Rc<RefCell<Expression>>,
        type_parameters: Option<Vec<Rc<RefCell<Expression>>>>,
        parameters: Vec<CallParameter>,
    },
    MemberAccess {
        object: Rc<RefCell<Expression>>,
        member: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
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
    If {
        condition: Rc<RefCell<Expression>>,
        then: Rc<RefCell<Expression>>,
        else_: Option<Rc<RefCell<Expression>>>,
    },
    Case {
        token: Box<Token>,
        enum_type: Box<Token>,
        case_name: Box<Token>,
        bindings: Vec<super::statement::Pattern>,
        expression: Rc<RefCell<Expression>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Cast {
        token: Box<Token>,
        kind_tokens: Option<(Box<Token>, Box<Token>)>,
        expression: Rc<RefCell<Expression>>,
        target_type: Rc<RefCell<Expression>>,
        kind: CastKind,
        ty: Option<Rc<RefCell<Type>>>,
    },
    TupleLiteral {
        left: Box<Token>,
        elements: Vec<Rc<RefCell<Expression>>>,
        right: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    TupleType {
        left: Box<Token>,
        elements: Vec<Rc<RefCell<Expression>>>,
        right: Box<Token>,
    },
    TupleIndexAccess {
        object: Rc<RefCell<Expression>>,
        index: Box<Token>,
        index_value: u64,
        ty: Option<Rc<RefCell<Type>>>,
    },
}

impl Expression {
    pub fn get_ty(&self) -> Result<Option<Rc<RefCell<Type>>>> {
        match self {
            Self::IntegerLiteral { ty, .. } => Ok(ty.clone()),
            Self::Variable { ty, .. } => Ok(ty.clone()),
            Self::Type { ty, .. } => Ok(ty.clone()),
            Self::Cast { ty, .. } => Ok(ty.clone()),
            Self::TupleLiteral { ty, .. } => Ok(ty.clone()),
            _ => Err(anyhow!("")),
        }
    }
    pub fn get_ty_ref(&self) -> Result<&Option<Rc<RefCell<Type>>>> {
        match self {
            Self::IntegerLiteral { ty, .. } => Ok(ty),
            Self::Variable { ty, .. } => Ok(ty),
            Self::Type { ty, .. } => Ok(ty),
            Self::Cast { ty, .. } => Ok(ty),
            Self::TupleLiteral { ty, .. } => Ok(ty),
            _ => Err(anyhow!("")),
        }
    }
    pub fn get_ty_mut_ref(&mut self) -> Result<&mut Option<Rc<RefCell<Type>>>> {
        match self {
            Self::IntegerLiteral { ty, .. } => Ok(ty),
            Self::Variable { ty, .. } => Ok(ty),
            Self::Type { ty, .. } => Ok(ty),
            Self::Cast { ty, .. } => Ok(ty),
            Self::TupleLiteral { ty, .. } => Ok(ty),
            _ => Err(anyhow!("")),
        }
    }
    pub fn token(&self) -> Token {
        match self {
            Expression::IntegerLiteral { token, .. } => (**token).clone(),
            Expression::DecimalLiteral { token, .. } => (**token).clone(),
            Expression::BooleanLiteral { token } => (**token).clone(),
            Expression::NullLiteral { token } => (**token).clone(),
            Expression::NullptrLiteral { token, .. } => (**token).clone(),
            Expression::CharLiteral { token } => (**token).clone(),
            Expression::VoidLiteral { left, .. } => (**left).clone(),
            Expression::Variable { name, .. } => (**name).clone(),
            Expression::Type { name, .. } => (**name).clone(),
            Expression::PointerType { base, .. } => base.borrow().token(),
            Expression::Unary { expression, .. } => expression.borrow().token(),
            Expression::Binary { left, .. } => left.borrow().token(),
            Expression::Call { callee, .. } => callee.borrow().token(),
            Expression::Assignment { left, .. } => left.borrow().token(),
            Expression::If { condition, .. } => condition.borrow().token(),
            Expression::Case { token, .. } => (**token).clone(),
            Expression::Cast {
                token,
                kind_tokens,
                kind,
                ..
            } => match kind {
                CastKind::ForceBitcast => {
                    if let Some((_, second)) = kind_tokens {
                        (**second).clone()
                    } else {
                        (**token).clone()
                    }
                }
                _ => (**token).clone(),
            },
            Expression::Block { statements, .. } => {
                if let Some(last) = statements.last() {
                    match &*last.borrow() {
                        Statement::ExpressionStatement { expression } => {
                            expression.borrow().token()
                        }
                        Statement::Return { token, value } => {
                            if let Some(value) = value {
                                value.borrow().token()
                            } else {
                                (**token).clone()
                            }
                        }
                        _ => Token::new(
                            "".to_string(),
                            TokenType::Identifier,
                            Position {
                                pos: 0,
                                line: 0,
                                col: 0,
                                len: 0,
                            },
                            Rc::new("".to_string()),
                        ),
                    }
                } else {
                    Token::new(
                        "".to_string(),
                        TokenType::Identifier,
                        Position {
                            pos: 0,
                            line: 0,
                            col: 0,
                            len: 0,
                        },
                        Rc::new("".to_string()),
                    )
                }
            }
            Expression::MemberAccess { object, .. } => object.borrow().token(),
            Expression::TupleLiteral { left, .. } => (**left).clone(),
            Expression::TupleType { left, .. } => (**left).clone(),
            Expression::TupleIndexAccess { index, .. } => (**index).clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastKind {
    Regular,
    Conditional,
    Force,
    ForceBitcast,
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
    BitAnd,
    BitOr,
    BitXor,
    LeftShift,
    RightShift,
    RangeTo,
    RangeUntil,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Inc,
    Dec,
    NotNullAssertation,
    OpenRange,
    BitNot,
    Deref,
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
            OperatorType::BitAnd => Some(BinaryOperator::BitAnd),
            OperatorType::BitOr => Some(BinaryOperator::BitOr),
            OperatorType::LeftShift => Some(BinaryOperator::LeftShift),
            OperatorType::RightShift => Some(BinaryOperator::RightShift),
            OperatorType::RangeTo => Some(BinaryOperator::RangeTo),
            OperatorType::RangeUntil => Some(BinaryOperator::RangeUntil),
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
            OperatorType::OpenRange => Some(UnaryOperator::OpenRange),
            OperatorType::BitNot => Some(UnaryOperator::BitNot),
            OperatorType::Multiply => Some(UnaryOperator::Deref),
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
