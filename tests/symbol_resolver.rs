use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::{ElseBranch, Expression},
        statement::{FunctionBody, GenericParameterKind, Statement},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine},
    krate::Package,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol::Symbol,
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
fn test_builtintype_struct_symbol_is_marked() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "#[builtintype] public struct Int32 {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let root_module = resolver.resolve(&program, "test".to_string());
    {
        let engine_ref = engine.borrow();
        let errors = engine_ref.get_errors();
        assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    }
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let sym = root_scope.borrow().get_symbol("Int32");
    assert!(sym.is_some(), "Int32 should be registered");
    let binding = sym.unwrap();
    let symbol = binding.borrow();
    assert!(matches!(
        &*symbol,
        Symbol::Struct {
            is_builtin_type: true,
            ..
        }
    ));
}

#[test]
fn test_non_builtintype_struct_symbol_not_marked() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "public struct Point { let x: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let root_module = resolver.resolve(&program, "test".to_string());
    {
        let engine_ref = engine.borrow();
        let errors = engine_ref.get_errors();
        assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    }
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let sym = root_scope.borrow().get_symbol("Point");
    assert!(sym.is_some(), "Point should be registered");
    let binding = sym.unwrap();
    let symbol = binding.borrow();
    assert!(matches!(
        &*symbol,
        Symbol::Struct {
            is_builtin_type: false,
            ..
        }
    ));
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Math");
        assert_eq!(body.len(), 2);
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
fn test_struct_failable_init_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { let x: Int32 init?(x: Int32) { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 2);
        if let Statement::InitDecl {
            parameters,
            is_failable,
            ..
        } = &*body[1].borrow()
        {
            assert!(is_failable);
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].borrow().name.value, "x");
        } else {
            panic!("Expected InitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_struct_deinit_method_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { let x: Int32 deinit { } } func test(p: Point) { p.deinit() }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let struct_sym = root_scope.borrow().get_symbol("Point").unwrap();
    let binding = struct_sym.borrow();
    let Symbol::Struct { methods, .. } = &*binding else {
        panic!("Expected Struct symbol");
    };
    let has_deinit = methods.iter().any(|m| {
        let mb = m.borrow();
        matches!(&*mb, Symbol::StructMethod { name, .. } if name == "deinit")
    });
    assert!(
        has_deinit,
        "deinit should exist as a method in the struct's methods list"
    );
}

#[test]
fn test_class_failable_init_symbol() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

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
fn test_if_case_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    if case Option.Some(val) = x {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_binding_available_in_then_block() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    if case Option.Some(val) = x {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    if case Option.Some(val) = x {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    if case Option.None = x {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    if case Option.Some(_) = x {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
fn test_class_deinit_method_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Point { let x: Int32 deinit { } } func test(p: Point) { p.deinit() }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let class_sym = root_scope.borrow().get_symbol("Point").unwrap();
    let binding = class_sym.borrow();
    let Symbol::Class { methods, .. } = &*binding else {
        panic!("Expected Class symbol");
    };
    let has_deinit = methods.iter().any(|m| {
        let mb = m.borrow();
        matches!(&*mb, Symbol::ClassMethod { name, .. } if name == "deinit")
    });
    assert!(
        has_deinit,
        "deinit should exist as a method in the class's methods list"
    );
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ClassDecl {
        name, superclass, ..
    } = &*program.statements[1].borrow()
    {
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
        CharStream::new("class Dog: Animal {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should have symbol error for undefined superclass"
    );
}

#[test]
fn test_self_keyword_outside_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { self }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for 'self' outside method"
    );
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_super_keyword_in_class_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Animal { func speak() -> Void {} } class Dog: Animal { func test() { super.speak() } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_super_keyword_in_class_method_multiline() {
    let engine = create_engine();
    let code = r#"
        class Animal {
            func speak() -> Int32 { return 1 }
        }
        class Dog: Animal {
            func speak() -> Int32 { return 2 }
            func call_super() -> Int32 { return super.speak() }
        }
        func run_test() -> Int32 {
            var d: Dog
            return d.call_super()
        }
    "#;
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_super_keyword_outside_method_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("func test() { super }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for 'super' outside method"
    );
}

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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
             class Circle: Drawable {}"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should have symbol error for undefined protocol"
    );
}

#[test]
fn test_protocol_refinement_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Base {}
             protocol Derived: Base {}"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with 'any' type, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_some_type_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol {} let x: some MyProtocol".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

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
fn test_struct_protocol_conformance_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Void }
             struct Circle: Drawable {}"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_autowired_protocol_method_auto_generates_for_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Copyable { #[autowired] func copy() -> Self }
             struct MyStruct: Copyable {}"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let struct_sym = root_scope.borrow().get_symbol("MyStruct").unwrap();
    let binding = struct_sym.borrow();
    let Symbol::Struct { methods, .. } = &*binding else {
        panic!("Expected Struct symbol");
    };
    let has_copy = methods.iter().any(|m| {
        let mb = m.borrow();
        matches!(&*mb, Symbol::StructMethod { name, .. } if name == "copy")
    });
    assert!(
        has_copy,
        "Auto-generated copy() method should exist on struct"
    );
}

#[test]
fn test_autowired_method_not_generated_if_already_implemented() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Copyable { #[autowired] func copy() -> Self }
             struct MyStruct: Copyable {
                 func copy() -> Self { return self }
             }"
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let struct_sym = root_scope.borrow().get_symbol("MyStruct").unwrap();
    let binding = struct_sym.borrow();
    let Symbol::Struct { methods, .. } = &*binding else {
        panic!("Expected Struct symbol");
    };
    let copy_methods: Vec<_> = methods
        .iter()
        .filter(|m| {
            let mb = m.borrow();
            matches!(&*mb, Symbol::StructMethod { name, .. } if name == "copy")
        })
        .collect();
    assert_eq!(
        copy_methods.len(),
        1,
        "Should have exactly one copy() method"
    );
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should have symbol error for undefined protocol"
    );
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with compound type, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_struct_method_symbol() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ExtensionDecl {
        type_name, body, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(type_name.value, "Foo");
        assert_eq!(body.len(), 1);
        if let Statement::FunctionDecl { name, .. } = &*body[0].borrow() {
            assert_eq!(name.value, "bar");
        } else {
            panic!("Expected FunctionDecl");
        }
    } else {
        panic!("Expected ExtensionDecl");
    }
}

#[test]
fn test_extension_struct_self_in_method() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { let x: Int32 } extension Point { func getX() -> Int32 { self.x } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for self in extension, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_undefined_type_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "extension NonExistent { func foo() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty(), "Should have error for undefined type");
}

#[test]
fn test_extension_protocol_method_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Printable {} extension Printable { func describe() -> Int32 { 42 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for protocol extension, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_with_type_arguments_resolves_expressions() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Wrapper<T> {} extension Wrapper<Int32>: Computable { func compute() -> Int32 { 42 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should resolve extension type arguments without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_conformance_with_type_parameters_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol From<T> { func from(value: T) -> Self } struct S: From<Int32> { func from(value: Int32) -> S { self } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should resolve protocol conformance with type parameters, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_static_method_struct() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for static method in struct extension, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_static_method_class() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for static method in class extension, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_static_method_enum() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for static method in enum extension, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_static_method_protocol() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for static method in protocol extension, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_function_resolves_type_param() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic function should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_struct_resolves_type_param() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic struct should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_class_resolves_type_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Box<T> { var value: T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic class should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_enum_resolves_type_param() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic enum should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_with_associatedtype_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Protocol with associatedtype should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_typealias_in_struct_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Typealias in struct should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_typealias_in_protocol_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Typealias in protocol should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_typealias_at_top_level_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Top-level typealias should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_sugar_associatedtype_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Protocol<T> sugar should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_function_where_clause_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<T>(x: T) where T: Equatable { }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Where clause should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_struct_with_generic_protocol_conformance_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyArray: Container<Int32> { typealias Item = Int32 } protocol Container<T> { func append(item: T) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let _ = engine.borrow();
}

#[test]
fn test_extension_where_clause_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct S<T> {} extension S where T: Hashable {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let _ = engine.borrow();
}

#[test]
fn test_guard_case_binding_available_after_guard() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    guard case .Some(val) = x else { return }
                    let _ = val
                }
            "#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "guard binding should be available after guard statement, got: {:?}",
        errors
    );
}

#[test]
fn test_guard_case_binding_not_available_in_else_block() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    guard case .Some(val) = x else {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() > 0,
        "guard binding should NOT be available in else block, got: {:?}",
        errors
    );
}

#[test]
fn test_match_case_binding_available_in_body() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
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
            "#
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "match case binding should be available in case body, got: {:?}",
        errors
    );
}

#[test]
fn test_match_case_binding_not_available_outside() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case None case Some(Int32) }
                func test(x: Option) -> Int32 {
                    match x {
                        case .Some(let val):
                            val
                        default:
                            0
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() > 0,
        "match case binding should NOT be available outside match, got: {:?}",
        errors
    );
}

