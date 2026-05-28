use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::Expression,
        statement::{FunctionBody, Statement},
    },
    diag::TrussDiagnosticEngine,
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
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a = 1 a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
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
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let _ = 1 let _ = 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine,
    );
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_underscore_parameter_no_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(_ _: Int32) { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine,
    );
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_variable_shadowing() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a = 1 let a = 2 a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine,
    );
    resolver.resolve(&program, "test".to_string());

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[2].borrow()
        && let Expression::Variable { symbol, .. } = &*expression.borrow()
    {
        assert_ne!(*symbol, None);
    } else {
        panic!();
    }
}

#[test]
fn test_struct_field_symbol() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 3);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_struct_method_symbol() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Math");
        assert_eq!(body.len(), 1);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_struct_init_deinit_symbol() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

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
fn test_if_case_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case none case some(Int32) }
                func test(x: Option) {
                    if case Option.some(val) = x {
                        let _ = val
                    }
                }
            "#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors, got: {:?}",
        errors
    );
}

#[test]
fn test_if_case_binding_available_in_then_block() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case none case some(Int32) }
                func test(x: Option) {
                    if case Option.some(val) = x {
                        val
                    }
                }
            "#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for binding variable in then block, got: {:?}",
        errors
    );
}

#[test]
fn test_if_case_binding_not_available_outside() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case none case some(Int32) }
                func test(x: Option) {
                    if case Option.some(val) = x {
                    }
                    val
                }
            "#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() > 0,
        "Should have errors for binding variable outside then block, got: {:?}",
        errors
    );
}

#[test]
fn test_if_case_no_bindings() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case none case some(Int32) }
                func test(x: Option) {
                    if case Option.none = x {
                    }
                }
            "#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for if case with no bindings, got: {:?}",
        errors
    );
}

#[test]
fn test_if_case_underscore_binding() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case none case some(Int32) }
                func test(x: Option) {
                    if case Option.some(_) = x {
                    }
                }
            "#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with underscore binding, got: {:?}",
        errors
    );
}

#[test]
fn test_type_instantiation_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { let x: Int32 init(x: Int32) {} }
             func test() { Point(x: 1) }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_class_field_symbol() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 2);
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_class_method_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Math { func square(x: Int32) -> Int32 { return x * x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Math");
        assert_eq!(body.len(), 1);
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_class_init_deinit_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Point { let x: Int32 init(x: Int32) { } deinit { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ClassDecl { name, body, .. } = &*program.statements[0].borrow() {
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
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_class_superclass_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Animal {} class Dog: Animal {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ClassDecl { name, superclass, .. } = &*program.statements[1].borrow() {
        assert_eq!(name.value, "Dog");
        assert!(superclass.is_some());
    } else {
        panic!("Expected ClassDecl for Dog");
    }
}

#[test]
fn test_class_undefined_superclass_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Dog: Animal {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty(), "Should have symbol error for undefined superclass");
}

#[test]
fn test_self_keyword_outside_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { self }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty(), "Should emit error for 'self' outside method");
}

#[test]
fn test_self_keyword_in_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { func method() { self } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_self_keyword_member_access() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { var x: Int32; func method() { self.x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_self_keyword_method_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { func method() { self.other() } func other() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());
}

// --- Protocol symbol resolver tests ---

#[test]
fn test_protocol_symbol_registered() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_with_method_symbol() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_with_property_symbol() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_with_default_impl_symbol() {
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
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_conformance_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Void }
             class Circle: Drawable {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_undefined_protocol_conformance_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Circle: UndefinedProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty(), "Should have symbol error for undefined protocol");
}

#[test]
fn test_protocol_refinement_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Base {}
             protocol Derived: Base {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_any_type_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol {} let x: any MyProtocol".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors with 'any' type, got: {:?}", errors);
}

#[test]
fn test_struct_protocol_conformance_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Void }
             struct Circle: Drawable {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_struct_undefined_protocol_conformance_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Circle: UndefinedProtocol {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty(), "Should have symbol error for undefined protocol");
}

#[test]
fn test_protocol_compound_type_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol A {} protocol B {} let x: A & B".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let mut resolver = SymbolResolver::new(
        Rc::new(RefCell::new(Crate::new("test".to_string()))),
        engine.clone(),
    );
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors with compound type, got: {:?}", errors);
}
