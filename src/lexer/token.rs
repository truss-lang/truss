use std::rc::Rc;

use strum::EnumIter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub pos: usize,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Keyword { keyword: KeywordType },
    Identifier,
    Separator { separator: SeparatorType },
    Operator { operator: OperatorType },
    StringLiteral { value: String },
    CharLiteral { value: char },
    IntegerLiteral { value: i128 },
    DecimalLiteral { value: f64 },
    BooleanLiteral { value: bool },
    NullLiteral,
    NullptrLiteral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter)]
pub enum KeywordType {
    Func,
    Let,
    Var,
    If,
    Else,
    Loop,
    While,
    Repeat,
    For,
    In,
    Return,
    Throw,
    Extern,
    As,
    Struct,
    Class,
    Enum,
    Case,
    Init,
    Deinit,
    Open,
    Public,
    Internal,
    Fileprivate,
    Private,
    SelfKw,
    SuperKw,
    Protocol,
    Any,
    Some,
    Extension,
    SelfType,
    Where,
    Associatedtype,
    Typealias,
    Match,
    Guard,
    Default,
    Break,
    Fallthrough,
    Defer,
    Yield,
    Module,
    Import,
    Package,
    Static,
    Mutating,
    Subscript,
    Macro,
    Prefix,
    Postfix,
    SizeOf,
    Asm,
    Do,
    Inline,
    Override,
    Abstract,
    Final,
    Try,
    Catch,
    Throws,
    Finally,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeparatorType {
    OpenParen,    // (
    CloseParen,   // )
    OpenBracket,  // [
    CloseBracket, // ]
    OpenBrace,    // {
    CloseBrace,   // }
    Colon,        // :
    SemiColon,    // ;
    Comma,        // ,
    At,           // @
    Backtick,     // `
    DoubleColon,  // ::
    Hash,         // #
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorType {
    QuestionMark,     // ?
    Plus,             // +
    Minus,            // -
    Multiply,         // *
    Divide,           // /
    Modulus,          // %
    PlusAssign,       // +=
    MinusAssign,      // -=
    MultiplyAssign,   // *=
    DivideAssign,     // /=
    ModulusAssign,    // %=
    Inc,              // ++
    Dec,              // --
    Assign,           // =
    Equal,            // ==
    NotEqual,         // !=
    Less,             // <
    LessEqual,        // <=
    Greater,          // >
    GreaterEqual,     // >=
    And,              // &&
    Or,               // ||
    Not,              // !
    BitAnd,           // &
    BitOr,            // |
    BitAndAssign,     // &=
    BitOrAssign,      // |=
    BitNot,           // ~
    LeftShift,        // <<
    RightShift,       // >>
    LeftShiftAssign,  // <<=
    RightShiftAssign, // >>=
    Arrow,            // ->
    Dot,              // .
    RangeTo,          // ..
    RangeUntil,       // ..<
    OpenRange,        // ...
    Dollar,           // $
}

impl KeywordType {
    pub fn code(&self) -> String {
        match self {
            KeywordType::Func => "func",
            KeywordType::Let => "let",
            KeywordType::Var => "var",
            KeywordType::If => "if",
            KeywordType::Else => "else",
            KeywordType::Loop => "loop",
            KeywordType::While => "while",
            KeywordType::Repeat => "repeat",
            KeywordType::For => "for",
            KeywordType::In => "in",
            KeywordType::Return => "return",
            KeywordType::Throw => "throw",
            KeywordType::Extern => "extern",
            KeywordType::As => "as",
            KeywordType::Struct => "struct",
            KeywordType::Class => "class",
            KeywordType::Enum => "enum",
            KeywordType::Case => "case",
            KeywordType::Init => "init",
            KeywordType::Deinit => "deinit",
            KeywordType::Open => "open",
            KeywordType::Public => "public",
            KeywordType::Internal => "internal",
            KeywordType::Fileprivate => "fileprivate",
            KeywordType::Private => "private",
            KeywordType::SelfKw => "self",
            KeywordType::SuperKw => "super",
            KeywordType::Protocol => "protocol",
            KeywordType::Any => "any",
            KeywordType::Some => "some",
            KeywordType::Extension => "extension",
            KeywordType::SelfType => "Self",
            KeywordType::Where => "where",
            KeywordType::Associatedtype => "associatedtype",
            KeywordType::Typealias => "typealias",
            KeywordType::Match => "match",
            KeywordType::Guard => "guard",
            KeywordType::Default => "default",
            KeywordType::Break => "break",
            KeywordType::Fallthrough => "fallthrough",
            KeywordType::Defer => "defer",
            KeywordType::Yield => "yield",
            KeywordType::Module => "module",
            KeywordType::Import => "import",
            KeywordType::Package => "package",
            KeywordType::Static => "static",
            KeywordType::Mutating => "mutating",
            KeywordType::Subscript => "subscript",
            KeywordType::Macro => "macro",
            KeywordType::Prefix => "prefix",
            KeywordType::Postfix => "postfix",
            KeywordType::SizeOf => "sizeof",
            KeywordType::Asm => "asm",
            KeywordType::Do => "do",
            KeywordType::Inline => "inline",
            KeywordType::Override => "override",
            KeywordType::Abstract => "abstract",
            KeywordType::Final => "final",
            KeywordType::Try => "try",
            KeywordType::Catch => "catch",
            KeywordType::Throws => "throws",
            KeywordType::Finally => "finally",
        }
        .to_string()
    }
    pub fn is_keyword(token: &Token, kw: KeywordType) -> bool {
        if let TokenType::Keyword { keyword } = token.ty
            && kw == keyword
        {
            true
        } else {
            false
        }
    }
}
impl SeparatorType {
    pub fn is_separator(token: &Token, sep: SeparatorType) -> bool {
        if let TokenType::Separator { separator } = token.ty
            && sep == separator
        {
            true
        } else {
            false
        }
    }
}
impl OperatorType {
    pub fn is_operator(token: &Token, op: OperatorType) -> bool {
        if let TokenType::Operator { operator } = token.ty
            && op == operator
        {
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub value: String,
    pub ty: TokenType,
    pub position: Position,
    pub file: Rc<String>,
}

impl Token {
    pub fn new(value: String, ty: TokenType, position: Position, file: Rc<String>) -> Self {
        Self {
            value,
            ty,
            position,
            file,
        }
    }
}