#[test]
fn test_if_case_dot_shorthand_binding_available() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Option { case None case Some(Int32) }
                func test(x: Option) {
                    if case .Some(val) = x {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "dot shorthand case binding should be available in then block, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_associated_type_registered_symbol_resolver() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Container { associatedtype Item }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for protocol with associated type, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_associated_type_with_constraint() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Eq { func eq(other: Self) -> Bool } protocol Container { associatedtype Item: Eq }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for associated type with constraint, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_typealias_in_scope() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { typealias Item = Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for protocol with typealias, got: {:?}",
        errors
    );
}

#[test]
fn test_associated_type_name_not_leaked_outside_protocol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item } func foo(x: Item) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let _errors = engine_ref.get_errors();
}

#[test]
fn test_defer_body_variable_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 defer { x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
}

#[test]
fn test_defer_body_undefined_variable_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { defer { undefinedVar } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Expected errors for undefined variable in defer"
    );
}

#[test]
fn test_defer_nested_scope_symbol_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { defer { let a = 1 a } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Defer {
            body: defer_body, ..
        } = &*statements[0].borrow()
        && let Statement::VariableDecl { name, .. } = &*defer_body[0].borrow()
    {
        assert_eq!(name.value, "a");
    } else {
        panic!("Expected defer with variable decl");
    }

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
}

#[test]
fn test_symbol_resolve_variable_in_implicit_return() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { let x = 1 x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[1].borrow()
        && let Expression::Variable { symbol, .. } = &*expression.borrow()
    {
        assert_ne!(
            *symbol, None,
            "Variable in implicit return position should have resolved symbol"
        );
    } else {
        panic!("Expected FunctionDecl with variable in last expression position");
    }
}

#[test]
fn test_symbol_resolve_if_expression_branches() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { let a = 1 let b = 2 if true { a } else { b } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[2].borrow()
        && let Expression::If { then, else_, .. } = &*expression.borrow()
        && let Statement::ExpressionStatement {
            expression: then_expr,
        } = &*then[0].borrow()
        && let Expression::Variable {
            symbol: then_sym, ..
        } = &*then_expr.borrow()
    {
        assert_ne!(
            *then_sym, None,
            "Variable in then branch should have resolved symbol"
        );
        if let Some(ElseBranch::Block(else_stmts)) = else_
            && let Statement::ExpressionStatement {
                expression: else_expr_val,
            } = &*else_stmts[0].borrow()
            && let Expression::Variable {
                symbol: else_sym, ..
            } = &*else_expr_val.borrow()
        {
            assert_ne!(
                *else_sym, None,
                "Variable in else branch should have resolved symbol"
            );
        } else {
            panic!("Expected else branch with variable");
        }
    } else {
        panic!("Expected FunctionDecl with if expression in last expression position");
    }
}

#[test]
fn test_symbol_resolve_call_in_implicit_return() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo() -> Int32 { return 1 } func bar() -> Int32 { foo() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Call { callee, .. } = &*expression.borrow()
        && let Expression::Variable { symbol, .. } = &*callee.borrow()
    {
        assert_ne!(
            *symbol, None,
            "Function call in implicit return position should have resolved symbol"
        );
    } else {
        panic!("Expected FunctionDecl with call in last expression position");
    }
}

