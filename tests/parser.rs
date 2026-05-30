use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::{AssignmentOperator, BinaryOperator, CastKind, Expression, UnaryOperator},
        statement::{
            AccessModifier, AccessorKind, FunctionBody, ImportKind, Modifier, ModifierType,
            Parameter, Pattern, ProtocolMember, Statement, VariadicKind, WhereRequirementKind,
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
fn test_parse_return_without_value_before_brace() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { return }".to_string(),
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
        assert_eq!(body.len(), 1);
        assert!(matches!(&*body[0].borrow(), Statement::InitDecl { .. }));
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
        assert_eq!(body.len(), 3);
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
        assert_eq!(body.len(), 2);
        if let Statement::FunctionDecl {
            name: func_name, ..
        } = &*body[0].borrow()
        {
            assert_eq!(func_name.value, "add");
        } else {
            panic!("Expected FunctionDecl for method add");
        }
        assert!(matches!(&*body[1].borrow(), Statement::InitDecl { .. }));
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
        assert_eq!(body.len(), 4);
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
        assert!(matches!(&*body[3].borrow(), Statement::InitDecl { .. }));
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
        assert_eq!(body.len(), 2);
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
        assert!(matches!(&*body[1].borrow(), Statement::InitDecl { .. }));
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
        assert_eq!(body.len(), 2);
        if let Statement::DeinitDecl { .. } = &*body[0].borrow() {
        } else {
            panic!("Expected DeinitDecl");
        }
        assert!(matches!(&*body[1].borrow(), Statement::InitDecl { .. }));
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

#[test]
fn test_parse_extension_with_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "extension Foo { func bar() -> Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl { name, .. } = &*body[0].borrow() {
            assert_eq!(name.value, "bar");
        } else {
            panic!("Expected FunctionDecl in extension body");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_with_protocol_conformance() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "extension Foo: Printable, Serializable { func dump() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExtensionDecl {
        type_name,
        conformances,
        body,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert_eq!(conformances.len(), 2);
        assert_eq!(body.len(), 1);
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_empty_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("extension Foo {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert!(body.is_empty());
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_of_protocol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "extension Printable { func describe() -> String { \"hello\" } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(type_name.value, "Printable");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl { name, .. } = &*body[0].borrow() {
            assert_eq!(name.value, "describe");
        } else {
            panic!("Expected FunctionDecl in extension body");
        }
    } else {
        panic!("Expected ExtensionDecl");
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
        assert_eq!(body.len(), 2);
        assert_has_access_modifier(&body[0].borrow(), AccessModifier::Private);
        assert!(matches!(&*body[1].borrow(), Statement::InitDecl { .. }));
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
        assert_eq!(body.len(), 2);
        if let Statement::FunctionDecl { name: fn_name, .. } = &*body[0].borrow() {
            assert_eq!(fn_name.value, "bar");
            assert_has_access_modifier(&body[0].borrow(), AccessModifier::Public);
        } else {
            panic!("Expected FunctionDecl inside struct");
        }
        assert!(matches!(&*body[1].borrow(), Statement::InitDecl { .. }));
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
        assert_eq!(body.len(), 2);
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

#[test]
fn test_parse_var_get_shorthand() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { return 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        name,
        accessors,
        type_expression,
        initializer,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert!(type_expression.is_some());
        assert!(initializer.is_none());
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
        assert!(accessors[0].parameter.is_none());
        assert_eq!(accessors[0].body.len(), 1);
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_get_explicit() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { return 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        name, accessors, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_get_set() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { return _v } set(value) { _v = value } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        name, accessors, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert_eq!(accessors.len(), 2);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
        assert!(accessors[0].parameter.is_none());
        assert_eq!(accessors[1].kind, AccessorKind::Set);
        assert_eq!(accessors[1].parameter.as_ref().unwrap().value, "value");
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_all_accessors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { return _v } set(v) { _v = v } willSet { } didSet { } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_var_didset_only() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var didSetOnly: Int = 0 { didSet { print(1) } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        name,
        accessors,
        initializer,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "didSetOnly");
        assert!(initializer.is_some());
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::DidSet);
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_get_willset_didset() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { 1 } willSet(newValue) {} didSet(oldValue) {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_var_set_no_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { 1 } set { _v = newValue } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        name, accessors, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert_eq!(accessors.len(), 2);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
        assert_eq!(accessors[1].kind, AccessorKind::Set);
        assert!(accessors[1].parameter.is_none());
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_accessors_in_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { var v: Int32 { return 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            name, accessors, ..
        } = &*statements[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
    } else {
        panic!("Expected FunctionDecl containing VariableDecl");
    }
}

#[test]
fn test_parse_var_accessors_in_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { var v: Int32 { get { return _v } set { _v = newValue } } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow()
        && let Statement::VariableDecl {
            name, accessors, ..
        } = &*body[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert_eq!(accessors.len(), 2);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
        assert_eq!(accessors[1].kind, AccessorKind::Set);
    } else {
        panic!("Expected StructDecl containing VariableDecl");
    }
}

#[test]
fn test_parse_var_get_set_with_willset_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { 1 } set { } willSet { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_var_get_set_with_didset_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { 1 } set { } didSet { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_var_get_set_with_willset_didset_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { 1 } set(v) { } willSet(v) { } didSet(v) { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_var_didset_only_no_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 = 0 { didSet { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    if let Statement::VariableDecl { accessors, .. } = &*program.statements[0].borrow() {
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::DidSet);
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_willset_only_no_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 = 0 { willSet { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    if let Statement::VariableDecl { accessors, .. } = &*program.statements[0].borrow() {
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::WillSet);
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_get_set_only_no_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "var v: Int32 { get { 1 } set { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::IncompatibleAccessors
    ));
    if let Statement::VariableDecl { accessors, .. } = &*program.statements[0].borrow() {
        assert_eq!(accessors.len(), 2);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
        assert_eq!(accessors[1].kind, AccessorKind::Set);
    } else {
        panic!("Expected VariableDecl");
    }
}

fn engine_has_error(
    engine: &Rc<RefCell<TrussDiagnosticEngine>>,
    code: TrussDiagnosticCode,
) -> bool {
    engine.borrow().get_errors().iter().any(|e| e.code == code)
}

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
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::DuplicateModifier
    ));
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
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::DuplicateModifier
    ));
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
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::DuplicateModifier
    ));
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
    assert!(engine_has_error(
        &engine,
        TrussDiagnosticCode::DuplicateModifier
    ));
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
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::DuplicateModifier
    ));
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_valid_private_struct_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("private struct Foo {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::DuplicateModifier
    ));
    assert!(!engine_has_error(
        &engine,
        TrussDiagnosticCode::ModifierNotAllowedHere
    ));
}

