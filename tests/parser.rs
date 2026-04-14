use std::rc::Rc;

use truss::{
    ast::{expression::Expression, statement::Statement},
    lexer::{CharStream, Lexer},
    parser::Parser,
};

#[test]
fn test_parse_function_decl() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() -> Int32 { 1 } func test2() {}".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "test");
    }
    if let Statement::FunctionDecl { name, .. } = &*program.statements[1].borrow() {
        assert_eq!(name.value, "test2");
    }
}
#[test]
fn test_parse_variable_decl() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() -> Int32 { let a = 1 1 }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let Expression::Block { statements } = &*body.borrow()
        && let Statement::VariableDecl { name, .. } = &*statements[0].borrow()
    {
        assert_eq!(name.value, "a");
    }
}