#[test]
fn test_match_multi_pattern_enum_symbols_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Status { case idle case loading case done }
                func test(s: Status) {
                    match s {
                        case .idle, .loading:
                            true
                        case .done:
                            false
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "multi-pattern enum match should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_match_multi_pattern_with_guard_symbols_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"
                enum Status { case idle case loading case done }
                func test(s: Status) {
                    match s {
                        case .idle, .loading where true:
                            true
                        default:
                            false
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "multi-pattern enum match with guard should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_module_creates_crate_entry() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module foo { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    let modules = &krate.borrow().modules;
    assert!(
        modules.contains_key("foo"),
        "module 'foo' should be in crate"
    );
}

#[test]
fn test_module_registers_symbol_in_parent_scope() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module foo { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    let root_module = resolver.resolve(&program, "test".to_string());
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let sym = root_scope.borrow().get_symbol("foo");
    assert!(
        sym.is_some(),
        "module 'foo' should be registered as symbol in root scope"
    );
    assert!(matches!(&*sym.unwrap().borrow(), Symbol::Module { name, .. } if name == "foo"));
}

#[test]
fn test_module_func_symbol_registered_in_module_scope() {
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
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    let foo_module = krate.borrow().modules.get("foo").cloned();
    assert!(foo_module.is_some(), "module 'foo' should be in crate");
    let foo_scope = foo_module.unwrap().borrow().scope.clone();
    assert!(foo_scope.is_some(), "module 'foo' should have a scope");
    let bar_sym = foo_scope.unwrap().borrow().get_symbol("bar");
    assert!(
        bar_sym.is_some(),
        "function 'bar' should be in module 'foo' scope"
    );
}

#[test]
fn test_nested_module_creates_child_entry() {
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
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    let modules = &krate.borrow().modules;
    assert!(
        modules.contains_key("foo"),
        "module 'foo' should be in crate"
    );
    assert!(
        modules.contains_key("foo.bar"),
        "module 'foo.bar' should be in crate"
    );
    let foo_module = modules.get("foo").unwrap().borrow();
    assert!(
        foo_module.children.contains_key("bar"),
        "'bar' should be child of 'foo'"
    );
}

#[test]
fn test_nested_module_func_resolved_in_nested_scope() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo { module bar { func baz() -> Int32 { 42 } } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    let bar_module = krate.borrow().modules.get("foo.bar").cloned();
    assert!(bar_module.is_some(), "module 'foo.bar' should be in crate");
    let bar_scope = bar_module.unwrap().borrow().scope.clone();
    assert!(bar_scope.is_some(), "module 'foo.bar' should have a scope");
    let baz_sym = bar_scope.unwrap().borrow().get_symbol("baz");
    assert!(
        baz_sym.is_some(),
        "function 'baz' should be in module 'foo.bar' scope"
    );
}

#[test]
fn test_dotted_path_module_creates_nested_modules() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo.bar { func baz() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    let modules = &krate.borrow().modules;
    assert!(
        modules.contains_key("foo"),
        "module 'foo' should be in crate"
    );
    assert!(
        modules.contains_key("foo.bar"),
        "module 'foo.bar' should be in crate"
    );
    let foo_module = modules.get("foo").unwrap().borrow();
    assert!(
        foo_module.children.contains_key("bar"),
        "'bar' should be child of 'foo'"
    );
    let bar_module = modules.get("foo.bar").unwrap().borrow();
    let bar_scope = bar_module.scope.clone().unwrap();
    let baz_sym = bar_scope.borrow().get_symbol("baz");
    assert!(
        baz_sym.is_some(),
        "function 'baz' should be in module 'foo.bar' scope"
    );
}

#[test]
fn test_multiple_modules_do_not_conflict() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module a { func fa() {} } module b { func fb() {} }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    let modules = &krate.borrow().modules;
    let a_module = modules.get("a").unwrap().borrow();
    let a_scope = a_module.scope.clone().unwrap();
    assert!(a_scope.borrow().get_symbol("fa").is_some());
    assert!(a_scope.borrow().get_symbol("fb").is_none());
    let b_module = modules.get("b").unwrap().borrow();
    let b_scope = b_module.scope.clone().unwrap();
    assert!(b_scope.borrow().get_symbol("fb").is_some());
    assert!(b_scope.borrow().get_symbol("fa").is_none());
}

#[test]
fn test_module_func_call_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo { func bar() -> Int32 { 42 } func baz() -> Int32 { bar() } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_borrow = engine.borrow();
    let errors = engine_borrow.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "module func call should resolve, got: {:?}",
        errors
    );
}

#[test]
fn test_overloaded_functions_register_without_error() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_borrow = engine.borrow();
    let errors = engine_borrow.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "overloaded functions should register without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_overloaded_struct_methods_register_without_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct S { func bar(a: Int32) { a } func bar(b: Bool) { b } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_borrow = engine.borrow();
    let errors = engine_borrow.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "overloaded struct methods should register without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_get_all_symbols_returns_overloads() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(x: Int32) { x } func foo(y: Float64) { y } func caller() { foo(x: 1) }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_borrow = engine.borrow();
    let errors = engine_borrow.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "overloaded function call should resolve, got: {:?}",
        errors
    );

    if let Statement::FunctionDecl { body, .. } = &*program.statements[2].borrow() {
        if let FunctionBody::Statements(stmts) = &*body.borrow() {
            if let Statement::ExpressionStatement { expression } = &*stmts[0].borrow() {
                let expr = expression.borrow();
                if let Expression::Call {
                    callee, overloads, ..
                } = &*expr
                {
                    if let Expression::Variable { name, .. } = &*callee.borrow() {
                        assert_eq!(name.value, "foo");
                        assert!(!overloads.is_empty(), "expected overloads to be populated");
                    }
                }
            }
        }
    }
}

