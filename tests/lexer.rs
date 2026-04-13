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
    let tokens = lexer.parse();
    assert_eq!(tokens[0].value, "0x1f".to_string());
    assert_eq!(tokens[1].ty, TokenType::IntegerLiteral { value: 3 });
    assert_eq!(tokens[2].ty, TokenType::IntegerLiteral { value: 10 });
    assert_eq!(tokens[3].ty, TokenType::IntegerLiteral { value: 12 });
    assert_eq!(tokens[4].ty, TokenType::DecimalLiteral { value: 0.5 });
    assert_eq!(tokens[5].ty, TokenType::DecimalLiteral { value: 1.5e3 });
    assert_eq!(tokens[6].position.len, 1);
}

#[test]
fn test_parse_identifier() {
    let mut lexer = Lexer::new(CharStream::new(
        "abc a_".to_string(),
        Rc::new("".to_string()),
    ));
    let tokens = lexer.parse();
    assert_eq!(tokens[0].ty, TokenType::Identifier);
    assert_eq!(tokens[1].ty, TokenType::Identifier);
}
#[test]
fn test_parse_keyword() {
    let mut lexer = Lexer::new(CharStream::new(
        "func let".to_string(),
        Rc::new("".to_string()),
    ));
    let tokens = lexer.parse();
    assert_eq!(
        tokens[0].ty,
        TokenType::Keyword {
            keyword: KeywordType::Func
        }
    );
    assert_eq!(
        tokens[1].ty,
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
    let tokens = lexer.parse();
    assert_eq!(tokens[0].ty, TokenType::CharLiteral { value: 'a' });
    assert_eq!(tokens[1].ty, TokenType::CharLiteral { value: '\n' });
}
