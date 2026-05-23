use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::{AssignmentOperator, BinaryOperator, CastKind, Expression, UnaryOperator},
        statement::{
            AccessModifier, FunctionBody, Modifier, ModifierType, Parameter, Pattern, Statement,
            VariadicKind,
        },
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine},
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

#[test]
fn test_parse_extern_block_single_func() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" { func printf(_ format: String) -> Int32 }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExternBlock { linkage, items, .. } = &*program.statements[0].borrow() {
        assert_eq!(linkage.value, r#""C""#);
        assert_eq!(items.len(), 1);
        if let Statement::FunctionDecl { name, .. } = &*items[0].borrow() {
            assert_eq!(name.value, "printf");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_extern_block_multiple_decls() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" { func printf(_ format: String) -> Int32 func malloc(_ size: Int32) -> Int64 let errno: Int32 }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExternBlock { items, .. } = &*program.statements[0].borrow() {
        assert_eq!(items.len(), 3);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_extern_single_func() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" func printf(_ format: String) -> Int32"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExternDecl {
        linkage, statement, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(linkage.value, r#""C""#);
        if let Statement::FunctionDecl { name, .. } = &*statement.borrow() {
            assert_eq!(name.value, "printf");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_extern_single_let() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" let errno: Int32"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExternDecl {
        linkage, statement, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(linkage.value, r#""C""#);
        if let Statement::VariableDecl { name, .. } = &*statement.borrow() {
            assert_eq!(name.value, "errno");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_extern_single_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" var globalCounter: Int32"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExternDecl {
        linkage, statement, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(linkage.value, r#""C""#);
        if let Statement::VariableDecl { name, .. } = &*statement.borrow() {
            assert_eq!(name.value, "globalCounter");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_extern_variadic_func() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" func printf(_ formatter: String, args: String...)"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExternDecl {
        linkage, statement, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(linkage.value, r#""C""#);
        if let Statement::FunctionDecl { parameters, .. } = &*statement.borrow() {
            assert_eq!(parameters.len(), 2);
            assert_eq!(
                parameters[0].borrow().variadic_kind,
                VariadicKind::NotVariadic
            );
            assert_eq!(
                parameters[1].borrow().variadic_kind,
                VariadicKind::TypedVariadic
            );
            assert_eq!(parameters[1].borrow().name.value, "args");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_extern_variadic_func_bare() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" func printf(_ formatter: String, ...)"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExternDecl {
        linkage, statement, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(linkage.value, r#""C""#);
        if let Statement::FunctionDecl { parameters, .. } = &*statement.borrow() {
            assert_eq!(parameters.len(), 2);
            assert_eq!(
                parameters[0].borrow().variadic_kind,
                VariadicKind::NotVariadic
            );
            assert_eq!(
                parameters[1].borrow().variadic_kind,
                VariadicKind::BareVariadic
            );
            assert_eq!(parameters[1].borrow().name.value, "...");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_deref() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = *ptr }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression,
            operator,
            is_prefix,
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::Deref);
        assert!(is_prefix);
        if let Expression::Variable { name, .. } = &*expression.borrow() {
            assert_eq!(name.value, "ptr");
        } else {
            panic!("Expected variable expression");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_deref_nested() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = **ptr }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression: outer_expr,
            operator,
            is_prefix,
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::Deref);
        assert!(is_prefix);
        if let Expression::Unary {
            expression: inner_expr,
            operator: inner_op,
            is_prefix: inner_prefix,
        } = &*outer_expr.borrow()
        {
            assert_eq!(inner_op, &UnaryOperator::Deref);
            assert!(inner_prefix);
            if let Expression::Variable { name, .. } = &*inner_expr.borrow() {
                assert_eq!(name.value, "ptr");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected nested unary expression");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_deref_with_binary() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = *ptr + 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Binary { left, operator, .. } = &*init_expr.borrow()
    {
        assert_eq!(operator, &BinaryOperator::Plus);
        if let Expression::Unary {
            expression,
            operator: unary_op,
            ..
        } = &*left.borrow()
        {
            assert_eq!(unary_op, &UnaryOperator::Deref);
            if let Expression::Variable { name, .. } = &*expression.borrow() {
                assert_eq!(name.value, "ptr");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected unary expression on left");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_regular() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 as Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Cast {
            expression,
            target_type,
            kind,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(kind, &CastKind::Regular);
        assert!(matches!(
            *expression.borrow(),
            Expression::IntegerLiteral { .. }
        ));
        assert!(
            matches!(&*target_type.borrow(), Expression::Type { name, .. } if name.value == "Int32")
        );
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_conditional() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 as? Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Cast {
            expression,
            target_type,
            kind,
            kind_tokens,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(kind, &CastKind::Conditional);
        assert!(kind_tokens.is_some());
        assert_eq!(kind_tokens.as_ref().unwrap().0.value, "?");
        assert!(matches!(
            *expression.borrow(),
            Expression::IntegerLiteral { .. }
        ));
        assert!(
            matches!(&*target_type.borrow(), Expression::Type { name, .. } if name.value == "Int32")
        );
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_force() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 as! Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Cast {
            expression,
            target_type,
            kind,
            kind_tokens,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(kind, &CastKind::Force);
        assert!(kind_tokens.is_some());
        assert_eq!(kind_tokens.as_ref().unwrap().0.value, "!");
        assert!(matches!(
            *expression.borrow(),
            Expression::IntegerLiteral { .. }
        ));
        assert!(
            matches!(&*target_type.borrow(), Expression::Type { name, .. } if name.value == "Int32")
        );
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_force_bitcast() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 as!! Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Cast {
            expression,
            target_type,
            kind,
            kind_tokens,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(kind, &CastKind::ForceBitcast);
        assert!(kind_tokens.is_some());
        assert_eq!(kind_tokens.as_ref().unwrap().0.value, "!");
        assert_eq!(kind_tokens.as_ref().unwrap().1.value, "!");
        assert!(matches!(
            *expression.borrow(),
            Expression::IntegerLiteral { .. }
        ));
        assert!(
            matches!(&*target_type.borrow(), Expression::Type { name, .. } if name.value == "Int32")
        );
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_chained() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 as Int32 as Float64 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Cast {
            expression: outer_expr,
            target_type: outer_target,
            kind: outer_kind,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(outer_kind, &CastKind::Regular);
        assert!(
            matches!(&*outer_target.borrow(), Expression::Type { name, .. } if name.value == "Float64")
        );
        if let Expression::Cast {
            expression: inner_expr,
            target_type: inner_target,
            kind: inner_kind,
            ..
        } = &*outer_expr.borrow()
        {
            assert_eq!(inner_kind, &CastKind::Regular);
            assert!(
                matches!(&*inner_target.borrow(), Expression::Type { name, .. } if name.value == "Int32")
            );
            assert!(matches!(
                *inner_expr.borrow(),
                Expression::IntegerLiteral { .. }
            ));
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_precedence() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 + 2 as Float64 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Binary {
            left,
            operator,
            right,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &BinaryOperator::Plus);
        assert!(matches!(*left.borrow(), Expression::IntegerLiteral { .. }));
        if let Expression::Cast { target_type, .. } = &*right.borrow() {
            assert!(
                matches!(&*target_type.borrow(), Expression::Type { name, .. } if name.value == "Float64")
            );
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_to_pointer() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = ptr as Int32* }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Cast { target_type, .. } = &*init_expr.borrow()
    {
        assert!(matches!(
            *target_type.borrow(),
            Expression::PointerType { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_cast_conditional_to_pointer() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = ptr as? Int32* }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Cast {
            kind, target_type, ..
        } = &*init_expr.borrow()
    {
        assert_eq!(kind, &CastKind::Conditional);
        assert!(matches!(
            *target_type.borrow(),
            Expression::PointerType { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_decl_empty() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("struct Point {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert!(body.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_decl_with_fields() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { let x: Int32 let y: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 2);
        if let Statement::VariableDecl {
            name: field_name, ..
        } = &*body[0].borrow()
        {
            assert_eq!(field_name.value, "x");
        } else {
            panic!("Expected VariableDecl for field x");
        }
        if let Statement::VariableDecl {
            name: field_name, ..
        } = &*body[1].borrow()
        {
            assert_eq!(field_name.value, "y");
        } else {
            panic!("Expected VariableDecl for field y");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_decl_with_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Calculator { func add(a: Int32, b: Int32) -> Int32 { a + b } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Calculator");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl {
            name: func_name, ..
        } = &*body[0].borrow()
        {
            assert_eq!(func_name.value, "add");
        } else {
            panic!("Expected FunctionDecl for method add");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_decl_mixed_members() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Person { let name: String let age: Int32 func greet() { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Person");
        assert_eq!(body.len(), 3);
        if let Statement::VariableDecl {
            name: field_name, ..
        } = &*body[0].borrow()
        {
            assert_eq!(field_name.value, "name");
        } else {
            panic!("Expected VariableDecl for field name");
        }
        if let Statement::VariableDecl {
            name: field_name, ..
        } = &*body[1].borrow()
        {
            assert_eq!(field_name.value, "age");
        } else {
            panic!("Expected VariableDecl for field age");
        }
        if let Statement::FunctionDecl {
            name: func_name, ..
        } = &*body[2].borrow()
        {
            assert_eq!(func_name.value, "greet");
        } else {
            panic!("Expected FunctionDecl for method greet");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_decl_nested_function_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Math { func square(x: Int32) -> Int32 { return x * x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Math");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl {
            body: func_body, ..
        } = &*body[0].borrow()
            && let FunctionBody::Statements(statements) = &*func_body.borrow()
        {
            assert_eq!(statements.len(), 1);
            assert!(matches!(&*statements[0].borrow(), Statement::Return { .. }));
        } else {
            panic!("Expected FunctionDecl with statements");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_member_access_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { obj.field }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::MemberAccess { object, member, .. } = &*expression.borrow()
    {
        if let Expression::Variable { name, .. } = &*object.borrow() {
            assert_eq!(name.value, "obj");
        } else {
            panic!("Expected variable expression");
        }
        assert_eq!(member.value, "field");
    } else {
        panic!("Expected MemberAccess expression");
    }
}

#[test]
fn test_parse_member_access_chain() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { a.b.c }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::MemberAccess { object, member, .. } = &*expression.borrow()
    {
        assert_eq!(member.value, "c");
        if let Expression::MemberAccess {
            object: inner_obj,
            member: inner_member,
            ..
        } = &*object.borrow()
        {
            assert_eq!(inner_member.value, "b");
            if let Expression::Variable { name, .. } = &*inner_obj.borrow() {
                assert_eq!(name.value, "a");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected nested MemberAccess expression");
        }
    } else {
        panic!("Expected MemberAccess expression");
    }
}

#[test]
fn test_parse_member_access_with_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { obj.method() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call { callee, .. } = &*expression.borrow()
        && let Expression::MemberAccess { object, member, .. } = &*callee.borrow()
    {
        if let Expression::Variable { name, .. } = &*object.borrow() {
            assert_eq!(name.value, "obj");
        } else {
            panic!("Expected variable expression");
        }
        assert_eq!(member.value, "method");
    } else {
        panic!("Expected Call expression on MemberAccess");
    }
}

#[test]
fn test_parse_member_access_in_assignment() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { obj.field = 42 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Assignment { left, .. } = &*expression.borrow()
        && let Expression::MemberAccess { object, member, .. } = &*left.borrow()
    {
        if let Expression::Variable { name, .. } = &*object.borrow() {
            assert_eq!(name.value, "obj");
        } else {
            panic!("Expected variable expression");
        }
        assert_eq!(member.value, "field");
    } else {
        panic!("Expected Assignment expression with MemberAccess on left");
    }
}

#[test]
fn test_parse_init_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { init(x: Int32, y: Int32) {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 1);
        if let Statement::InitDecl { parameters, .. } = &*body[0].borrow() {
            assert_eq!(parameters.len(), 2);
            assert_eq!(parameters[0].borrow().name.value, "x");
            assert_eq!(parameters[1].borrow().name.value, "y");
        } else {
            panic!("Expected InitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_init_decl_empty_params() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { init() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert_eq!(body.len(), 1);
        if let Statement::InitDecl { parameters, .. } = &*body[0].borrow() {
            assert_eq!(parameters.len(), 0);
        } else {
            panic!("Expected InitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_deinit_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { deinit {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 1);
        if let Statement::DeinitDecl { .. } = &*body[0].borrow() {
        } else {
            panic!("Expected DeinitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_struct_decl_with_init_deinit() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { let x: Int32 init(x: Int32) { } deinit { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 3);
        if let Statement::VariableDecl {
            name: field_name, ..
        } = &*body[0].borrow()
        {
            assert_eq!(field_name.value, "x");
        } else {
            panic!("Expected VariableDecl for field x");
        }
        if let Statement::InitDecl { parameters, .. } = &*body[1].borrow() {
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].borrow().name.value, "x");
        } else {
            panic!("Expected InitDecl");
        }
        if let Statement::DeinitDecl { .. } = &*body[2].borrow() {
        } else {
            panic!("Expected DeinitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_type_instantiation_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { Point(x: 1, y: 2) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Call {
            callee, parameters, ..
        } = &*expression.borrow()
        && let Expression::Variable { name, .. } = &*callee.borrow()
    {
        assert_eq!(name.value, "Point");
        assert_eq!(parameters.len(), 2);
        assert_eq!(
            parameters[0].label.as_ref().map(|t| t.value.as_str()),
            Some("x")
        );
        assert_eq!(
            parameters[1].label.as_ref().map(|t| t.value.as_str()),
            Some("y")
        );
    } else {
        panic!("Expected Call with Variable callee named Point");
    }
}

fn collect_modifiers(stmt: &Statement) -> Vec<Modifier> {
    match stmt {
        Statement::FunctionDecl { modifiers, .. }
        | Statement::VariableDecl { modifiers, .. }
        | Statement::StructDecl { modifiers, .. }
        | Statement::InitDecl { modifiers, .. }
        | Statement::DeinitDecl { modifiers, .. } => modifiers.clone(),
        _ => vec![],
    }
}

fn assert_has_access_modifier(stmt: &Statement, expected: AccessModifier) {
    let modifiers = collect_modifiers(stmt);
    assert!(
        modifiers
            .iter()
            .any(|m| m.ty == ModifierType::Access(expected.clone())),
        "Expected access modifier {:?} not found in {:?}",
        expected,
        modifiers,
    );
}

fn assert_has_no_modifiers(stmt: &Statement) {
    let modifiers = collect_modifiers(stmt);
    assert!(
        modifiers.is_empty(),
        "Expected no modifiers, found {:?}",
        modifiers,
    );
}

#[test]
fn test_parse_public_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("public func foo() {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "foo");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Public);
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_private_variable() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("private var x: Int32".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "x");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Private);
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_internal_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "internal struct Bar {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Bar");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Internal);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_public_init_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { public init(x: Int32) {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert_eq!(body.len(), 1);
        assert_has_access_modifier(&body[0].borrow(), AccessModifier::Public);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_private_deinit_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { private deinit {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert_eq!(body.len(), 1);
        assert_has_access_modifier(&body[0].borrow(), AccessModifier::Private);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_struct_with_public_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { public func bar() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Foo");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl { name: fn_name, .. } = &*body[0].borrow() {
            assert_eq!(fn_name.value, "bar");
            assert_has_access_modifier(&body[0].borrow(), AccessModifier::Public);
        } else {
            panic!("Expected FunctionDecl inside struct");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_function_without_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func foo() {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_has_no_modifiers(&program.statements[0].borrow());
}

#[test]
fn test_parse_variable_without_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: Int32".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_has_no_modifiers(&program.statements[0].borrow());
}

#[test]
fn test_parse_struct_without_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("struct Empty {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_has_no_modifiers(&program.statements[0].borrow());
}

#[test]
fn test_parse_private_struct_field() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Person { private let name: String }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert_eq!(body.len(), 1);
        if let Statement::VariableDecl { name, .. } = &*body[0].borrow() {
            assert_eq!(name.value, "name");
            assert_has_access_modifier(&body[0].borrow(), AccessModifier::Private);
        } else {
            panic!("Expected VariableDecl field");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

fn engine_has_error(engine: &Rc<RefCell<TrussDiagnosticEngine>>, code: TrussDiagnosticCode) -> bool {
    engine
        .borrow()
        .get_errors()
        .iter()
        .any(|e| e.code == code)
}

// =================== Modifier validity tests ===================

#[test]
fn test_duplicate_access_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public public func foo() {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(&engine, TrussDiagnosticCode::DuplicateModifier));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_conflicting_access_modifiers() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public private func foo() {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(&engine, TrussDiagnosticCode::DuplicateModifier));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_duplicate_same_access_modifier_twice() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "private private func foo() {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(&engine, TrussDiagnosticCode::DuplicateModifier));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_triple_access_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public internal private func foo() {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(&engine, TrussDiagnosticCode::DuplicateModifier));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_modifier_on_return() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { public return 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_modifier_on_loop() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { public loop {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_modifier_on_while() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { private while true {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_modifier_on_for() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { internal for _ in 1..<3 {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_modifier_on_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { public 1 + 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_modifier_on_expression_at_top_level() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("private 42".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_modifier_on_throw() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { public throw 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_valid_public_func_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("public func foo() {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(!engine_has_error(&engine, TrussDiagnosticCode::DuplicateModifier));
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_valid_private_struct_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "private struct Foo {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(!engine_has_error(&engine, TrussDiagnosticCode::DuplicateModifier));
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}
