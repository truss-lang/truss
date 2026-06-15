use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::{
            AssignmentOperator, BinaryOperator, CastKind, ClosureCapture, ElseBranch, Expression,
            MacroDelimiter, TryKind, UnaryOperator,
        },
        statement::{
            AccessModifier, AccessorKind, AsmDirection, Condition, FunctionBody,
            GenericParameterKind, ImportKind, MacroMetaVarType, MacroPatternFragment, Modifier,
            ModifierType, OperatorFixity, OwnershipModifier, Parameter, Pattern, ProtocolMember,
            SelectiveAlias, Statement, VariadicKind, WhereRequirementKind,
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_tuple_pattern_let() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let (x, y) = (1, 2) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { pattern: Some(Pattern::Tuple(items)), .. } = &*statements[0].borrow()
    {
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], Pattern::Identifier(t) if t.value == "x"));
        assert!(matches!(&items[1], Pattern::Identifier(t) if t.value == "y"));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_pattern_wildcard() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let (x, _) = (1, 2) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { pattern: Some(Pattern::Tuple(items)), .. } = &*statements[0].borrow()
    {
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], Pattern::Identifier(t) if t.value == "x"));
        assert!(matches!(&items[1], Pattern::Ignore));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_pattern_typed() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let (x, y): (Int32, String) = (1, \"hello\") }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { pattern: Some(Pattern::Tuple(items)), type_expression: Some(_), .. } = &*statements[0].borrow()
    {
        assert_eq!(items.len(), 2);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_tuple_nested_pattern() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let (x, (y, z)) = (1, (2, 3)) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { pattern: Some(Pattern::Tuple(items)), .. } = &*statements[0].borrow()
    {
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], Pattern::Identifier(t) if t.value == "x"));
        assert!(matches!(&items[1], Pattern::Tuple(inner) if inner.len() == 2));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_var_tuple_pattern() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { var (a, b) = (10, 20) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { pattern: Some(Pattern::Tuple(items)), .. } = &*statements[0].borrow()
    {
        assert_eq!(items.len(), 2);
        assert!(matches!(&items[0], Pattern::Identifier(t) if t.value == "a"));
        assert!(matches!(&items[1], Pattern::Identifier(t) if t.value == "b"));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_ignore_pattern() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let _ = 42 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { pattern: Some(Pattern::Ignore), .. } = &*statements[0].borrow()
    {
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression,
            operator,
            is_prefix,
            ..
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression: outer_expr,
            operator,
            is_prefix,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::Deref);
        assert!(is_prefix);
        if let Expression::Unary {
            expression: inner_expr,
            operator: inner_op,
            is_prefix: inner_prefix,
            ..
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_address_of() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = &v }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression,
            operator,
            is_prefix,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::AddressOf);
        assert!(is_prefix);
        if let Expression::Variable { name, .. } = &*expression.borrow() {
            assert_eq!(name.value, "v");
        } else {
            panic!("Expected variable expression");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_address_of_deref() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = &*p }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression: outer,
            operator,
            is_prefix,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::AddressOf);
        assert!(is_prefix);
        if let Expression::Unary {
            expression: inner,
            operator: inner_op,
            is_prefix: inner_prefix,
            ..
        } = &*outer.borrow()
        {
            assert_eq!(inner_op, &UnaryOperator::Deref);
            assert!(inner_prefix);
            if let Expression::Variable { name, .. } = &*inner.borrow() {
                assert_eq!(name.value, "p");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected deref inside address-of");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_deref_address_of() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = *&v }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression: outer,
            operator,
            is_prefix,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::Deref);
        assert!(is_prefix);
        if let Expression::Unary {
            expression: inner,
            operator: inner_op,
            is_prefix: inner_prefix,
            ..
        } = &*outer.borrow()
        {
            assert_eq!(inner_op, &UnaryOperator::AddressOf);
            assert!(inner_prefix);
            if let Expression::Variable { name, .. } = &*inner.borrow() {
                assert_eq!(name.value, "v");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected address-of inside deref");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_address_of_binary() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = &a + &b }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Binary {
            left,
            right,
            operator,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &BinaryOperator::Plus);
        if let Expression::Unary {
            expression: left_inner,
            operator: left_op,
            ..
        } = &*left.borrow()
        {
            assert_eq!(left_op, &UnaryOperator::AddressOf);
            if let Expression::Variable { name, .. } = &*left_inner.borrow() {
                assert_eq!(name.value, "a");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected address-of on left");
        }
        if let Expression::Unary {
            expression: right_inner,
            operator: right_op,
            ..
        } = &*right.borrow()
        {
            assert_eq!(right_op, &UnaryOperator::AddressOf);
            if let Expression::Variable { name, .. } = &*right_inner.borrow() {
                assert_eq!(name.value, "b");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected address-of on right");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_fn_ptr_type_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func apply(fn: (Int32) -> Bool, x: Int32) -> Bool { return fn(x) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters.len(), 2);
        let fn_param = parameters[0].borrow();
        assert_eq!(fn_param.name.value, "fn");
        let te = fn_param.type_expression.borrow();
        if let Expression::FunctionType {
            param_types,
            return_type,
            ..
        } = &*te
        {
            assert_eq!(param_types.len(), 1);
            assert!(matches!(
                &*param_types[0].borrow(),
                Expression::Type { name, .. } if name.value == "Int32"
            ));
            assert!(matches!(
                &*return_type.borrow(),
                Expression::Type { name, .. } if name.value == "Bool"
            ));
        } else {
            panic!("Expected FunctionType expression for parameter type");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_addr_of_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let m = &MyStruct.method".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Unary {
            expression,
            operator,
            is_prefix,
            ..
        } = &*init
        {
            assert_eq!(operator, &UnaryOperator::AddressOf);
            assert!(is_prefix);
            if let Expression::MemberAccess { object, member, .. } = &*expression.borrow() {
                assert_eq!(member.value, "method");
                if let Expression::Variable { name, .. } = &*object.borrow() {
                    assert_eq!(name.value, "MyStruct");
                } else {
                    panic!("Expected Variable expression for type name");
                }
            } else {
                panic!("Expected MemberAccess expression");
            }
        } else {
            panic!("Expected Unary expression with AddressOf");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_addr_of_init() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let m = &MyStruct.init".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Unary {
            expression,
            operator,
            is_prefix,
            ..
        } = &*init
        {
            assert_eq!(operator, &UnaryOperator::AddressOf);
            assert!(is_prefix);
            if let Expression::MemberAccess { object, member, .. } = &*expression.borrow() {
                assert_eq!(member.value, "init");
                if let Expression::Variable { name, .. } = &*object.borrow() {
                    assert_eq!(name.value, "MyStruct");
                } else {
                    panic!("Expected Variable expression for type name");
                }
            } else {
                panic!("Expected MemberAccess expression");
            }
        } else {
            panic!("Expected Unary expression with AddressOf");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_addr_of_deinit() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let m = &MyClass.deinit".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Unary {
            expression,
            operator,
            is_prefix,
            ..
        } = &*init
        {
            assert_eq!(operator, &UnaryOperator::AddressOf);
            assert!(is_prefix);
            if let Expression::MemberAccess { object, member, .. } = &*expression.borrow() {
                assert_eq!(member.value, "deinit");
                if let Expression::Variable { name, .. } = &*object.borrow() {
                    assert_eq!(name.value, "MyClass");
                } else {
                    panic!("Expected Variable expression for type name");
                }
            } else {
                panic!("Expected MemberAccess expression");
            }
        } else {
            panic!("Expected Unary expression with AddressOf");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_deref_postfix_inc() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = *p++ }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression: outer,
            operator,
            is_prefix,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::Deref);
        assert!(is_prefix);
        if let Expression::Unary {
            expression: inner,
            operator: inner_op,
            is_prefix: inner_prefix,
            ..
        } = &*outer.borrow()
        {
            assert_eq!(inner_op, &UnaryOperator::Inc);
            assert!(!inner_prefix);
            if let Expression::Variable { name, .. } = &*inner.borrow() {
                assert_eq!(name.value, "p");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected postfix inc inside deref");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_deref_prefix_inc() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = *++p }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init_expr) = initializer
        && let Expression::Unary {
            expression: outer,
            operator,
            is_prefix,
            ..
        } = &*init_expr.borrow()
    {
        assert_eq!(operator, &UnaryOperator::Deref);
        assert!(is_prefix);
        if let Expression::Unary {
            expression: inner,
            operator: inner_op,
            is_prefix: inner_prefix,
            ..
        } = &*outer.borrow()
        {
            assert_eq!(inner_op, &UnaryOperator::Inc);
            assert!(inner_prefix);
            if let Expression::Variable { name, .. } = &*inner.borrow() {
                assert_eq!(name.value, "p");
            } else {
                panic!("Expected variable expression");
            }
        } else {
            panic!("Expected prefix inc inside deref");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_member_access_deinit_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { obj.deinit() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call {
            callee, parameters, ..
        } = &*expression.borrow()
        && let Expression::MemberAccess { object, member, .. } = &*callee.borrow()
    {
        if let Expression::Variable { name, .. } = &*object.borrow() {
            assert_eq!(name.value, "obj");
        } else {
            panic!("Expected variable expression");
        }
        assert_eq!(member.value, "deinit");
        assert_eq!(parameters.len(), 0);
    } else {
        panic!("Expected Call expression on MemberAccess with member 'deinit'");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_failable_init_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { init?() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 1);
        if let Statement::InitDecl {
            parameters,
            is_failable,
            ..
        } = &*body[0].borrow()
        {
            assert!(is_failable);
            assert_eq!(parameters.len(), 0);
        } else {
            panic!("Expected InitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_failable_init_decl_with_params() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { init?(x: Int32, y: Int32) {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 1);
        if let Statement::InitDecl {
            parameters,
            is_failable,
            ..
        } = &*body[0].borrow()
        {
            assert!(is_failable);
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
fn test_parse_non_failable_init_not_affected() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { init(x: Int32) {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        if let Statement::InitDecl { is_failable, .. } = &*body[0].borrow() {
            assert!(!is_failable);
        } else {
            panic!("Expected InitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_failable_init_in_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Point { init?() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 1);
        if let Statement::InitDecl { is_failable, .. } = &*body[0].borrow() {
            assert!(is_failable);
        } else {
            panic!("Expected InitDecl");
        }
    } else {
        panic!("Expected ClassDecl");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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

#[test]
fn test_parse_extension_static_method_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo {} extension Foo { static func bar() -> Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl {
            name,
            static_method,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(name.value, "bar");
            assert!(static_method);
        } else {
            panic!("Expected FunctionDecl in extension body");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_static_method_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Foo {} extension Foo { static func bar() -> Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl {
            name,
            static_method,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(name.value, "bar");
            assert!(static_method);
        } else {
            panic!("Expected FunctionDecl in extension body");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_static_method_enum() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Foo { case a } extension Foo { static func bar() -> Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl {
            name,
            static_method,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(name.value, "bar");
            assert!(static_method);
        } else {
            panic!("Expected FunctionDecl in extension body");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_static_method_protocol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Foo {} extension Foo { static func bar() -> Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl {
            name,
            static_method,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(name.value, "bar");
            assert!(static_method);
        } else {
            panic!("Expected FunctionDecl in extension body");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_instance_method_not_static() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo {} extension Foo { func bar() -> Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ExtensionDecl { body, .. } = &*program.statements[1].borrow() {
        if let Statement::FunctionDecl {
            name,
            static_method,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(name.value, "bar");
            assert!(!static_method);
        }
    }
}

#[test]
fn test_parse_extension_with_type_arguments() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Wrapper<T> {} extension Wrapper<Int32>: Computable {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ExtensionDecl {
        type_name,
        type_arguments,
        conformances,
        ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(type_name.value, "Wrapper");
        assert!(type_arguments.is_some());
        let args = type_arguments.as_ref().unwrap();
        assert_eq!(args.len(), 1);
        if let Expression::Type { name, .. } = &*args[0].borrow() {
            assert_eq!(name.value, "Int32");
        } else {
            panic!("Expected Type expression in type_arguments");
        }
        assert_eq!(conformances.len(), 1);
        if let Expression::Type { name, .. } = &*conformances[0].borrow() {
            assert_eq!(name.value, "Computable");
        } else {
            panic!("Expected Type expression for conformance");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_with_multiple_type_arguments() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo<A, B> {} extension Foo<Int32, String>: Bar {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ExtensionDecl { type_arguments, .. } = &*program.statements[1].borrow() {
        assert!(type_arguments.is_some());
        let args = type_arguments.as_ref().unwrap();
        assert_eq!(args.len(), 2);
        if let Expression::Type { name, .. } = &*args[0].borrow() {
            assert_eq!(name.value, "Int32");
        } else {
            panic!("Expected Int32");
        }
        if let Expression::Type { name, .. } = &*args[1].borrow() {
            assert_eq!(name.value, "String");
        } else {
            panic!("Expected String");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_extension_without_type_arguments_still_works() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo {} extension Foo: Bar {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ExtensionDecl { type_arguments, .. } = &*program.statements[1].borrow() {
        assert!(type_arguments.is_none());
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_parse_inline_method_where_clause() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Wrapper<T> { func compute() -> Int32 where T: Equatable { 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        if let Statement::FunctionDecl {
            name, where_clause, ..
        } = &*body[0].borrow()
        {
            assert_eq!(name.value, "compute");
            assert!(where_clause.is_some());
        } else {
            panic!("Expected FunctionDecl in struct body");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_inline_method_where_equality() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Wrapper<T> { func compute() -> Int32 where T == Int32 { 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        if let Statement::FunctionDecl {
            name, where_clause, ..
        } = &*body[0].borrow()
        {
            assert_eq!(name.value, "compute");
            assert!(where_clause.is_some());
            let requirements = where_clause.as_ref().unwrap();
            assert!(!requirements.is_empty());
            assert!(matches!(
                requirements[0].kind,
                WhereRequirementKind::Equality { .. }
            ));
        }
    }
}

fn collect_modifiers(stmt: &Statement) -> Vec<Modifier> {
    match stmt {
        Statement::FunctionDecl { modifiers, .. }
        | Statement::VariableDecl { modifiers, .. }
        | Statement::StructDecl { modifiers, .. }
        | Statement::ClassDecl { modifiers, .. }
        | Statement::EnumDecl { modifiers, .. }
        | Statement::ProtocolDecl { modifiers, .. }
        | Statement::InitDecl { modifiers, .. }
        | Statement::DeinitDecl { modifiers, .. }
        | Statement::ModuleDecl { modifiers, .. } => modifiers.clone(),
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Bar");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Internal);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_package_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("package func foo() {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "foo");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Package);
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_package_variable() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("package var x: Int32".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "x");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Package);
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_package_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("package struct Bar {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Bar");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Package);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_package_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "package class Point { let x: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ClassDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_has_access_modifier(&program.statements[0].borrow(), AccessModifier::Package);
    } else {
        panic!("Expected ClassDecl");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_duplicate_package_with_public() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "package public func foo() {}".to_string(),
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
fn test_duplicate_public_with_package() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public package func foo() {}".to_string(),
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
fn test_duplicate_package_with_private() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "package private func foo() {}".to_string(),
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
fn test_parse_private_set_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "private(set) var name: Int32".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl { modifiers, .. } = &*program.statements[0].borrow() {
        assert_eq!(modifiers.len(), 1);
        assert!(matches!(
            modifiers[0].ty,
            ModifierType::AccessSet(AccessModifier::Private)
        ));
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_public_private_set_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public private(set) var name: Int32".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl { modifiers, .. } = &*program.statements[0].borrow() {
        assert_eq!(modifiers.len(), 2);
        assert!(matches!(
            modifiers[0].ty,
            ModifierType::Access(AccessModifier::Public)
        ));
        assert!(matches!(
            modifiers[1].ty,
            ModifierType::AccessSet(AccessModifier::Private)
        ));
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_internal_set_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public internal(set) var name: Int32".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl { modifiers, .. } = &*program.statements[0].borrow() {
        assert_eq!(modifiers.len(), 2);
        assert!(matches!(
            modifiers[0].ty,
            ModifierType::Access(AccessModifier::Public)
        ));
        assert!(matches!(
            modifiers[1].ty,
            ModifierType::AccessSet(AccessModifier::Internal)
        ));
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_var_private_set_accessor() {
    let engine = create_engine();
    let lexer_result = {
        let mut lexer = Lexer::new(
            CharStream::new(
                "var v: Int32 { get { 1 } private set {} }".to_string(),
                Rc::new("".to_string()),
            ),
            engine.clone(),
        );
        lexer.parse()
    };
    let mut parser = Parser::new(Rc::new("".to_string()), lexer_result, engine.clone());
    let program = parser.parse();
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    drop(engine_ref);
    if program.statements.is_empty() {
        panic!("No statements parsed");
    }
    if let Statement::VariableDecl {
        name, accessors, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert_eq!(accessors.len(), 2);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
        assert!(accessors[0].set_access_modifier.is_none());
        assert_eq!(accessors[1].kind, AccessorKind::Set);
        assert_eq!(
            accessors[1].set_access_modifier,
            Some(AccessModifier::Private)
        );
    } else {
        panic!(
            "Expected VariableDecl, got {:?}",
            program.statements[0].borrow()
        );
    }
}

#[test]
fn test_parse_var_internal_set_accessor() {
    let engine = create_engine();
    let lexer_result = {
        let mut lexer = Lexer::new(
            CharStream::new(
                "var v: Int32 { get { 1 } internal set(newValue) { _v = newValue } }".to_string(),
                Rc::new("".to_string()),
            ),
            engine.clone(),
        );
        lexer.parse()
    };
    let mut parser = Parser::new(Rc::new("".to_string()), lexer_result, engine.clone());
    let program = parser.parse();
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    drop(engine_ref);
    if let Statement::VariableDecl {
        name, accessors, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "v");
        assert_eq!(accessors.len(), 2);
        assert_eq!(accessors[1].kind, AccessorKind::Set);
        assert_eq!(
            accessors[1].set_access_modifier,
            Some(AccessModifier::Internal)
        );
        assert!(accessors[1].parameter.is_some());
        assert_eq!(accessors[1].parameter.as_ref().unwrap().value, "newValue");
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_subscript_private_set_accessor() {
    let engine = create_engine();
    let lexer_result = {
        let mut lexer = Lexer::new(
            CharStream::new(
                "struct Foo { subscript(i: Int32) -> Int32 { get { _v } private set(v) { _v = v } } }"
                    .to_string(),
                Rc::new("".to_string()),
            ),
            engine.clone(),
        );
        lexer.parse()
    };
    let mut parser = Parser::new(Rc::new("".to_string()), lexer_result, engine.clone());
    let program = parser.parse();
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty(), "Unexpected errors: {:?}", errors);
    drop(engine_ref);
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        let member = &body[0];
        if let Statement::SubscriptDecl { accessors, .. } = &*member.borrow() {
            assert_eq!(accessors.len(), 2);
            assert_eq!(accessors[0].kind, AccessorKind::Get);
            assert_eq!(accessors[1].kind, AccessorKind::Set);
            assert_eq!(
                accessors[1].set_access_modifier,
                Some(AccessModifier::Private)
            );
        } else {
            panic!("Expected SubscriptDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_enum_with_raw_value_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum E: UInt8 { case a, b }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::EnumDecl {
        name, conformances, cases, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "E");
        assert_eq!(conformances.len(), 1);
        if let Expression::Type { name: ty_name, .. } = &*conformances[0].borrow() {
            assert_eq!(ty_name.value, "UInt8");
        } else {
            panic!("Expected Type expression for raw value type");
        }
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].name.value, "a");
        assert_eq!(cases[1].name.value, "b");
    } else {
        panic!("Expected EnumDecl");
    }
}

#[test]
fn test_parse_enum_with_multiple_conformances() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum E: UInt8, Equatable { case a, b }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::EnumDecl {
        name, conformances, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "E");
        assert_eq!(conformances.len(), 2);
        if let Expression::Type { name: ty_name, .. } = &*conformances[0].borrow() {
            assert_eq!(ty_name.value, "UInt8");
        } else {
            panic!("Expected Type expression for UInt8");
        }
        if let Expression::Type { name: ty_name, .. } = &*conformances[1].borrow() {
            assert_eq!(ty_name.value, "Equatable");
        } else {
            panic!("Expected Type expression for Equatable");
        }
    } else {
        panic!("Expected EnumDecl");
    }
}

#[test]
fn test_parse_enum_with_protocol_conformance() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum E: Equatable { case a, b }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::EnumDecl {
        name, conformances, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "E");
        assert_eq!(conformances.len(), 1);
        if let Expression::Type { name: ty_name, .. } = &*conformances[0].borrow() {
            assert_eq!(ty_name.value, "Equatable");
        } else {
            panic!("Expected Type expression for Equatable");
        }
    } else {
        panic!("Expected EnumDecl");
    }
}

#[test]
fn test_parse_if_case_no_bindings() {
    let code = r#"
        enum Option { case None case Some }
        func test(x: Option) {
            if case Option.None = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert_eq!(case_name.value, "None");
        assert!(bindings.is_empty());
        assert!(then.is_empty());
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
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case Option.Some(val) = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert_eq!(case_name.value, "Some");
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
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case Option.Some(val) = x {
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert_eq!(case_name.value, "Some");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        if let Some(ElseBranch::If(else_expr)) = else_ {
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        enum Option { case None case Some }
        func test(x: Option) -> Bool {
            return case Option.None = x
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_non_null_pointer_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Int32*! }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression: Some(type_expr),
            ..
        } = &*statements[0].borrow()
        && let Expression::PointerType { base, non_null, .. } = &*type_expr.borrow()
        && let Expression::Type { name, .. } = &*base.borrow()
    {
        assert_eq!(name.value, "Int32");
        assert!(non_null);
    } else {
        panic!("Expected non-null pointer type");
    }
}

#[test]
fn test_parse_non_null_pointer_in_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(p: Int32*!) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow()
        && let Some(param) = parameters.first()
        && let Expression::PointerType { base, non_null, .. } =
            &*param.borrow().type_expression.borrow()
        && let Expression::Type { name, .. } = &*base.borrow()
    {
        assert_eq!(name.value, "Int32");
        assert!(non_null);
    } else {
        panic!("Expected non-null pointer parameter");
    }
}

#[test]
fn test_parse_non_null_ptr_star_bang() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Int32**! }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression: Some(type_expr),
            ..
        } = &*statements[0].borrow()
        && let Expression::PointerType { base, non_null, .. } = &*type_expr.borrow()
        && *non_null
        && let Expression::PointerType {
            base: inner_base,
            non_null: inner_non_null,
            ..
        } = &*base.borrow()
        && !*inner_non_null
        && let Expression::Type { name, .. } = &*inner_base.borrow()
    {
        assert_eq!(name.value, "Int32");
    } else {
        panic!("Expected Int32**! (non-null outer, nullable inner)");
    }
}

#[test]
fn test_parse_regular_pointer_still_has_non_null_false() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Int32* }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression: Some(type_expr),
            ..
        } = &*statements[0].borrow()
        && let Expression::PointerType { non_null, .. } = &*type_expr.borrow()
    {
        assert!(!non_null);
    } else {
        panic!("Expected regular pointer type");
    }
}

#[test]
fn test_parse_tuple_non_null_pointer_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: (Int32, Bool)*! }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression: Some(type_expr),
            ..
        } = &*statements[0].borrow()
        && let Expression::PointerType { base, non_null, .. } = &*type_expr.borrow()
        && *non_null
        && let Expression::TupleType { elements, .. } = &*base.borrow()
    {
        assert_eq!(elements.len(), 2);
    } else {
        panic!("Expected non-null tuple pointer type");
    }
}

#[test]
fn test_parse_tuple_index_access_dot() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let a = (1, 2).0".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_super_keyword() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { super }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
    {
        assert!(matches!(
            *expression.borrow(),
            Expression::SuperKeyword { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_super_member_access() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { super.field }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::MemberAccess { object, member, .. } = &*expression.borrow()
    {
        assert_eq!(member.value, "field");
        assert!(matches!(*object.borrow(), Expression::SuperKeyword { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_super_method_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { super.method() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call { callee, .. } = &*expression.borrow()
        && let Expression::MemberAccess { object, member, .. } = &*callee.borrow()
    {
        assert_eq!(member.value, "method");
        assert!(matches!(*object.borrow(), Expression::SuperKeyword { .. }));
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_protocol_with_autowired_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Copyable { #[autowired] func copy() -> Self }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::ProtocolDecl { name, members, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Copyable");
        assert_eq!(members.len(), 1);
        if let ProtocolMember::Method {
            attributes, decl, ..
        } = &members[0]
        {
            assert_eq!(attributes.len(), 1);
            assert_eq!(attributes[0].name, "autowired");
            if let Statement::FunctionDecl {
                name: func_name,
                return_type,
                ..
            } = &*decl.borrow()
            {
                assert_eq!(func_name.value, "copy");
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
fn test_parse_internal_used_attribute_on_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#[internalUsed] public func internalHelper() -> Int32 { return 42 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::FunctionDecl {
        attributes, name, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "internalHelper");
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name, "internalUsed");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_some_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let x: some MyProtocol".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl {
        name,
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "x");
        assert!(
            matches!(&*ty_expr.borrow(), Expression::SomeType { .. }),
            "Expected SomeType, got {:?}",
            &*ty_expr.borrow()
        );
        if let Expression::SomeType { inner, .. } = &*ty_expr.borrow() {
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
fn test_parse_some_type_in_function_return() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo() -> some MyProtocol { return makeMyProtocol() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        name,
        return_type: Some(ret_ty),
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "foo");
        assert!(
            matches!(&*ret_ty.borrow(), Expression::SomeType { .. }),
            "Expected SomeType return type, got {:?}",
            &*ret_ty.borrow()
        );
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_some_type_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let x: some MyProtocol".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with 'some' type, got: {:?}",
        errors
    );
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_inline_type_auto() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: inline Dog".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl {
        name,
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "x");
        assert!(
            matches!(
                &*ty_expr.borrow(),
                Expression::InlineType { size: None, .. }
            ),
            "Expected InlineType with no size, got {:?}",
            &*ty_expr.borrow()
        );
        if let Expression::InlineType { base, .. } = &*ty_expr.borrow() {
            assert!(
                matches!(&*base.borrow(), Expression::Type { name, .. } if name.value == "Dog"),
                "Expected Type(Dog), got {:?}",
                &*base.borrow()
            );
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_inline_type_explicit_size() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let x: inline<256> Dog".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl {
        name,
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "x");
        assert!(
            matches!(
                &*ty_expr.borrow(),
                Expression::InlineType { size: Some(_), .. }
            ),
            "Expected InlineType with size, got {:?}",
            &*ty_expr.borrow()
        );
        if let Expression::InlineType { size, base, .. } = &*ty_expr.borrow() {
            assert!(size.is_some());
            assert!(
                matches!(&*base.borrow(), Expression::Type { name, .. } if name.value == "Dog"),
                "Expected Type(Dog), got {:?}",
                &*base.borrow()
            );
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_inline_type_empty_brackets() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: inline<> Dog".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::VariableDecl {
        name,
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "x");
        assert!(
            matches!(
                &*ty_expr.borrow(),
                Expression::InlineType { size: None, .. }
            ),
            "Expected InlineType with None (empty brackets), got {:?}",
            &*ty_expr.borrow()
        );
        if let Expression::InlineType { base, .. } = &*ty_expr.borrow() {
            assert!(
                matches!(&*base.borrow(), Expression::Type { name, .. } if name.value == "Dog"),
                "Expected Type(Dog), got {:?}",
                &*base.borrow()
            );
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert!(
            matches!(&generic_parameters[0].kind, GenericParameterKind::Type { constraints } if constraints.is_empty())
        );
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        let constraints = match &generic_parameters[0].kind {
            GenericParameterKind::Type { constraints } => constraints,
            _ => panic!("expected Type constraint"),
        };
        assert_eq!(constraints.len(), 1);
        if let Expression::Type { name, .. } = &*constraints[0].borrow() {
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        let constraints = match &generic_parameters[0].kind {
            GenericParameterKind::Type { constraints } => constraints,
            _ => panic!("expected Type constraint"),
        };
        assert_eq!(constraints.len(), 1);
        if let Expression::CompoundType { types, .. } = &*constraints[0].borrow() {
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert!(
            matches!(&generic_parameters[0].kind, GenericParameterKind::Type { constraints } if constraints.len() == 1)
        );
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_enum() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Option<T> { case None case Some(T) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert!(matches!(members[0], ProtocolMember::Method { .. }));
        assert_eq!(members.len(), 1);
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_const_generic_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<let N: Int32>(x: Int32) -> Int32 { return x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "N");
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Const { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_const_generic_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Buffer<T, let N: Int32> { var data: T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl {
        name,
        generic_parameters,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Buffer");
        assert_eq!(generic_parameters.len(), 2);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Type { .. }
        ));
        assert_eq!(generic_parameters[1].name.value, "N");
        assert!(matches!(
            &generic_parameters[1].kind,
            GenericParameterKind::Const { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_const_generic_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Wrapper<T, let N: Int32> { var item: T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        generic_parameters,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Wrapper");
        assert_eq!(generic_parameters.len(), 2);
        assert_eq!(generic_parameters[1].name.value, "N");
        assert!(matches!(
            &generic_parameters[1].kind,
            GenericParameterKind::Const { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_const_generic_enum() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum MyEnum<T, let N: Int32> {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::EnumDecl {
        name,
        generic_parameters,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "MyEnum");
        assert_eq!(generic_parameters.len(), 2);
        assert_eq!(generic_parameters[1].name.value, "N");
        assert!(matches!(
            &generic_parameters[1].kind,
            GenericParameterKind::Const { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_const_generic_multi_params() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func multi<let Width: Int32, let Height: Int32>() -> Int32 { return 0 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 2);
        assert_eq!(generic_parameters[0].name.value, "Width");
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Const { .. }
        ));
        assert_eq!(generic_parameters[1].name.value, "Height");
        assert!(matches!(
            &generic_parameters[1].kind,
            GenericParameterKind::Const { .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_const_generic_missing_colon_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<let N Int32>(x: Int32) -> Int32 { return x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _ = parser.parse();
    assert!(engine.borrow().get_errors().len() > 0);
}

#[test]
fn test_parse_type_params_with_literal_at_call_site() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = foo<256>() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(program.statements.len() > 0);
}

#[test]
fn test_parse_type_params_with_mixed_type_and_literal() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = make<Int32, 256>() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(program.statements.len() > 0);
}

#[test]
fn test_parse_protocol_with_const_generic() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Container<T, let N: Int32> { func get() -> T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ProtocolDecl {
        name,
        generic_parameters,
        members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Container");
        assert_eq!(generic_parameters.len(), 2);
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Type { .. }
        ));
        assert!(matches!(
            &generic_parameters[1].kind,
            GenericParameterKind::Const { .. }
        ));
        let associated_count = members
            .iter()
            .filter(|m| matches!(m, ProtocolMember::AssociatedType { .. }))
            .count();
        assert_eq!(associated_count, 0);
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_repr_c_attribute() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#[repr(C)] struct S { var x: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl { name, attributes, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "S");
        assert!(
            attributes.iter().any(|a| a.name == "repr" && a.value.as_deref() == Some("C")),
            "Expected #[repr(C)] attribute on struct S"
        );
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_builtintype_attribute() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#[builtintype] public struct Int32: SignedInteger {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::StructDecl {
        attributes,
        name,
        modifiers,
        conformances,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Int32");
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name, "builtintype");
        assert_eq!(modifiers.len(), 1);
        assert!(matches!(
            modifiers[0].ty,
            ModifierType::Access(AccessModifier::Public)
        ));
        assert_eq!(conformances.len(), 1);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_builtintype_attribute_on_empty_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#[builtintype] public struct Bool {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::StructDecl {
        attributes, name, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Bool");
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name, "builtintype");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_struct_without_attribute_still_works() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public struct Point { let x: Int32 let y: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::StructDecl {
        attributes, name, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Point");
        assert!(attributes.is_empty());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { .. } = &*program.statements[0].borrow() {
    } else {
        panic!();
    }
}

#[test]
fn test_parse_if_case_dot_shorthand() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case .Some(val) = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert_eq!(case_name.value, "Some");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::If { condition, .. } = &*expression.borrow()
        && let Expression::Binary {
            left,
            operator,
            right,
            ..
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
        enum Option { case None case Some(Int32) }
        func test(x: Option) -> Int32 {
            match x {
                case .Some(let val):
                    val
                case .None:
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { value, cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 3, "expected 3 cases (some, none, default)");
        assert!(matches!(&*value.borrow(), Expression::Variable { name, .. } if name.value == "x"));
        assert!(matches!(cases[0].patterns[0].as_ref(),
            Pattern::EnumCase { case_name, .. } if case_name.value == "Some"));
        if let Pattern::EnumCase { bindings, .. } = cases[0].patterns[0].as_ref() {
            assert_eq!(bindings.len(), 1);
            assert!(matches!(&bindings[0], Pattern::ValueBinding(_)));
        }
        assert!(
            matches!(cases[1].patterns[0].as_ref(), Pattern::EnumCase { case_name, .. } if case_name.value == "None")
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            guard case .Some(val) = x else {
                return
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert!(!else_body.is_empty());
    } else {
        panic!("Expected Guard statement");
    }
}

#[test]
fn test_parse_fallthrough_and_break() {
    let code = r#"
        enum Option { case None case Some }
        func test(x: Option) {
            match x {
                case .None:
                    fallthrough
                case .Some:
                    break
            }
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Match { cases, .. } = &*expression.borrow()
    {
        assert_eq!(cases.len(), 2);
        assert!(matches!(
            &*cases[0].body[0].borrow(),
            Statement::Fallthrough { .. }
        ));
        assert!(matches!(
            &*cases[1].body[0].borrow(),
            Statement::Break { .. }
        ));
    } else {
        panic!("Expected Match with fallthrough/break");
    }
}

#[test]
fn test_parse_pattern_value_binding() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case .Some(let val) = x {}
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert_eq!(case_name.value, "Some");
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Defer {
            body: defer_body, ..
        } = &*statements[0].borrow()
    {
        assert_eq!(defer_body.len(), 1);
        assert!(matches!(
            &*defer_body[0].borrow(),
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Defer {
            body: outer_defer_body,
            ..
        } = &*statements[0].borrow()
    {
        assert_eq!(outer_defer_body.len(), 2);
        assert!(matches!(
            &*outer_defer_body[0].borrow(),
            Statement::Defer { .. }
        ));
        assert!(matches!(
            &*outer_defer_body[1].borrow(),
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        assert!(!then.is_empty());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::If { else_, .. } = &*init.borrow()
    {
        assert!(matches!(else_, Some(ElseBranch::If(_))));
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
        CharStream::new("import A.B.C.D.foo".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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

#[test]
fn test_parse_import_package_module() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import package.Foo".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_import_package_module_nested() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import package.Foo.Bar".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_import_package_member() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import package.Foo.Bar.baz".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_import_package_wildcard() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import package.Foo.*".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ImportDecl { path, kind, .. } = &*program.statements[0].borrow() {
        assert_eq!(path, &vec!["Foo".to_string()]);
        assert_eq!(*kind, ImportKind::Wildcard);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_package_deep_member() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import package.A.B.C.D.foo".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_import_package_deep_wildcard() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import package.A.B.C.*".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_import_package_not_followed_by_dot_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import package".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine.borrow().get_errors().len() > 0,
        "Expected error for 'import package' without '.'"
    );
}

#[test]
fn test_parse_import_package_dot_only_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import package.".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine.borrow().get_errors().len() > 0,
        "Expected error for 'import package.'"
    );
}

#[test]
fn test_parse_import_regular_ident_still_works() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("import Foo".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
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
fn test_parse_import_inside_function_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { import Foo }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine.borrow().get_errors().len() > 0,
        "Expected error for import inside function body"
    );
}

#[test]
fn test_parse_import_inside_struct_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { import Bar }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine.borrow().get_errors().len() > 0,
        "Expected error for import inside struct body"
    );
}

#[test]
fn test_parse_import_at_file_level_ok() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import Foo\nimport Bar".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors for import at file level"
    );
}

#[test]
fn test_parse_generic_call_with_type_args() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { foo<Int32>(42) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Call {
            callee,
            type_parameters,
            parameters,
            ..
        } = &*expression.borrow()
        && let Expression::Variable { name, .. } = &*callee.borrow()
    {
        assert_eq!(name.value, "foo");
        assert!(type_parameters.is_some());
        let tps = type_parameters.as_ref().unwrap();
        assert_eq!(tps.len(), 1);
        assert!(
            matches!(&*tps[0].borrow(), Expression::Type { name, .. } if name.value == "Int32")
        );
        assert_eq!(parameters.len(), 1);
    } else {
        panic!("Expected generic Call with type parameter Int32");
    }
}

#[test]
fn test_parse_generic_call_multi_type_args() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { pair<Int32, String>(1, 2) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Call {
            type_parameters,
            parameters,
            ..
        } = &*expression.borrow()
    {
        assert!(type_parameters.is_some());
        let tps = type_parameters.as_ref().unwrap();
        assert_eq!(tps.len(), 2);
        assert!(
            matches!(&*tps[0].borrow(), Expression::Type { name, .. } if name.value == "Int32")
        );
        assert!(
            matches!(&*tps[1].borrow(), Expression::Type { name, .. } if name.value == "String")
        );
        assert_eq!(parameters.len(), 2);
    } else {
        panic!("Expected generic Call with two type parameters");
    }
}

#[test]
fn test_parse_generic_call_nested_type_args() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { foo<Array<Int32>>(x) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Call {
            type_parameters, ..
        } = &*expression.borrow()
    {
        assert!(type_parameters.is_some());
        let tps = type_parameters.as_ref().unwrap();
        assert_eq!(tps.len(), 1);
        assert!(
            matches!(&*tps[0].borrow(), Expression::Type { name, type_parameters: inner_tps, .. }
                if name.value == "Array" && inner_tps.is_some() && inner_tps.as_ref().unwrap().len() == 1)
        );
    } else {
        panic!("Expected generic Call with nested Array<Int32> type parameter");
    }
}

#[test]
fn test_parse_nested_generic_type_double_gt() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Array<Array<Int32>> }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression, ..
        } = &*stmts[0].borrow()
        && let Some(ty_expr) = type_expression
        && let Expression::Type {
            name,
            type_parameters,
            ..
        } = &*ty_expr.borrow()
    {
        assert_eq!(name.value, "Array");
        assert!(type_parameters.is_some());
        let tps = type_parameters.as_ref().unwrap();
        assert_eq!(tps.len(), 1);
        assert!(
            matches!(&*tps[0].borrow(), Expression::Type { name, type_parameters: inner_tps, .. }
                if name.value == "Array" && inner_tps.is_some() && inner_tps.as_ref().unwrap().len() == 1)
        );
    } else {
        panic!("Expected nested generic Array<Array<Int32>> type");
    }
}

#[test]
fn test_parse_generic_method_call_with_type_args() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(x: Foo) { x.identity<Int32>() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Call {
            callee,
            type_parameters,
            ..
        } = &*expression.borrow()
        && let Expression::MemberAccess { object, member, .. } = &*callee.borrow()
    {
        assert_eq!(member.value, "identity");
        assert!(type_parameters.is_some());
        let tps = type_parameters.as_ref().unwrap();
        assert_eq!(tps.len(), 1);
        assert!(
            matches!(&*tps[0].borrow(), Expression::Type { name, .. } if name.value == "Int32")
        );
        assert!(
            matches!(&*object.borrow(), Expression::Variable { name, .. } if name.value == "x")
        );
    } else {
        panic!("Expected generic method call x.identity<Int32>()");
    }
}

#[test]
fn test_parse_where_clause_equality() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func same<T, U>(a: T, b: U) -> Bool where T == U".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { where_clause, .. } = &*program.statements[0].borrow() {
        assert!(where_clause.is_some());
        let reqs = where_clause.as_ref().unwrap();
        assert_eq!(reqs.len(), 1);
        assert!(matches!(
            reqs[0].kind,
            WhereRequirementKind::Equality { .. }
        ));
    } else {
        panic!("Expected FunctionDecl with equality where clause");
    }
}

#[test]
fn test_parse_generic_class_with_where_clause() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Box<T> where T: Hashable { var value: T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ClassDecl {
        name,
        generic_parameters,
        where_clause,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Box");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(where_clause.is_some());
    } else {
        panic!("Expected ClassDecl with where clause");
    }
}

#[test]
fn test_parse_generic_enum_with_where_clause() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "enum Option<T> where T: Hashable { case None case Some(T) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::EnumDecl {
        name,
        generic_parameters,
        where_clause,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Option");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(where_clause.is_some());
    } else {
        panic!("Expected EnumDecl with where clause");
    }
}

#[test]
fn test_parse_generic_protocol_with_where_clause() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Container<T> where T: Equatable { func get() -> T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ProtocolDecl {
        name,
        generic_parameters,
        where_clause,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Container");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(where_clause.is_some());
    } else {
        panic!("Expected ProtocolDecl with where clause");
    }
}

#[test]
fn test_parse_closure_typed_params() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { (x: Int32, y: Int32) -> Int32 in x + y }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure {
            parameters,
            return_type,
            body,
            ..
        } = &*init
        {
            assert_eq!(parameters.len(), 2);
            let p0 = parameters[0].borrow();
            assert_eq!(p0.name.value, "x");
            assert!(p0.type_annotation.is_some());
            let p1 = parameters[1].borrow();
            assert_eq!(p1.name.value, "y");
            assert!(p1.type_annotation.is_some());
            assert!(return_type.is_some());
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_no_return_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { (x, y) in x + y }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure {
            parameters,
            return_type,
            ..
        } = &*init
        {
            assert_eq!(parameters.len(), 2);
            let p0 = parameters[0].borrow();
            assert_eq!(p0.name.value, "x");
            assert!(p0.type_annotation.is_none());
            let p1 = parameters[1].borrow();
            assert_eq!(p1.name.value, "y");
            assert!(p1.type_annotation.is_none());
            assert!(return_type.is_none());
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_no_params() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let f = { in 42 }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure {
            parameters,
            return_type,
            ..
        } = &*init
        {
            assert_eq!(parameters.len(), 0);
            assert!(return_type.is_none());
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_multi_statement_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { (x: Int32) in let y = x + 1; return y }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure {
            parameters, body, ..
        } = &*init
        {
            assert_eq!(parameters.len(), 1);
            assert!(body.len() >= 1);
            assert!(matches!(
                &*body[body.len() - 1].borrow(),
                Statement::Return { .. }
            ));
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_function_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f: (Int32, Int32) -> Bool".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl {
        type_expression, ..
    } = &*program.statements[0].borrow()
    {
        let t = type_expression.as_ref().unwrap().borrow();
        if let Expression::FunctionType {
            param_types,
            return_type,
            ..
        } = &*t
        {
            assert_eq!(param_types.len(), 2);
            assert!(matches!(
                &*param_types[0].borrow(),
                Expression::Type { name, .. } if name.value == "Int32"
            ));
            assert!(matches!(
                &*return_type.borrow(),
                Expression::Type { name, .. } if name.value == "Bool"
            ));
        } else {
            panic!("Expected FunctionType expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_function_type_single_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f: (Int32) -> Bool".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl {
        type_expression, ..
    } = &*program.statements[0].borrow()
    {
        let t = type_expression.as_ref().unwrap().borrow();
        if let Expression::FunctionType { param_types, .. } = &*t {
            assert_eq!(param_types.len(), 1);
            assert!(matches!(
                &*param_types[0].borrow(),
                Expression::Type { name, .. } if name.value == "Int32"
            ));
        } else {
            panic!("Expected FunctionType expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_function_type_void_params() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let f: () -> Void".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl {
        type_expression, ..
    } = &*program.statements[0].borrow()
    {
        let t = type_expression.as_ref().unwrap().borrow();
        if let Expression::FunctionType { param_types, .. } = &*t {
            assert_eq!(param_types.len(), 0);
        } else {
            panic!("Expected FunctionType expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_with_function_type_annotation() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f: (Int32) -> Int32 = { (x: Int32) -> Int32 in x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl {
        type_expression,
        initializer,
        ..
    } = &*program.statements[0].borrow()
    {
        assert!(type_expression.is_some());
        let t = type_expression.as_ref().unwrap().borrow();
        assert!(matches!(&*t, Expression::FunctionType { .. }));
        let init = initializer.as_ref().unwrap().borrow();
        assert!(matches!(&*init, Expression::Closure { .. }));
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_block_not_closure() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let b = { 42 }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        assert!(matches!(&*init, Expression::Closure { .. }));
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_shorthand_argument() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { $0 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
            && let Some(init) = initializer
            && let Expression::Closure {
                body: closure_body, ..
            } = &*init.borrow()
        {
            if let Statement::ExpressionStatement { expression } = &*closure_body[0].borrow() {
                assert!(matches!(
                    &*expression.borrow(),
                    Expression::ShorthandArgument { .. }
                ));
            } else {
                panic!("Expected ExpressionStatement in closure body");
            }
        } else {
            panic!("Expected VariableDecl with closure initializer");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_shorthand_argument_plus() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { $0 + $1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
            && let Some(init) = initializer
            && let Expression::Closure {
                body: closure_body, ..
            } = &*init.borrow()
        {
            assert_eq!(closure_body.len(), 1);
            if let Statement::ExpressionStatement { expression } = &*closure_body[0].borrow() {
                if let Expression::Binary {
                    left,
                    right,
                    operator,
                    ..
                } = &*expression.borrow()
                {
                    assert_eq!(*operator, BinaryOperator::Plus);
                    assert!(matches!(
                        &*left.borrow(),
                        Expression::ShorthandArgument { index: 0, .. }
                    ));
                    assert!(matches!(
                        &*right.borrow(),
                        Expression::ShorthandArgument { index: 1, .. }
                    ));
                } else {
                    panic!("Expected Binary expression");
                }
            } else {
                panic!("Expected ExpressionStatement");
            }
        } else {
            panic!("Expected VariableDecl with closure initializer");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_shorthand_argument_multi() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { $0 + $1 * $2 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
            && let Some(init) = initializer
            && let Expression::Closure {
                body: closure_body, ..
            } = &*init.borrow()
        {
            assert_eq!(closure_body.len(), 1);
            if let Statement::ExpressionStatement { expression } = &*closure_body[0].borrow() {
                let mut shorthand_count = 0;
                collect_shorthand_args(expression, &mut shorthand_count);
                assert_eq!(shorthand_count, 3);
            }
        } else {
            panic!("Expected VariableDecl with closure initializer");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_subscript_decl_get() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "subscript(index: Int32) -> Int32 { get { return 0 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::SubscriptDecl {
        parameters,
        accessors,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].borrow().name.value, "index");
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
    } else {
        panic!("Expected SubscriptDecl");
    }
}

#[test]
fn test_parse_subscript_decl_get_set() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "subscript(index: Int32) -> Int32 { get { return 0 } set(newValue) { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::SubscriptDecl {
        parameters,
        accessors,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(parameters.len(), 1);
        assert_eq!(accessors.len(), 2);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
        assert_eq!(accessors[1].kind, AccessorKind::Set);
        assert_eq!(accessors[1].parameter.as_ref().unwrap().value, "newValue");
    } else {
        panic!("Expected SubscriptDecl");
    }
}

#[test]
fn test_parse_subscript_decl_implicit_get() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "subscript(index: Int32) -> Int32 { return 0 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::SubscriptDecl {
        parameters,
        accessors,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(parameters.len(), 1);
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].kind, AccessorKind::Get);
    } else {
        panic!("Expected SubscriptDecl");
    }
}

#[test]
fn test_parse_subscript_with_label() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "subscript(key: String) -> Int32 { get { return 0 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::SubscriptDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].borrow().name.value, "key");
    } else {
        panic!("Expected SubscriptDecl");
    }
}

#[test]
fn test_parse_subscript_access_single_index() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("arr[0]".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        if let Expression::SubscriptAccess {
            object, parameters, ..
        } = &*expression.borrow()
        {
            if let Expression::Variable { name, .. } = &*object.borrow() {
                assert_eq!(name.value, "arr");
            } else {
                panic!("Expected Variable as object");
            }
            assert_eq!(parameters.len(), 1);
            if let Expression::IntegerLiteral { value, .. } = &*parameters[0].expression.borrow() {
                assert_eq!(*value, 0);
            } else {
                panic!("Expected IntegerLiteral as index");
            }
        } else {
            panic!("Expected SubscriptAccess");
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_subscript_access_multi_index() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("matrix[row, col]".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        if let Expression::SubscriptAccess { parameters, .. } = &*expression.borrow() {
            assert_eq!(parameters.len(), 2);
        } else {
            panic!("Expected SubscriptAccess");
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_subscript_in_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Matrix { subscript(row: Int32, col: Int32) -> Int32 { get { return 0 } } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert!(
            body.iter()
                .any(|s| matches!(&*s.borrow(), Statement::SubscriptDecl { .. })),
            "struct body has {} items, expected SubscriptDecl",
            body.len()
        );
        let subscript_stmt = body
            .iter()
            .find(|s| matches!(&*s.borrow(), Statement::SubscriptDecl { .. }))
            .unwrap();
        if let Statement::SubscriptDecl { parameters, .. } = &*subscript_stmt.borrow() {
            assert_eq!(parameters.len(), 2);
            assert_eq!(parameters[0].borrow().name.value, "row");
            assert_eq!(parameters[1].borrow().name.value, "col");
        } else {
            panic!("Expected SubscriptDecl in struct body");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_subscript_in_protocol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Subscriptable { subscript(index: Int32) -> Int32 { get set } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        let subscript_members: Vec<_> = members
            .iter()
            .filter(|m| matches!(m, ProtocolMember::Subscript { .. }))
            .collect();
        assert_eq!(subscript_members.len(), 1);
        if let ProtocolMember::Subscript {
            parameters,
            accessors,
            ..
        } = subscript_members[0]
        {
            assert_eq!(parameters.len(), 1);
            assert_eq!(accessors.get, true);
            assert_eq!(accessors.set, true);
        } else {
            panic!("Expected ProtocolMember::Subscript");
        }
    } else {
        panic!("Expected ProtocolDecl");
    }
}

#[test]
fn test_parse_subscript_in_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class MyArray { subscript(index: Int32) -> Int32 { get { return 0 } set { } } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ClassDecl { body, .. } = &*program.statements[0].borrow() {
        assert!(
            body.iter()
                .any(|s| matches!(&*s.borrow(), Statement::SubscriptDecl { .. }))
        );
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_parse_macro_decl_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "macro id { ($x:expr) => { $x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::MacroDecl { name, arms, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "id");
        assert_eq!(arms.len(), 1);
        assert_eq!(arms[0].pattern.len(), 1);
        if let MacroPatternFragment::MetaVar { name: n, var_type } = &arms[0].pattern[0] {
            assert_eq!(n, "x");
            assert_eq!(*var_type, MacroMetaVarType::Expr);
        } else {
            panic!("Expected MetaVar pattern fragment");
        }
    } else {
        panic!("Expected MacroDecl");
    }
}

#[test]
fn test_parse_macro_decl_multi_arm() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "macro test { ($x:expr) => { $x } ($x:expr, $y:expr) => { $x + $y } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::MacroDecl { name, arms, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "test");
        assert_eq!(arms.len(), 2);
        assert_eq!(arms[0].pattern.len(), 1);
        assert_eq!(arms[1].pattern.len(), 3);
        if let MacroPatternFragment::Lit(ref t) = arms[1].pattern[1] {
            assert_eq!(t.value, ",");
        } else {
            panic!("Expected comma literal in pattern");
        }
    } else {
        panic!("Expected MacroDecl");
    }
}

#[test]
fn test_parse_macro_invocation_paren() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("my_macro!(1 + 2)".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        if let Expression::MacroInvocation {
            name,
            delimiter,
            arguments,
            ..
        } = &*expression.borrow()
        {
            assert_eq!(name.value, "my_macro");
            assert_eq!(*delimiter, MacroDelimiter::Paren);
            assert!(!arguments.is_empty());
        } else {
            panic!("Expected MacroInvocation expression");
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_macro_invocation_brace() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("my_macro! { 1 + 2 }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        if let Expression::MacroInvocation {
            name, delimiter, ..
        } = &*expression.borrow()
        {
            assert_eq!(name.value, "my_macro");
            assert_eq!(*delimiter, MacroDelimiter::Brace);
        } else {
            panic!("Expected MacroInvocation expression");
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_macro_invocation_bracket() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("my_macro![1, 2, 3]".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        if let Expression::MacroInvocation {
            name, delimiter, ..
        } = &*expression.borrow()
        {
            assert_eq!(name.value, "my_macro");
            assert_eq!(*delimiter, MacroDelimiter::Bracket);
        } else {
            panic!("Expected MacroInvocation expression");
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_macro_decl_var_types() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "macro types { ($e:expr, $t:ty, $i:ident, $l:literal) => { $e } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::MacroDecl { arms, .. } = &*program.statements[0].borrow() {
        assert_eq!(arms[0].pattern.len(), 7);
        assert_eq!(
            arms[0].pattern[0],
            MacroPatternFragment::MetaVar {
                name: "e".to_string(),
                var_type: MacroMetaVarType::Expr,
            }
        );
        assert_eq!(
            arms[0].pattern[2],
            MacroPatternFragment::MetaVar {
                name: "t".to_string(),
                var_type: MacroMetaVarType::Ty,
            }
        );
        assert_eq!(
            arms[0].pattern[4],
            MacroPatternFragment::MetaVar {
                name: "i".to_string(),
                var_type: MacroMetaVarType::Ident,
            }
        );
        assert_eq!(
            arms[0].pattern[6],
            MacroPatternFragment::MetaVar {
                name: "l".to_string(),
                var_type: MacroMetaVarType::Literal,
            }
        );
    } else {
        panic!("Expected MacroDecl");
    }
}

#[test]
fn test_parse_macro_decl_and_invocation() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "macro id { ($x:expr) => { $x } }\nid!(42)".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(
        &*program.statements[0].borrow(),
        Statement::MacroDecl { .. }
    ));
    if let Statement::ExpressionStatement { expression } = &*program.statements[1].borrow() {
        assert!(matches!(
            &*expression.borrow(),
            Expression::MacroInvocation { .. }
        ));
    } else {
        panic!("Expected ExpressionStatement");
    }
}

fn collect_shorthand_args(expr: &Rc<RefCell<Expression>>, count: &mut u32) {
    match &*expr.borrow() {
        Expression::ShorthandArgument { .. } => *count += 1,
        Expression::Binary { left, right, .. } => {
            collect_shorthand_args(left, count);
            collect_shorthand_args(right, count);
        }
        Expression::Unary { expression, .. } => {
            collect_shorthand_args(expression, count);
        }
        _ => {}
    }
}

#[test]
fn test_parse_operator_function_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func + (left: Int32, right: Int32) -> Int32 { left + right }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::FunctionDecl {
        name,
        operator_fixity,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "+");
        assert_eq!(*operator_fixity, None);
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_prefix_operator_function_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "prefix func - (value: Int32) -> Int32 { -value }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::FunctionDecl {
        name,
        operator_fixity,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "-");
        assert_eq!(*operator_fixity, Some(OperatorFixity::Prefix));
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_operator_function_decl_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func + (a: Int32) {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let mut parser = Parser::new(lexer.get_file(), tokens, engine);
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::FunctionDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "+");
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_postfix_keyword_as_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "postfix func - (v: Int32) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let mut parser = Parser::new(lexer.get_file(), tokens, engine.clone());
    let program = parser.parse();
    if program.statements.len() == 0 || engine.borrow().get_errors().len() > 0 {
        if let Statement::FunctionDecl {
            name,
            operator_fixity,
            ..
        } = &*program.statements[0].borrow()
        {
            assert_eq!(name.value, "-");
            assert_eq!(*operator_fixity, Some(OperatorFixity::Postfix));
        } else {
            panic!(
                "Expected FunctionDecl, got {:?}",
                *program.statements[0].borrow()
            );
        }
    } else {
        if let Statement::FunctionDecl {
            name,
            operator_fixity,
            ..
        } = &*program.statements[0].borrow()
        {
            assert_eq!(name.value, "-");
            assert_eq!(*operator_fixity, Some(OperatorFixity::Postfix));
        } else {
            panic!("Expected FunctionDecl");
        }
    }
}

#[test]
fn test_parse_prefix_keyword_as_modifier() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "prefix func - (v: Int32) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let mut parser = Parser::new(lexer.get_file(), tokens, engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        name,
        operator_fixity,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "-");
        assert_eq!(*operator_fixity, Some(OperatorFixity::Prefix));
    } else {
        panic!(
            "Expected FunctionDecl, got {:?}",
            *program.statements[0].borrow()
        );
    }
}

#[test]
fn test_parse_postfix_operator_function_decl() {
    let engine = create_engine();
    let code = "postfix func ++ (value: Int32) -> Int32 { value }";
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let mut parser = Parser::new(lexer.get_file(), tokens, engine.clone());
    let program = parser.parse();
    assert_eq!(
        program.statements.len(),
        1,
        "expected 1 stmt, got {} (errors: {})",
        program.statements.len(),
        engine.borrow().get_errors().len()
    );
    if let Statement::FunctionDecl {
        name,
        operator_fixity,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "++");
        assert_eq!(*operator_fixity, Some(OperatorFixity::Postfix));
    } else {
        panic!(
            "Expected FunctionDecl, got {:?}",
            *program.statements[0].borrow()
        );
    }
}

#[test]
fn test_parse_compound_assignment_operator_function_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func += (left: Int32, right: Int32) { left = left + right }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(
        program.statements.len(),
        1,
        "expected 1 statement, got {} (errors: {})",
        program.statements.len(),
        engine.borrow().get_errors().len()
    );
    if let Statement::FunctionDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "+=");
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_static_operator_function_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "static func + (left: Int32, right: Int32) -> Int32 { left + right }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::FunctionDecl {
        name,
        static_method,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "+");
        assert!(*static_method);
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_static_prefix_operator_function_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "static prefix func - (value: Int32) -> Int32 { -value }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::FunctionDecl {
        name,
        static_method,
        operator_fixity,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "-");
        assert!(*static_method);
        assert_eq!(*operator_fixity, Some(OperatorFixity::Prefix));
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_operator_function_inside_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyInt { var value: Int32; static func + (left: MyInt, right: MyInt) -> MyInt { left } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        let func_decl = body.iter().find_map(|s| {
            if let Statement::FunctionDecl {
                name,
                static_method,
                operator_fixity,
                ..
            } = &*s.borrow()
            {
                if name.value == "+" {
                    Some((*static_method, *operator_fixity))
                } else {
                    None
                }
            } else {
                None
            }
        });
        assert!(
            func_decl.is_some(),
            "Expected '+' operator function in struct body"
        );
        let (static_method, operator_fixity) = func_decl.unwrap();
        assert!(static_method);
        assert_eq!(operator_fixity, None);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_member_operator_function_inside_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyInt { var value: Int32; func + (other: MyInt) -> MyInt { self } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        let func_decl = body.iter().find_map(|s| {
            if let Statement::FunctionDecl {
                name,
                static_method,
                ..
            } = &*s.borrow()
            {
                if name.value == "+" {
                    Some(*static_method)
                } else {
                    None
                }
            } else {
                None
            }
        });
        assert!(
            func_decl.is_some(),
            "Expected '+' operator function in struct body"
        );
        assert!(!func_decl.unwrap(), "Member operator should not be static");
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_conditional_block_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if DEBUG\nfunc foo() {}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses.len(), 1);
        assert!(clauses[0].condition.is_some());
        assert_eq!(clauses[0].body.len(), 1);
    } else {
        panic!("Expected ConditionalBlock");
    }
}

#[test]
fn test_parse_conditional_block_if_else() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if DEBUG\nfunc foo() {}\n#else\nfunc bar() {}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses.len(), 2);
        assert!(clauses[0].condition.is_some());
        assert_eq!(clauses[0].body.len(), 1);
        assert!(clauses[1].condition.is_none());
        assert_eq!(clauses[1].body.len(), 1);
    } else {
        panic!("Expected ConditionalBlock");
    }
}

#[test]
fn test_parse_conditional_block_if_elseif_else() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if DEBUG\nfunc a() {}\n#elseif RELEASE\nfunc b() {}\n#else\nfunc c() {}\n#endif"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses.len(), 3);
        assert!(clauses[0].condition.is_some());
        assert!(clauses[1].condition.is_some());
        assert!(clauses[2].condition.is_none());
    } else {
        panic!("Expected ConditionalBlock");
    }
}

#[test]
fn test_parse_conditional_block_condition_types() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if true\n#endif\n#if false\n#endif\n#if A && B\n#endif\n#if A || B\n#endif\n#if !A\n#endif\n#if (A)\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(program.statements.len() >= 6);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses[0].condition, Some(Condition::Bool(true)));
    }
    if let Statement::ConditionalBlock { clauses } = &*program.statements[1].borrow() {
        assert_eq!(clauses[0].condition, Some(Condition::Bool(false)));
    }
}

#[test]
fn test_parse_ifdef_and_ifndef() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#ifdef FOO\n#endif\n#ifndef BAR\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[1].borrow() {
        assert!(matches!(&clauses[0].condition, Some(Condition::Not(..))));
    }
}

#[test]
fn test_parse_defined_in_condition() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if defined(DEBUG)\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_nested_conditional_blocks() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if A\n#if B\nfunc inner() {}\n#endif\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses.len(), 1);
        assert_eq!(clauses[0].body.len(), 1);
        assert!(matches!(
            &*clauses[0].body[0].borrow(),
            Statement::ConditionalBlock { .. }
        ));
    }
}

#[test]
fn test_parse_pragma_error_and_warning() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#error \"Something is wrong\"\n#warning \"This is a warning\"".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 2);
    assert!(matches!(
        &*program.statements[0].borrow(),
        Statement::PragmaError { .. }
    ));
    assert!(matches!(
        &*program.statements[1].borrow(),
        Statement::PragmaWarning { .. }
    ));
}

#[test]
fn test_parse_empty_conditional_block() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("#if FOO\n#endif".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_missing_endif_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("#if A\nfunc foo() {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        engine
            .borrow()
            .format_all_plain("")
            .contains("Expected #endif")
    );
    assert_eq!(program.statements.len(), 0);
}

#[test]
fn test_parse_else_without_if_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("#else\nfunc foo() {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine
            .borrow()
            .format_all_plain("")
            .contains("without matching")
    );
}

#[test]
fn test_parse_endif_without_if_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("#endif".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine
            .borrow()
            .format_all_plain("")
            .contains("without matching")
    );
}

#[test]
fn test_parse_multi_else_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if A\nfunc a() {}\n#else\nfunc b() {}\n#else\nfunc c() {}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine
            .borrow()
            .format_all_plain("")
            .contains("Multiple #else")
    );
}

#[test]
fn test_parse_elseif_after_else_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if A\nfunc a() {}\n#else\nfunc b() {}\n#elseif C\nfunc c() {}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(
        engine
            .borrow()
            .format_all_plain("")
            .contains("#elseif after #else")
    );
}

#[test]
fn test_parse_conditional_block_with_function_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if DEBUG\nfunc foo() {\nlet x = 1\nreturn x\n}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
}

#[test]
fn test_parse_os_condition() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if os(macOS)\nfunc foo() {}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses.len(), 1);
        assert!(matches!(
            &clauses[0].condition,
            Some(Condition::Os(token)) if token.value == "macOS"
        ));
    } else {
        panic!("Expected ConditionalBlock");
    }
}

#[test]
fn test_parse_arch_condition() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if arch(x86_64)\nfunc foo() {}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses.len(), 1);
        assert!(matches!(
            &clauses[0].condition,
            Some(Condition::Arch(token)) if token.value == "x86_64"
        ));
    } else {
        panic!("Expected ConditionalBlock");
    }
}

#[test]
fn test_parse_os_arch_combined_condition() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#if os(macOS) && arch(x86_64)\nfunc foo() {}\n#endif".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ConditionalBlock { clauses } = &*program.statements[0].borrow() {
        assert_eq!(clauses.len(), 1);
        if let Some(Condition::And(..)) = clauses[0].condition {
        } else {
            panic!("Expected And condition");
        }
    } else {
        panic!("Expected ConditionalBlock");
    }
}

#[test]
fn test_parse_sizeof_int32() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { sizeof(Int32) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        let Statement::ExpressionStatement { expression } = &*statements[0].borrow() else {
            panic!();
        };
        assert!(matches!(*expression.borrow(), Expression::SizeOf { .. }));
        if let Expression::SizeOf { argument, .. } = &*expression.borrow() {
            if let Expression::Type { name, .. } = &*argument.borrow() {
                assert_eq!(name.value, "Int32");
            } else {
                panic!("Expected Type expression");
            }
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_sizeof_struct_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { let x: Int32 } func test() { sizeof(Foo) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    assert_eq!(program.statements.len(), 2);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        let Statement::ExpressionStatement { expression } = &*statements[0].borrow() else {
            panic!();
        };
        assert!(matches!(*expression.borrow(), Expression::SizeOf { .. }));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_sizeof_missing_parens() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { sizeof Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine.borrow().has_errors());
}

#[test]
fn test_parse_asm_block_empty() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("asm {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        clobbers,
        ..
    } = &*program.statements[0].borrow()
    {
        assert!(instructions.is_empty());
        assert!(outputs.is_empty());
        assert!(inputs.is_empty());
        assert!(clobbers.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_instructions_only() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "nop" "mov rax, 42" }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        clobbers,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(instructions.len(), 2);
        assert_eq!(instructions[0].value, "\"nop\"");
        assert_eq!(instructions[1].value, "\"mov rax, 42\"");
        assert!(outputs.is_empty());
        assert!(inputs.is_empty());
        assert!(clobbers.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_with_outputs() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "mov {dst}, 42" : dst = out(reg) result }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        clobbers,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(instructions.len(), 1);
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].label.value, "dst");
        assert_eq!(outputs[0].direction, AsmDirection::Out);
        assert_eq!(outputs[0].constraint.value, "reg");
        assert!(inputs.is_empty());
        assert!(clobbers.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_with_inputs() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "mov rax, {src}" : : src = in(reg) 42 }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        clobbers,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(instructions.len(), 1);
        assert!(outputs.is_empty());
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].label.value, "src");
        assert_eq!(inputs[0].direction, AsmDirection::In);
        assert_eq!(inputs[0].constraint.value, "reg");
        assert!(clobbers.is_empty());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_full() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "add {dst}, {src}" : dst = out(reg) result : src = in(reg) 42 : "rax", "rbx" }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        clobbers,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(instructions.len(), 1);
        assert_eq!(instructions[0].value, "\"add {dst}, {src}\"");
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].label.value, "dst");
        assert_eq!(outputs[0].direction, AsmDirection::Out);
        assert_eq!(outputs[0].constraint.value, "reg");
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].label.value, "src");
        assert_eq!(inputs[0].direction, AsmDirection::In);
        assert_eq!(inputs[0].constraint.value, "reg");
        assert_eq!(clobbers.len(), 2);
        assert_eq!(clobbers[0].value, "\"rax\"");
        assert_eq!(clobbers[1].value, "\"rbx\"");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_multiple_instructions() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm {
                "mov rax, {src}"
                "add rax, 1"
                "mov {dst}, rax"
                : dst = out(reg) result
                : src = in(reg) 10
            }"#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(instructions.len(), 3);
        assert_eq!(outputs.len(), 1);
        assert_eq!(inputs.len(), 1);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_clobbers_only() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "nop" : : : "cc", "memory" }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        clobbers,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(instructions.len(), 1);
        assert!(outputs.is_empty());
        assert!(inputs.is_empty());
        assert_eq!(clobbers.len(), 2);
        assert_eq!(clobbers[0].value, "\"cc\"");
        assert_eq!(clobbers[1].value, "\"memory\"");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_missing_brace() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("asm".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    parser.parse();
    assert!(engine.borrow().has_errors());
}

#[test]
fn test_parse_asm_block_multiple_outputs() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "inst" : a = out(reg) x, b = out(reg) y }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock { outputs, .. } = &*program.statements[0].borrow() {
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].label.value, "a");
        assert_eq!(outputs[1].label.value, "b");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_in_expression_statement_fallback() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(r#"asm { "nop" }"#.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    assert!(matches!(
        &*program.statements[0].borrow(),
        Statement::AsmBlock { .. }
    ));
}

#[test]
fn test_parse_asm_block_comma_separated_instructions() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "mov rax, {src}", "add rax, 1", "mov {dst}, rax" : dst = out(reg) result : src = in(reg) 10 }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock {
        instructions,
        outputs,
        inputs,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(instructions.len(), 3);
        assert_eq!(instructions[0].value, "\"mov rax, {src}\"");
        assert_eq!(instructions[1].value, "\"add rax, 1\"");
        assert_eq!(instructions[2].value, "\"mov {dst}, rax\"");
        assert_eq!(outputs.len(), 1);
        assert_eq!(inputs.len(), 1);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_comma_separated_no_operands() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm { "nop", "bar", "baz" }"#.to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock { instructions, .. } = &*program.statements[0].borrow() {
        assert_eq!(instructions.len(), 3);
        assert_eq!(instructions[0].value, "\"nop\"");
        assert_eq!(instructions[1].value, "\"bar\"");
        assert_eq!(instructions[2].value, "\"baz\"");
    } else {
        panic!();
    }
}

#[test]
fn test_parse_asm_block_mixed_separators() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"asm {
                "inst1", "inst2"
                "inst3"
            }"#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::AsmBlock { instructions, .. } = &*program.statements[0].borrow() {
        assert_eq!(instructions.len(), 3);
    } else {
        panic!();
    }
}

