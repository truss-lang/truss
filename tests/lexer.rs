use std::rc::Rc;

use truss::lexer::{
    CharStream, Lexer,
    token::{KeywordType, TokenType},
};

#[test]
fn test_parse_integer() {
    let mut lexer = Lexer::new(CharStream::new(
        "0x1f 0b11 012 12 0.5 1.5e3 1".to_string(),
        Rc::new("".to_string()),
    ));
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::IntegerLiteral { value: 31 }
    );
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::IntegerLiteral { value: 3 }
    );
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::IntegerLiteral { value: 10 }
    );
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::IntegerLiteral { value: 12 }
    );
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::DecimalLiteral { value: 0.5 }
    );
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::DecimalLiteral { value: 1.5e3 }
    );
    assert_eq!(lexer.next().unwrap().position.len, 1);
}

#[test]
fn test_parse_identifier() {
    let mut lexer = Lexer::new(CharStream::new(
        "abc a_".to_string(),
        Rc::new("".to_string()),
    ));
    assert_eq!(lexer.next().unwrap().ty, TokenType::Identifier);
    assert_eq!(lexer.next().unwrap().ty, TokenType::Identifier);
}
#[test]
fn test_parse_keyword() {
    let mut lexer = Lexer::new(CharStream::new(
        "func let".to_string(),
        Rc::new("".to_string()),
    ));
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::Keyword {
            keyword: KeywordType::Func
        }
    );
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::Keyword {
            keyword: KeywordType::Let
        }
    );
}

#[test]
fn test_parse_char_literal() {
    let mut lexer = Lexer::new(CharStream::new(
        "'a' '\\n'".to_string(),
        Rc::new("".to_string()),
    ));
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::CharLiteral { value: 'a' }
    );
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::CharLiteral { value: '\n' }
    );
}
