use std::rc::Rc;

use truss::{
    ast::{
        expression::{AssignmentOperator, BinaryOperator, Expression, UnaryOperator},
        statement::{Parameter, Pattern, Statement},
    },
    lexer::{CharStream, Lexer},
    parser::Parser,
};

#[test]
fn test_parse_function_decl() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() -> Int32 { 1 } func test2(a: Int32) { a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "test");
    } else {
        panic!();
    }
    if let Statement::FunctionDecl {
        name, parameters, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(name.value, "test2");
        assert!(matches!(
            &*parameters[0].borrow(),
            Parameter { name:name2, .. } if name2.clone().value == "a"
        ));
    } else {
        panic!();
    }
}
#[test]
fn test_parse_variable_decl() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() -> Int32 { let a = 1 a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let Expression::Block { statements } = &*body.borrow()
        && let Statement::VariableDecl { name, .. } = &*statements[0].borrow()
    {
        assert_eq!(name.value, "a");
    } else {
        panic!();
    }
}
#[test]
fn test_parse_function_call() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() {} func test2() { test() }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let Expression::Block { statements } = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call { callee, .. } = &*expression.borrow()
        && let Expression::Variable { name, .. } = &*callee.borrow()
    {
        assert_eq!(name.value, "test");
    } else {
        panic!();
    }
}
#[test]
fn test_parse_unary() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() {let a = 1; ++a a-- }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let Expression::Block { statements } = &*body.borrow()
    {
        let Statement::ExpressionStatement { expression } = &*statements[2].borrow() else {
            panic!();
        };
        assert!(matches!(
            *expression.borrow(),
            Expression::Unary {
                operator: UnaryOperator::Inc,
                is_prefix: true,
                ..
            }
        ));
        let Statement::ExpressionStatement {
            expression: expression2,
        } = &*statements[3].borrow()
        else {
            panic!();
        };
        assert!(matches!(
            *expression2.borrow(),
            Expression::Unary {
                operator: UnaryOperator::Dec,
                is_prefix: false,
                ..
            }
        ));
    } else {
        panic!();
    }
}
#[test]
fn test_parse_binary() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() {let a = 1 a+1 }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let Expression::Block { statements } = &*body.borrow()
    {
        let Statement::ExpressionStatement { expression } = &*statements[1].borrow() else {
            panic!();
        };
        assert!(matches!(
            *expression.borrow(),
            Expression::Binary {
                operator: BinaryOperator::Plus,
                ..
            }
        ));
    } else {
        panic!();
    }
}
#[test]
fn test_parse_assignment() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() {let a = 1 a += 2 }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let Expression::Block { statements } = &*body.borrow()
    {
        let Statement::ExpressionStatement { expression } = &*statements[1].borrow() else {
            panic!();
        };
        assert!(matches!(
            *expression.borrow(),
            Expression::Assignment {
                operator: AssignmentOperator::PlusAssign,
                ..
            }
        ));
    } else {
        panic!();
    }
}
#[test]
fn test_parse_return() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() { return \n return 1 }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let Expression::Block { statements } = &*body.borrow()
    {
        assert!(matches!(
            &*statements[0].borrow(),
            Statement::Return { value: None }
        ));
        assert!(matches!(
            &*statements[1].borrow(),
            Statement::Return { value } if value.is_some()
        ));
    } else {
        panic!();
    }
}
#[test]
fn test_parse_for() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test() { for _ in 1..<3 {} for i in 0..2 {} }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let Expression::Block { statements } = &*body.borrow()
    {
        assert!(matches!(
            &*statements[0].borrow(),
            Statement::For {
                pattern,
                ..
            } if Pattern::Ignore == *pattern.clone()
        ));
    } else {
        panic!();
    }
}
