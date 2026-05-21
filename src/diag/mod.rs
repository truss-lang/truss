use duck_diagnostic::{Diagnostic, DiagnosticCode, DiagnosticEngine, Label, Severity, Span};
use std::sync::Arc;

use crate::lexer::token::{Position, Token};

pub type TrussDiagnostic = Diagnostic<TrussDiagnosticCode>;
pub type TrussDiagnosticEngine = DiagnosticEngine<TrussDiagnosticCode>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrussDiagnosticCode {
    LexerError,
    UnexpectedCharacter,
    UnterminatedString,
    UnterminatedChar,
    InvalidNumber,

    ParserError,
    UnexpectedToken,
    ExpectedExpression,
    ExpectedIdentifier,
    ExpectedType,
    InvalidFunctionName,
    InvalidVariableName,
    InvalidStructName,
    MissingSeparator,

    SymbolError,
    UndefinedVariable,
    UndefinedFunction,
    ShadowedVariable,
    DuplicateFunction,
    UnusedVariable,

    TypeError,
    TypeMismatch,
    UnknownType,
    InvalidOperator,
    InvalidOperand,
    ReturnTypeMismatch,
    ArgumentCountMismatch,
    ArgumentTypeMismatch,
    ArgumentLabelMismatch,
    MissingArgumentLabel,
    MissingTypeAnnotation,
    InvalidConditionType,
    BranchTypeMismatch,
    CallingNonFunction,

    IRError,
    UnsupportedFeature,
    NeverTypeConversion,
    VoidTypeConversion,
    NestedFunctionType,
    TypeInferenceFailed,
    IRVariableNotFound,
}

impl DiagnosticCode for TrussDiagnosticCode {
    fn code(&self) -> &str {
        match self {
            Self::LexerError => "E0001",
            Self::UnexpectedCharacter => "E0002",
            Self::UnterminatedString => "E0003",
            Self::UnterminatedChar => "E0004",
            Self::InvalidNumber => "E0005",

            Self::ParserError => "E0101",
            Self::UnexpectedToken => "E0102",
            Self::ExpectedExpression => "E0103",
            Self::ExpectedIdentifier => "E0104",
            Self::ExpectedType => "E0105",
            Self::InvalidFunctionName => "E0106",
            Self::InvalidVariableName => "E0107",
            Self::InvalidStructName => "E0108",
            Self::MissingSeparator => "E0109",

            Self::SymbolError => "E0201",
            Self::UndefinedVariable => "E0202",
            Self::UndefinedFunction => "E0203",
            Self::ShadowedVariable => "W0204",
            Self::DuplicateFunction => "E0205",
            Self::UnusedVariable => "W0206",

            Self::TypeError => "E0301",
            Self::TypeMismatch => "E0302",
            Self::UnknownType => "E0303",
            Self::InvalidOperator => "E0304",
            Self::InvalidOperand => "E0305",
            Self::ReturnTypeMismatch => "E0306",
            Self::ArgumentCountMismatch => "E0307",
            Self::ArgumentTypeMismatch => "E0308",
            Self::ArgumentLabelMismatch => "E0309",
            Self::MissingArgumentLabel => "E0310",
            Self::MissingTypeAnnotation => "E0311",
            Self::InvalidConditionType => "E0312",
            Self::BranchTypeMismatch => "E0313",
            Self::CallingNonFunction => "E0314",

            Self::IRError => "E0401",
            Self::UnsupportedFeature => "E0402",
            Self::NeverTypeConversion => "E0403",
            Self::VoidTypeConversion => "E0404",
            Self::NestedFunctionType => "E0405",
            Self::TypeInferenceFailed => "E0406",
            Self::IRVariableNotFound => "E0407",
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Self::ShadowedVariable | Self::UnusedVariable => Severity::Warning,
            _ => Severity::Error,
        }
    }

    fn url(&self) -> Option<&'static str> {
        None
    }
}

pub fn position_to_span(file: &str, pos: &Position) -> Span {
    Span::from_zero_based(Arc::from(file), pos.line, pos.col, pos.len)
}

pub fn token_to_span(token: &Token) -> Span {
    Span::from_zero_based(
        Arc::from(token.file.as_str()),
        token.position.line,
        token.position.col,
        token.position.len,
    )
}

pub fn primary_label_from_token(token: &Token, message: &str) -> Label {
    Label::primary(token_to_span(token), Some(message.to_string()))
}

pub fn secondary_label_from_token(token: &Token, message: &str) -> Label {
    Label::secondary(token_to_span(token), Some(message.to_string()))
}

pub fn new_diagnostic(code: TrussDiagnosticCode, message: impl Into<String>) -> TrussDiagnostic {
    TrussDiagnostic::new(code, message)
}

pub use duck_diagnostic::Label as DiagLabel;
