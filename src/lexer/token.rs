use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub pos: usize,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenType {
    Keyword { keyword: KeywordType },
    Integer { value: u64 },
    Decimal { value: f64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeywordType {
    Let,
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