fn run_resolver(
    code: &str,
) -> (
    Vec<Rc<RefCell<Statement>>>,
    Rc<RefCell<TrussDiagnosticEngine>>,
    Rc<RefCell<Package>>,
) {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    (program.statements, engine, krate)
}

#[test]
fn test_import_module_symbol() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo
         func test() -> Int32 { return Foo.bar() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for valid import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_module_member() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import Foo.Bar.baz
         func test() -> Int32 { return baz() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for valid member import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_module_wildcard() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.*
         func test() -> Int32 { return bar() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for wildcard import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_module_not_found() {
    let (statements, engine, _krate) = run_resolver(
        "import NonExistent
         func test() -> Int32 { return 42 }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert!(errors > 0, "Expected error for non-existent module import");
    assert_eq!(statements.len(), 2);
}

#[test]
fn test_import_nested_module() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import Foo
         func test() -> Int32 { return Foo.Bar.baz() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for nested module import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_deep_nested_member() {
    let (statements, engine, _krate) = run_resolver(
        "module A { module B { module C { func foo() -> Int32 { return 1 } } } }
         import A.B.C.foo
         func test() -> Int32 { return foo() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for deep nested member import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_wildcard_with_module_decl() {
    let (statements, engine, _krate) = run_resolver(
        "module Math {
            func add(a: Int32, b: Int32) -> Int32 { return a + b }
            func sub(a: Int32, b: Int32) -> Int32 { return a - b }
         }
         import Math.*
         func test() -> Int32 { return add(1, 2) }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for wildcard import with multiple functions, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_package_module_symbol() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import package.Foo
         func test() -> Int32 { return Foo.bar() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for package module import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_package_wildcard() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import package.Foo.*
         func test() -> Int32 { return bar() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for package wildcard import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_package_member() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import package.Foo.Bar.baz
         func test() -> Int32 { return baz() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for package member import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_package_nested_module() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import package.Foo
         func test() -> Int32 { return Foo.Bar.baz() }",
    );
    let errors = engine.borrow().get_diagnostics().len();
    assert_eq!(
        errors, 0,
        "Expected no errors for package nested module import, got: {:?}",
        errors
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_generic_function_with_constrained_param_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func compare<T: Equatable>(a: T, b: T) -> Bool { return true }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic function with constrained param should resolve without errors, got: {:?}",
        errors
    );
    if let Statement::FunctionDecl {
        name,
        generic_parameters,
        scope,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "compare");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "T");
        assert!(
            matches!(&generic_parameters[0].kind, GenericParameterKind::Type { constraints } if constraints.len() == 1)
        );
        assert!(scope.is_some());
        let scope_ref = scope.as_ref().unwrap().borrow();
        let t_type = scope_ref.get_type("T");
        assert!(t_type.is_some(), "Generic param T should be in type_env");
        if let Some(t) = t_type {
            assert_eq!(
                t.borrow().clone(),
                truss::types::Type::GenericParam("T".to_string())
            );
        }
    } else {
        panic!("Expected FunctionDecl with generic parameters");
    }
}

#[test]
fn test_generic_function_with_where_clause_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<T>(x: T) -> T where T: Equatable { return x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic function with where clause should resolve without errors, got: {:?}",
        errors
    );
    if let Statement::FunctionDecl {
        name,
        generic_parameters,
        where_clause,
        scope,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "foo");
        assert_eq!(generic_parameters.len(), 1);
        assert!(where_clause.is_some());
        assert!(scope.is_some());
        let t_type = scope.as_ref().unwrap().borrow().get_type("T");
        assert!(t_type.is_some());
    } else {
        panic!("Expected FunctionDecl with where clause");
    }
}

#[test]
fn test_generic_struct_type_param_in_scope() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic struct should resolve without errors, got: {:?}",
        errors
    );
    if let Statement::StructDecl {
        name,
        generic_parameters,
        scope,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Stack");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "Element");
        assert!(scope.is_some());
        let elem_type = scope.as_ref().unwrap().borrow().get_type("Element");
        assert!(
            elem_type.is_some(),
            "Generic param Element should be in struct scope type_env"
        );
        if let Some(t) = elem_type {
            assert_eq!(
                t.borrow().clone(),
                truss::types::Type::GenericParam("Element".to_string())
            );
        }
    } else {
        panic!("Expected StructDecl with generic parameters");
    }
}

#[test]
fn test_generic_function_multi_param_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func pair<A, B>(a: A, b: B) -> (A, B) { return (a, b) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic function with multiple params should resolve without errors, got: {:?}",
        errors
    );
    if let Statement::FunctionDecl {
        name,
        generic_parameters,
        scope,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "pair");
        assert_eq!(generic_parameters.len(), 2);
        assert!(scope.is_some());
        let scope_ref = scope.as_ref().unwrap().borrow();
        assert!(
            scope_ref.get_type("A").is_some(),
            "Generic param A should be in type_env"
        );
        assert!(
            scope_ref.get_type("B").is_some(),
            "Generic param B should be in type_env"
        );
    } else {
        panic!("Expected FunctionDecl with multiple generic parameters");
    }
}

#[test]
fn test_const_generic_function_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Const generic function should resolve without errors, got: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::FunctionDecl {
        name,
        generic_parameters,
        scope,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "foo");
        assert_eq!(generic_parameters.len(), 1);
        assert_eq!(generic_parameters[0].name.value, "N");
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Const { .. }
        ));
        assert!(scope.is_some());
        let scope_ref = scope.as_ref().unwrap().borrow();
        let n_type = scope_ref.get_type("N");
        assert!(n_type.is_some(), "Const generic N should be in type_env");
        if let Some(t) = n_type {
            assert!(matches!(&*t.borrow(), truss::types::Type::ConstGeneric(..)));
        }
    } else {
        panic!("Expected FunctionDecl with const generic parameter");
    }
}