#[test]
fn test_parse_do_expression() {
    let code = "func test() { do { let x = 1 x } }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Do { body, .. } = &*expression.borrow()
    {
        assert_eq!(body.len(), 2);
    } else {
        panic!("Expected do expression");
    }
}

#[test]
fn test_parse_do_empty_body() {
    let code = "func test() { let _ = do {} }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*stmts[0].borrow()
        && let Some(init) = initializer
        && let Expression::Do { body, .. } = &*init.borrow()
    {
        assert_eq!(body.len(), 0);
    } else {
        panic!("Expected do expression as initializer");
    }
}

#[test]
fn test_parse_do_missing_brace() {
    let code = "func test() { do }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty(), "Expected error for 'do' without brace");
}

#[test]
fn test_parse_do_nested() {
    let code = "func test() { do { do { 1 } } }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Do { body, .. } = &*expression.borrow()
    {
        assert_eq!(body.len(), 1);
        if let Statement::ExpressionStatement { expression } = &*body[0].borrow()
            && let Expression::Do { .. } = &*expression.borrow()
        {
        } else {
            panic!("Expected nested do");
        }
    } else {
        panic!("Expected outer do expression");
    }
}

#[test]
fn test_parse_yield_with_value() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { yield 42 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        assert!(matches!(
            &*statements[0].borrow(),
            Statement::Yield { value, .. } if value.is_some()
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_yield_without_value() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { yield }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        assert!(matches!(
            &*statements[0].borrow(),
            Statement::Yield { value: None, .. }
        ));
    } else {
        panic!();
    }
}

#[test]
fn test_parse_yield_in_do_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = do { yield 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        assert!(matches!(
            &*statements[0].borrow(),
            Statement::VariableDecl { .. }
        ));
        if let Statement::VariableDecl {
            initializer: Some(init),
            ..
        } = &*statements[0].borrow()
            && let Expression::Do { body, .. } = &*init.borrow()
        {
            assert!(matches!(
                &*body[0].borrow(),
                Statement::Yield { value, .. } if value.is_some()
            ));
        } else {
            panic!("Expected VariableDecl with do expression initializer");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_parse_yield_in_defer_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { defer { yield 42 } }".to_string(),
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
fn test_parse_function_decl_with_default_value() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(a: Int32 = 5) -> Int32 { a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters.len(), 1);
        let param = parameters[0].borrow();
        assert_eq!(param.name.value, "a");
        assert!(param.default_value.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_decl_with_label_and_default_value() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(from a: Int32 = 0, to b: Int32) { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0].borrow().label.as_ref().unwrap().value, "from");
        assert_eq!(parameters[0].borrow().name.value, "a");
        assert!(parameters[0].borrow().default_value.is_some());
        assert_eq!(parameters[1].borrow().label.as_ref().unwrap().value, "to");
        assert_eq!(parameters[1].borrow().name.value, "b");
        assert!(parameters[1].borrow().default_value.is_none());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_decl_with_expression_default_value() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(a: Int32 = 5 + 3) -> Int32 { a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters.len(), 1);
        let param = parameters[0].borrow();
        assert_eq!(param.name.value, "a");
        assert!(param.default_value.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_function_decl_no_label_with_default_value() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(a: Int32 = 10) -> Int32 { a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        assert_eq!(parameters.len(), 1);
        assert!(parameters[0].borrow().label.is_none());
        assert!(parameters[0].borrow().default_value.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function_with_default_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func identity<T = Int32>(x: T) -> T { x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Type { .. }
        ));
        assert!(generic_parameters[0].default_value.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function_with_constraint_and_default_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func compare<T: Equatable = String>(a: T, b: T) -> Bool { a == b }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        match &generic_parameters[0].kind {
            GenericParameterKind::Type { constraints } => {
                assert_eq!(constraints.len(), 1);
            }
            _ => panic!("expected Type"),
        }
        assert!(generic_parameters[0].default_value.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_struct_with_default_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Container<T = Int32> { var items: T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::StructDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(generic_parameters[0].default_value.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_const_generic_with_default_value() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<let N: Int32 = 42>() -> Int32 { N }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "N");
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Const { .. }
        ));
        assert!(generic_parameters[0].default_value.is_some());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_generic_function_no_default_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func identity<T>(x: T) -> T { x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        generic_parameters, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(generic_parameters.len(), 1);
        assert!(generic_parameters[0].default_value.is_none());
    } else {
        panic!();
    }
}

#[test]
fn test_parse_import_selective() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import Foo.Bar.{baz, qux}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].name, "baz");
        assert_eq!(members[0].alias, SelectiveAlias::Direct);
        assert_eq!(members[1].name, "qux");
        assert_eq!(members[1].alias, SelectiveAlias::Direct);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_selective_with_alias() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import Foo.Bar.{baz as myBaz, qux}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].name, "baz");
        assert_eq!(members[0].alias, SelectiveAlias::Named("myBaz".to_string()));
        assert_eq!(members[1].name, "qux");
        assert_eq!(members[1].alias, SelectiveAlias::Direct);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_selective_with_skip() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import Foo.Bar.{baz as _, qux}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].name, "baz");
        assert_eq!(members[0].alias, SelectiveAlias::Skip);
        assert_eq!(members[1].name, "qux");
        assert_eq!(members[1].alias, SelectiveAlias::Direct);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_single_alias() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import Foo.Bar.baz as myBaz".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "baz");
        assert_eq!(members[0].alias, SelectiveAlias::Named("myBaz".to_string()));
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_single_skip() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import Foo.Bar.baz as _".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string(), "Bar".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "baz");
        assert_eq!(members[0].alias, SelectiveAlias::Skip);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_selective_package() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import package.Foo.{bar, baz}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].name, "bar");
        assert_eq!(members[0].alias, SelectiveAlias::Direct);
        assert_eq!(members[1].name, "baz");
        assert_eq!(members[1].alias, SelectiveAlias::Direct);
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_selective_package_with_alias() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import package.Foo.{bar as myBar}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "bar");
        assert_eq!(members[0].alias, SelectiveAlias::Named("myBar".to_string()));
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_import_package_single_alias() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "import package.Foo.bar as myBar".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::ImportDecl {
        path,
        kind,
        selective_members,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(path, &vec!["Foo".to_string()]);
        assert_eq!(*kind, ImportKind::Module);
        let members = selective_members.as_ref().unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].name, "bar");
        assert_eq!(members[0].alias, SelectiveAlias::Named("myBar".to_string()));
    } else {
        panic!("Expected ImportDecl");
    }
}

