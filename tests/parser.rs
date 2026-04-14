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
    assert!(
        if let Statement::FunctionDecl { name, .. } = &*program.statements[0].borrow() {
            name.value == "test"
        } else {
            false
        }
    );
    assert!(
        if let Statement::FunctionDecl { name, .. } = &*program.statements[1].borrow() {
            name.value == "test2"
        } else {
            false
        }
    );
}
#[test]
fn test_parse_variable_decl() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() -> Int32 { let a = 1 a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    assert!(
        if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
            && let Expression::Block { statements } = &*body.borrow()
            && let Statement::VariableDecl { name, .. } = &*statements[0].borrow()
        {
            name.value == "a"
        } else {
            false
        }
    );
}
#[test]
fn test_parse_function_call() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() {} func test2() { test() }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    assert!(
        if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
            && let Expression::Block { statements } = &*body.borrow()
            && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
            && let Expression::Call { expression, .. } = &*expression.borrow()
            && let Expression::Variable { name, .. } = &*expression.borrow()
        {
            name.clone().unwrap().value == "test"
        } else {
            false
        }
    );
}