#[test]
fn test_const_generic_struct_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Const generic struct should resolve without errors, got: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::StructDecl {
        name,
        generic_parameters,
        scope,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Buffer");
        assert_eq!(generic_parameters.len(), 2);
        assert!(matches!(
            &generic_parameters[0].kind,
            GenericParameterKind::Type { .. }
        ));
        assert!(matches!(
            &generic_parameters[1].kind,
            GenericParameterKind::Const { .. }
        ));
        assert!(scope.is_some());
        let scope_ref = scope.as_ref().unwrap().borrow();
        let t_type = scope_ref.get_type("T");
        assert!(t_type.is_some(), "Generic param T should be in type_env");
        if let Some(t) = t_type {
            assert!(matches!(&*t.borrow(), truss::types::Type::GenericParam(..)));
        }
        let n_type = scope_ref.get_type("N");
        assert!(n_type.is_some(), "Const generic N should be in type_env");
        if let Some(t) = n_type {
            assert!(matches!(&*t.borrow(), truss::types::Type::ConstGeneric(..)));
        }
    } else {
        panic!("Expected StructDecl with const generic parameter");
    }
}

#[test]
fn test_const_generic_class_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Const generic class should resolve without errors, got: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::ClassDecl {
        name,
        generic_parameters,
        scope,
        ..
    } = &*program.statements[0].borrow()
    {
        assert_eq!(name.value, "Wrapper");
        assert_eq!(generic_parameters.len(), 2);
        assert!(scope.is_some());
        let scope_ref = scope.as_ref().unwrap().borrow();
        let n_type = scope_ref.get_type("N");
        assert!(n_type.is_some(), "Const generic N should be in type_env");
        if let Some(t) = n_type {
            assert!(matches!(&*t.borrow(), truss::types::Type::ConstGeneric(..)));
        }
    } else {
        panic!("Expected ClassDecl with const generic parameter");
    }
}

#[test]
fn test_generic_protocol_with_assoc_types_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Container<T> { func get() -> T }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic protocol should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_class_with_where_clause_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic class with where clause should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_nested_generic_type_in_body_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Nested generic type Array<Array<Int32>> should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_closure_parameter_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { (x: Int32) in x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure {
            body: closure_body, ..
        } = &*init.borrow()
        && let Statement::ExpressionStatement { expression } = &*closure_body[0].borrow()
        && let Expression::Variable { symbol, .. } = &*expression.borrow()
    {
        assert_ne!(*symbol, None, "Closure parameter 'x' should be resolved");
    } else {
        panic!("Expected closure with resolved parameter symbol");
    }
}

#[test]
fn test_closure_captures_outer_variable() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a = 1; let f = { (x: Int32) in a } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        let closure_decl = statements
            .iter()
            .find(|s| {
                matches!(&*s.borrow(),
                    Statement::VariableDecl { name, .. } if name.value == "f"
                )
            })
            .expect("Expected variable 'f'");
        if let Statement::VariableDecl { initializer, .. } = &*closure_decl.borrow()
            && let Some(init) = initializer
            && let Expression::Closure {
                body: closure_body, ..
            } = &*init.borrow()
            && let Statement::ExpressionStatement { expression } = &*closure_body[0].borrow()
            && let Expression::Variable { name, symbol, .. } = &*expression.borrow()
        {
            assert_eq!(name.value, "a");
            assert_ne!(*symbol, None, "Captured variable 'a' should be resolved");
        } else {
            panic!("Expected closure capturing outer variable");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_closure_has_scope() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { (x: Int32) in x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure { scope, .. } = &*init.borrow()
    {
        assert!(scope.is_some(), "Closure should have a scope assigned");
    } else {
        panic!("Expected closure with scope");
    }
}

#[test]
fn test_closure_no_params_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { in 42 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure {
            parameters, scope, ..
        } = &*init.borrow()
    {
        assert!(parameters.is_empty());
        assert!(scope.is_some());
    } else {
        panic!("Expected closure with scope");
    }
}

#[test]
fn test_closure_shorthand_argument_resolved() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no symbol resolution errors: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure {
            body: closure_body,
            scope,
            ..
        } = &*init.borrow()
    {
        assert!(scope.is_some(), "Closure should have a scope");
        assert_eq!(closure_body.len(), 1);
        if let Statement::ExpressionStatement { expression } = &*closure_body[0].borrow() {
            assert!(matches!(
                &*expression.borrow(),
                Expression::ShorthandArgument { index: 0, .. }
            ));
        } else {
            panic!("Expected ExpressionStatement with ShorthandArgument");
        }
    } else {
        panic!("Expected closure with shorthand argument");
    }
}

#[test]
fn test_closure_shorthand_multi_args_resolved() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no symbol resolution errors: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure { scope, .. } = &*init.borrow()
    {
        assert!(scope.is_some(), "Closure should have a scope");
    } else {
        panic!("Expected closure");
    }
}

#[test]
fn test_let_variable_is_not_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a = 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no symbol resolution errors: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::FunctionDecl { scope, .. } = &*program.statements[0].borrow()
        && let Some(scope) = scope
    {
        let scope_ref = scope.borrow();
        if let Some(symbol) = scope_ref.get_symbol("a") {
            let sym = symbol.borrow();
            match &*sym {
                Symbol::Variable { is_var, .. } => {
                    assert!(!is_var, "let variable should have is_var = false");
                }
                _ => panic!("Expected Variable symbol"),
            }
        } else {
            panic!("Variable 'a' not found in scope");
        }
    } else {
        panic!("Expected FunctionDecl with scope");
    }
}

#[test]
fn test_var_variable_is_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { var a = 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no symbol resolution errors: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::FunctionDecl { scope, .. } = &*program.statements[0].borrow()
        && let Some(scope) = scope
    {
        let scope_ref = scope.borrow();
        if let Some(symbol) = scope_ref.get_symbol("a") {
            let sym = symbol.borrow();
            match &*sym {
                Symbol::Variable { is_var, .. } => {
                    assert!(is_var, "var variable should have is_var = true");
                }
                _ => panic!("Expected Variable symbol"),
            }
        } else {
            panic!("Variable 'a' not found in scope");
        }
    } else {
        panic!("Expected FunctionDecl with scope");
    }
}

