use std::rc::Rc;

use truss::{
    ast::statement::Statement,
    lexer::{
        CharStream, Lexer,
        token::{Token, TokenType},
    },
    parser::{self, Parser},
};

#[test]
fn test_parse_function_decl() {
    let lexer = Lexer::new(CharStream::new(
        "func test() func test2()".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer);
    let program = parser.parse().unwrap();
    assert!(matches!(
        &program.statements[0],
        Statement::FunctionDecl {
            name: Token {
                value,
                ty: TokenType::Identifier,
                ..
            },
            ..
        } if value == "test"
    ));
    assert!(matches!(
        &program.statements[1],
        Statement::FunctionDecl {
            name: Token {
                value,
                ty: TokenType::Identifier,
                ..
            },
            ..
        } if value == "test2"
    ));
}
