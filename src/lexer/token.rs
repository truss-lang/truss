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
}

impl KeywordType {
    pub fn code(&self) -> String {
        match self {
            KeywordType::Func => "func",
            KeywordType::Let => "let",
            _ => "unknown",
        }
        .to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeparatorType {
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    OpenBrace,
    CloseBrace,
    Colon,
    SemiColon,
    Comma,
    At,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OperatorType {
    QuestionMark,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulus,
    PlusAssign,
    MinusAssign,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
    Inc,
    Dec,
    Assign,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    Not,
    BitAnd,
    BitOr,
    BitAndAssign,
    BitOrAssign,
    LeftShift,
    RightShift,
    LeftShiftAssign,
    RightShiftAssign,
    Arrow,
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