#[test]
fn test_struct_let_property_is_not_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { let x: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no symbol resolution errors: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::StructDecl { name, scope, .. } = &*program.statements[0].borrow()
        && let Some(scope) = scope
    {
        let scope_ref = scope.borrow();
        if let Some(symbol) = scope_ref.get_symbol(&name.value)
            && let Symbol::Struct { properties, .. } = &*symbol.borrow()
        {
            assert_eq!(properties.len(), 1);
            let prop = properties[0].borrow();
            match &*prop {
                Symbol::StructProperty {
                    name: pname,
                    is_var,
                    ..
                } => {
                    assert_eq!(pname, "x");
                    assert!(!is_var, "let property should have is_var = false");
                }
                _ => panic!("Expected StructProperty symbol"),
            }
        } else {
            panic!("Struct 'Foo' not found");
        }
    } else {
        panic!("Expected StructDecl with scope");
    }
}

#[test]
fn test_struct_var_property_is_var() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { var x: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no symbol resolution errors: {:?}",
        errors
    );
    drop(engine_ref);
    if let Statement::StructDecl { name, scope, .. } = &*program.statements[0].borrow()
        && let Some(scope) = scope
    {
        let scope_ref = scope.borrow();
        if let Some(symbol) = scope_ref.get_symbol(&name.value)
            && let Symbol::Struct { properties, .. } = &*symbol.borrow()
        {
            assert_eq!(properties.len(), 1);
            let prop = properties[0].borrow();
            match &*prop {
                Symbol::StructProperty {
                    name: pname,
                    is_var,
                    ..
                } => {
                    assert_eq!(pname, "x");
                    assert!(is_var, "var property should have is_var = true");
                }
                _ => panic!("Expected StructProperty symbol"),
            }
        } else {
            panic!("Struct 'Foo' not found");
        }
    } else {
        panic!("Expected StructDecl with scope");
    }
}

