use std::rc::Rc;

use truss::lexer::{CharStream, Lexer, token::TokenType};

#[test]
fn test_parse_integer() {
    let mut lexer = Lexer::new(CharStream::new(
        "0x1f 0b11 012 12 0.5 1.5e3 1".to_string(),
        Rc::new("".to_string()),
    ));
    assert_eq!(lexer.next().unwrap().ty, TokenType::Integer { value: 31 });
    assert_eq!(lexer.next().unwrap().ty, TokenType::Integer { value: 3 });
    assert_eq!(lexer.next().unwrap().ty, TokenType::Integer { value: 10 });
    assert_eq!(lexer.next().unwrap().ty, TokenType::Integer { value: 12 });
    assert_eq!(lexer.next().unwrap().ty, TokenType::Decimal { value: 0.5 });
    assert_eq!(
        lexer.next().unwrap().ty,
        TokenType::Decimal { value: 1.5e3 }
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
