pub mod token;

use std::rc::Rc;

use token::{OperatorType, Position, SeparatorType, Token, TokenType};

pub struct CharStream {
    chars: Vec<char>,
    pub file: Rc<String>,
    pub pos: usize,
    pub line: usize,
    pub col: usize,
}

impl CharStream {
    pub fn new(str: String, file: Rc<String>) -> Self {
        Self {
            chars: str.chars().collect::<Vec<_>>(),
            file,
            pos: 0,
            line: 0,
            col: 0,
        }
    }
    pub fn get_current_position(&self) -> Position {
        Position {
            pos: self.pos,
            line: self.line,
            col: self.col,
            len: 1,
        }
    }
    #[inline]
    pub fn peek(&self) -> char {
        self.chars[self.pos]
    }
    pub fn len(&self) -> usize {
        self.chars.len()
    }
    pub fn is_empty(&self) -> bool {
        self.pos >= self.chars.len()
    }
}
impl Iterator for CharStream {
    type Item = char;
    fn next(&mut self) -> Option<char> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
            Some(c)
        } else {
            None
        }
    }
}

pub struct Lexer {
    input: CharStream,
}

impl Lexer {
    pub fn new(input: CharStream) -> Self {
        Self { input }
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.input.is_empty()
    }
    fn skip_whitechars(&mut self) {
        let mut c = self.input.peek();
        while c.is_whitespace() {
            self.input.pos += 1;
            if c == '\n' {
                self.input.line += 1;
                self.input.col = 0;
            } else {
                self.input.col += 1;
            }
            if self.input.pos >= self.input.len() {
                break;
            }
            c = self.input.peek();
        }
    }
    #[inline]
    fn get_position_with_begin(&self, begin: Position) -> Position {
        Position {
            pos: begin.pos,
            line: begin.line,
            col: begin.col,
            len: self.input.get_current_position().pos - begin.pos,
        }
    }
    fn parse_number(&mut self) -> Token {
        let mut c = self.input.peek();
        let begin_pos = self.input.get_current_position();
        self.input.pos += 1;
        self.input.col += 1;
        let mut literal = String::new();
        if c == '0' {
            c = self.input.peek();
            if c == 'x' || c == 'X' {
                self.input.pos += 1;
                self.input.col += 1;
                let mut n = self.input.peek();
                while n.is_ascii_hexdigit() {
                    self.input.pos += 1;
                    self.input.col += 1;
                    literal.push(n);
                    if self.input.is_empty() {
                        break;
                    }
                    n = self.input.peek();
                }
                return Token::new(
                    format!("0{}{}", c, literal.clone()),
                    TokenType::Integer {
                        value: u64::from_str_radix(&literal, 16).unwrap(),
                    },
                    self.get_position_with_begin(begin_pos),
                    self.input.file.clone(),
                );
            } else if c == 'b' || c == 'B' {
                self.input.pos += 1;
                self.input.col += 1;
                let mut n = self.input.peek();
                while n == '0' || n == '1' {
                    self.input.pos += 1;
                    self.input.col += 1;
                    literal.push(n);
                    if self.input.is_empty() {
                        break;
                    }
                    n = self.input.peek();
                }
                return Token::new(
                    format!("0{}{}", c, literal.clone()),
                    TokenType::Integer {
                        value: u64::from_str_radix(&literal, 2).unwrap(),
                    },
                    self.get_position_with_begin(begin_pos),
                    self.input.file.clone(),
                );
            } else if c.is_digit(8) {
                self.input.pos += 1;
                self.input.col += 1;
                literal.push(c);
                let mut n = self.input.peek();
                while n.is_digit(8) {
                    self.input.pos += 1;
                    self.input.col += 1;
                    literal.push(n);
                    if self.input.is_empty() {
                        break;
                    }
                    n = self.input.peek();
                }
                return Token::new(
                    format!("0{}", literal.clone()),
                    TokenType::Integer {
                        value: u64::from_str_radix(&literal, 8).unwrap(),
                    },
                    self.get_position_with_begin(begin_pos),
                    self.input.file.clone(),
                );
            } else {
                literal.push('0');
            }
        } else {
            literal.push(c);
            c = self.input.peek();
            while c.is_ascii_digit() {
                self.input.pos += 1;
                self.input.col += 1;
                literal.push(c);
                if self.input.is_empty() {
                    break;
                }
                c = self.input.peek();
            }
        }
        c = self.input.peek();
        if c == '.' {
            self.input.pos += 1;
            self.input.col += 1;
            literal.push(c);
            c = self.input.peek();
            while c.is_ascii_digit() {
                self.input.pos += 1;
                self.input.col += 1;
                literal.push(c);
                if self.input.is_empty() {
                    break;
                }
                c = self.input.peek();
            }
            if c == 'e' || c == 'E' {
                self.input.pos += 1;
                self.input.col += 1;
                literal.push(c);
                c = self.input.peek();
                while c.is_ascii_digit() {
                    self.input.pos += 1;
                    self.input.col += 1;
                    literal.push(c);
                    if self.input.is_empty() {
                        break;
                    }
                    c = self.input.peek();
                }
            }
            Token::new(
                literal.clone(),
                TokenType::Decimal {
                    value: literal.parse::<f64>().unwrap(),
                },
                self.get_position_with_begin(begin_pos),
                self.input.file.clone(),
            )
        } else if c == 'e' || c == 'E' {
            self.input.pos += 1;
            self.input.col += 1;
            literal.push(c);
            c = self.input.peek();
            while c.is_ascii_digit() {
                self.input.pos += 1;
                self.input.col += 1;
                literal.push(c);
                if self.input.is_empty() {
                    break;
                }
                c = self.input.peek();
            }
            Token::new(
                literal.clone(),
                TokenType::Decimal {
                    value: literal.parse::<f64>().unwrap(),
                },
                self.get_position_with_begin(begin_pos),
                self.input.file.clone(),
            )
        } else {
            Token::new(
                literal.clone(),
                TokenType::Integer {
                    value: literal.parse::<u64>().unwrap(),
                },
                self.get_position_with_begin(begin_pos),
                self.input.file.clone(),
            )
        }
    }
}
impl Iterator for Lexer {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        if self.is_empty() {
            return None;
        }
        self.skip_whitechars();
        if self.is_empty() {
            return None;
        }
        let c = self.input.peek();
        if c.is_ascii_digit() {
            Some(self.parse_number())
        } else if c == '(' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                '('.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::OpenParen,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == ')' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                ')'.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::CloseParen,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == '[' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                '['.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::OpenBracket,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == ']' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                ']'.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::CloseBracket,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == '{' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                '{'.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::OpenBrace,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == '}' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                '}'.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::CloseBrace,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == ':' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                ':'.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::Colon,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == ';' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                ';'.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::SemiColon,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == ',' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                ','.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::Comma,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == '@' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                '@'.to_string(),
                TokenType::Separator {
                    separator: SeparatorType::At,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == '?' {
            let position = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            Some(Token::new(
                '?'.to_string(),
                TokenType::Operator {
                    operator: OperatorType::QuestionMark,
                },
                position,
                self.input.file.clone(),
            ))
        } else if c == '+' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "++".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Inc,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "+=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::PlusAssign,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '+'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Plus,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '-' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '-' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "--".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Dec,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else if self.input.peek() == '>' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "->".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Arrow,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "-=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::MinusAssign,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '-'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Minus,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '*' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "*=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::MultiplyAssign,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '*'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Multiply,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '/' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "/=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::DivideAssign,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '/'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Divide,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '%' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "%=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::ModulusAssign,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '%'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Modulus,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '=' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "==".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Equal,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '='.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Assign,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '!' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "!=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::NotEqual,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '!'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Not,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '<' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '<' {
                let pos = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                if self.input.peek() == '=' {
                    let position = self.get_position_with_begin(begin_pos);
                    self.input.pos += 1;
                    self.input.col += 1;
                    Some(Token::new(
                        "<<=".to_string(),
                        TokenType::Operator {
                            operator: OperatorType::LeftShiftAssign,
                        },
                        position,
                        self.input.file.clone(),
                    ))
                } else {
                    Some(Token::new(
                        "<<".to_string(),
                        TokenType::Operator {
                            operator: OperatorType::LeftShift,
                        },
                        pos,
                        self.input.file.clone(),
                    ))
                }
            } else if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "<=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::LessEqual,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '<'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Less,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '>' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '>' {
                let pos = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                if self.input.peek() == '=' {
                    let position = self.get_position_with_begin(begin_pos);
                    self.input.pos += 1;
                    self.input.col += 1;
                    Some(Token::new(
                        ">>=".to_string(),
                        TokenType::Operator {
                            operator: OperatorType::RightShiftAssign,
                        },
                        position,
                        self.input.file.clone(),
                    ))
                } else {
                    Some(Token::new(
                        ">>".to_string(),
                        TokenType::Operator {
                            operator: OperatorType::RightShift,
                        },
                        pos,
                        self.input.file.clone(),
                    ))
                }
            } else if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    ">=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::GreaterEqual,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '>'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Greater,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '&' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '&' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "&&".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::And,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "&=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::BitAndAssign,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '&'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::BitAnd,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else if c == '|' {
            let begin_pos = self.input.get_current_position();
            self.input.pos += 1;
            self.input.col += 1;
            if self.input.peek() == '|' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "||".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::Or,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else if self.input.peek() == '=' {
                let position = self.get_position_with_begin(begin_pos);
                self.input.pos += 1;
                self.input.col += 1;
                Some(Token::new(
                    "|=".to_string(),
                    TokenType::Operator {
                        operator: OperatorType::BitOrAssign,
                    },
                    position,
                    self.input.file.clone(),
                ))
            } else {
                Some(Token::new(
                    '|'.to_string(),
                    TokenType::Operator {
                        operator: OperatorType::BitOr,
                    },
                    begin_pos,
                    self.input.file.clone(),
                ))
            }
        } else {
            None
        }
    }
}
