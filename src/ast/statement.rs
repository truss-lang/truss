use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use super::expression::Expression;
use crate::{lexer::token::Token, scope::Scope, types::Type};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Rc<RefCell<Parameter>>>,
        return_type: Option<Rc<RefCell<Expression>>>,
        body: Rc<RefCell<FunctionBody>>,
        where_clause: Option<Vec<WhereRequirement>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    VariableDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        type_expression: Option<Rc<RefCell<Expression>>>,
        initializer: Option<Rc<RefCell<Expression>>>,
        accessors: Vec<Accessor>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    StructDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        conformances: Vec<Rc<RefCell<Expression>>>,
        body: Vec<Rc<RefCell<Statement>>>,
        where_clause: Option<Vec<WhereRequirement>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    ClassDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        superclass: Option<Rc<RefCell<Expression>>>,
        conformances: Vec<Rc<RefCell<Expression>>>,
        body: Vec<Rc<RefCell<Statement>>>,
        where_clause: Option<Vec<WhereRequirement>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    ProtocolDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        conformances: Vec<Rc<RefCell<Expression>>>,
        members: Vec<ProtocolMember>,
        where_clause: Option<Vec<WhereRequirement>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    EnumDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        cases: Vec<EnumCase>,
        body: Vec<Rc<RefCell<Statement>>>,
        where_clause: Option<Vec<WhereRequirement>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    InitDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        parameters: Vec<Rc<RefCell<Parameter>>>,
        body: Rc<RefCell<FunctionBody>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    DeinitDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        body: Rc<RefCell<FunctionBody>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    ExpressionStatement {
        expression: Rc<RefCell<Expression>>,
    },
    Return {
        token: Box<Token>,
        value: Option<Rc<RefCell<Expression>>>,
    },
    Loop {
        token: Box<Token>,
        body: Rc<RefCell<Expression>>,
    },
    While {
        token: Box<Token>,
        condition: Rc<RefCell<Expression>>,
        body: Rc<RefCell<Expression>>,
    },
    RepeatWhile {
        token: Box<Token>,
        body: Rc<RefCell<Expression>>,
        condition: Rc<RefCell<Expression>>,
    },
    For {
        token: Box<Token>,
        pattern: Rc<Pattern>,
        iterator: Rc<RefCell<Expression>>,
        body: Rc<RefCell<Expression>>,
    },
    Throw {
        token: Box<Token>,
        exception: Rc<RefCell<Expression>>,
    },
    EmptyStatement {
        token: Box<Token>,
    },
    ExternBlock {
        token: Box<Token>,
        linkage: Box<Token>,
        items: Vec<Rc<RefCell<Statement>>>,
    },
    ExternDecl {
        token: Box<Token>,
        linkage: Box<Token>,
        statement: Rc<RefCell<Statement>>,
    },
    ExtensionDecl {
        token: Box<Token>,
        type_name: Box<Token>,
        conformances: Vec<Rc<RefCell<Expression>>>,
        body: Vec<Rc<RefCell<Statement>>>,
        where_clause: Option<Vec<WhereRequirement>>,
        scope: Option<Rc<RefCell<Scope>>>,
    },
    TypeAlias {
        token: Box<Token>,
        name: Box<Token>,
        type_expression: Rc<RefCell<Expression>>,
    },
}

impl Statement {
    pub fn token(&self) -> Token {
        match self {
            Self::FunctionDecl { token, .. } => (**token).clone(),
            Self::VariableDecl { token, .. } => (**token).clone(),
            Self::StructDecl { token, .. } => (**token).clone(),
            Self::ClassDecl { token, .. } => (**token).clone(),
            Self::EnumDecl { token, .. } => (**token).clone(),
            Self::InitDecl { token, .. } => (**token).clone(),
            Self::DeinitDecl { token, .. } => (**token).clone(),
            Self::ExpressionStatement { expression } => expression.borrow().token(),
            Self::Return { token, .. } => (**token).clone(),
            Self::Loop { token, .. } => (**token).clone(),
            Self::While { token, .. } => (**token).clone(),
            Self::RepeatWhile { token, .. } => (**token).clone(),
            Self::For { token, .. } => (**token).clone(),
            Self::Throw { token, .. } => (**token).clone(),
            Self::EmptyStatement { token } => (**token).clone(),
            Self::ExternBlock { token, .. } => (**token).clone(),
            Self::ExternDecl { token, .. } => (**token).clone(),
            Self::ProtocolDecl { token, .. } => (**token).clone(),
            Self::ExtensionDecl { token, .. } => (**token).clone(),
            Self::TypeAlias { token, .. } => (**token).clone(),
        }
    }
    pub fn modifiers(&self) -> Result<Vec<Modifier>> {
        match self {
            Self::FunctionDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::VariableDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::StructDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::ClassDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::EnumDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::InitDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::DeinitDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::ProtocolDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::TypeAlias { .. } => Ok(vec![]),
            _ => anyhow::bail!(""),
        }
    }
    pub fn access_modifier(&self) -> Result<Option<Modifier>> {
        self.modifiers().map(|modifiers| {
            modifiers
                .iter()
                .find(|m| matches!(m.ty, ModifierType::Access(_)))
                .cloned()
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Modifier {
    pub token: Box<Token>,
    pub ty: ModifierType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModifierType {
    Access(AccessModifier),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessModifier {
    Open,
    Public,
    Internal,
    Fileprivate,
    Private,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FunctionBody {
    Statements(Vec<Rc<RefCell<Statement>>>),
    Expression(Rc<RefCell<Expression>>),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AccessorKind {
    Get,
    Set,
    WillSet,
    DidSet,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Accessor {
    pub kind: AccessorKind,
    pub parameter: Option<Box<Token>>,
    pub body: Vec<Rc<RefCell<Statement>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParameter {
    pub name: Box<Token>,
    pub constraints: Vec<Rc<RefCell<Expression>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhereRequirement {
    pub kind: WhereRequirementKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhereRequirementKind {
    Conformance {
        type_expr: Rc<RefCell<Expression>>,
        constraint: Rc<RefCell<Expression>>,
    },
    Equality {
        left: Rc<RefCell<Expression>>,
        right: Rc<RefCell<Expression>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VariadicKind {
    NotVariadic,
    BareVariadic,
    TypedVariadic,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub label: Option<Box<Token>>,
    pub name: Box<Token>,
    pub type_expression: Rc<RefCell<Expression>>,
    pub ty: Option<Rc<RefCell<Type>>>,
    pub variadic_kind: VariadicKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(Box<Token>),
    Tuple(Vec<Pattern>),
    Ignore,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCase {
    pub token: Box<Token>,
    pub name: Box<Token>,
    pub parameters: Vec<EnumCaseParameter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumCaseParameter {
    pub label: Option<Box<Token>>,
    pub type_expression: Rc<RefCell<Expression>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProtocolMember {
    Method {
        modifiers: Vec<Modifier>,
        decl: Rc<RefCell<Statement>>,
    },
    Property {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        type_expression: Rc<RefCell<Expression>>,
        accessors: ProtocolAccessorSet,
    },
    AssociatedType {
        token: Box<Token>,
        name: Box<Token>,
        constraints: Vec<Rc<RefCell<Expression>>>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtocolAccessorSet {
    pub get: bool,
    pub set: bool,
}
