use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::{AssignmentOperator, BinaryOperator, Expression, UnaryOperator},
        statement::{FunctionBody, Parameter, Pattern, Statement},
    },
    diag::TrussDiagnosticEngine,
    lexer::{CharStream, Lexer},
    parser::Parser,
};

fn create_engine() -> Rc<RefCell<TrussDiagnosticEngine>> {
    Rc::new(RefCell::new(TrussDiagnosticEngine::new()))
}

#[test]
fn test_parse_function_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { 1 } func test2(_ a: Int32) { a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
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
fn test_parse_function_decl_with_label() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(_ a: Int32) { a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters[0].borrow().label.as_ref().unwrap().value, "_");
        assert_eq!(parameters[0].borrow().name.value, "a");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_decl_with_custom_label() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(label1 a: Int32, label2 b: String) { a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(
            parameters[0].borrow().label.as_ref().unwrap().value,
            "label1"
        );
        assert_eq!(parameters[0].borrow().name.value, "a");
        assert_eq!(
            parameters[1].borrow().label.as_ref().unwrap().value,
            "label2"
        );
        assert_eq!(parameters[1].borrow().name.value, "b");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_decl_with_multiple_underscore_labels() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(_ v1: Int32, _ v2: String) { v1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters[0].borrow().label.as_ref().unwrap().value, "_");
        assert_eq!(parameters[0].borrow().name.value, "v1");
        assert_eq!(parameters[1].borrow().label.as_ref().unwrap().value, "_");
        assert_eq!(parameters[1].borrow().name.value, "v2");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_call_with_label() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(label1 a: Int32) {} func test2() { test(label1: 42) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call { parameters, .. } = &*expression.borrow()
    {
        assert_eq!(parameters[0].label.as_ref().unwrap().value, "label1");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_call_without_label() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(_ a: Int32) {} func test2() { test(42) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call { parameters, .. } = &*expression.borrow()
    {
        assert!(parameters[0].label.is_none());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_underscore_pattern() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { for _ in 1..<3 {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::For { pattern, .. } = &*statements[0].borrow()
    {
        assert!(matches!(*pattern.clone(), Pattern::Ignore));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_variable_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { let a = 1 a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { name, .. } = &*statements[0].borrow()
    {
        assert_eq!(name.value, "a");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() {} func test2() { test() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
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
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() {let a = 1; ++a a-- }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
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
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() {let a = 1 a+1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
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
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() {let a = 1 a += 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
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
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { return \n return 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        assert!(matches!(
            &*statements[0].borrow(),
            Statement::Return { value: None, .. }
        ));
        assert!(matches!(
            &*statements[1].borrow(),
            Statement::Return { value, .. } if value.is_some()
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_for() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { for _ in 1..<3 {} for i in 0..2 {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
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

#[test]
fn test_parse_char_literal() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new("'a'".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow()
        && let Expression::CharLiteral { token } = &*expression.borrow()
    {
        assert_eq!(token.value, "'a'");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_variable_decl_at_eof() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a: Never".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "a");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_variable_decl_no_type_at_eof() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "a");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_variable_decl_in_function_at_eof() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Never }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { name, .. } = &*statements[0].borrow()
    {
        assert_eq!(name.value, "a");
    } else {
        panic!();
    }
}