#[test]
fn test_parse_optional_type_sugar() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: Int32? = 10".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        type_expression: Some(ty),
        ..
    } = &*program.statements[0].borrow()
    {
        let ty = ty.borrow();
        if let Expression::OptionalType { inner, .. } = &*ty {
            if let Expression::Type { name, .. } = &*inner.borrow() {
                assert_eq!(name.value, "Int32");
            } else {
                panic!("Expected Type for Optional inner");
            }
        } else {
            panic!("Expected OptionalType expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_optional_type_on_tuple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let x: (Int32, Bool)? = nil".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        type_expression: Some(ty),
        ..
    } = &*program.statements[0].borrow()
    {
        let ty = ty.borrow();
        if let Expression::OptionalType { .. } = &*ty {
        } else {
            panic!("Expected OptionalType expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_optional_with_pointer() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: Int32*? = nil".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        type_expression: Some(ty),
        ..
    } = &*program.statements[0].borrow()
    {
        let ty = ty.borrow();
        if let Expression::OptionalType { inner, .. } = &*ty {
            if let Expression::PointerType { non_null, .. } = &*inner.borrow() {
                assert!(!non_null);
            } else {
                panic!("Expected PointerType inside Optional");
            }
        } else {
            panic!("Expected OptionalType expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_array_type_sugar() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: [Int32] = {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        type_expression: Some(ty),
        ..
    } = &*program.statements[0].borrow()
    {
        let ty = ty.borrow();
        if let Expression::ArrayType { inner, .. } = &*ty {
            if let Expression::Type { name, .. } = &*inner.borrow() {
                assert_eq!(name.value, "Int32");
            } else {
                panic!("Expected Type for Array inner");
            }
        } else {
            panic!("Expected ArrayType expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_array_type_nested() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: [[Int32]] = {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        type_expression: Some(ty),
        ..
    } = &*program.statements[0].borrow()
    {
        let ty = ty.borrow();
        if let Expression::ArrayType { inner, .. } = &*ty {
            if let Expression::ArrayType { inner: inner2, .. } = &*inner.borrow() {
                if let Expression::Type { name, .. } = &*inner2.borrow() {
                    assert_eq!(name.value, "Int32");
                } else {
                    panic!("Expected Type inside nested Array");
                }
            } else {
                panic!("Expected nested ArrayType");
            }
        } else {
            panic!("Expected ArrayType for outer Array");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_string_literal_expression() {
    let engine = create_engine();
    let src = "let s = \"hello\"";
    let mut lexer = Lexer::new(
        CharStream::new(src.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        if let Expression::StringLiteral { value, .. } = &*init.borrow() {
            assert_eq!(value, "hello");
        } else {
            panic!("Expected StringLiteral expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_string_literal_empty() {
    let engine = create_engine();
    let src = "let s = \"\"";
    let mut lexer = Lexer::new(
        CharStream::new(src.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        if let Expression::StringLiteral { value, .. } = &*init.borrow() {
            assert_eq!(value, "");
        } else {
            panic!("Expected StringLiteral expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_optional_type_in_func_return() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32? { return nil }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_parse_array_type_in_func_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(x: [Int32]) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_parse_array_literal_empty() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = []".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        let init = init.borrow();
        assert!(matches!(&*init, Expression::ArrayLiteral { elements, .. } if elements.is_empty()));
    } else {
        panic!("Expected VariableDecl with initializer");
    }
}

#[test]
fn test_parse_array_literal_single_element() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = [42]".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        let init = init.borrow();
        if let Expression::ArrayLiteral { elements, .. } = &*init {
            assert_eq!(elements.len(), 1);
            assert!(matches!(
                &*elements[0].borrow(),
                Expression::IntegerLiteral { value: 42, .. }
            ));
        } else {
            panic!("Expected ArrayLiteral");
        }
    } else {
        panic!("Expected VariableDecl with initializer");
    }
}

#[test]
fn test_parse_array_literal_multi_element() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = [1, 2, 3]".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        let init = init.borrow();
        if let Expression::ArrayLiteral { elements, .. } = &*init {
            assert_eq!(elements.len(), 3);
        } else {
            panic!("Expected ArrayLiteral");
        }
    } else {
        panic!("Expected VariableDecl with initializer");
    }
}

#[test]
fn test_parse_array_literal_member_access() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = [1, 2].count".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        let init = init.borrow();
        assert!(matches!(&*init, Expression::MemberAccess { .. }));
    } else {
        panic!("Expected VariableDecl with initializer");
    }
}

#[test]
fn test_parse_array_literal_subscript_not_confused() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = a[0]".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        let init = init.borrow();
        assert!(matches!(&*init, Expression::SubscriptAccess { .. }));
    } else {
        panic!("Expected SubscriptAccess");
    }
}

#[test]
fn test_parse_array_literal_missing_bracket() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = [1, 2".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(engine.borrow().has_errors());
}

#[test]
fn test_parse_array_literal_in_function_return() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { return [] }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_parse_array_literal_nested() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let x = [[1, 2], [3, 4]]".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_parse_array_literal_trailing_comma() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = [1, 2,]".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl {
        initializer: Some(init),
        ..
    } = &*program.statements[0].borrow()
    {
        let init = init.borrow();
        if let Expression::ArrayLiteral { elements, .. } = &*init {
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected ArrayLiteral");
        }
    } else {
        panic!("Expected VariableDecl with initializer");
    }
}

#[test]
fn test_parse_self_type_as_primary() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("Self".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        assert!(matches!(&*expr, Expression::SelfType { .. }));
    } else {
        panic!("Expected ExpressionStatement with SelfType");
    }
}

#[test]
fn test_parse_self_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "Self(count: self.count, capacity: self.capacity)".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        if let Expression::Call {
            callee, parameters, ..
        } = &*expr
        {
            assert!(matches!(&*callee.borrow(), Expression::SelfType { .. }));
            assert_eq!(parameters.len(), 2);
            assert_eq!(parameters[0].label.as_ref().unwrap().value, "count");
            assert_eq!(parameters[1].label.as_ref().unwrap().value, "capacity");
        } else {
            panic!("Expected Call expression");
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_if_let_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "if let x = optional { x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    assert_eq!(program.statements.len(), 1);
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        if let Expression::If { condition, .. } = &*expr {
            if let Expression::Case {
                case_name,
                bindings,
                ..
            } = &*condition.borrow()
            {
                assert_eq!(case_name.value, "Some");
                assert_eq!(bindings.len(), 1);
            } else {
                panic!("Expected Case expression in if-let condition");
            }
        } else {
            panic!("Expected If expression");
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_guard_let_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "guard let x = optional else { return }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_parse_while_let_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "while let x = iterator() { x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_parse_if_let_ignore() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "if let _ = optional { 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_parse_tuple_expression_statement() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { (1, 2) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_parse_range_operator() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("0..<count".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}
#[test]
fn test_parse_range_operator_to() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("0..count".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}
#[test]
fn test_parse_if_let_mixed_condition() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "if let x = opt && (a > 0) || (b < 0) { x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let _program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_parse_abstract_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "abstract class Shape { abstract func draw() -> Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ClassDecl {
        name,
        modifiers,
        body,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Shape");
        assert!(modifiers.iter().any(|m| m.ty == ModifierType::Abstract));
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl {
            name: fn_name,
            modifiers: fn_mods,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(fn_name.value, "draw");
            assert!(fn_mods.iter().any(|m| m.ty == ModifierType::Abstract));
        } else {
            panic!("Expected FunctionDecl in class body");
        }
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_parse_final_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "final class Dog { func bark() -> Int32 { return 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ClassDecl {
        name, modifiers, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Dog");
        assert!(modifiers.iter().any(|m| m.ty == ModifierType::Final));
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_parse_final_method_in_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Animal { final func run() -> Int32 { return 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Animal");
        if let Statement::FunctionDecl {
            name: fn_name,
            modifiers: fn_mods,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(fn_name.value, "run");
            assert!(fn_mods.iter().any(|m| m.ty == ModifierType::Final));
        } else {
            panic!("Expected FunctionDecl");
        }
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_parse_override_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Animal { func speak() -> Int32 { return 1 } }
             class Dog: Animal { override func speak() -> Int32 { return 2 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[1].borrow() {
        assert_eq!(name.value, "Dog");
        if let Statement::FunctionDecl {
            name: fn_name,
            modifiers: fn_mods,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(fn_name.value, "speak");
            assert!(fn_mods.iter().any(|m| m.ty == ModifierType::Override));
        } else {
            panic!("Expected FunctionDecl");
        }
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_parse_override_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Base { var name: Int32 { get { return 1 } set { } } }
             class Derived: Base { override var name: Int32 { get { return 2 } set { } } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[1].borrow() {
        assert_eq!(name.value, "Derived");
        if let Statement::VariableDecl {
            name: var_name,
            modifiers: var_mods,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(var_name.value, "name");
            assert!(var_mods.iter().any(|m| m.ty == ModifierType::Override));
        } else {
            panic!("Expected VariableDecl");
        }
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_parse_final_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "final class Constants { final let maxSize: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Constants");
        if let Statement::VariableDecl {
            name: var_name,
            modifiers: var_mods,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(var_name.value, "maxSize");
            assert!(var_mods.iter().any(|m| m.ty == ModifierType::Final));
        } else {
            panic!("Expected VariableDecl");
        }
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_parse_modifier_abstract_on_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { return 1 }
             abstract func absTest() -> Int32"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    if let Statement::FunctionDecl {
        name, modifiers, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(name.value, "absTest");
        assert!(modifiers.iter().any(|m| m.ty == ModifierType::Abstract));
    } else {
        panic!("Expected FunctionDecl");
    }
}

// --- Exception handling tests ---

#[test]
fn test_parse_throws_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo() throws -> Int32 { return 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::FunctionDecl {
        name, throws_types, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "foo");
        assert!(throws_types.is_some());
        assert!(throws_types.as_ref().unwrap().is_empty());
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_throws_function_with_types() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func bar() throws(MyError, IOError) -> String { return \"\" }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::FunctionDecl {
        name, throws_types, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "bar");
        let types = throws_types.as_ref().unwrap();
        assert_eq!(types.len(), 2);
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_throws_function_no_return() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func baz() throws { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::FunctionDecl {
        name, throws_types, ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "baz");
        assert!(throws_types.is_some());
        assert!(throws_types.as_ref().unwrap().is_empty());
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_parse_throw_statement() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "throw MyError.someCase".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    assert!(matches!(
        &*program.statements[0].borrow(),
        Statement::Throw { .. }
    ));
}

#[test]
fn test_parse_try_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("try foo()".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        assert!(matches!(
            &*expr,
            Expression::Try {
                kind: TryKind::Plain,
                ..
            }
        ));
    } else {
        panic!("Expected ExpressionStatement with Try");
    }
}

#[test]
fn test_parse_try_force_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("try! foo()".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        assert!(matches!(
            &*expr,
            Expression::Try {
                kind: TryKind::Force,
                ..
            }
        ));
    } else {
        panic!("Expected ExpressionStatement with Try!");
    }
}

#[test]
fn test_parse_try_optional_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("try? foo()".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        assert!(matches!(
            &*expr,
            Expression::Try {
                kind: TryKind::Optional,
                ..
            }
        ));
    } else {
        panic!("Expected ExpressionStatement with Try?");
    }
}

#[test]
fn test_parse_do_catch() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "do { try foo() } catch { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        match &*expr {
            Expression::Do {
                catch_clauses,
                finally_body,
                ..
            } => {
                assert_eq!(catch_clauses.len(), 1);
                assert!(catch_clauses[0].pattern.is_none());
                assert!(finally_body.is_empty());
            }
            _ => panic!("Expected Do expression"),
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_do_catch_finally() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "do { try foo() } catch MyError { } finally { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        match &*expr {
            Expression::Do {
                catch_clauses,
                finally_body,
                ..
            } => {
                assert_eq!(catch_clauses.len(), 1);
                assert!(catch_clauses[0].pattern.is_some());
                assert_eq!(finally_body.len(), 0);
            }
            _ => panic!("Expected Do expression"),
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_do_catch_finally_with_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "do { try foo() } catch { } finally { cleanup() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        match &*expr {
            Expression::Do {
                catch_clauses,
                finally_body,
                ..
            } => {
                assert_eq!(catch_clauses.len(), 1);
                assert!(!finally_body.is_empty());
            }
            _ => panic!("Expected Do expression"),
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_do_catch_multi_catch() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "do { try foo() } catch IOError { } catch MyError { } catch { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        match &*expr {
            Expression::Do { catch_clauses, .. } => {
                assert_eq!(catch_clauses.len(), 3);
                assert!(catch_clauses[0].pattern.is_some());
                assert!(catch_clauses[1].pattern.is_some());
                assert!(catch_clauses[2].pattern.is_none());
            }
            _ => panic!("Expected Do expression"),
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_do_catch_with_guard() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "do { try foo() } catch MyError where condition { } catch { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        match &*expr {
            Expression::Do { catch_clauses, .. } => {
                assert_eq!(catch_clauses.len(), 2);
                assert!(catch_clauses[0].guard.is_some());
                assert!(catch_clauses[1].pattern.is_none());
            }
            _ => panic!("Expected Do expression"),
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_do_without_catch() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("do { let x = 1 }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ExpressionStatement { expression } = &*program.statements[0].borrow() {
        let expr = expression.borrow();
        match &*expr {
            Expression::Do {
                catch_clauses,
                finally_body,
                ..
            } => {
                assert!(catch_clauses.is_empty());
                assert!(finally_body.is_empty());
            }
            _ => panic!("Expected Do expression"),
        }
    } else {
        panic!("Expected ExpressionStatement");
    }
}

#[test]
fn test_parse_throws_method_in_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { func bar() throws -> Int32 { return 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Foo");
        if let Statement::FunctionDecl {
            name: fn_name,
            throws_types,
            ..
        } = &*body[0].borrow()
        {
            assert_eq!(fn_name.value, "bar");
            assert!(throws_types.is_some());
        } else {
            panic!("Expected FunctionDecl in struct");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_protocol_throws_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { func foo() throws -> Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    if let Statement::ProtocolDecl { name, members, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "P");
        if let ProtocolMember::Method { decl, .. } = &members[0] {
            if let Statement::FunctionDecl {
                name: fn_name,
                throws_types,
                ..
            } = &*decl.borrow()
            {
                assert_eq!(fn_name.value, "foo");
                assert!(throws_types.is_some());
            } else {
                panic!("Expected FunctionDecl in protocol method");
            }
        } else {
            panic!("Expected Method protocol member");
        }
    } else {
        panic!("Expected ProtocolDecl");
    }
}

#[test]
fn test_implicit_member_access() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { .Executable }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let tokens = lexer.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Lexer should not have errors for .Executable"
    );
    let mut parser = Parser::new(Rc::new("test.truss".to_string()), tokens, engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Parser should succeed");
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
    {
        let expr = expression.borrow();
        match &*expr {
            Expression::ImplicitMemberAccess { member, .. } => {
                assert_eq!(member.value, "Executable");
            }
            other => panic!("Expected ImplicitMemberAccess, got {:?}", other),
        }
    } else {
        panic!("Expected FunctionDecl with ExpressionStatement");
    }
}

#[test]
fn test_implicit_member_access_in_call() {
    let engine = create_engine();
    let code = r#"func test() { Target(name: "my-app", kind: .Executable) }"#;
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let tokens = lexer.parse();
    assert!(!engine.borrow().has_errors());
    let mut parser = Parser::new(Rc::new("test.truss".to_string()), tokens, engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
    {
        let expr = expression.borrow();
        if let Expression::Call { parameters, .. } = &*expr {
            let kind_param = parameters
                .iter()
                .find(|p| p.label.as_ref().map_or(false, |l| l.value == "kind"));
            assert!(kind_param.is_some(), "Should have 'kind' parameter");
            if let Some(param) = kind_param {
                let param_expr = param.expression.borrow();
                match &*param_expr {
                    Expression::ImplicitMemberAccess { member, .. } => {
                        assert_eq!(member.value, "Executable");
                    }
                    other => panic!("Expected ImplicitMemberAccess in kind, got {:?}", other),
                }
            }
        } else {
            panic!("Expected Call expression, got {:?}", expr);
        }
    } else {
        panic!("Expected FunctionDecl with ExpressionStatement");
    }
}

#[test]
fn test_parse_mutating_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { mutating func foo() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Expected no errors");
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        let mutating_func = body.iter().find(|s| {
            if let Statement::FunctionDecl { name, .. } = &*s.borrow() {
                name.value == "foo"
            } else {
                false
            }
        });
        assert!(mutating_func.is_some(), "Should find func foo");
        if let Statement::FunctionDecl { mutating, .. } = &*mutating_func.unwrap().borrow() {
            assert!(*mutating, "Expected mutating to be true");
        } else {
            panic!("Expected FunctionDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_mutating_function_with_modifiers() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { public mutating func foo() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Expected no errors");
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        let mutating_func = body.iter().find(|s| {
            if let Statement::FunctionDecl { name, .. } = &*s.borrow() {
                name.value == "foo"
            } else {
                false
            }
        });
        assert!(mutating_func.is_some(), "Should find func foo");
        if let Statement::FunctionDecl {
            modifiers, mutating, ..
        } = &*mutating_func.unwrap().borrow()
        {
            assert!(*mutating, "Expected mutating to be true");
            assert!(
                modifiers.iter().any(|m| matches!(m.ty, ModifierType::Access(_))),
                "Expected access modifier"
            );
        } else {
            panic!("Expected FunctionDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_non_mutating_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { func foo() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Expected no errors");
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        let non_mutating_func = body.iter().find(|s| {
            if let Statement::FunctionDecl { name, .. } = &*s.borrow() {
                name.value == "foo"
            } else {
                false
            }
        });
        assert!(non_mutating_func.is_some(), "Should find func foo");
        if let Statement::FunctionDecl { mutating, .. } = &*non_mutating_func.unwrap().borrow() {
            assert!(!*mutating, "Expected mutating to be false");
        } else {
            panic!("Expected FunctionDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_self_init_call_in_struct_init() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { let x: Int32; init(x: Int32) { self.x = x } init() { self.init(x: 0) } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Expected no errors, got: {:?}",
        engine.borrow().get_errors()
    );
    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        let init_stmt = body.iter().find(|s| matches!(&*s.borrow(), Statement::InitDecl { .. }));
        assert!(init_stmt.is_some(), "Should find init");
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_parse_closure_capture_simple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [x] in x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure {
            captures,
            parameters,
            ..
        } = &*init
        {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].name.value, "x");
            assert!(captures[0].expression.is_none());
            assert!(!captures[0].is_var);
            assert_eq!(parameters.len(), 0);
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_capture_multiple() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [x, y] (a: Int32) -> Int32 in x + y + a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure {
            captures,
            parameters,
            ..
        } = &*init
        {
            assert_eq!(captures.len(), 2);
            assert_eq!(captures[0].name.value, "x");
            assert_eq!(captures[1].name.value, "y");
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].borrow().name.value, "a");
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_capture_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [var x] in x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure { captures, .. } = &*init {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].name.value, "x");
            assert!(captures[0].is_var);
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_capture_let() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [let x] in x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure { captures, .. } = &*init {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].name.value, "x");
            assert!(!captures[0].is_var);
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_capture_with_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [a = x + 1] in a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure { captures, .. } = &*init {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].name.value, "a");
            assert!(captures[0].expression.is_some());
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_capture_var_with_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [var a = x + 1] in a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure { captures, .. } = &*init {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].name.value, "a");
            assert!(captures[0].expression.is_some());
            assert!(captures[0].is_var);
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_closure_no_capture_unaffected() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { (x: Int32) -> Int32 in x + 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!program.statements.is_empty());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure {
            captures,
            parameters,
            ..
        } = &*init
        {
            assert_eq!(captures.len(), 0);
            assert_eq!(parameters.len(), 1);
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_weak_variable() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "weak var x: SomeClass?".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl { ownership, name, .. } = &*program.statements[0].borrow() {
        assert_eq!(*ownership, OwnershipModifier::Weak);
        assert_eq!(name.value, "x");
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_unowned_variable() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "unowned var x: SomeClass".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl { ownership, name, .. } = &*program.statements[0].borrow() {
        assert_eq!(*ownership, OwnershipModifier::Unowned);
        assert_eq!(name.value, "x");
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_weak_closure_capture() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [weak x] in x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure { captures, .. } = &*init {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].ownership, OwnershipModifier::Weak);
            assert_eq!(captures[0].name.value, "x");
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_unowned_closure_capture() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "let f = { [unowned x] in x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl { initializer, .. } = &*program.statements[0].borrow() {
        let init = initializer.as_ref().unwrap().borrow();
        if let Expression::Closure { captures, .. } = &*init {
            assert_eq!(captures.len(), 1);
            assert_eq!(captures[0].ownership, OwnershipModifier::Unowned);
            assert_eq!(captures[0].name.value, "x");
        } else {
            panic!("Expected Closure expression");
        }
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_strong_variable_default() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("var x: SomeClass".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors());
    if let Statement::VariableDecl { ownership, name, .. } = &*program.statements[0].borrow() {
        assert_eq!(*ownership, OwnershipModifier::Strong);
        assert_eq!(name.value, "x");
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_parse_ownership_modifier_default() {
    assert_eq!(OwnershipModifier::default(), OwnershipModifier::Strong);
}
