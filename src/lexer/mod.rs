pub mod token;

use std::rc::Rc;

use token::{Position, Token, TokenType};

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
    fn parse_number(&mut self) -> Token {
        let mut c = self.input.peek();
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
                    self.input.get_current_position(),
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
                    self.input.get_current_position(),
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
                    self.input.get_current_position(),
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
                self.input.get_current_position(),
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
                self.input.get_current_position(),
                self.input.file.clone(),
            )
        } else {
            Token::new(
                literal.clone(),
                TokenType::Integer {
                    value: literal.parse::<u64>().unwrap(),
                },
                self.input.get_current_position(),
                self.input.file.clone(),
            )
        }
    }
}
impl Iterator for Lexer {
    type Item = Token;
    fn next(&mut self) -> Option<Token> {
        self.skip_whitechars();
        let c = self.input.peek();
        if c.is_ascii_digit() {
            Some(self.parse_number())
        } else {
            None
        }
    }
}