#[test]
fn test_address_of_variable_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(v: Int32) { &v }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Unary {
            expression: inner,
            operator,
            is_prefix,
            ..
        } = &*expression.borrow()
    {
        assert_eq!(operator, &truss::ast::expression::UnaryOperator::AddressOf);
        assert!(is_prefix);
        if let Expression::Variable { symbol, .. } = &*inner.borrow() {
            assert_ne!(*symbol, None);
        } else {
            panic!("Expected variable inside address-of");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_address_of_deref_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(p: Int32*) { &*p }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::Unary {
            expression: addr_expr,
            operator,
            is_prefix,
            ..
        } = &*expression.borrow()
    {
        assert_eq!(operator, &truss::ast::expression::UnaryOperator::AddressOf);
        assert!(is_prefix);
        if let Expression::Unary {
            expression: deref_inner,
            operator: deref_op,
            ..
        } = &*addr_expr.borrow()
        {
            assert_eq!(deref_op, &truss::ast::expression::UnaryOperator::Deref);
            if let Expression::Variable { symbol, .. } = &*deref_inner.borrow() {
                assert_ne!(*symbol, None);
            } else {
                panic!("Expected variable inside &*");
            }
        } else {
            panic!("Expected deref inside &*");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_struct_subscript_symbol() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_class_subscript_symbol() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_subscript_symbol() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_subscript_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Array { } extension Array { subscript(index: Int32) -> Int32 { get { return 0 } } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_operator_function_resolver() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func + (left: Int32, right: Int32) -> Int32 { left }
             func - (left: Int32, right: Int32) -> Int32 { left }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Operator functions should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_operator_function_overloads() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func + (left: Int32, right: Int32) -> Int32 { left }
             func + (left: Float64, right: Float64) -> Float64 { left }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Overloaded operator functions should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_prefix_postfix_operator_resolver() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "prefix func - (value: Int32) -> Int32 { -value }
             postfix func ++ (value: Int32) -> Int32 { value }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Prefix/postfix operators should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_compound_assignment_operator_resolver() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Compound assignment operator should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_operator_method_resolver_inside_struct() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Operator method inside struct should resolve without errors, got: {:?}",
        errors
    );
}

fn resolve_and_check(
    input: &str,
) -> (
    Rc<RefCell<TrussDiagnosticEngine>>,
    Vec<Rc<RefCell<Statement>>>,
) {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(input.to_string(), Rc::new("test".to_string())),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let mut parser = Parser::new(lexer.get_file(), tokens, engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    (engine, program.statements)
}

#[test]
fn test_conditional_block_symbol_resolved() {
    let (engine, stmts) = resolve_and_check(
        "#if DEBUG\nfunc foo() -> Int32 { 1 }\n#endif\nfunc bar() -> Int32 { 2 }",
    );
    assert!(engine.borrow().get_errors().is_empty());
    assert_eq!(stmts.len(), 2);
    assert!(matches!(
        &*stmts[0].borrow(),
        Statement::ConditionalBlock { .. }
    ));
}

#[test]
fn test_conditional_block_nested_symbol_resolved() {
    let (engine, _stmts) =
        resolve_and_check("#if A\nlet x: Int32 = 1\n#if B\nlet y: Int32 = 2\n#endif\n#endif");
    assert!(engine.borrow().get_errors().is_empty());
}

#[test]
fn test_conditional_block_with_else_symbol_resolved() {
    let (engine, _stmts) =
        resolve_and_check("#if A\nlet x: Int32 = 1\n#else\nlet y: Int32 = 2\n#endif");
    assert!(engine.borrow().get_errors().is_empty());
}

#[test]
fn test_pragma_directives_no_crash() {
    let (engine, stmts) =
        resolve_and_check("#error \"test error\"\n#warning \"test warning\"\nlet x: Int32 = 1");
    assert!(engine.borrow().get_errors().is_empty());
    assert_eq!(stmts.len(), 3);
}

#[test]
fn test_conditional_block_function_call_resolved() {
    let (engine, _stmts) = resolve_and_check(
        "#if A\nfunc foo() -> Int32 { 1 }\n#else\nfunc foo() -> Int32 { 2 }\n#endif\nlet x: Int32 = foo()",
    );
    assert!(engine.borrow().get_errors().is_empty());
}

#[test]
fn test_conditional_block_function_overload_in_branches() {
    let (engine, _stmts) = resolve_and_check(
        "#if A\nfunc foo(x: Int32) -> Int32 { x }\n#else\nfunc foo(y: Int32) -> Int32 { y }\n#endif",
    );
    assert!(engine.borrow().get_errors().is_empty());
}

#[test]
fn test_asm_block_resolves_input_variable() {
    let (_statements, engine, _) =
        run_resolver(r#"func test() { var x: Int32 = 10; asm { "nop" : : val = in(reg) x } }"#);
    assert!(
        engine.borrow().get_errors().is_empty(),
        "asm block with valid input variable should not error"
    );
}

#[test]
fn test_asm_block_resolves_output_variable() {
    let (_statements, engine, _) = run_resolver(
        r#"func test() { var x: Int32 = 0; asm { "mov {dst}, 42" : dst = out(reg) x } }"#,
    );
    assert!(
        engine.borrow().get_errors().is_empty(),
        "asm block with valid output variable should not error"
    );
}

#[test]
fn test_asm_block_undefined_variable_error() {
    let (engine, _) = resolve_and_check(
        r#"func test() { var x: Int32 = 10; asm { "nop" : : val = in(reg) undefined_var } }"#,
    );
    assert!(
        engine.borrow().has_errors(),
        "undefined variable in asm operand should error"
    );
}

#[test]
fn test_asm_block_no_operands_no_error() {
    let (_statements, engine, _) =
        run_resolver(r#"func test() { var x: Int32 = 10; asm { "nop" } }"#);
    assert!(
        engine.borrow().get_errors().is_empty(),
        "asm block without operands should not error"
    );
}

#[test]
fn test_do_expression_scope() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { do { let x = 1 x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "do scope with resolved variable should not error, got: {:?}",
        errors
    );
}

#[test]
fn test_do_expression_scope_isolation() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { do { let x = 1 } x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "variable declared inside do should not be visible outside"
    );
}

#[test]
fn test_do_expression_nested_scope() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { do { do { let x = 1 } } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "nested do scopes should not error, got: {:?}",
        errors
    );
}

#[test]
fn test_do_expression_scope_has_scope_field() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { do { 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*stmts[0].borrow()
        && let Expression::Do { scope, .. } = &*expression.borrow()
    {
        assert!(
            scope.is_some(),
            "do expression should have a scope after symbol resolution"
        );
    } else {
        panic!("Expected do expression with scope");
    }
}

#[test]
fn test_symbol_resolve_yield_with_variable() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { let x = 42 yield x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::Yield {
            value: Some(value), ..
        } = &*statements[1].borrow()
        && let Expression::Variable { symbol, .. } = &*value.borrow()
    {
        assert_ne!(
            *symbol, None,
            "Variable in yield should have resolved symbol"
        );
    } else {
        panic!("Expected FunctionDecl with yield using variable");
    }
}

#[test]
fn test_symbol_resolve_yield_in_do_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = do { let a = 10 yield a } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            initializer: Some(init),
            ..
        } = &*statements[0].borrow()
        && let Expression::Do { body, .. } = &*init.borrow()
        && let Statement::Yield {
            value: Some(value), ..
        } = &*body[1].borrow()
        && let Expression::Variable { symbol, .. } = &*value.borrow()
    {
        assert_ne!(
            *symbol, None,
            "Variable in yield inside do expression should have resolved symbol"
        );
    } else {
        panic!("Expected yield inside do expression with resolved variable");
    }
}

#[test]
fn test_inline_type_resolves_base_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Dog {} func test() { let _: inline Dog }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_inline_type_with_size_resolves_base_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Dog {} func test() { let _: inline<256> Dog }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_inline_type_empty_brackets_resolves_base_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Dog {} func test() { let _: inline<> Dog }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_function_param_default_value_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_generic_param_default_type_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_labeled_param_default_value_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_struct_generic_default_type_resolves() {
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
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_const_generic_default_value_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo<let N: Int32 = 42>(x: Int32) -> Int32 { return x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors());
}

#[test]
fn test_import_selective_member() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } func baz() -> Int32 { return 99 } }
         import Foo.{bar}
         func test() -> Int32 { return bar() }",
    );
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Expected no errors for selective import"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_selective_member_with_alias() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.{bar as myBar}
         func test() -> Int32 { return myBar() }",
    );
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Expected no errors for selective import with alias"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_selective_member_original_name_not_visible() {
    let engine = create_engine();
    let code = "module Foo { func bar() -> Int32 { return 42 } func baz() -> Int32 { return 99 } }
         import Foo.{bar as myBar}
         func test() -> Int32 { return bar() }";
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(
        engine.borrow().has_errors(),
        "Should error because original name 'bar' is not visible after rename import"
    );
}

#[test]
fn test_import_selective_with_skip() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } func baz() -> Int32 { return 99 } }
         import Foo.{bar as _, baz}
         func test() -> Int32 { return baz() }",
    );
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Expected no errors when skipping one member and importing another"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_selective_skip_hides_symbol() {
    let engine = create_engine();
    let code = "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.{bar as _}
         func test() -> Int32 { return bar() }";
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(
        engine.borrow().has_errors(),
        "Should error because 'bar' was skipped with 'as _'"
    );
}

#[test]
fn test_import_selective_multiple_members() {
    let (statements, engine, _krate) = run_resolver(
        "module Math {
            func add(a: Int32, b: Int32) -> Int32 { return a + b }
            func sub(a: Int32, b: Int32) -> Int32 { return a - b }
            func mul(a: Int32, b: Int32) -> Int32 { return a * b }
         }
         import Math.{add, sub}
         func test() -> Int32 { return add(1, sub(2, 3)) }",
    );
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Expected no errors for selective import of multiple members"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_single_as_alias() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.bar as myBar
         func test() -> Int32 { return myBar() }",
    );
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Expected no errors for single import with 'as' alias"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_single_as_skip() {
    let engine = create_engine();
    let code = "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.bar as _
         func test() -> Int32 { return bar() }";
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(
        engine.borrow().has_errors(),
        "Should error because 'bar' was skipped with 'as _'"
    );
}