#[test]
fn test_parse_enum_simple_cases() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Direction { case north, south, east, west }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::EnumDecl { name, cases, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Direction");
        assert_eq!(cases.len(), 4);
        assert_eq!(cases[0].name.value, "north");
        assert_eq!(cases[1].name.value, "south");
        assert_eq!(cases[2].name.value, "east");
        assert_eq!(cases[3].name.value, "west");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_enum_with_payload() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Result { case success(Int32), error(String) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::EnumDecl { name, cases, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Result");
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].name.value, "success");
        assert_eq!(cases[0].parameters.len(), 1);
        assert!(cases[0].parameters[0].label.is_none());
        assert_eq!(cases[1].name.value, "error");
        assert_eq!(cases[1].parameters.len(), 1);
        assert!(cases[1].parameters[0].label.is_none());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_enum_with_labeled_parameters() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Point { case origin(x: Int32, y: Int32) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::EnumDecl { name, cases, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].name.value, "origin");
        assert_eq!(cases[0].parameters.len(), 2);
        assert_eq!(cases[0].parameters[0].label.as_ref().unwrap().value, "x");
        assert_eq!(cases[0].parameters[1].label.as_ref().unwrap().value, "y");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_enum_with_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Name { case a, b, c case d(Int32) case e(x: Int64, y: Int8) var x: Int32 }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::EnumDecl {
        name, cases, body, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Name");
        assert_eq!(cases.len(), 5);
        assert_eq!(cases[0].name.value, "a");
        assert_eq!(cases[1].name.value, "b");
        assert_eq!(cases[2].name.value, "c");
        assert_eq!(cases[3].name.value, "d");
        assert_eq!(cases[3].parameters.len(), 1);
        assert!(cases[3].parameters[0].label.is_none());
        assert_eq!(cases[4].name.value, "e");
        assert_eq!(cases[4].parameters.len(), 2);
        assert_eq!(cases[4].parameters[0].label.as_ref().unwrap().value, "x");
        assert_eq!(cases[4].parameters[1].label.as_ref().unwrap().value, "y");
        assert_eq!(body.len(), 1);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_enum_with_multiple_case_lines() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Status { case idle case running(Int32) case stopped }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::EnumDecl { name, cases, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Status");
        assert_eq!(cases.len(), 3);
        assert_eq!(cases[0].name.value, "idle");
        assert_eq!(cases[1].name.value, "running");
        assert_eq!(cases[1].parameters.len(), 1);
        assert!(cases[1].parameters[0].label.is_none());
        assert_eq!(cases[2].name.value, "stopped");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_enum_case_constructor() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Result { case success(Int32) } func test() { let r = Result.success(42) }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Call {
            callee, parameters, ..
        } = &*init.borrow()
        && let Expression::MemberAccess { object, member, .. } = &*callee.borrow()
    {
        assert_eq!(member.value, "success");
        assert_eq!(parameters.len(), 1);
        match &*object.borrow() {
            Expression::Type { name, .. } | Expression::Variable { name, .. } => {
                assert_eq!(name.value, "Result");
            }
            _ => panic!("Expected Type or Variable expression for object"),
        }
    } else {
        panic!(
            "Expected FunctionDecl -> VariableDecl -> Call -> MemberAccess, got: {:?}",
            program.statements
        );
    }
}

#[test]
fn test_parse_if_case_no_bindings() {
    let code = r#"
        enum Option { case none case some }
        func test(x: Option) {
            if case Option.none = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If {
            condition, then, ..
        } = &*expression.borrow()
        && let Expression::Case {
            enum_type,
            case_name,
            bindings,
            ..
        } = &*condition.borrow()
    {
        assert_eq!(enum_type.as_ref().unwrap().value, "Option");
        assert_eq!(case_name.value, "none");
        assert!(bindings.is_empty());
        if let Expression::Block {
            statements: then_stmts,
            ..
        } = &*then.borrow()
        {
            assert!(then_stmts.is_empty());
        } else {
            panic!("Expected block expression for then branch");
        }
    } else {
        panic!(
            "Expected If with Case condition, got: {:?}",
            program.statements
        );
    }
}

#[test]
fn test_parse_if_case_with_bindings() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.some(val) = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { condition, .. } = &*expression.borrow()
        && let Expression::Case {
            enum_type,
            case_name,
            bindings,
            ..
        } = &*condition.borrow()
    {
        assert_eq!(enum_type.as_ref().unwrap().value, "Option");
        assert_eq!(case_name.value, "some");
        assert_eq!(bindings.len(), 1);
        if let Pattern::Identifier(token) = &bindings[0] {
            assert_eq!(token.value, "val");
        } else {
            panic!("Expected Identifier pattern");
        }
    } else {
        panic!(
            "Expected If with Case condition, got: {:?}",
            program.statements
        );
    }
}

#[test]
fn test_parse_if_case_with_else() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.some(val) = x {
                let _ = val
            } else {
                let _ = 0
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If {
            condition, else_, ..
        } = &*expression.borrow()
        && let Expression::Case {
            enum_type,
            case_name,
            bindings,
            ..
        } = &*condition.borrow()
    {
        assert_eq!(enum_type.as_ref().unwrap().value, "Option");
        assert_eq!(case_name.value, "some");
        assert_eq!(bindings.len(), 1);
        assert!(else_.is_some());
    } else {
        panic!(
            "Expected If with Case condition and else, got: {:?}",
            program.statements
        );
    }
}

#[test]
fn test_parse_if_case_else_if() {
    let code = r#"
        enum Status { case idle case loading case done }
        func test(s: Status) {
            if case Status.idle = s {
                let _ = 1
            } else if case Status.loading = s {
                let _ = 2
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If {
            condition, else_, ..
        } = &*expression.borrow()
        && let Expression::Case {
            enum_type,
            case_name,
            ..
        } = &*condition.borrow()
    {
        assert_eq!(enum_type.as_ref().unwrap().value, "Status");
        assert_eq!(case_name.value, "idle");
        assert!(else_.is_some());
        if let Some(else_expr) = else_ {
            if let Expression::If {
                condition: else_cond,
                ..
            } = &*else_expr.borrow()
                && let Expression::Case {
                    enum_type: else_enum_type,
                    case_name: else_case_name,
                    ..
                } = &*else_cond.borrow()
            {
                assert_eq!(else_enum_type.as_ref().unwrap().value, "Status");
                assert_eq!(else_case_name.value, "loading");
            } else {
                panic!("Expected If with Case condition in else branch");
            }
        }
    } else {
        panic!(
            "Expected If with Case condition, got: {:?}",
            program.statements
        );
    }
}

#[test]
fn test_parse_if_case_multiple_bindings() {
    let code = r#"
        enum Status { case error(Int32, Bool) }
        func test(s: Status) {
            if case Status.error(code, flag) = s {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { condition, .. } = &*expression.borrow()
        && let Expression::Case {
            enum_type,
            case_name,
            bindings,
            ..
        } = &*condition.borrow()
    {
        assert_eq!(enum_type.as_ref().unwrap().value, "Status");
        assert_eq!(case_name.value, "error");
        assert_eq!(bindings.len(), 2);
    } else {
        panic!(
            "Expected If with Case condition, got: {:?}",
            program.statements
        );
    }
}

#[test]
fn test_parse_if_case_normal_if_still_works() {
    let code = "func test(x: Bool) { if x {} }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
    {
        assert!(matches!(&*expression.borrow(), Expression::If { .. }));
    } else {
        panic!("Expected regular If expression");
    }
}

#[test]
fn test_parse_case_alone() {
    let code = r#"
        enum Option { case none case some }
        func test(x: Option) -> Bool {
            return case Option.none = x
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Return {
            value: Some(value), ..
        } = &*statements[0].borrow()
    {
        assert!(matches!(&*value.borrow(), Expression::Case { .. }));
    } else {
        panic!(
            "Expected Return with Case expression, got: {:?}",
            program.statements
        );
    }
}

#[test]
fn test_parse_class_decl_empty() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("class Point {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert!(body.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_class_decl_with_fields() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Point { let x: Int32 let y: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
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
fn test_parse_class_decl_with_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Calculator { func add(a: Int32, b: Int32) -> Int32 { a + b } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
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
fn test_parse_class_decl_with_init_deinit() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Point { let x: Int32 init() {} deinit {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 3);
        assert!(matches!(&*body[0].borrow(), Statement::VariableDecl { .. }));
        assert!(matches!(&*body[1].borrow(), Statement::InitDecl { .. }));
        assert!(matches!(&*body[2].borrow(), Statement::DeinitDecl { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_public_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public class Point { let x: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name, modifiers, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Point");
        assert_eq!(modifiers.len(), 1);
        assert!(matches!(
            modifiers[0].ty,
            ModifierType::Access(AccessModifier::Public)
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_class_decl_with_superclass() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("class Dog: Animal {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        superclass,
        body,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Dog");
        assert!(superclass.is_some());
        if let Expression::Type {
            name: super_name, ..
        } = &*superclass.as_ref().unwrap().borrow()
        {
            assert_eq!(super_name.value, "Animal");
        } else {
            panic!("Expected superclass to be a type reference");
        }
        assert!(body.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_class_decl_with_superclass_and_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Dog: Animal { func bark() -> Int32 { 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        superclass,
        body,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Dog");
        assert!(superclass.is_some());
        if let Expression::Type {
            name: super_name, ..
        } = &*superclass.as_ref().unwrap().borrow()
        {
            assert_eq!(super_name.value, "Animal");
        } else {
            panic!("Expected superclass to be a type reference");
        }
        assert_eq!(body.len(), 1);
        assert!(matches!(&*body[0].borrow(), Statement::FunctionDecl { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_class_decl_without_superclass() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("class Empty {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name, superclass, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Empty");
        assert!(superclass.is_none());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_class_decl_with_superclass_type_parameters() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Box: Array<Int32> {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        superclass,
        body,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Box");
        assert!(superclass.is_some());
        if let Expression::Type {
            name: super_name,
            type_parameters,
            ..
        } = &*superclass.as_ref().unwrap().borrow()
        {
            assert_eq!(super_name.value, "Array");
            assert!(type_parameters.is_some());
            assert_eq!(type_parameters.as_ref().unwrap().len(), 1);
        } else {
            panic!("Expected superclass to be a Type expression");
        }
        assert!(body.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_void_literal_empty_parens() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a: () = ()".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert!(
        !program.statements.is_empty(),
        "program should have statements"
    );
    if let Statement::VariableDecl {
        type_expression: Some(type_expr),
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        assert!(matches!(*type_expr.borrow(), Expression::Type { .. }));
        assert!(matches!(*init.borrow(), Expression::VoidLiteral { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_grouped_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (1)".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        assert!(matches!(
            *init.borrow(),
            Expression::IntegerLiteral { value: 1, .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_literal_two_elements() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (1, 2)".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleLiteral { elements, .. } = &*init.borrow()
    {
        assert_eq!(elements.len(), 2);
        assert!(matches!(
            *elements[0].1.borrow(),
            Expression::IntegerLiteral { value: 1, .. }
        ));
        assert!(matches!(
            *elements[1].1.borrow(),
            Expression::IntegerLiteral { value: 2, .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_literal_three_elements() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (1, 2, 3)".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleLiteral { elements, .. } = &*init.borrow()
    {
        assert_eq!(elements.len(), 3);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_type_in_variable_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: (Int32, Bool) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression: Some(type_expr),
            ..
        } = &*statements[0].borrow()
        && let Expression::TupleType { elements, .. } = &*type_expr.borrow()
    {
        assert_eq!(elements.len(), 2);
        assert!(matches!(*elements[0].1.borrow(), Expression::Type { .. }));
        assert!(matches!(*elements[1].1.borrow(), Expression::Type { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_grouped_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: (Int32) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression: Some(type_expr),
            ..
        } = &*statements[0].borrow()
    {
        assert!(matches!(*type_expr.borrow(), Expression::Type { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_pointer_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: (Int32, Bool)* }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression: Some(type_expr),
            ..
        } = &*statements[0].borrow()
        && let Expression::PointerType { base, .. } = &*type_expr.borrow()
        && let Expression::TupleType { elements, .. } = &*base.borrow()
    {
        assert_eq!(elements.len(), 2);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_nested_tuple_literal() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = ((1, 2), 3)".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleLiteral { elements, .. } = &*init.borrow()
    {
        assert_eq!(elements.len(), 2);
        assert!(matches!(
            *elements[0].1.borrow(),
            Expression::TupleLiteral { .. }
        ));
        assert!(matches!(
            *elements[1].1.borrow(),
            Expression::IntegerLiteral { value: 3, .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_index_access_dot() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (1, 2).0".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleIndexAccess {
            object,
            index_value: 0,
            ..
        } = &*init.borrow()
        && let Expression::TupleLiteral { elements, .. } = &*object.borrow()
    {
        assert_eq!(elements.len(), 2);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_index_access_second_element() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (1, 2).1".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleIndexAccess { index_value: 1, .. } = &*init.borrow()
    {
    } else {
        panic!();
    }
}

#[test]
fn test_parse_member_access_still_works() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = obj.field".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::MemberAccess { member, .. } = &*init.borrow()
    {
        assert_eq!(member.value, "field");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_self_keyword() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { self }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
    {
        assert!(matches!(
            *expression.borrow(),
            Expression::SelfKeyword { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_self_member_access() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { self.field }".to_string(),
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
        assert_eq!(member.value, "field");
        assert!(matches!(*object.borrow(), Expression::SelfKeyword { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_self_method_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { self.method() }".to_string(),
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
        assert_eq!(member.value, "method");
        assert!(matches!(*object.borrow(), Expression::SelfKeyword { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_self_as_expression_statement() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("self".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        assert!(matches!(
            *expression.borrow(),
            Expression::SelfKeyword { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_named_tuple_literal() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (x: 1, y: 2)".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleLiteral { elements, .. } = &*init.borrow()
    {
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].0.as_deref(), Some("x"));
        assert!(matches!(
            *elements[0].1.borrow(),
            Expression::IntegerLiteral { value: 1, .. }
        ));
        assert_eq!(elements[1].0.as_deref(), Some("y"));
        assert!(matches!(
            *elements[1].1.borrow(),
            Expression::IntegerLiteral { value: 2, .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_single_element_named_tuple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (x: 1)".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleLiteral { elements, .. } = &*init.borrow()
    {
        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0].0.as_deref(), Some("x"));
        assert!(matches!(
            *elements[0].1.borrow(),
            Expression::IntegerLiteral { value: 1, .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_named_tuple_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let a: (x: Int32, y: Bool) = (x: 1, y: true)".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        type_expression: Some(type_expr),
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
        && let Expression::TupleType {
            elements: type_elements,
            ..
        } = &*type_expr.borrow()
        && let Expression::TupleLiteral {
            elements: lit_elements,
            ..
        } = &*init.borrow()
    {
        assert_eq!(type_elements.len(), 2);
        assert_eq!(type_elements[0].0.as_deref(), Some("x"));
        assert_eq!(type_elements[1].0.as_deref(), Some("y"));

        assert_eq!(lit_elements.len(), 2);
        assert_eq!(lit_elements[0].0.as_deref(), Some("x"));
        assert_eq!(lit_elements[1].0.as_deref(), Some("y"));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_empty() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl {
        name,
        members,
        conformances,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyProtocol");
        assert!(members.is_empty());
        assert!(conformances.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_with_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public protocol MyProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl {
        name, modifiers, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyProtocol");
        assert_eq!(modifiers.len(), 1);
        assert_eq!(
            modifiers[0].ty,
            ModifierType::Access(AccessModifier::Public)
        );
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_with_conformances() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol: SomeProtocol, AnotherProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl {
        name, conformances, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyProtocol");
        assert_eq!(conformances.len(), 2);
        if let Expression::Type { name: first, .. } = &*conformances[0].borrow() {
            assert_eq!(first.value, "SomeProtocol");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_with_method_requirement() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol { func doSomething() -> Void }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { name, members, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "MyProtocol");
        assert_eq!(members.len(), 1);
        if let ProtocolMember::Method { decl, .. } = &members[0] {
            if let Statement::FunctionDecl {
                name: func_name,
                return_type,
                ..
            } = &*decl.borrow()
            {
                assert_eq!(func_name.value, "doSomething");
                assert!(return_type.is_some());
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_with_default_implementation() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol { func greet() -> Int32 { return 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { name, members, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "MyProtocol");
        assert_eq!(members.len(), 1);
        if let ProtocolMember::Method { decl, .. } = &members[0] {
            if let Statement::FunctionDecl {
                name: func_name,
                body,
                ..
            } = &*decl.borrow()
            {
                assert_eq!(func_name.value, "greet");
                assert!(matches!(&*body.borrow(), FunctionBody::Statements(_)));
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_with_property_requirements() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol { var name: String { get } var age: Int32 { get set } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { name, members, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "MyProtocol");
        assert_eq!(members.len(), 2);

        if let ProtocolMember::Property {
            name: first_name,
            accessors: first_accessors,
            ..
        } = &members[0]
        {
            assert_eq!(first_name.value, "name");
            assert!(first_accessors.get);
            assert!(!first_accessors.set);
        } else {
            panic!("Expected property member");
        }

        if let ProtocolMember::Property {
            name: second_name,
            accessors: second_accessors,
            ..
        } = &members[1]
        {
            assert_eq!(second_name.value, "age");
            assert!(second_accessors.get);
            assert!(second_accessors.set);
        } else {
            panic!("Expected property member");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_with_mixed_members() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol { func doit() -> Void var x: Int32 { get set } func greet() -> Int32 { return 0 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { name, members, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "MyProtocol");
        assert_eq!(members.len(), 3);
        assert!(matches!(members[0], ProtocolMember::Method { .. }));
        assert!(matches!(members[1], ProtocolMember::Property { .. }));
        assert!(matches!(members[2], ProtocolMember::Method { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_property_get_only_default() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol { var x: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        assert_eq!(members.len(), 1);
        if let ProtocolMember::Property { accessors, .. } = &members[0] {
            assert!(accessors.get);
            assert!(!accessors.set);
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_class_with_protocol_conformance() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class MyClass: MyProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        superclass,
        conformances,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyClass");
        assert!(superclass.is_some());
        assert!(conformances.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_class_with_superclass_and_protocols() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class MyClass: SuperClass, SomeProtocol, AnotherProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        superclass,
        conformances,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyClass");
        assert!(superclass.is_some());
        if let Expression::Type {
            name: super_name, ..
        } = &*superclass.as_ref().unwrap().borrow()
        {
            assert_eq!(super_name.value, "SuperClass");
        } else {
            panic!();
        }
        assert_eq!(conformances.len(), 2);
        if let Expression::Type { name: first, .. } = &*conformances[0].borrow() {
            assert_eq!(first.value, "SomeProtocol");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_with_protocol_conformance() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyStruct: MyProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl {
        name, conformances, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyStruct");
        assert_eq!(conformances.len(), 1);
        if let Expression::Type { name: first, .. } = &*conformances[0].borrow() {
            assert_eq!(first.value, "MyProtocol");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_with_multiple_protocol_conformances() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyStruct: SomeProtocol, AnotherProtocol, YetAnother {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl {
        name, conformances, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyStruct");
        assert_eq!(conformances.len(), 3);
        if let Expression::Type { name: first, .. } = &*conformances[0].borrow() {
            assert_eq!(first.value, "SomeProtocol");
        } else {
            panic!();
        }
        if let Expression::Type { name: second, .. } = &*conformances[1].borrow() {
            assert_eq!(second.value, "AnotherProtocol");
        } else {
            panic!();
        }
        if let Expression::Type { name: third, .. } = &*conformances[2].borrow() {
            assert_eq!(third.value, "YetAnother");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_without_conformances_still_works() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("struct Empty {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl {
        name, conformances, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Empty");
        assert!(conformances.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_any_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: any MyProtocol".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        name,
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "x");
        assert!(
            matches!(&*ty_expr.borrow(), Expression::AnyType { .. }),
            "Expected AnyType, got {:?}",
            &*ty_expr.borrow()
        );
        if let Expression::AnyType { inner, .. } = &*ty_expr.borrow() {
            assert!(
                matches!(&*inner.borrow(), Expression::Type { name, .. } if name.value == "MyProtocol"),
                "Expected Type(MyProtocol), got {:?}",
                &*inner.borrow()
            );
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_compound_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let x: Copyable & Clonable".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[0].borrow()
    {
        if let Expression::CompoundType { types, .. } = &*ty_expr.borrow() {
            assert_eq!(types.len(), 2);
            let first = &types[0];
            assert!(
                matches!(&*first.borrow(), Expression::Type { name, .. } if name.value == "Copyable"),
                "Expected Type(Copyable), got {:?}",
                &*first.borrow()
            );
            let second = &types[1];
            assert!(
                matches!(&*second.borrow(), Expression::Type { name, .. } if name.value == "Clonable"),
                "Expected Type(Clonable), got {:?}",
                &*second.borrow()
            );
        } else {
            panic!("Expected CompoundType");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_compound_type_three() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: A & B & C".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::VariableDecl {
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[0].borrow()
    {
        if let Expression::CompoundType { types, .. } = &*ty_expr.borrow() {
            assert_eq!(types.len(), 3);
            assert!(
                matches!(&*types[0].borrow(), Expression::Type { name, .. } if name.value == "A")
            );
            assert!(
                matches!(&*types[1].borrow(), Expression::Type { name, .. } if name.value == "B")
            );
            assert!(
                matches!(&*types[2].borrow(), Expression::Type { name, .. } if name.value == "C")
            );
        } else {
            panic!("Expected CompoundType");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func identity<T>(x: T) -> T".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl {
        name,
        generic_parameters,
        parameters,
        return_type,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "identity");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(generic_parameters[0].constraints.is_empty());
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].borrow().name.value, "x");
        assert!(return_type.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function_param_with_constraints() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func compare<T: Equatable>(a: T, b: T) -> Bool".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert_eq!(generic_parameters[0].constraints.len(), 1);
        if let Expression::Type { name, .. } = &*generic_parameters[0].constraints[0].borrow() {
            assert_eq!(name.value, "Equatable");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function_multi_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func map<T, U>(x: T, y: U) -> U".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 2);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert_eq!(generic_parameters[1].name.value, "U");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function_with_combined_constraint() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test<T: Equatable & Hashable>(x: T)".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].constraints.len(), 1);
        if let Expression::CompoundType { types, .. } =
            &*generic_parameters[0].constraints[0].borrow()
        {
            assert_eq!(types.len(), 2);
            assert!(
                matches!(&*types[0].borrow(), Expression::Type { name, .. } if name.value == "Equatable")
            );
            assert!(
                matches!(&*types[1].borrow(), Expression::Type { name, .. } if name.value == "Hashable")
            );
        } else {
            panic!("Expected CompoundType");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Stack<Element> { var items: Array<Element> }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl {
        name,
        generic_parameters,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Stack");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "Element");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Box<T: Hashable> { var value: T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        generic_parameters,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Box");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert_eq!(generic_parameters[0].constraints.len(), 1);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_enum() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Option<T> { case none case some(T) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::EnumDecl {
        name,
        generic_parameters,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Option");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_protocol_with_sugar() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Container<T> { func append(item: T) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl {
        name,
        generic_parameters,
        members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Container");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(matches!(members[0], ProtocolMember::AssociatedType { .. }));
        assert_eq!(members.len(), 2);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_associatedtype() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item func get() -> Item }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        assert!(matches!(members[0], ProtocolMember::AssociatedType { .. }));
        if let ProtocolMember::AssociatedType {
            name, constraints, ..
        } = &members[0]
        {
            assert_eq!(name.value, "Item");
            assert!(constraints.is_empty());
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_protocol_associatedtype_with_constraint() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype T: Equatable }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        if let ProtocolMember::AssociatedType {
            name, constraints, ..
        } = &members[0]
        {
            assert_eq!(name.value, "T");
            assert_eq!(constraints.len(), 1);
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_typealias_in_protocol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { typealias Inner = Int32 func get() -> Inner }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        assert!(matches!(members[0], ProtocolMember::TypeAlias { .. }));
        if let ProtocolMember::TypeAlias { name, .. } = &members[0] {
            assert_eq!(name.value, "Inner");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_typealias_in_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Wrapper { typealias Inner = Int32; var x: Inner }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert!(matches!(&*body[0].borrow(), Statement::TypeAlias { .. }));
        if let Statement::TypeAlias { name, .. } = &*body[0].borrow() {
            assert_eq!(name.value, "Inner");
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_typealias_at_top_level() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "typealias MyInt = Int32 var x: MyInt".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert!(matches!(
        &*program.statements[0].borrow(),
        Statement::TypeAlias { .. }
    ));
    if let Statement::TypeAlias { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "MyInt");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_typealias_in_function_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo() { typealias Inner = Int32 var y: Inner }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow() {
        if let FunctionBody::Statements(stmts) = &*body.borrow() {
            assert!(matches!(&*stmts[0].borrow(), Statement::TypeAlias { .. }));
            if let Statement::TypeAlias { name, .. } = &*stmts[0].borrow() {
                assert_eq!(name.value, "Inner");
            } else {
                panic!();
            }
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_conformance_with_generic_protocol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyArray: Container<Int32> {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl {
        name,
        conformances,
        generic_parameters,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyArray");
        assert!(generic_parameters.is_empty());
        assert_eq!(conformances.len(), 1);
        if let Expression::Type {
            name,
            type_parameters,
            ..
        } = &*conformances[0].borrow()
        {
            assert_eq!(name.value, "Container");
            assert!(type_parameters.is_some());
            assert_eq!(type_parameters.as_ref().unwrap().len(), 1);
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_where_clause() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<T>(x: T) where T: Equatable".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters,
        where_clause,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert!(where_clause.is_some());
        let requirements = where_clause.as_ref().unwrap();
        assert_eq!(requirements.len(), 1);
        assert!(matches!(
            requirements[0].kind,
            WhereRequirementKind::Conformance { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_where_clause_multi_and() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<T, U>(x: T, y: U) where T: Equatable && U: Hashable".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { where_clause, .. } = &*program.statements[0].borrow() {
        assert!(where_clause.is_some());
        let reqs = where_clause.as_ref().unwrap();
        assert_eq!(reqs.len(), 2);
        assert!(matches!(
            reqs[0].kind,
            WhereRequirementKind::Conformance { .. }
        ));
        assert!(matches!(
            reqs[1].kind,
            WhereRequirementKind::Conformance { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_with_where_clause() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct S<T> where T: Hashable { var x: T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::StructDecl {
        generic_parameters,
        where_clause,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert!(where_clause.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_extension_where_clause() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "extension Array where Element: Hashable {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::ExtensionDecl {
        type_name,
        where_clause,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(type_name.value, "Array");
        assert!(where_clause.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_non_generic_function_still_works() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func plain(x: Int32) -> Int32 { x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl {
        name,
        generic_parameters,
        where_clause,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "plain");
        assert!(generic_parameters.is_empty());
        assert!(where_clause.is_none());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function_less_than_no_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func cmp(x: Int32, y: Int32) -> Bool { x < y }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert!(generic_parameters.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_type_instantiation_inside_generic_context() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Array<Int32> = Array() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { .. } = &*program.statements[0].borrow() {
    } else {
        panic!();
    }
}

#[test]
fn test_parse_if_case_dot_shorthand() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case .some(val) = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { condition, .. } = &*expression.borrow()
        && let Expression::Case {
            enum_type,
            case_name,
            bindings,
            ..
        } = &*condition.borrow()
    {
        assert!(enum_type.is_none());
        assert_eq!(case_name.value, "some");
        assert_eq!(bindings.len(), 1);
    } else {
        panic!("Expected If with Case (shorthand) condition");
    }
}

#[test]
fn test_parse_if_case_connect_with_and() {
    let code = r#"
        enum A { case x case y }
        func test(a: A, b: A) {
            if case .x = a && case .y = b {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { condition, .. } = &*expression.borrow()
        && let Expression::Binary {
            left,
            operator,
            right,
        } = &*condition.borrow()
    {
        assert_eq!(*operator, BinaryOperator::And);
        assert!(matches!(&*left.borrow(), Expression::Case { .. }));
        assert!(matches!(&*right.borrow(), Expression::Case { .. }));
    } else {
        panic!("Expected If with &&-connected cases");
    }
}

#[test]
fn test_parse_match_simple() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) -> Int32 {
            match x {
                case .some(let val):
                    val
                case .none:
                    0
                default:
                    -1
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { value, cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 3, "expected 3 cases (some, none, default)");
        assert!(matches!(&*value.borrow(), Expression::Variable { name, .. } if name.value == "x"));
        assert!(matches!(cases[0].patterns[0].as_ref(),
            Pattern::EnumCase { case_name, .. } if case_name.value == "some"));
        if let Pattern::EnumCase { bindings, .. } = cases[0].patterns[0].as_ref() {
            assert_eq!(bindings.len(), 1);
            assert!(matches!(&bindings[0], Pattern::ValueBinding(_)));
        }
        assert!(
            matches!(cases[1].patterns[0].as_ref(), Pattern::EnumCase { case_name, .. } if case_name.value == "none")
        );
        assert!(matches!(cases[2].patterns[0].as_ref(), Pattern::Ignore));
    } else {
        panic!("Expected Return with Match expression");
    }
}

#[test]
fn test_parse_match_with_guard() {
    let code = r#"
        enum Status { case idle case loading case done(Int32) }
        func test(s: Status) {
            match s {
                case .done(let val) where val > 0:
                    val
                default:
                    0
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 2);
        assert!(cases[0].guard.is_some());
        assert!(cases[1].guard.is_none());
    } else {
        panic!("Expected Match with guard");
    }
}

#[test]
fn test_parse_guard_statement() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            guard case .some(val) = x else {
                return
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Guard {
            condition,
            else_body,
            ..
        } = &*statements[0].borrow()
    {
        assert!(matches!(&*condition.borrow(), Expression::Case { .. }));
        assert!(matches!(&*else_body.borrow(), Expression::Block { .. }));
    } else {
        panic!("Expected Guard statement");
    }
}

#[test]
fn test_parse_fallthrough_and_break() {
    let code = r#"
        enum Option { case none case some }
        func test(x: Option) {
            match x {
                case .none:
                    fallthrough
                case .some:
                    break
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 2);
        if let Expression::Block {
            statements: stmts, ..
        } = &*cases[0].body.borrow()
        {
            assert!(matches!(&*stmts[0].borrow(), Statement::Fallthrough { .. }));
        } else {
            panic!("Expected block with fallthrough");
        }
        if let Expression::Block {
            statements: stmts, ..
        } = &*cases[1].body.borrow()
        {
            assert!(matches!(&*stmts[0].borrow(), Statement::Break { .. }));
        } else {
            panic!("Expected block with break");
        }
    } else {
        panic!("Expected Match with fallthrough/break");
    }
}

#[test]
fn test_parse_pattern_value_binding() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case .some(let val) = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { condition, .. } = &*expression.borrow()
        && let Expression::Case {
            case_name,
            bindings,
            ..
        } = &*condition.borrow()
    {
        assert_eq!(case_name.value, "some");
        assert_eq!(bindings.len(), 1);
        assert!(
            matches!(&bindings[0], Pattern::ValueBinding(inner) if matches!(inner.as_ref(), Pattern::Identifier(_)))
        );
    } else {
        panic!("Expected Case with value binding pattern");
    }
}

#[test]
fn test_parse_mixed_literal_and_binding_patterns() {
    let code = r#"
        enum Status { case error(Int32, String) }
        func test(s: Status) {
            if case .error(404, let msg) = s {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { condition, .. } = &*expression.borrow()
        && let Expression::Case {
            case_name,
            bindings,
            ..
        } = &*condition.borrow()
    {
        assert_eq!(case_name.value, "error");
        assert_eq!(bindings.len(), 2);
        assert!(matches!(&bindings[0], Pattern::Expr(_)));
        assert!(matches!(&bindings[1], Pattern::ValueBinding(_)));
    } else {
        panic!("Expected Case with mixed literal+binding patterns");
    }
}

#[test]
fn test_parse_match_in_expression_context() {
    let code = r#"
        func test(x: Bool) -> Int32 {
            match x {
                case true:
                    1
                case false:
                    0
                default:
                    -1
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { .. } = &*expression.borrow()
    {
    } else {
        panic!("Expected ExpressionStatement with Match expression");
    }
}

#[test]
fn test_parse_match_malformed_no_brace_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("match x".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert!(
        program.statements.is_empty()
            || matches!(
                &*program.statements[0].borrow(),
                Statement::ExpressionStatement { .. }
            )
    );
}

#[test]
fn test_parse_guard_without_else_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("guard case .x = x".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert!(
        program.statements.is_empty()
            || matches!(&*program.statements[0].borrow(), Statement::Guard { .. })
    );
}

#[test]
fn test_parse_associated_type_access_in_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item } func foo(x: P.Item) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert!(program.statements.len() >= 2);
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[1].borrow() {
        let ty_expr = parameters[0].borrow().type_expression.clone();
        let ty_expr_ref = ty_expr.borrow();
        assert!(
            matches!(&*ty_expr_ref, Expression::AssociatedTypeAccess { member, .. } if member.value == "Item"),
            "Expected AssociatedTypeAccess with member 'Item', got: {:?}",
            ty_expr_ref
        );
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_associated_type_access_chained() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item } func foo(x: P.Item.Sub) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[1].borrow() {
        let ty_expr = parameters[0].borrow().type_expression.clone();
        let ty_expr_ref = ty_expr.borrow();
        assert!(
            matches!(&*ty_expr_ref, Expression::AssociatedTypeAccess { member, .. } if member.value == "Sub"),
            "Outer level should be 'Sub', got: {:?}",
            ty_expr_ref
        );
        if let Expression::AssociatedTypeAccess { object, .. } = &*ty_expr_ref {
            let inner = object.borrow();
            assert!(
                matches!(&*inner, Expression::AssociatedTypeAccess { member, .. } if member.value == "Item"),
                "Inner level should be 'Item', got: {:?}",
                inner
            );
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_associated_type_with_pointer() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Buf { associatedtype Elem } func foo(x: Buf.Elem*) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[1].borrow() {
        let ty_expr = parameters[0].borrow().type_expression.clone();
        let ty_expr_ref = ty_expr.borrow();
        assert!(
            matches!(&*ty_expr_ref, Expression::PointerType { .. }),
            "Expected PointerType"
        );
        if let Expression::PointerType { base, .. } = &*ty_expr_ref {
            let base = base.borrow();
            assert!(
                matches!(&*base, Expression::AssociatedTypeAccess { member, .. } if member.value == "Elem"),
                "PointerType base should be AssociatedTypeAccess, got: {:?}",
                base
            );
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_associated_type_in_variable_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol C { associatedtype T } func test() { let x: C.T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression, ..
        } = &*stmts[0].borrow()
    {
        let ty_expr = type_expression.as_ref().unwrap().borrow();
        assert!(
            matches!(&*ty_expr, Expression::AssociatedTypeAccess { member, .. } if member.value == "T"),
            "Expected AssociatedTypeAccess, got: {:?}",
            ty_expr
        );
    } else {
        panic!("Expected FunctionDecl with body");
    }
}

#[test]
fn test_parse_associated_type_in_parenthesized_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item } func foo(x: (P).Item) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[1].borrow() {
        let ty_expr = parameters[0].borrow().type_expression.clone();
        let ty_expr_ref = ty_expr.borrow();
        assert!(
            matches!(&*ty_expr_ref, Expression::AssociatedTypeAccess { member, .. } if member.value == "Item"),
            "Expected AssociatedTypeAccess from parenthesized base, got: {:?}",
            ty_expr_ref
        );
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_defer_statement() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { defer { cleanup() } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Defer {
            body: defer_body, ..
        } = &*statements[0].borrow()
        && let Expression::Block {
            statements: block_stmts,
            ..
        } = &*defer_body.borrow()
    {
        assert_eq!(block_stmts.len(), 1);
        assert!(matches!(
            &*block_stmts[0].borrow(),
            Statement::ExpressionStatement { .. }
        ));
    } else {
        panic!("Expected FunctionDecl with defer statement");
    }
}

#[test]
fn test_parse_defer_nested() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { defer { defer { f() } g() } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Defer {
            body: outer_defer_body,
            ..
        } = &*statements[0].borrow()
        && let Expression::Block {
            statements: outer_stmts,
            ..
        } = &*outer_defer_body.borrow()
    {
        assert_eq!(outer_stmts.len(), 2);
        assert!(matches!(&*outer_stmts[0].borrow(), Statement::Defer { .. }));
        assert!(matches!(
            &*outer_stmts[1].borrow(),
            Statement::ExpressionStatement { .. }
        ));
    } else {
        panic!("Expected FunctionDecl with nested defer");
    }
}

#[test]
fn test_parse_defer_with_return_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { defer { return } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        assert!(matches!(&*statements[0].borrow(), Statement::Defer { .. }));
    } else {
        panic!("Expected FunctionDecl with defer");
    }
    assert!(engine.borrow().has_errors());
}

#[test]
fn test_parse_defer_missing_block_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { defer }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine.borrow().has_errors());
}

#[test]
fn test_parse_if_as_variable_initializer() {
    let code = "func test() { let x = if true { 1 } else { 2 } }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::If {
            condition,
            then,
            else_,
            ..
        } = &*init.borrow()
    {
        assert!(matches!(
            &*condition.borrow(),
            Expression::BooleanLiteral { .. }
        ));
        assert!(matches!(&*then.borrow(), Expression::Block { .. }));
        assert!(else_.is_some());
    } else {
        panic!("Expected VariableDecl with If initializer");
    }
}

#[test]
fn test_parse_if_as_function_body_last_expression() {
    let code = "func test() -> Int32 { if true { 1 } else { 2 } }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { else_, .. } = &*expression.borrow()
    {
        assert!(else_.is_some());
    } else {
        panic!("Expected FunctionDecl with if as last expression statement");
    }
}

#[test]
fn test_parse_if_elseif_chain() {
    let code = "func test() { let x = if true { 1 } else if false { 2 } else { 3 } }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::If { else_, .. } = &*init.borrow()
        && let Some(else_expr) = else_
    {
        assert!(matches!(&*else_expr.borrow(), Expression::If { .. }));
    } else {
        panic!("Expected VariableDecl with if-else if chain");
    }
}

#[test]
fn test_parse_if_without_else_as_expression() {
    let code = "func test() { let x = if true { 1 } }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::If { else_, .. } = &*init.borrow()
    {
        assert!(else_.is_none());
    } else {
        panic!("Expected VariableDecl with If (no else) initializer");
    }
}

#[test]
fn test_parse_match_multi_pattern_literals() {
    let code = r#"
        func test(x: Int32) -> Int32 {
            match x {
                case 1, 2, 3:
                    10
                case 4, 5:
                    20
                default:
                    0
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 3);
        assert_eq!(
            cases[0].patterns.len(),
            3,
            "case 1,2,3 should have 3 patterns"
        );
        assert!(matches!(cases[0].patterns[0].as_ref(), Pattern::Expr(_)));
        assert!(matches!(cases[0].patterns[1].as_ref(), Pattern::Expr(_)));
        assert!(matches!(cases[0].patterns[2].as_ref(), Pattern::Expr(_)));
        assert_eq!(
            cases[1].patterns.len(),
            2,
            "case 4,5 should have 2 patterns"
        );
        assert!(matches!(cases[1].patterns[0].as_ref(), Pattern::Expr(_)));
        assert!(matches!(cases[1].patterns[1].as_ref(), Pattern::Expr(_)));
        assert_eq!(cases[2].patterns.len(), 1);
        assert!(matches!(cases[2].patterns[0].as_ref(), Pattern::Ignore));
    } else {
        panic!("Expected Match with multi-pattern cases");
    }
}

#[test]
fn test_parse_match_multi_pattern_enum() {
    let code = r#"
        enum Status { case idle case loading case done }
        func test(s: Status) {
            match s {
                case .idle, .loading:
                    true
                case .done:
                    false
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 2);
        assert_eq!(
            cases[0].patterns.len(),
            2,
            "case .idle, .loading should have 2 patterns"
        );
        assert!(matches!(cases[0].patterns[0].as_ref(),
            Pattern::EnumCase { case_name, .. } if case_name.value == "idle"));
        assert!(matches!(cases[0].patterns[1].as_ref(),
            Pattern::EnumCase { case_name, .. } if case_name.value == "loading"));
        assert_eq!(cases[1].patterns.len(), 1);
        assert!(matches!(cases[1].patterns[0].as_ref(),
            Pattern::EnumCase { case_name, .. } if case_name.value == "done"));
    } else {
        panic!("Expected Match with multi-pattern enum cases");
    }
}

#[test]
fn test_parse_match_multi_pattern_with_guard() {
    let code = r#"
        enum Status { case idle case loading case done }
        func test(s: Status) {
            match s {
                case .idle, .loading where true:
                    true
                default:
                    false
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 2);
        assert_eq!(
            cases[0].patterns.len(),
            2,
            "case .idle, .loading should have 2 patterns"
        );
        assert!(cases[0].guard.is_some(), "guard should be present");
        assert!(cases[1].guard.is_none());
    } else {
        panic!("Expected Match with guard on multi-pattern case");
    }
}

#[test]
fn test_parse_empty_module() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module foo {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    let stmt = program.statements[0].borrow();
    if let Statement::ModuleDecl { name, body, .. } = &*stmt {
        assert_eq!(name.value, "foo");
        assert!(body.is_empty());
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_parse_module_with_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo { func bar() -> Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    let stmt = program.statements[0].borrow();
    if let Statement::ModuleDecl { name, body, .. } = &*stmt {
        assert_eq!(name.value, "foo");
        assert_eq!(body.len(), 1);
        assert!(
            matches!(&*body[0].borrow(), Statement::FunctionDecl { name, .. } if name.value == "bar")
        );
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_parse_module_dotted_path() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module foo.bar { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    let stmt = program.statements[0].borrow();
    if let Statement::ModuleDecl {
        name: outer_name,
        body,
        ..
    } = &*stmt
    {
        assert_eq!(outer_name.value, "foo");
        assert_eq!(body.len(), 1);
        let inner = body[0].borrow();
        if let Statement::ModuleDecl {
            name: inner_name,
            body: inner_body,
            ..
        } = &*inner
        {
            assert_eq!(inner_name.value, "bar");
            assert!(inner_body.is_empty());
        } else {
            panic!("Expected nested ModuleDecl");
        }
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_parse_nested_module() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo { module bar { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    let stmt = program.statements[0].borrow();
    if let Statement::ModuleDecl {
        name: outer_name,
        body,
        ..
    } = &*stmt
    {
        assert_eq!(outer_name.value, "foo");
        assert_eq!(body.len(), 1);
        let inner = body[0].borrow();
        if let Statement::ModuleDecl {
            name: inner_name,
            body: inner_body,
            ..
        } = &*inner
        {
            assert_eq!(inner_name.value, "bar");
            assert!(inner_body.is_empty());
        } else {
            panic!("Expected nested ModuleDecl");
        }
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_parse_multiple_modules() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo { } module bar { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    let stmt0 = program.statements[0].borrow();
    let stmt1 = program.statements[1].borrow();
    assert!(matches!(&*stmt0, Statement::ModuleDecl { name, .. } if name.value == "foo"));
    assert!(matches!(&*stmt1, Statement::ModuleDecl { name, .. } if name.value == "bar"));
}

#[test]
fn test_parse_module_missing_name() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    let diagnostics = engine.borrow().format_all_plain("");
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostic for missing module name"
    );
}

#[test]
fn test_parse_module_missing_brace() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module foo".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    let diagnostics = engine.borrow().format_all_plain("");
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostic for missing '{{'"
    );
}

#[test]
fn test_dotted_and_nested_module_equivalence() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module foo.bar { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    let stmt = program.statements[0].borrow();
    if let Statement::ModuleDecl {
        name: outer_name,
        body,
        ..
    } = &*stmt
    {
        assert_eq!(outer_name.value, "foo");
        assert_eq!(body.len(), 1);
        let inner = body[0].borrow();
        if let Statement::ModuleDecl {
            name: inner_name,
            body: inner_body,
            ..
        } = &*inner
        {
            assert_eq!(inner_name.value, "bar");
            assert!(inner_body.is_empty());
        } else {
            panic!("Expected nested ModuleDecl");
        }

        let engine2 = create_engine();
        let mut lexer2 = Lexer::new(
            CharStream::new(
                "module foo { module bar { } }".to_string(),
                Rc::new("".to_string()),
            ),
            engine2.clone(),
        );
        let mut parser2 = Parser::new(lexer2.get_file(), lexer2.parse(), engine2);
        let program2 = parser2.parse();
        let stmt2 = program2.statements[0].borrow();
        if let Statement::ModuleDecl {
            name: outer_name2,
            body: body2,
            ..
        } = &*stmt2
        {
            assert_eq!(outer_name2.value, "foo");
            assert_eq!(body2.len(), 1);
            let inner2 = body2[0].borrow();
            if let Statement::ModuleDecl {
                name: inner_name2,
                body: inner_body2,
                ..
            } = &*inner2
            {
                assert_eq!(inner_name2.value, "bar");
                assert!(inner_body2.is_empty());
            } else {
                panic!("Expected nested ModuleDecl");
            }
        } else {
            panic!("Expected ModuleDecl");
        }
    } else {
        panic!("Expected ModuleDecl");
    }
}

#[test]
fn test_parse_module_deep_dotted_path() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module a.b.c { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    let stmt = program.statements[0].borrow();
    if let Statement::ModuleDecl {
        name: name_a, body, ..
    } = &*stmt
    {
        assert_eq!(name_a.value, "a");
        assert_eq!(body.len(), 1);
        let b_stmt = body[0].borrow();
        if let Statement::ModuleDecl {
            name: name_b,
            body: body_b,
            ..
        } = &*b_stmt
        {
            assert_eq!(name_b.value, "b");
            assert_eq!(body_b.len(), 1);
            let c_stmt = body_b[0].borrow();
            if let Statement::ModuleDecl {
                name: name_c,
                body: body_c,
                ..
            } = &*c_stmt
            {
                assert_eq!(name_c.value, "c");
                assert!(body_c.is_empty());
            } else {
                panic!("Expected ModuleDecl for c");
            }
        } else {
            panic!("Expected ModuleDecl for b");
        }
    } else {
        panic!("Expected ModuleDecl for a");
    }
}

#[test]
fn test_parse_overloaded_top_level_functions() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(x: Int32) { x } func foo(y: Float64) { y }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    if let Statement::FunctionDecl {
        name, parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "foo");
        assert_eq!(parameters.len(), 1);
        assert!(
            matches!(
                &*parameters[0].borrow().type_expression.borrow(),
                Expression::Type { name, .. } if name.value == "Int32"
            ),
            "Expected Int32 parameter type"
        );
    } else {
        panic!("Expected FunctionDecl");
    }
    if let Statement::FunctionDecl {
        name, parameters, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(name.value, "foo");
        assert_eq!(parameters.len(), 1);
        assert!(
            matches!(
                &*parameters[0].borrow().type_expression.borrow(),
                Expression::Type { name, .. } if name.value == "Float64"
            ),
            "Expected Float64 parameter type"
        );
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_import_module_single() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import Foo".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(path, &vec!["Foo".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_module_nested() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import Foo.Bar".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(path, &vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_member() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import Foo.Bar.baz".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(
            path,
            &vec!["Foo".to_string(), "Bar".to_string(), "baz".to_string()]
        );
        assert_eq!(*kind, ImportKind::Member);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_wildcard() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import Foo.Bar.*".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(path, &vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(*kind, ImportKind::Wildcard);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_member_deep() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import A.B.C.D.foo".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(
            path,
            &vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
                "foo".to_string()
            ]
        );
        assert_eq!(*kind, ImportKind::Member);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_wildcard_deep() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import A.B.C.*".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(
            path,
            &vec!["A".to_string(), "B".to_string(), "C".to_string()]
        );
        assert_eq!(*kind, ImportKind::Wildcard);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_multiple_imports() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import Foo\nimport Bar.Baz\nimport X.Y.z\nimport U.V.*".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 4);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(path, &vec!["Foo".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
    } else {
        panic!("Expected ImportDecl");
    }
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[1].borrow() {
        assert_eq!(path, &vec!["Bar".to_string(), "Baz".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
    } else {
        panic!("Expected ImportDecl");
    }
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[2].borrow() {
        assert_eq!(
            path,
            &vec!["X".to_string(), "Y".to_string(), "z".to_string()]
        );
        assert_eq!(*kind, ImportKind::Member);
    } else {
        panic!("Expected ImportDecl");
    }
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[3].borrow() {
        assert_eq!(path, &vec!["U".to_string(), "V".to_string()]);
        assert_eq!(*kind, ImportKind::Wildcard);
    } else {
        panic!("Expected ImportDecl");
    }
}

