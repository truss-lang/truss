use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use super::expression::Expression;
use crate::{lexer::token::Token, scope::Scope, types::Type};

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    FunctionDecl {
        attributes: Vec<Attribute>,
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Rc<RefCell<Parameter>>>,
        return_type: Option<Rc<RefCell<Expression>>>,
        throws_types: Option<Vec<Rc<RefCell<Expression>>>>,
        body: Rc<RefCell<FunctionBody>>,
        where_clause: Option<Vec<WhereRequirement>>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
        static_method: bool,
        mutating: bool,
        operator_fixity: Option<OperatorFixity>,
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
        attributes: Vec<Attribute>,
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
        is_failable: bool,
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
    Yield {
        token: Box<Token>,
        value: Option<Rc<RefCell<Expression>>>,
    },
    Loop {
        token: Box<Token>,
        body: Vec<Rc<RefCell<Statement>>>,
    },
    While {
        token: Box<Token>,
        condition: Rc<RefCell<Expression>>,
        body: Vec<Rc<RefCell<Statement>>>,
    },
    RepeatWhile {
        token: Box<Token>,
        body: Vec<Rc<RefCell<Statement>>>,
        condition: Rc<RefCell<Expression>>,
    },
    For {
        token: Box<Token>,
        pattern: Rc<Pattern>,
        iterator: Rc<RefCell<Expression>>,
        body: Vec<Rc<RefCell<Statement>>>,
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
        type_arguments: Option<Vec<Rc<RefCell<Expression>>>>,
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
    Guard {
        token: Box<Token>,
        condition: Rc<RefCell<Expression>>,
        else_body: Vec<Rc<RefCell<Statement>>>,
    },
    Fallthrough {
        token: Box<Token>,
    },
    Break {
        token: Box<Token>,
    },
    Defer {
        token: Box<Token>,
        body: Vec<Rc<RefCell<Statement>>>,
    },
    ModuleDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        name: Box<Token>,
        body: Vec<Rc<RefCell<Statement>>>,
        scope: Option<Rc<RefCell<Scope>>>,
    },
    ImportDecl {
        token: Box<Token>,
        path: Vec<String>,
        kind: ImportKind,
        selective_members: Option<Vec<SelectiveMember>>,
        is_current_package: bool,
    },
    SubscriptDecl {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Rc<RefCell<Parameter>>>,
        return_type_expression: Rc<RefCell<Expression>>,
        where_clause: Option<Vec<WhereRequirement>>,
        accessors: Vec<Accessor>,
        scope: Option<Rc<RefCell<Scope>>>,
        ty: Option<Rc<RefCell<Type>>>,
    },
    MacroDecl {
        token: Box<Token>,
        name: Box<Token>,
        arms: Vec<MacroArm>,
    },
    ConditionalBlock {
        clauses: Vec<ConditionalClause>,
    },
    PragmaError {
        token: Box<Token>,
        message: String,
    },
    PragmaWarning {
        token: Box<Token>,
        message: String,
    },
    AsmBlock {
        token: Box<Token>,
        instructions: Vec<Token>,
        outputs: Vec<AsmOperand>,
        inputs: Vec<AsmOperand>,
        clobbers: Vec<Token>,
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
            Self::Yield { token, .. } => (**token).clone(),
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
            Self::Guard { token, .. } => (**token).clone(),
            Self::Fallthrough { token, .. } => (**token).clone(),
            Self::Break { token, .. } => (**token).clone(),
            Self::Defer { token, .. } => (**token).clone(),
            Self::ModuleDecl { token, .. } => (**token).clone(),
            Self::ImportDecl { token, .. } => (**token).clone(),
            Self::SubscriptDecl { token, .. } => (**token).clone(),
            Self::MacroDecl { token, .. } => (**token).clone(),
            Self::ConditionalBlock { clauses } => {
                clauses.first().map(|c| c.token.as_ref().clone()).unwrap()
            }
            Self::PragmaError { token, .. } => (**token).clone(),
            Self::PragmaWarning { token, .. } => (**token).clone(),
            Self::AsmBlock { token, .. } => (**token).clone(),
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
            Self::Guard { .. } => Ok(vec![]),
            Self::Fallthrough { .. } => Ok(vec![]),
            Self::Break { .. } => Ok(vec![]),
            Self::Defer { .. } => Ok(vec![]),
            Self::ModuleDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::ImportDecl { .. } => Ok(vec![]),
            Self::SubscriptDecl { modifiers, .. } => Ok(modifiers.clone()),
            Self::ConditionalBlock { .. } => Ok(vec![]),
            Self::PragmaError { .. } => Ok(vec![]),
            Self::PragmaWarning { .. } => Ok(vec![]),
            Self::AsmBlock { .. } => Ok(vec![]),
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperatorFixity {
    Prefix,
    Postfix,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModifierType {
    Access(AccessModifier),
    AccessSet(AccessModifier),
    Static,
    Mutating,
    OperatorFixity(OperatorFixity),
    Override,
    Abstract,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AccessModifier {
    Open,
    Public,
    Internal,
    Fileprivate,
    Package,
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
    pub set_access_modifier: Option<AccessModifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GenericParameterKind {
    Type {
        constraints: Vec<Rc<RefCell<Expression>>>,
    },
    Const {
        const_type: Rc<RefCell<Expression>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericParameter {
    pub name: Box<Token>,
    pub kind: GenericParameterKind,
    pub default_value: Option<Rc<RefCell<Expression>>>,
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
    pub default_value: Option<Rc<RefCell<Expression>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    Identifier(Box<Token>),
    Tuple(Vec<Pattern>),
    Ignore,
    ValueBinding(Box<Pattern>),
    EnumCase {
        case_name: Box<Token>,
        bindings: Vec<Pattern>,
    },
    Expr(Rc<RefCell<Expression>>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchCase {
    pub token: Box<Token>,
    pub patterns: Vec<Rc<Pattern>>,
    pub guard: Option<Rc<RefCell<Expression>>>,
    pub body: Vec<Rc<RefCell<Statement>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CatchClause {
    pub pattern: Option<Pattern>,
    pub guard: Option<Rc<RefCell<Expression>>>,
    pub body: Vec<Rc<RefCell<Statement>>>,
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
        attributes: Vec<Attribute>,
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
    TypeAlias {
        token: Box<Token>,
        name: Box<Token>,
        type_expression: Rc<RefCell<Expression>>,
    },
    Subscript {
        modifiers: Vec<Modifier>,
        token: Box<Token>,
        generic_parameters: Vec<GenericParameter>,
        parameters: Vec<Rc<RefCell<Parameter>>>,
        return_type_expression: Rc<RefCell<Expression>>,
        accessors: ProtocolAccessorSet,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProtocolAccessorSet {
    pub get: bool,
    pub set: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Module,
    Member,
    Wildcard,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectiveAlias {
    Direct,
    Named(String),
    Skip,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectiveMember {
    pub name: String,
    pub alias: SelectiveAlias,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MacroArm {
    pub pattern: Vec<MacroPatternFragment>,
    pub expansion: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MacroPatternFragment {
    Lit(Token),
    MetaVar {
        name: String,
        var_type: MacroMetaVarType,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MacroMetaVarType {
    Expr,
    Ty,
    Ident,
    Stmt,
    Block,
    Literal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalClause {
    pub token: Box<Token>,
    pub condition: Option<Condition>,
    pub body: Vec<Rc<RefCell<Statement>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    Platform(Token),
    Bool(bool),
    Defined(Token),
    Os(Token),
    Arch(Token),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Group(Box<Condition>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AsmDirection {
    In,
    Out,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsmOperand {
    pub label: Box<Token>,
    pub direction: AsmDirection,
    pub constraint: Box<Token>,
    pub expression: Rc<RefCell<Expression>>,
}
