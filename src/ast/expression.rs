use std::{cell::RefCell, rc::Rc};

use anyhow::{Result, anyhow};

use crate::{
    lexer::token::{OperatorType, Position, Token, TokenType},
    scope::Scope,
    symbol::{Symbol, WeakSymbol},
    types::Type,
};

use super::statement::{MatchCase, Statement};

#[derive(Debug, Clone, PartialEq)]
pub struct CallParameter {
    pub label: Option<Box<Token>>,
    pub expression: Rc<RefCell<Expression>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClosureParameter {
    pub name: Box<Token>,
    pub type_annotation: Option<Rc<RefCell<Expression>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
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
        ty: Option<Rc<RefCell<Type>>>,
    },
    StringLiteral {
        token: Box<Token>,
        value: String,
        ty: Option<Rc<RefCell<Type>>>,
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
        non_null: bool,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Call {
        callee: Rc<RefCell<Expression>>,
        type_parameters: Option<Vec<Rc<RefCell<Expression>>>>,
        parameters: Vec<CallParameter>,
        overloads: Vec<Rc<RefCell<Symbol>>>,
        selected_index: Option<usize>,
    },
    MemberAccess {
        object: Rc<RefCell<Expression>>,
        member: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    AssociatedTypeAccess {
        object: Rc<RefCell<Expression>>,
        member: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Binary {
        left: Rc<RefCell<Expression>>,
        operator: BinaryOperator,
        right: Rc<RefCell<Expression>>,
        overloads: Vec<Rc<RefCell<Symbol>>>,
        selected_index: Option<usize>,
    },
    Unary {
        expression: Rc<RefCell<Expression>>,
        operator: UnaryOperator,
        is_prefix: bool,
        overloads: Vec<Rc<RefCell<Symbol>>>,
        selected_index: Option<usize>,
    },
    Assignment {
        left: Rc<RefCell<Expression>>,
        operator: AssignmentOperator,
        right: Rc<RefCell<Expression>>,
    },
    If {
        condition: Rc<RefCell<Expression>>,
        then: Vec<Rc<RefCell<Statement>>>,
        else_: Option<ElseBranch>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Case {
        token: Box<Token>,
        enum_type: Option<Box<Token>>,
        case_name: Box<Token>,
        bindings: Vec<super::statement::Pattern>,
        expression: Rc<RefCell<Expression>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Match {
        token: Box<Token>,
        value: Rc<RefCell<Expression>>,
        cases: Vec<MatchCase>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    ShorthandArgument {
        index: u32,
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
        elements: Vec<(Option<String>, Rc<RefCell<Expression>>)>,
        right: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    TupleType {
        left: Box<Token>,
        elements: Vec<(Option<String>, Rc<RefCell<Expression>>)>,
        right: Box<Token>,
    },
    TupleIndexAccess {
        object: Rc<RefCell<Expression>>,
        index: Box<Token>,
        index_value: u64,
        ty: Option<Rc<RefCell<Type>>>,
    },
    SelfKeyword {
        token: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
        symbol: Option<WeakSymbol>,
    },
    SuperKeyword {
        token: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    SelfType {
        token: Box<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    AnyType {
        inner: Rc<RefCell<Expression>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    SomeType {
        inner: Rc<RefCell<Expression>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    CompoundType {
        types: Vec<Rc<RefCell<Expression>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Closure {
        parameters: Vec<Rc<RefCell<ClosureParameter>>>,
        return_type: Option<Rc<RefCell<Expression>>>,
        body: Vec<Rc<RefCell<Statement>>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    FunctionType {
        param_types: Vec<Rc<RefCell<Expression>>>,
        return_type: Rc<RefCell<Expression>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    SubscriptAccess {
        object: Rc<RefCell<Expression>>,
        parameters: Vec<CallParameter>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    MacroInvocation {
        name: Box<Token>,
        delimiter: MacroDelimiter,
        arguments: Vec<Token>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    SizeOf {
        token: Box<Token>,
        argument: Rc<RefCell<Expression>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    Do {
        token: Box<Token>,
        body: Vec<Rc<RefCell<Statement>>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    InlineType {
        token: Box<Token>,
        size: Option<Rc<RefCell<Expression>>>,
        base: Rc<RefCell<Expression>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MacroDelimiter {
    Paren,
    Bracket,
    Brace,
}

impl Expression {
    pub fn get_ty(&self) -> Result<Option<Rc<RefCell<Type>>>> {
        match self {
            Self::IntegerLiteral { ty, .. } => Ok(ty.clone()),
            Self::Variable { ty, .. } => Ok(ty.clone()),
            Self::Type { ty, .. } => Ok(ty.clone()),
            Self::Cast { ty, .. } => Ok(ty.clone()),
            Self::TupleLiteral { ty, .. } => Ok(ty.clone()),
            Self::SelfKeyword { ty, .. } => Ok(ty.clone()),
            Self::SuperKeyword { ty, .. } => Ok(ty.clone()),
            Self::SelfType { ty, .. } => Ok(ty.clone()),
            Self::AnyType { ty, .. } => Ok(ty.clone()),
            Self::SomeType { ty, .. } => Ok(ty.clone()),
            Self::CompoundType { ty, .. } => Ok(ty.clone()),
            Self::Closure { ty, .. } => Ok(ty.clone()),
            Self::FunctionType { ty, .. } => Ok(ty.clone()),
            Self::ShorthandArgument { ty, .. } => Ok(ty.clone()),
            Self::AssociatedTypeAccess { ty, .. } => Ok(ty.clone()),
            Self::SubscriptAccess { ty, .. } => Ok(ty.clone()),
            Self::MacroInvocation { ty, .. } => Ok(ty.clone()),
            Self::SizeOf { ty, .. } => Ok(ty.clone()),
            Self::Do { ty, .. } => Ok(ty.clone()),
            Self::InlineType { ty, .. } => Ok(ty.clone()),
            Self::StringLiteral { ty, .. } => Ok(ty.clone()),
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
            Self::SelfKeyword { ty, .. } => Ok(ty),
            Self::SuperKeyword { ty, .. } => Ok(ty),
            Self::SelfType { ty, .. } => Ok(ty),
            Self::AnyType { ty, .. } => Ok(ty),
            Self::SomeType { ty, .. } => Ok(ty),
            Self::CompoundType { ty, .. } => Ok(ty),
            Self::Closure { ty, .. } => Ok(ty),
            Self::FunctionType { ty, .. } => Ok(ty),
            Self::ShorthandArgument { ty, .. } => Ok(ty),
            Self::AssociatedTypeAccess { ty, .. } => Ok(ty),
            Self::SubscriptAccess { ty, .. } => Ok(ty),
            Self::MacroInvocation { ty, .. } => Ok(ty),
            Self::SizeOf { ty, .. } => Ok(ty),
            Self::Do { ty, .. } => Ok(ty),
            Self::InlineType { ty, .. } => Ok(ty),
            Self::StringLiteral { ty, .. } => Ok(ty),
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
            Self::SelfKeyword { ty, .. } => Ok(ty),
            Self::SuperKeyword { ty, .. } => Ok(ty),
            Self::SelfType { ty, .. } => Ok(ty),
            Self::AnyType { ty, .. } => Ok(ty),
            Self::SomeType { ty, .. } => Ok(ty),
            Self::CompoundType { ty, .. } => Ok(ty),
            Self::Closure { ty, .. } => Ok(ty),
            Self::FunctionType { ty, .. } => Ok(ty),
            Self::ShorthandArgument { ty, .. } => Ok(ty),
            Self::AssociatedTypeAccess { ty, .. } => Ok(ty),
            Self::SubscriptAccess { ty, .. } => Ok(ty),
            Self::MacroInvocation { ty, .. } => Ok(ty),
            Self::SizeOf { ty, .. } => Ok(ty),
            Self::Do { ty, .. } => Ok(ty),
            Self::InlineType { ty, .. } => Ok(ty),
            Self::StringLiteral { ty, .. } => Ok(ty),
            _ => Err(anyhow!("")),
        }
    }
    pub fn token(&self) -> Token {
        match self {
            Expression::IntegerLiteral { token, .. } => (**token).clone(),
            Expression::DecimalLiteral { token, .. } => (**token).clone(),
            Expression::BooleanLiteral { token } => (**token).clone(),
            Expression::NullLiteral { token, .. } => (**token).clone(),
            Expression::StringLiteral { token, .. } => (**token).clone(),
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
            Expression::Match { token, .. } => (**token).clone(),
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
            Expression::MemberAccess { object, .. } => object.borrow().token(),
            Expression::TupleLiteral { left, .. } => (**left).clone(),
            Expression::TupleType { left, .. } => (**left).clone(),
            Expression::TupleIndexAccess { index, .. } => (**index).clone(),
            Expression::SelfKeyword { token, .. } => (**token).clone(),
            Expression::SuperKeyword { token, .. } => (**token).clone(),
            Expression::SelfType { token, .. } => (**token).clone(),
            Expression::AnyType { inner, .. } => inner.borrow().token(),
            Expression::SomeType { inner, .. } => inner.borrow().token(),
            Expression::CompoundType { types, .. } => types[0].borrow().token(),
            Expression::AssociatedTypeAccess { object, .. } => object.borrow().token(),
            Expression::Closure { body, .. } => {
                if let Some(last) = body.last() {
                    match &*last.borrow() {
                        Statement::ExpressionStatement { expression } => {
                            expression.borrow().token()
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
            Expression::FunctionType { param_types, .. } => param_types[0].borrow().token(),
            Expression::ShorthandArgument { .. } => Token::new(
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
            Expression::SubscriptAccess { object, .. } => object.borrow().token(),
            Expression::MacroInvocation { name, .. } => (**name).clone(),
            Expression::SizeOf { token, .. } => (**token).clone(),
            Expression::Do { token, .. } => (**token).clone(),
            Expression::InlineType { base, .. } => base.borrow().token(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    Block(Vec<Rc<RefCell<Statement>>>),
    If(Rc<RefCell<Expression>>),
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
    AddressOf,
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
    pub fn operator_name(&self) -> &'static str {
        match self {
            BinaryOperator::Plus => "+",
            BinaryOperator::Minus => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulus => "%",
            BinaryOperator::Equal => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::LessEqual => "<=",
            BinaryOperator::Greater => ">",
            BinaryOperator::GreaterEqual => ">=",
            BinaryOperator::And => "&&",
            BinaryOperator::Or => "||",
            BinaryOperator::BitAnd => "&",
            BinaryOperator::BitOr => "|",
            BinaryOperator::BitXor => "^",
            BinaryOperator::LeftShift => "<<",
            BinaryOperator::RightShift => ">>",
            BinaryOperator::RangeTo => "..",
            BinaryOperator::RangeUntil => "..<",
        }
    }

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
    pub fn operator_name(&self) -> &'static str {
        match self {
            UnaryOperator::Plus => "+",
            UnaryOperator::Minus => "-",
            UnaryOperator::Inc => "++",
            UnaryOperator::Dec => "--",
            UnaryOperator::NotNullAssertation => "!!",
            UnaryOperator::OpenRange => "...",
            UnaryOperator::BitNot => "~",
            UnaryOperator::Deref => "*",
            UnaryOperator::AddressOf => "&",
        }
    }

    pub fn from_operator(operator: OperatorType) -> Option<UnaryOperator> {
        match operator {
            OperatorType::Plus => Some(UnaryOperator::Plus),
            OperatorType::Minus => Some(UnaryOperator::Minus),
            OperatorType::Inc => Some(UnaryOperator::Inc),
            OperatorType::Dec => Some(UnaryOperator::Dec),
            OperatorType::OpenRange => Some(UnaryOperator::OpenRange),
            OperatorType::BitNot => Some(UnaryOperator::BitNot),
            OperatorType::Multiply => Some(UnaryOperator::Deref),
            OperatorType::BitAnd => Some(UnaryOperator::AddressOf),
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
