use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::Expression,
        statement::{FunctionBody, Statement},
    },
    diag::TrussDiagnosticEngine,
    id::CrateId,
    krate::Crate,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
};

fn create_engine() -> Rc<RefCell<TrussDiagnosticEngine>> {
    Rc::new(RefCell::new(TrussDiagnosticEngine::new()))
}

#[test]
fn test_variable_resolver() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() { let a = 1 a }".to_string(),
        Rc::new("".to_string()),
    ));
    let engine = create_engine();
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string(), CrateId { id: 0 }))),
        engine,
    );
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[1].borrow()
        && let Expression::Variable { symbol, .. } = &*expression.borrow()
    {
        assert_ne!(*symbol, None);
    } else {
        panic!();
    }
}

#[test]
fn test_function_resolver() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() {} func test2() { test() }".to_string(),
        Rc::new("".to_string()),
    ));
    let engine = create_engine();
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string(), CrateId { id: 0 }))),
        engine,
    );
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call { callee, .. } = &*expression.borrow()
        && let Expression::Variable { symbol, .. } = &*callee.borrow()
    {
        assert_ne!(*symbol, None);
    } else {
        panic!();
    }
}

#[test]
fn test_underscore_variable_no_symbol() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() { let _ = 1 let _ = 2 }".to_string(),
        Rc::new("".to_string()),
    ));
    let engine = create_engine();
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string(), CrateId { id: 0 }))),
        engine,
    );
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_underscore_parameter_no_symbol() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test(_ _: Int32) { }".to_string(),
        Rc::new("".to_string()),
    ));
    let engine = create_engine();
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string(), CrateId { id: 0 }))),
        engine,
    );
    resolver.resolve(&program, "test".to_string());
}
