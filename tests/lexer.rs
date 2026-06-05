use std::{cell::RefCell, rc::Rc};

use truss::{
    diag::TrussDiagnosticEngine,
    lexer::{
        CharStream, Lexer,
        token::{KeywordType, SeparatorType, TokenType},
    },
};

#[test]
fn test_parse_integer() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(
            "0x1f 0b11 012 12 0.5 1.5e3 1".to_string(),
            Rc::new("".to_string()),
        ),
        engine,
    );
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
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("abc a_".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(tokens[0].ty, TokenType::Identifier);
    assert_eq!(tokens[1].ty, TokenType::Identifier);
}
#[test]
fn test_parse_keyword() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("func let".to_string(), Rc::new("".to_string())),
        engine,
    );
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
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("'a' '\\n'".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(tokens[0].ty, TokenType::CharLiteral { value: 'a' });
    assert_eq!(tokens[1].ty, TokenType::CharLiteral { value: '\n' });
}

#[test]
fn test_parse_super_keyword() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("super".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(
        tokens[0].ty,
        TokenType::Keyword {
            keyword: KeywordType::SuperKw
        }
    );
}

#[test]
fn test_parse_hash_token() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("#if #else #endif".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(tokens.len(), 6);
    assert_eq!(tokens[0].ty, TokenType::Separator { separator: SeparatorType::Hash });
    assert_eq!(tokens[1].ty, TokenType::Keyword { keyword: KeywordType::If });
    assert_eq!(tokens[2].ty, TokenType::Separator { separator: SeparatorType::Hash });
    assert_eq!(tokens[3].ty, TokenType::Keyword { keyword: KeywordType::Else });
    assert_eq!(tokens[4].ty, TokenType::Separator { separator: SeparatorType::Hash });
    assert_eq!(tokens[5].value, "endif");
}

#[test]
fn test_parse_hash_identifier_separated() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("# if".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].ty, TokenType::Separator { separator: SeparatorType::Hash });
    assert_eq!(tokens[1].ty, TokenType::Keyword { keyword: KeywordType::If });
}

#[test]
fn test_parse_sizeof_keyword() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("sizeof".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].ty, TokenType::Keyword { keyword: KeywordType::SizeOf });
}

#[test]
fn test_parse_asm_keyword() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("asm".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].ty, TokenType::Keyword { keyword: KeywordType::Asm });
}

#[test]
fn test_parse_yield_keyword() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("yield".to_string(), Rc::new("".to_string())),
        engine,
    );
    let tokens = lexer.parse();
    assert_eq!(tokens.len(), 1);
    assert_eq!(
        tokens[0].ty,
        TokenType::Keyword {
            keyword: KeywordType::Yield
        }
    );
    assert_eq!(tokens[0].value, "yield");
}
