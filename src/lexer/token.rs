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
    IntegerLiteral { value: u64 },
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
    Backtick,     // ~
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
    LeftShift,        // <<
    RightShift,       // >>
    LeftShiftAssign,  // <<=
    RightShiftAssign, // >>=
    Arrow,            // ->
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