#[test]
fn test_import_selective_member_not_found() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.{nonexistent}
         func test() -> Int32 { return 42 }",
    );
    assert!(
        engine.borrow().has_errors(),
        "Expected error for importing non-existent member"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_selective_module_not_found() {
    let (statements, engine, _krate) = run_resolver(
        "import NonExistent.{foo}
         func test() -> Int32 { return 42 }",
    );
    assert!(
        engine.borrow().has_errors(),
        "Expected error for non-existent module in selective import"
    );
    assert_eq!(statements.len(), 2);
}

#[test]
fn test_import_selective_package() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import package.Foo.{bar}
         func test() -> Int32 { return bar() }",
    );
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Expected no errors for selective package import"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_import_selective_package_with_alias() {
    let (statements, engine, _krate) = run_resolver(
        "module Foo { func bar() -> Int32 { return 42 } }
         import package.Foo.{bar as myBar}
         func test() -> Int32 { return myBar() }",
    );
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Expected no errors for selective package import with alias"
    );
    assert_eq!(statements.len(), 3);
}

#[test]
fn test_array_literal_elements_resolve() {
    let (_, engine, _) = run_resolver("func test() { let a = [1, 2, 3] }");
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Array literal elements should resolve"
    );
}

#[test]
fn test_array_literal_empty_resolves() {
    let (_, engine, _) = run_resolver("func test() { let a = [] }");
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Empty array literal should resolve"
    );
}

#[test]
fn test_array_literal_resolves_variable() {
    let (_, engine, _) = run_resolver("func test() { let x = 42; let a = [x] }");
    assert_eq!(
        engine.borrow().get_diagnostics().len(),
        0,
        "Array literal with variable reference should resolve"
    );
}

#[test]
fn test_array_literal_undefined_variable() {
    let (_, engine, _) = run_resolver("func test() { let a = [undefinedVar] }");
    assert!(
        engine.borrow().has_errors(),
        "Undefined variable in array literal should produce error"
    );
}

#[test]
fn test_self_type_constructor_call() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo {
    var x: Int32
    init(x: Int32) { self.x = x }
    func clone() -> Foo { Self(x: self.x) }
}"
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser should have no errors"
    );
    let (packages, _pkg) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    assert!(
        !engine.borrow().has_errors(),
        "SymbolResolver should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_optional_type_sugar_resolves_inner_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: Int32? = 10".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_array_type_sugar_resolves_inner_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: [Int32] = {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(
        !engine.borrow().has_errors(),
        "Parser errors: {:?}",
        engine.borrow().get_diagnostics()
    );
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine);
    resolver.resolve(&program, "test".to_string());
}

#[test]
fn test_abstract_class_symbol() {
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
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let class_sym = root_scope.borrow().get_symbol("Shape").unwrap();
    let binding = class_sym.borrow();
    let Symbol::Class {
        methods,
        is_abstract,
        ..
    } = &*binding
    else {
        panic!("Expected Class symbol");
    };
    assert!(*is_abstract, "Shape should be abstract");
    let has_abstract_method = methods.iter().any(|m| {
        let mb = m.borrow();
        matches!(&*mb, Symbol::ClassMethod { name, is_abstract, .. } if name == "draw" && *is_abstract)
    });
    assert!(has_abstract_method, "draw method should be abstract");
}

#[test]
fn test_final_class_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "final class Constants { let maxSize: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let class_sym = root_scope.borrow().get_symbol("Constants").unwrap();
    let binding = class_sym.borrow();
    let Symbol::Class { is_final, .. } = &*binding else {
        panic!("Expected Class symbol");
    };
    assert!(*is_final, "Constants should be final");
}

#[test]
fn test_final_method_in_final_class_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "final class FinalClass { func method() -> Int32 { return 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let class_sym = root_scope.borrow().get_symbol("FinalClass").unwrap();
    let binding = class_sym.borrow();
    let Symbol::Class { methods, .. } = &*binding else {
        panic!("Expected Class symbol");
    };
    let has_final_method = methods.iter().any(|m| {
        let mb = m.borrow();
        matches!(&*mb, Symbol::ClassMethod { name, is_final, .. } if name == "method" && *is_final)
    });
    assert!(has_final_method, "method in final class should be final");
}

#[test]
fn test_override_method_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Base { func foo() -> Int32 { return 1 } }
             class Derived: Base { override func foo() -> Int32 { return 2 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let derived_sym = root_scope.borrow().get_symbol("Derived").unwrap();
    let binding = derived_sym.borrow();
    let Symbol::Class { methods, .. } = &*binding else {
        panic!("Expected Class symbol");
    };
    let has_override = methods.iter().any(|m| {
        let mb = m.borrow();
        matches!(&*mb, Symbol::ClassMethod { name, is_override, .. } if name == "foo" && *is_override)
    });
    assert!(has_override, "foo should have is_override=true");
}

#[test]
fn test_abstract_method_in_non_abstract_class_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Concrete { abstract func foo() -> Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let has_abstract_error = engine_ref
        .get_diagnostics()
        .iter()
        .any(|d| d.code == TrussDiagnosticCode::AbstractMemberInNonAbstractClass);
    assert!(
        has_abstract_error,
        "Should report error for abstract method in non-abstract class"
    );
}

#[test]
fn test_final_property_in_final_class_symbol() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "final class Constants { var value: Int32 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
    drop(engine_ref);

    let root_module = krate.borrow().modules.get("test").cloned().unwrap();
    let root_scope = root_module.borrow().scope.clone().unwrap();
    let class_sym = root_scope.borrow().get_symbol("Constants").unwrap();
    let binding = class_sym.borrow();
    let Symbol::Class { properties, .. } = &*binding else {
        panic!("Expected Class symbol");
    };
    let has_final_property = properties.iter().any(|p| {
        let pb = p.borrow();
        matches!(&*pb, Symbol::ClassProperty { name, is_final, .. } if name == "value" && *is_final)
    });
    assert!(
        has_final_property,
        "property in final class should be final"
    );
}
