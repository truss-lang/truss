use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub pos: usize,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Keyword { keyword: KeywordType },
    Separator { separator: SeparatorType },
    Operator { operator: OperatorType },
    Integer { value: u64 },
    Decimal { value: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordType {
    Let,
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
}

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
