use std::{cell::RefCell, collections::HashMap, rc::Rc};

use truss::{
    ast::{
        expression::Expression,
        statement::{FunctionBody, ProtocolMember, Statement},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine},
    krate::Package,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol::WeakSymbol,
    symbol_resolver::SymbolResolver,
    type_resolver::TypeResolver,
    types::Type,
};

fn type_of(name: &str) -> Type {
    Type::Struct(
        name.to_string(),
        truss::symbol::WeakSymbol(std::rc::Weak::new()),
        vec![],
    )
}

fn create_engine() -> Rc<RefCell<TrussDiagnosticEngine>> {
    Rc::new(RefCell::new(TrussDiagnosticEngine::new()))
}

#[test]
fn test_infer_variable_decl() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Int32 { let a = 1 return a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int32"));
        } else {
            panic!("VariableDecl should have Int32 type");
        }
        if let Statement::Return { value, .. } = &*statements[1].borrow()
            && let Some(value) = value
            && let Expression::Variable { ty, .. } = &*value.borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int32"));
        } else {
            panic!("Return should have Int32 typed variable");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_check_variable_decl_with_annotation() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Int32 { let a: Int32 = 1 return a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int32"));
        } else {
            panic!("VariableDecl should have Int32 type");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_return_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Bool { return true }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::Return { value, .. } = &*statements[0].borrow()
            && let Some(value) = value
            && let Expression::BooleanLiteral { .. } = &*value.borrow()
        {
        } else {
            panic!("Expected return with BooleanLiteral");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_binary_expression_infer() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Int32 { let a = 1 let b = a + 1 return b }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[1].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int32"));
        } else {
            panic!("Second variable should be Int32");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_expression_body_function() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Int32 = 42".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Expression(expr) = &*body.borrow()
        && let Expression::IntegerLiteral { ty, .. } = &*expr.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int32"));
    } else {
        panic!("Expected expression body function returning Int32");
    }
}

#[test]
fn test_variable_decl_with_bool_annotation() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Bool { let a: Bool = true return a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Bool"));
        } else {
            panic!("VariableDecl should have Bool type");
        }
        if let Statement::Return { value, .. } = &*statements[1].borrow()
            && let Some(value) = value
            && let Expression::Variable { ty, .. } = &*value.borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Bool"));
        } else {
            panic!("Return should have Bool typed variable");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_type_annotation_mismatch() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Int32 { let a: Bool = 1 return a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty());
    assert_eq!(errors[0].code, TrussDiagnosticCode::TypeMismatch);
    assert!(errors[0].message.contains("Type mismatch"));
    assert!(errors[0].message.contains("Bool"));
}

#[test]
fn test_never_type_annotation() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test()->Never { let a: Never return a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { return_type, .. } = &*program.statements[0].borrow()
        && let Some(return_type) = return_type
        && let Expression::Type { ty, .. } = &*return_type.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Never);
    } else {
        panic!("Function should have Never return type");
    }
}

#[test]
fn test_annotated_param_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test(_ a: Int32)->Int32 { return a }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl {
        parameters, body, ..
    } = &*program.statements[0].borrow()
    {
        if let Some(ty) = &parameters[0].borrow().ty {
            assert_eq!(ty.borrow().clone(), type_of("Int32"));
        } else {
            panic!("Parameter should have Int32 type");
        }
        if let FunctionBody::Statements(statements) = &*body.borrow()
            && let Statement::Return { value, .. } = &*statements[0].borrow()
            && let Some(value) = value
            && let Expression::Variable { ty, .. } = &*value.borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int32"));
        } else {
            panic!("Return should have Int32 typed variable");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

fn run_type_check_with_return(code: &str) -> Type {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        for stmt in statements.iter().rev() {
            if let Statement::Return {
                value: Some(value), ..
            } = &*stmt.borrow()
            {
                match &*value.borrow() {
                    Expression::IntegerLiteral { ty, .. }
                    | Expression::Variable { ty, .. }
                    | Expression::DecimalLiteral { ty, .. } => {
                        return ty.as_ref().unwrap().borrow().clone();
                    }
                    Expression::Unary { expression, .. } => match &*expression.borrow() {
                        Expression::IntegerLiteral { ty, .. }
                        | Expression::DecimalLiteral { ty, .. } => {
                            return ty.as_ref().unwrap().borrow().clone();
                        }
                        Expression::Variable { ty, .. } => {
                            return ty.as_ref().unwrap().borrow().clone();
                        }
                        _ => {}
                    },
                    Expression::BooleanLiteral { .. } => return type_of("Bool"),
                    _ => {}
                }
            }
        }
    }
    panic!("No return statement found");
}

fn run_type_check_var(code: &str, var_name: &str) -> Type {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        for stmt in statements {
            if let Statement::VariableDecl { name, ty, .. } = &*stmt.borrow()
                && name.value == var_name
            {
                return ty.as_ref().unwrap().borrow().clone();
            }
        }
    }
    panic!("Variable {} not found", var_name);
}

#[test]
fn test_small_positive_integer() {
    let ty = run_type_check_with_return("func test() -> Int32 { return 42 }");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_zero() {
    let ty = run_type_check_with_return("func test() -> Int32 { return 0 }");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_negative_integer() {
    let ty = run_type_check_with_return("func test() -> Int32 { return -100 }");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_large_positive_integer() {
    let ty = run_type_check_with_return("func test() -> Int64 { return 5000000000 }");
    assert_eq!(ty, type_of("Int64"));
}

#[test]
fn test_very_large_integer() {
    let ty = run_type_check_with_return("func test() -> Int128 { return 20000000000000000000 }");
    assert_eq!(ty, type_of("Int128"));
}

#[test]
fn test_int8_annotation() {
    let ty = run_type_check_var("func test() { let a: Int8 = 100 }", "a");
    assert_eq!(ty, type_of("Int8"));
}

#[test]
fn test_int16_annotation() {
    let ty = run_type_check_var("func test() { let a: Int16 = 10000 }", "a");
    assert_eq!(ty, type_of("Int16"));
}

#[test]
fn test_int32_annotation() {
    let ty = run_type_check_var("func test() { let a: Int32 = 100000 }", "a");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_int64_annotation() {
    let ty = run_type_check_var("func test() { let a: Int64 = 10000000000 }", "a");
    assert_eq!(ty, type_of("Int64"));
}

#[test]
fn test_int128_annotation() {
    let ty = run_type_check_var("func test() { let a: Int128 = 10000000000000000000 }", "a");
    assert_eq!(ty, type_of("Int128"));
}

#[test]
fn test_uint8_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt8 = 200 }", "a");
    assert_eq!(ty, type_of("UInt8"));
}

#[test]
fn test_uint16_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt16 = 50000 }", "a");
    assert_eq!(ty, type_of("UInt16"));
}

#[test]
fn test_uint32_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt32 = 3000000000 }", "a");
    assert_eq!(ty, type_of("UInt32"));
}

#[test]
fn test_uint64_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt64 = 100000000000000 }", "a");
    assert_eq!(ty, type_of("UInt64"));
}

#[test]
fn test_uint128_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt128 = 10000000000000000000 }", "a");
    assert_eq!(ty, type_of("UInt128"));
}

#[test]
fn test_float_literal_default() {
    let ty = run_type_check_with_return("func test() -> Float64 { return 3.14 }");
    assert_eq!(ty, type_of("Float64"));
}

#[test]
fn test_float_with_exponent() {
    let ty = run_type_check_with_return("func test() -> Float64 { return 1.5e10 }");
    assert_eq!(ty, type_of("Float64"));
}

#[test]
fn test_float32_annotation() {
    let ty = run_type_check_var("func test() { let a: Float32 = 3.14 }", "a");
    assert_eq!(ty, type_of("Float32"));
}

#[test]
fn test_float64_annotation() {
    let ty = run_type_check_var("func test() { let a: Float64 = 3.14159265358979 }", "a");
    assert_eq!(ty, type_of("Float64"));
}

#[test]
fn test_return_type_context_int32() {
    let ty = run_type_check_with_return("func test() -> Int32 { return 42 }");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_return_type_context_int64() {
    let ty = run_type_check_with_return("func test() -> Int64 { return 42 }");
    assert_eq!(ty, type_of("Int64"));
}

#[test]
fn test_return_type_context_uint32() {
    let ty = run_type_check_with_return("func test() -> UInt32 { return 42 }");
    assert_eq!(ty, type_of("UInt32"));
}

#[test]
fn test_return_type_context_float32() {
    let ty = run_type_check_with_return("func test() -> Float32 { return 3.14 }");
    assert_eq!(ty, type_of("Float32"));
}

#[test]
fn test_return_type_context_float64() {
    let ty = run_type_check_with_return("func test() -> Float64 { return 3.14 }");
    assert_eq!(ty, type_of("Float64"));
}

#[test]
fn test_parameter_type_context() {
    let code = "func test(_ a: Int64) -> Int64 { return a }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        let param_ty = parameters[0].borrow().ty.as_ref().unwrap().borrow().clone();
        assert_eq!(param_ty, type_of("Int64"));
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_binary_expression_same_type() {
    let ty = run_type_check_with_return("func test() -> Int32 { let a = 10 + 20 return a }");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_unary_expression() {
    let ty = run_type_check_with_return("func test() -> Int32 { let a = -42 return a }");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_variable_propagation() {
    let ty = run_type_check_with_return("func test() -> Int32 { let a = 42 return a }");
    assert_eq!(ty, type_of("Int32"));
}

#[test]
fn test_i8_max() {
    let ty = run_type_check_var("func test() { let a: Int8 = 127 }", "a");
    assert_eq!(ty, type_of("Int8"));
}

#[test]
fn test_i8_min() {
    let ty = run_type_check_var("func test() { let a: Int8 = -128 }", "a");
    assert_eq!(ty, type_of("Int8"));
}

#[test]
fn test_u8_max() {
    let ty = run_type_check_var("func test() { let a: UInt8 = 255 }", "a");
    assert_eq!(ty, type_of("UInt8"));
}

#[test]
fn test_u16_max() {
    let ty = run_type_check_var("func test() { let a: UInt16 = 65535 }", "a");
    assert_eq!(ty, type_of("UInt16"));
}

#[test]
fn test_u32_max() {
    let ty = run_type_check_var("func test() { let a: UInt32 = 4294967295 }", "a");
    assert_eq!(ty, type_of("UInt32"));
}

#[test]
fn test_type_mismatch_int_float() {
    let code = "func test() -> Int32 { let a: Float64 = 3.14 return a }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, TrussDiagnosticCode::TypeMismatch);
    assert!(errors[0].message.contains("Type mismatch"));
    assert!(errors[0].message.contains("Int32"));
    assert!(errors[0].message.contains("Float64"));
}

#[test]
fn test_type_mismatch_different_int_sizes() {
    let code = "func test() -> Int32 { let a: Int64 = 42 return a }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code, TrussDiagnosticCode::TypeMismatch);
    assert!(errors[0].message.contains("Type mismatch"));
    assert!(errors[0].message.contains("Int32"));
    assert!(errors[0].message.contains("Int64"));
}

#[test]
fn test_function_call_parameter_type_inference_int8() {
    let code = "func test(_ a: Int8) -> Int8 { return a } func main() { let result = test(42) return result }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int8"));
        } else {
            panic!("Variable should have Int8 type inferred from function parameter");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_function_call_parameter_type_inference_int64() {
    let code = "func test(_ a: Int64) -> Int64 { return a } func main() { let result = test(10000000000) return result }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int64"));
        } else {
            panic!("Variable should have Int64 type inferred from function parameter");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_function_call_parameter_type_inference_float32() {
    let code = "func test(_ a: Float32) -> Float32 { return a } func main() { let result = test(3.14) return result }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Float32"));
        } else {
            panic!("Variable should have Float32 type inferred from function parameter");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_function_call_multiple_parameters_type_inference() {
    let code = "func add(_ a: Int32, _ b: Int32) -> Int32 { return a + b } func main() { let result = add(10, 20) return result }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), type_of("Int32"));
        } else {
            panic!("Variable should have Int32 type inferred from function return type");
        }
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_function_call_missing_label() {
    let code = "func f(a a: Int64) -> Int64 { return a } func f2() { f(1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty());
    assert_eq!(errors[0].code, TrussDiagnosticCode::MissingArgumentLabel);
    assert!(errors[0].message.contains("Missing argument label"));
    assert!(errors[0].message.contains("a"));
}

#[test]
fn test_function_call_with_correct_label() {
    let code = "func f(a a: Int64) -> Int64 { return a } func f2() { f(a: 1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());
}

#[test]
fn test_function_call_with_wrong_label() {
    let code = "func f(a a: Int64) -> Int64 { return a } func f2() { f(b: 1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty());
    assert_eq!(errors[0].code, TrussDiagnosticCode::ArgumentLabelMismatch);
    assert!(errors[0].message.contains("Expected argument label"));
}

#[test]
fn test_function_call_underscore_label() {
    let code = "func f(_ a: Int64) -> Int64 { return a } func f2() { f(1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());
}

#[test]
fn test_function_call_no_label() {
    let code = "func f(a: Int64) -> Int64 { return a } func f2() { f(1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty());
    assert_eq!(errors[0].code, TrussDiagnosticCode::MissingArgumentLabel);
    assert!(errors[0].message.contains("Missing argument label"));
    assert!(errors[0].message.contains("a"));
}

#[test]
fn test_function_call_no_label_with_correct_usage() {
    let code = "func f(a: Int64) -> Int64 { return a } func f2() { f(a: 1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());
}

#[test]
fn test_function_call_parameter_order_must_match() {
    let code =
        "func f(a a: Int64, b b: Int64) -> Int64 { return a + b } func f2() { f(b: 2, a: 1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.is_empty(),
        "Labeled parameters in any order should succeed"
    );
}

#[test]
fn test_function_call_correct_parameter_order() {
    let code =
        "func f(a a: Int64, b b: Int64) -> Int64 { return a + b } func f2() { f(a: 1, b: 2) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());
}

#[test]
fn test_function_call_underscore_label_with_explicit_underscore() {
    let code = "func f(_ a: Int64) -> Int64 { return a } func f2() { f(_: 1) }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());
}

#[test]
fn test_pointer_type_annotation() {
    let code = "func test() { let p: Int32* }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert!(
            matches!(ty.borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Struct(ref n, _, _) if n == "Int32"))
        );
    } else {
        panic!("Expected variable declaration with pointer type");
    }
}

#[test]
fn test_deref_expression() {
    let code = "func test(p: Int32*) -> Int32 { return *p }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());

    if let Statement::FunctionDecl { return_type, .. } = &*program.statements[0].borrow()
        && let Some(return_type) = return_type
        && let Expression::Type { ty, .. } = &*return_type.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int32"));
    } else {
        panic!("Expected function with Int32 return type");
    }
}

#[test]
fn test_nested_deref_expression() {
    let code = "func test(p: Int64**) -> Int64 { return **p }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());

    if let Statement::FunctionDecl { return_type, .. } = &*program.statements[0].borrow()
        && let Some(return_type) = return_type
        && let Expression::Type { ty, .. } = &*return_type.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int64"));
    } else {
        panic!("Expected function with Int64 return type");
    }
}

#[test]
fn test_nullptr_literal() {
    let code = "func test() { let p: Void* = nullptr }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            ty, initializer, ..
        } = &*statements[0].borrow()
        && let Some(ty) = ty
        && let Some(init) = initializer
    {
        assert!(
            matches!(ty.borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Void))
        );
        if let Expression::NullptrLiteral { ty: init_ty, .. } = &*init.borrow() {
            assert!(init_ty.is_some());
            assert!(
                matches!(init_ty.as_ref().unwrap().borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Void))
            );
        } else {
            panic!("Expected nullptr literal");
        }
    } else {
        panic!("Expected variable declaration with pointer type");
    }
}

#[test]
fn test_nullptr_type_inference_with_annotation() {
    let code = "func test() { var p: Int32* = nullptr }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            ty, initializer, ..
        } = &*statements[0].borrow()
        && let Some(ty) = ty
        && let Some(init) = initializer
    {
        assert!(
            matches!(ty.borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Struct(ref n, _, _) if n == "Int32"))
        );
        if let Expression::NullptrLiteral { ty: init_ty, .. } = &*init.borrow() {
            assert!(init_ty.is_some());
            assert!(
                matches!(init_ty.as_ref().unwrap().borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Struct(ref n, _, _) if n == "Int32"))
            );
        } else {
            panic!("Expected nullptr literal");
        }
    } else {
        panic!("Expected variable declaration with pointer type");
    }
}

#[test]
fn test_nullptr_return_type_inference() {
    let code = "func getNull() -> Int32* { return nullptr }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());
}

#[test]
fn test_nullptr_default_void_pointer() {
    let code = "func test() { var p = nullptr }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            ty, initializer, ..
        } = &*statements[0].borrow()
        && let Some(ty) = ty
        && let Some(init) = initializer
    {
        assert!(
            matches!(ty.borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Void))
        );
        if let Expression::NullptrLiteral { ty: init_ty, .. } = &*init.borrow() {
            assert!(init_ty.is_some());
            assert!(
                matches!(init_ty.as_ref().unwrap().borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Void))
            );
        } else {
            panic!("Expected nullptr literal");
        }
    } else {
        panic!("Expected variable declaration with Void* type");
    }
}

#[test]
fn test_cast_int_to_float() {
    let code = "func test() -> Float64 { let x = 1 as Float64 return x }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            ty, initializer, ..
        } = &*statements[0].borrow()
        && let Some(ty) = ty
        && let Some(init) = initializer
    {
        assert_eq!(ty.borrow().clone(), type_of("Float64"));
        if let Expression::Cast { ty: cast_ty, .. } = &*init.borrow() {
            assert!(cast_ty.is_some());
            assert_eq!(cast_ty.as_ref().unwrap().borrow().clone(), type_of("Float64"));
        } else {
            panic!("Expected Cast expression");
        }
    } else {
        panic!();
    }
}

#[test]
fn test_cast_bool_to_int() {
    let code = "func test() -> Int32 { let x = true as Int32 return x }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int32"));
    } else {
        panic!();
    }
}

#[test]
fn test_cast_invalid_bool_to_float() {
    let code = "func test() { let x = true as Float64 }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("Cannot cast"));
}

#[test]
fn test_cast_int32_to_int64() {
    let code = "func test() -> Int64 { let x = 42 as Int64 return x }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int64"));
    } else {
        panic!();
    }
}

#[test]
fn test_cast_force_no_type_error() {
    let code = "func test() -> Int32 { let x = 1 as! Int32 return x }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int32"));
    } else {
        panic!();
    }
}

#[test]
fn test_cast_conditional_no_type_error() {
    let code = "func test() -> Int32 { let x = 1 as? Int32 return x }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int32"));
    } else {
        panic!();
    }
}

#[test]
fn test_cast_force_bitcast_same_size() {
    let code = "func test() -> Int32 { let x = 3.14 as Float32 as!! Int32 return x }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_mixed_extern_and_normal_funcs_no_crash() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" func putchar(_ c: Char)
func caller(_ c: Char) { putchar(c) }"#
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_cast_force_bitcast_mismatched_size_error() {
    let code = "func test() -> Int16 { let x = 3.14 as!! Int16 return x }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty());
    assert!(errors[0].message.contains("different sizes"));
}

#[test]
fn test_struct_decl_type_resolve() {
    let code = "struct Point { let x: Int32 let y: Int32 }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::StructDecl { name, body, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
        assert_eq!(body.len(), 3);
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_struct_type_reference() {
    let code = r#"
        struct Point { let x: Int32 let y: Int32 }
        func test() -> Point { var p: Point return p }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let unknown_type_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Unknown type"))
        .collect();
    assert!(
        unknown_type_errors.is_empty(),
        "Struct type 'Point' should be recognized, but found errors: {:?}",
        errors
    );
}

#[test]
fn test_member_access_type_inference() {
    let code = r#"
        struct Point { let x: Int32 let y: Int32 }
        func test() { let p: Point = ??? let val = p.x }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let field_not_found_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.message.contains("Field") && e.message.contains("not found"))
        .collect();
    assert!(
        field_not_found_errors.is_empty(),
        "Field 'x' should be found in struct Point, but found errors: {:?}",
        errors
    );

    if let Statement::StructDecl { name, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "Point");
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_struct_method_call() {
    let code = r#"
        struct Math { func double(x: Int32) -> Int32 { return x * 2 } }
        func test() { 
            let m: Math
            let val = m.double(5)
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_extern_func_no_crash() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(
            r#"extern "C" func putchar(_ c: Char)
extern "C" func malloc(_ size: UInt64) -> Void*
func f(_ a: UInt64, _ b: UInt64) -> UInt64 { a / b }"#
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_struct_init_deinit_type() {
    let code = r#"
        struct Point { let x: Int32 init(x: Int32) { } deinit { } }
        func test() {
            let p: Point
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert_eq!(body.len(), 3);
        if let Statement::InitDecl { ty, .. } = &*body[1].borrow() {
            assert!(ty.is_some());
            let fn_type = ty.as_ref().unwrap().borrow().clone();
            if let Type::Function(param_tys, ret_ty, is_vararg, None) = &fn_type {
                assert_eq!(param_tys.len(), 1);
                assert_eq!(*ret_ty.borrow(), Type::Void);
                assert!(!is_vararg);
            } else {
                panic!("Expected Function type for init");
            }
        } else {
            panic!("Expected InitDecl");
        }
        if let Statement::DeinitDecl { ty, .. } = &*body[2].borrow() {
            assert!(ty.is_some());
            let fn_type = ty.as_ref().unwrap().borrow().clone();
            if let Type::Function(param_tys, ret_ty, is_vararg, None) = &fn_type {
                assert_eq!(param_tys.len(), 0);
                assert_eq!(*ret_ty.borrow(), Type::Void);
                assert!(!is_vararg);
            } else {
                panic!("Expected Function type for deinit");
            }
        } else {
            panic!("Expected DeinitDecl");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_struct_deinit_method_call_type() {
    let code = r#"
        struct Data { var value: Int32 deinit { } }
        func test() {
            var d: Data
            d.deinit()
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_type_instantiation() {
    let code = r#"
        struct Point { let x: Int32 init(x: Int32) {} }
        func test() { Point(x: 1) }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_type_instantiation_arg_mismatch() {
    let code = r#"
        struct Point { let x: Int32 init(x: Int32) {} }
        func test() { Point(x: 1, y: 2) }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should have argument count mismatch error"
    );
}

#[test]
fn test_type_instantiation_with_func_call() {
    let code = r#"
        struct Point { init() {} }
        func foo() {}
        func test() {
            foo()
            Point()
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_type_resolved() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case Option.Some(val) = x {
                let _: Int32 = val
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_binding_type_inference() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case Option.Some(val) = x {
                val
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_no_bindings_type() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case Option.None = x {
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_enum_type_not_found() {
    let code = r#"
        func test(x: Int32) {
            if case Option.Some(val) = x {
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() > 0,
        "Should have errors for unknown enum type, got: {:?}",
        errors
    );
}

#[test]
fn test_if_case_case_not_found() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case Option.unknown(val) = x {
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() > 0,
        "Should have errors for unknown case, got: {:?}",
        errors
    );
}

#[test]
fn test_if_case_else_branch() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case Option.Some(val) = x {
                let _ = val
            } else {
                let _ = 42
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_class_superclass_type() {
    let code = "class Animal {} class Dog: Animal {}";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have type errors, got: {:?}",
        errors
    );

    if let Statement::ClassDecl {
        name, superclass, ..
    } = &*program.statements[1].borrow()
    {
        assert_eq!(name.value, "Dog");
        assert!(superclass.is_some());
        if let Expression::Type {
            name: super_name, ..
        } = &*superclass.as_ref().unwrap().borrow()
        {
            assert_eq!(super_name.value, "Animal");
        } else {
            panic!("Expected superclass to be a Type expression");
        }
    } else {
        panic!("Expected ClassDecl for Dog");
    }
}

#[test]
fn test_self_keyword_type_in_struct_method() {
    let code = r#"
        struct Point { let x: Int32 func get_self() -> Point { return self } }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        if let Statement::FunctionDecl { body: fn_body, .. } = &*body[1].borrow()
            && let FunctionBody::Statements(statements) = &*fn_body.borrow()
            && let Statement::Return { value, .. } = &*statements[0].borrow()
            && let Some(value) = value
            && let Expression::SelfKeyword { ty, .. } = &*value.borrow()
        {
            let self_ty = ty.as_ref().expect("self should have a type");
            match &*self_ty.borrow() {
                Type::Struct(name, ..) => assert_eq!(name, "Point"),
                other => panic!("Expected Struct type for self, got {:?}", other),
            }
        } else {
            panic!("Unexpected AST structure");
        }
    } else {
        panic!("Expected StructDecl");
    }
}

#[test]
fn test_self_keyword_type_in_class_method() {
    let code = r#"
        class Animal { func get_self() -> Animal { return self } }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ClassDecl { body, .. } = &*program.statements[0].borrow() {
        if let Statement::FunctionDecl { body: fn_body, .. } = &*body[0].borrow()
            && let FunctionBody::Statements(statements) = &*fn_body.borrow()
            && let Statement::Return { value, .. } = &*statements[0].borrow()
            && let Some(value) = value
            && let Expression::SelfKeyword { ty, .. } = &*value.borrow()
        {
            let self_ty = ty.as_ref().expect("self should have a type");
            match &*self_ty.borrow() {
                Type::Class(name, ..) => assert_eq!(name, "Animal"),
                other => panic!("Expected Class type for self, got {:?}", other),
            }
        } else {
            panic!("Unexpected AST structure");
        }
    } else {
        panic!("Expected ClassDecl");
    }
}

#[test]
fn test_protocol_type_registered() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_super_keyword_type_in_subclass_method() {
    let code = r#"
        class Animal {}
        class Dog: Animal {
            func get_super_type() -> Animal {
                return super
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ClassDecl { body, .. } = &*program.statements[1].borrow() {
        if let Statement::FunctionDecl { body: fn_body, .. } = &*body[0].borrow()
            && let FunctionBody::Statements(statements) = &*fn_body.borrow()
            && let Statement::Return { value, .. } = &*statements[0].borrow()
            && let Some(value) = value
            && let Expression::SuperKeyword { ty, .. } = &*value.borrow()
        {
            let super_ty = ty.as_ref().expect("super should have a type");
            match &*super_ty.borrow() {
                Type::Class(name, ..) => assert_eq!(name, "Animal"),
                other => panic!("Expected Class(Animal) type for super, got {:?}", other),
            }
        } else {
            panic!("Unexpected AST structure");
        }
    } else {
        panic!("Expected ClassDecl for Dog");
    }
}

#[test]
fn test_super_keyword_without_superclass_error() {
    let code = r#"
        class Animal {
            func test() -> Void {
                let x = super
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for 'super' in class without superclass"
    );
}

#[test]
fn test_private_set_blocks_external_write() {
    let code = r#"
        struct Foo {
            private(set) var x: Int32
        }
        func test() -> Int32 {
            var f: Foo
            f.x = 10
            return f.x
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for writing to private(set) property from outside"
    );
}

#[test]
fn test_inline_private_set_blocks_external_write() {
    let code = r#"
        struct Foo {
            var x: Int32 {
                get { _x }
                private set(v) { _x = v }
            }
        }
        func test() -> Int32 {
            var f: Foo
            f.x = 10
            return f.x
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for writing to property with inline private set from outside"
    );
}

#[test]
fn test_inline_private_set_allows_internal_write() {
    let code = r#"
        struct Foo {
            var x: Int32 {
                get { _x }
                private set(v) { _x = v }
            }
            func setX(_ v: Int32) {
                x = v
            }
        }
        func test() -> Int32 {
            var f: Foo
            f.setX(10)
            return f.x
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.is_empty(),
        "Should allow writing to property with inline private set from within struct, got: {:?}",
        errors
    );
}

#[test]
fn test_subscript_inline_private_set_blocks_external_write() {
    let code = r#"
        struct Vec {
            subscript(i: Int32) -> Int32 {
                get { _v }
                private set(v) { _v = v }
            }
        }
        func test() -> Int32 {
            var v: Vec
            v[0] = 10
            return v[0]
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for subscript assignment with inline private set from outside"
    );
}

#[test]
fn test_conflicting_setter_access_on_property() {
    let code = r#"
        struct Foo {
            private(set) var x: Int32 {
                get { _x }
                public set(v) { _x = v }
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for conflicting setter access modifiers on property"
    );
}

#[test]
fn test_conflicting_setter_access_on_subscript() {
    let code = r#"
        struct Foo {
            private(set) subscript(i: Int32) -> Int32 {
                get { _v }
                public set(v) { _v = v }
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Should emit error for conflicting setter access modifiers on subscript"
    );
}

#[test]
fn test_nonconflicting_setter_access_on_property() {
    let code = r#"
        struct Foo {
            private(set) var x: Int32 {
                get { _x }
                private set(v) { _x = v }
            }
        }
        func test() -> Int32 {
            var f: Foo
            return f.x
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.is_empty(),
        "Should NOT emit error when both private(set) and private set match"
    );
}

#[test]
fn test_protocol_type_with_method() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ProtocolDecl { name, members, .. } = &*program.statements[0].borrow() {
        assert_eq!(name.value, "MyProtocol");
        assert_eq!(members.len(), 1);
        if let ProtocolMember::Method { decl, .. } = &members[0] {
            if let Statement::FunctionDecl { ty, .. } = &*decl.borrow() {
                assert!(ty.is_some());
                if let Type::Function(params, ret, _, None) = &*ty.as_ref().unwrap().borrow() {
                    assert!(params.is_empty());
                    assert_eq!(*ret.borrow(), Type::Void);
                } else {
                    panic!("Expected Function type");
                }
            } else {
                panic!("Expected FunctionDecl");
            }
        } else {
            panic!("Expected Method member");
        }
    } else {
        panic!("Expected ProtocolDecl");
    }
}

#[test]
fn test_protocol_type_with_property() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol { var name: Int32 { get } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_type_with_method_params() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol MyProtocol { func greet(name: Int32) -> Int64 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        if let ProtocolMember::Method { decl, .. } = &members[0] {
            if let Statement::FunctionDecl { ty, .. } = &*decl.borrow() {
                assert!(ty.is_some());
                if let Type::Function(params, ret, _, None) = &*ty.as_ref().unwrap().borrow() {
                    assert_eq!(params.len(), 1);
                    assert_eq!(*params[0].borrow(), type_of("Int32"));
                    assert_eq!(*ret.borrow(), type_of("Int64"));
                } else {
                    panic!("Expected Function type");
                }
            }
        }
    }
}

#[test]
fn test_protocol_type_with_default_impl() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_conformance_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Void }
             class Circle: Drawable { func draw() -> Void { return } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_struct_protocol_conformance_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Void }
             struct Circle: Drawable { func draw() -> Void { return } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_struct_multiple_protocol_conformance_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Void }
             protocol Resettable { func reset() -> Void }
             struct Circle: Drawable, Resettable {
                 func draw() -> Void { return }
                 func reset() -> Void { return }
             }"
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_refinement_type() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_protocol_any_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with 'any' type, got: {:?}",
        errors
    );

    if let Statement::VariableDecl { ty, .. } = &*program.statements[1].borrow() {
        assert!(ty.is_some(), "Variable should have a type annotation");
        let resolved = ty.as_ref().unwrap().borrow();
        assert!(
            matches!(&*resolved, Type::Protocol(name, ..) if name == "MyProtocol"),
            "Expected Protocol type, got {:?}",
            resolved
        );
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_protocol_some_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with 'some' type, got: {:?}",
        errors
    );

    if let Statement::VariableDecl { ty, .. } = &*program.statements[1].borrow() {
        assert!(ty.is_some(), "Variable should have a type annotation");
        let resolved = ty.as_ref().unwrap().borrow();
        assert!(
            matches!(&*resolved, Type::Protocol(name, ..) if name == "MyProtocol"),
            "Expected Protocol type, got {:?}",
            resolved
        );
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_some_type_non_protocol_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct S {} let x: some S".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() >= 1,
        "Should have at least one error when 'some' is used with a non-protocol type, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_compound_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with compound type, got: {:?}",
        errors
    );

    if let Statement::VariableDecl { ty, .. } = &*program.statements[2].borrow() {
        assert!(ty.is_some(), "Variable should have a type annotation");
        let resolved = ty.as_ref().unwrap().borrow();
        assert!(
            matches!(&*resolved, Type::Compound(types) if types.len() == 2),
            "Expected Compound type with 2 elements, got {:?}",
            resolved
        );
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_protocol_member_access_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Int32 } let x: any Drawable".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with protocol member access, got: {:?}",
        errors
    );

    if let Statement::VariableDecl {
        type_expression: Some(ty_expr),
        ..
    } = &*program.statements[1].borrow()
    {
        assert!(
            matches!(&*ty_expr.borrow(), Expression::AnyType { .. }),
            "Expected AnyType expression, got {:?}",
            &*ty_expr.borrow()
        );
    } else {
        panic!("Expected VariableDecl with type expression");
    }
}

#[test]
fn test_any_compound_type_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol A { func foo() -> Int32 } protocol B { func bar() -> Void } let x: any A & B"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors with 'any A & B', got: {:?}",
        errors
    );

    if let Statement::VariableDecl { ty, .. } = &*program.statements[2].borrow() {
        assert!(ty.is_some(), "Variable should have a type annotation");
        let resolved = ty.as_ref().unwrap().borrow();
        assert!(
            matches!(&*resolved, Type::Compound(types) if types.len() == 2),
            "Expected Compound type with 2 elements, got {:?}",
            resolved
        );
    } else {
        panic!("Expected VariableDecl");
    }
}

#[test]
fn test_protocol_conformance_missing_method_error() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        1,
        "Should have 1 error for missing method, got: {:?}",
        errors
    );
    assert_eq!(
        errors[0].code,
        TrussDiagnosticCode::ProtocolRequirementNotImplemented,
        "Error code should be ProtocolRequirementNotImplemented, got: {:?}",
        errors[0].code,
    );
}

#[test]
fn test_protocol_conformance_with_default_impl_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Drawable { func draw() -> Void { return } }
             struct Circle: Drawable {}"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors when method has default impl, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_conformance_struct_missing_method_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { func foo() -> Int32 }
             struct S: P {}"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        1,
        "Should have 1 error for missing method, got: {:?}",
        errors
    );
    assert_eq!(
        errors[0].code,
        TrussDiagnosticCode::ProtocolRequirementNotImplemented,
        "Error code should be ProtocolRequirementNotImplemented, got: {:?}",
        errors[0].code,
    );
}

#[test]
fn test_protocol_conformance_missing_multiple_methods_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { func foo() -> Int32
                         func bar() -> Void }
             struct S: P {}"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        2,
        "Should have 2 errors for 2 missing methods, got: {:?}",
        errors
    );
    for error in errors.iter() {
        assert_eq!(
            error.code,
            TrussDiagnosticCode::ProtocolRequirementNotImplemented,
            "Error code should be ProtocolRequirementNotImplemented, got: {:?}",
            error.code,
        );
    }
}

#[test]
fn test_extension_method_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ExtensionDecl { body, .. } = &*program.statements[1].borrow()
        && let Statement::FunctionDecl { ty, .. } = &*body[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(
            ty.borrow().clone(),
            Type::Function(vec![], Rc::new(RefCell::new(type_of("Int32"))), false, None)
        );
    } else {
        panic!("Extension method should have function type");
    }
}

#[test]
fn test_struct_copyable_conformance_no_error() {
    let code = "protocol Copyable { #[autowired] func copy() -> Self }
                 struct MyStruct: Copyable {}";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    assert_eq!(engine.borrow().get_errors().len(), 0);
}

#[test]
fn test_unsupported_autowired_protocol_method_error() {
    let code = "protocol Foo { #[autowired] func bar() -> Int32 }
                 struct MyStruct: Foo {}";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.len() > 0, "Unsupported autowired should error");
}

#[test]
fn test_internal_used_function_call_warning() {
    let code = "#[internalUsed] public func internalHelper() -> Int32 { return 42 }
                func caller() -> Int32 { return internalHelper() }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let warnings = engine_ref.get_warnings();
    assert!(
        warnings.len() > 0,
        "Should emit warning for internalUsed function call"
    );
    assert_eq!(
        warnings[0].code,
        TrussDiagnosticCode::InternalUsedReferenced
    );
}

#[test]
fn test_extension_self_in_return_type() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Point { let x: Int32 } extension Point { func getSelf() -> Self { self } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for Self return type, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_struct_method_call_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Calc {} extension Calc { func add(a: Int32, b: Int32) -> Int32 { a + b } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_extension_static_method_struct_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Should not have errors for static method in struct extension, got: {:?}",
        errors
    );

    if let Statement::ExtensionDecl { body, .. } = &*program.statements[1].borrow()
        && let Statement::FunctionDecl {
            ty, static_method, ..
        } = &*body[0].borrow()
        && let Some(ty) = ty
    {
        assert!(static_method);
        assert_eq!(
            ty.borrow().clone(),
            Type::Function(vec![], Rc::new(RefCell::new(type_of("Int32"))), false, None)
        );
    } else {
        panic!("Static method should have function type");
    }
}

#[test]
fn test_extension_static_method_class_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

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
fn test_extension_static_method_enum_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

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
fn test_extension_static_method_protocol_type_resolved() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

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
fn test_extension_with_type_arguments_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Wrapper<T> {} protocol Computable { func compute() -> Int32 } extension Wrapper<Int32>: Computable { func compute() -> Int32 { 42 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Extension with type arguments should type-check without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_extension_type_arguments_override_generic_param() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Wrapper<T> { func identity(x: T) -> T { x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Simple generic struct with method should type-check, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_conformance_with_type_params() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Protocol conformance with type params should resolve, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_function_type_param_resolves() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic function type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_struct_type_param_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Stack<Element> { var items: Element }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic struct type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_class_type_param_resolves() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic class type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_enum_type_param_resolves() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Generic enum type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_typealias_in_struct_type_resolves() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Typealias type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_protocol_with_associatedtype_type_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item; func get() -> Item }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let _ = engine.borrow();
}

#[test]
fn test_if_case_dot_shorthand_type_resolved() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) {
            if case .Some(val) = x {
                let _: Int32 = val
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "dot shorthand should resolve type correctly, got: {:?}",
        errors
    );
}

#[test]
fn test_guard_case_type_check() {
    let code = r#"
        enum Option { case None case Some(Int32) }
        func test(x: Option) -> Int32 {
            guard case .Some(val) = x else { return 0 }
            return val
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "guard case should type check correctly, got: {:?}",
        errors
    );
}

#[test]
fn test_match_type_check() {
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
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "match should type check correctly, got: {:?}",
        errors
    );
}
#[test]
fn test_typealias_in_protocol_type_resolves() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Typealias in protocol type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_typealias_at_top_level_type_resolves() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Top-level typealias type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_generic_function_call_with_type_inference() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(x: Int32) -> Int32 { x } func bar() -> Int32 { foo(x: 1) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Normal function call type resolution, got: {:?}",
        errors
    );
}

#[test]
fn test_associated_type_access_on_protocol_resolves() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Associated type access on protocol, got: {:?}",
        errors
    );
}

#[test]
fn test_associated_type_access_typealias_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { typealias Item = Int32 } func foo(x: P.Item) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Associated type access with typealias, got: {:?}",
        errors
    );
}

#[test]
fn test_struct_typealias_access_resolves() {
    let errors = run_type_check(
        "struct Wrapper { typealias MyInt = Int32 }
         func test(x: Wrapper.MyInt) -> Int32 { return x }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for struct typealias access, got: {:?}",
        errors
    );
}

#[test]
fn test_struct_typealias_access_in_variable() {
    let errors = run_type_check(
        "struct Wrapper { typealias MyInt = Int32 }
         func test() -> Wrapper.MyInt { return 42 }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for struct typealias access in return type, got: {:?}",
        errors
    );
}

#[test]
fn test_class_typealias_access_resolves() {
    let errors = run_type_check(
        "class Wrapper { typealias MyInt = Int32 }
         func test(x: Wrapper.MyInt) -> Int32 { return x }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for class typealias access, got: {:?}",
        errors
    );
}

#[test]
fn test_associated_type_access_missing_member_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol P { associatedtype Item } func foo(x: P.Nonexistent) {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() > 0,
        "Expected error for missing associated type"
    );
}

#[test]
fn test_defer_body_type_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { let x = 1 defer { let y = x } return x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
}

#[test]
fn test_defer_body_type_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { defer { let x: UndefinedType = 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.len() > 0, "Expected type errors in defer body");
}

#[test]
fn test_implicit_return_last_expression() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for implicit return with Int32, got: {:?}",
        errors
    );
}

#[test]
fn test_implicit_return_type_mismatch() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Bool { 42 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        errors.len() > 0,
        "Expected type mismatch error for implicit return"
    );
    assert!(
        errors[0].message.contains("Type mismatch")
            || errors[0].code == TrussDiagnosticCode::TypeMismatch
    );
}

#[test]
fn test_implicit_return_if_expression() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { if true { 1 } else { 2 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for implicit return with if, got: {:?}",
        errors
    );
}

#[test]
fn test_implicit_return_void_function_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let x = 1 x }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for void function with trailing expression, got: {:?}",
        errors
    );
}

#[test]
fn test_implicit_return_int_literal_context() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int64 { 42 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for Int64 context, got: {:?}",
        errors
    );
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
        && let Expression::IntegerLiteral { ty, .. } = &*expression.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), type_of("Int64"));
    } else {
        panic!("Expected integer literal with Int64 type");
    }
}

#[test]
fn test_match_multi_pattern_type_check() {
    let code = r#"
        enum Status { case idle case loading case done }
        func test(s: Status) -> Bool {
            match s {
                case .idle, .loading:
                    true
                case .done:
                    false
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "multi-pattern match should type check correctly, got: {:?}",
        errors
    );
}

#[test]
fn test_match_multi_pattern_with_guard_type_check() {
    let code = r#"
        enum Status { case idle case loading case done }
        func test(s: Status) -> Bool {
            match s {
                case .idle, .loading where true:
                    true
                default:
                    false
            }
        }
    "#;
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "multi-pattern match with guard should type check correctly, got: {:?}",
        errors
    );
}

#[test]
fn test_module_func_body_type_resolved() {
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
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let module_id = krate.borrow().modules.get("foo").cloned().unwrap();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "module func body should type check, got: {:?}",
        errors
    );
}

#[test]
fn test_module_return_type_resolved() {
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
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let module_id = krate.borrow().modules.get("foo").cloned().unwrap();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "module func calling another func should type check, got: {:?}",
        errors
    );
}

#[test]
fn test_module_variable_decl_type_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo { func bar() -> Int32 { let a: Int32 = 42 return a } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let module_id = krate.borrow().modules.get("foo").cloned().unwrap();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "module variable decl should type check, got: {:?}",
        errors
    );
}

#[test]
fn test_nested_module_type_resolved() {
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
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let module_id = krate.borrow().modules.get("foo.bar").cloned().unwrap();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "nested module func should type check, got: {:?}",
        errors
    );
}

#[test]
fn test_empty_module_no_type_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("module foo { }".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    resolver.resolve(&program, "test".to_string());
    let module_id = krate.borrow().modules.get("foo").cloned().unwrap();
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "empty module should type check without errors"
    );
}

#[test]
fn test_module_with_full_pipeline_no_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "module foo { func add(_ a: Int32, _ b: Int32) -> Int32 { a + b } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let root_module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, root_module);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "full pipeline with module should resolve, got: {:?}",
        errors
    );
}

#[test]
fn test_overloaded_function_call_selects_correct_overload() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(x: Int32) -> Int32 { x } func foo(y: Float64) -> Float64 { y } func caller() -> Float64 { foo(y: 3.0) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "overloaded call should resolve, got: {:?}",
        errors
    );

    let caller_stmt = program.statements[2].borrow();
    if let Statement::FunctionDecl { body, .. } = &*caller_stmt {
        if let FunctionBody::Statements(stmts) = &*body.borrow() {
            if let Statement::ExpressionStatement { expression } = &*stmts[0].borrow() {
                let expr = expression.borrow();
                if let Expression::Call { selected_index, .. } = &*expr {
                    assert_eq!(
                        *selected_index,
                        Some(1),
                        "should select foo(y: Float64) overload"
                    );
                } else {
                    panic!("Expected Call expression");
                }
            }
        }
    }
}

#[test]
fn test_overloaded_function_call_no_match_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func foo(x: Int32) -> Int32 { x } func foo(y: Float64) -> Float64 { y } func caller() -> Int32 { foo(x: true) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert!(
        !errors.is_empty(),
        "expected NoMatchingOverload error for bool arg to overloaded foo"
    );
}

#[test]
fn test_overloaded_struct_method_call_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct S { func foo(x: Int32) -> Int32 { x } func foo(y: Float64) -> Float64 { y } } func test() -> Float64 { let s = S() return s.foo(y: 3.0) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "overloaded struct method call should resolve, got: {:?}",
        errors
    );

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow() {
        if let FunctionBody::Statements(stmts) = &*body.borrow()
            && let Statement::ExpressionStatement { expression } = &*stmts[1].borrow()
            && let Expression::Call { selected_index, .. } = &*expression.borrow()
        {
            assert_eq!(*selected_index, Some(1), "should select foo(y: Float64)");
        }
    }
}

#[test]
fn test_overloaded_struct_method_call_no_match_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct S { func foo(x: Int32) -> Int32 { x } func foo(y: Float64) -> Float64 { y } } func test() -> Int32 { let s = S() return s.foo(x: true) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    let errors = binding.get_errors();
    assert!(
        !errors.is_empty(),
        "expected NoMatchingOverload error for bool arg to overloaded method"
    );
}

fn run_type_check(code: &str) -> usize {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    binding.get_errors().len()
}

#[test]
fn test_import_module_call() {
    let errors = run_type_check(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo
         func test() -> Int32 { return Foo.bar() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for module import call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_wildcard_call() {
    let errors = run_type_check(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.*
         func test() -> Int32 { return bar() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for wildcard import call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_member_call() {
    let errors = run_type_check(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import Foo.Bar.baz
         func test() -> Int32 { return baz() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for member import call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_nested_module_call() {
    let errors = run_type_check(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import Foo
         func test() -> Int32 { return Foo.Bar.baz() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for nested module call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_module_not_found_error() {
    let errors = run_type_check(
        "import NonExistent
         func test() -> Int32 { return 42 }",
    );
    assert!(errors > 0, "Expected error for non-existent module import");
}

#[test]
fn test_import_deep_nested_call() {
    let errors = run_type_check(
        "module A { module B { module C { func foo() -> Int32 { return 1 } } } }
         import A
         func test() -> Int32 { return A.B.C.foo() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for deep nested call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_package_module_call() {
    let errors = run_type_check(
        "module Foo { func bar() -> Int32 { return 42 } }
         import package.Foo
         func test() -> Int32 { return Foo.bar() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for package module import call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_package_wildcard_call() {
    let errors = run_type_check(
        "module Foo { func bar() -> Int32 { return 42 } }
         import package.Foo.*
         func test() -> Int32 { return bar() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for package wildcard import call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_package_member_call() {
    let errors = run_type_check(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import package.Foo.Bar.baz
         func test() -> Int32 { return baz() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for package member import call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_package_nested_module_call() {
    let errors = run_type_check(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import package.Foo
         func test() -> Int32 { return Foo.Bar.baz() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for package nested module call, got: {:?}",
        errors
    );
}

#[test]
fn test_import_package_deep_nested_call() {
    let errors = run_type_check(
        "module A { module B { module C { func foo() -> Int32 { return 1 } } } }
         import package.A
         func test() -> Int32 { return A.B.C.foo() }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for package deep nested call, got: {:?}",
        errors
    );
}

fn run_type_check_var_in_func(code: &str, func_name: &str, var_name: &str) -> Type {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    for stmt in &program.statements {
        if let Statement::FunctionDecl { name, body, .. } = &*stmt.borrow()
            && name.value == func_name
            && let FunctionBody::Statements(statements) = &*body.borrow()
        {
            for s in statements {
                if let Statement::VariableDecl { name, ty, .. } = &*s.borrow()
                    && name.value == var_name
                {
                    return ty.as_ref().unwrap().borrow().clone();
                }
            }
        }
    }
    panic!("Variable {} not found in function {}", var_name, func_name);
}

#[test]
fn test_generic_function_infer_from_arg() {
    let ty = run_type_check_var_in_func(
        "func identity<T>(x: T) -> T { return x }
         func test() -> Int32 { let a = identity(42) return a }",
        "test",
        "a",
    );
    assert_eq!(
        ty,
        type_of("Int32"),
        "Variable 'a' = identity(42) should be Int32, got {}",
        ty
    );
}

#[test]
fn test_generic_function_infer_from_bool_arg() {
    let ty = run_type_check_var_in_func(
        "func identity<T>(x: T) -> T { return x }
         func test() -> Bool { let a = identity(true) return a }",
        "test",
        "a",
    );
    assert_eq!(
        ty,
        type_of("Bool"),
        "Variable 'a' = identity(true) should be Bool, got {}",
        ty
    );
}

#[test]
fn test_generic_function_explicit_type_arg() {
    let ty = run_type_check_var_in_func(
        "func identity<T>(x: T) -> T { return x }
         func test() -> Int32 { let a = identity<Int32>(42) return a }",
        "test",
        "a",
    );
    assert_eq!(
        ty,
        type_of("Int32"),
        "Variable 'a' = identity<Int32>(42) should be Int32, got {}",
        ty
    );
}

#[test]
fn test_generic_function_type_mismatch_error() {
    let errors = run_type_check(
        "func same<T>(x: T, y: T) -> T { return x }
         func test() -> Int32 { return same(42, true) }",
    );
    assert!(
        errors > 0,
        "same(42, true) should produce a type mismatch error since T cannot be both Int32 and Bool"
    );
}

#[test]
fn test_generic_function_variable_decl_annotation() {
    let ty = run_type_check_var_in_func(
        "func identity<T>(x: T) -> T { return x }
         func test() -> Int32 { let a: Int32 = identity(42) return a }",
        "test",
        "a",
    );
    assert_eq!(
        ty,
        type_of("Int32"),
        "Variable 'a' from identity(42) annotated as Int32 should be Int32, got {}",
        ty
    );
}

#[test]
fn test_private_struct_field_inaccessible_from_outside() {
    let errors = run_type_check(
        "struct Foo { private let x: Int32 }
         func test(f: Foo) -> Int32 { return f.x }",
    );
    assert!(
        errors > 0,
        "Expected InaccessibleMember error for private field access from outside, got 0 errors"
    );
}

#[test]
fn test_private_struct_method_inaccessible_from_outside() {
    let errors = run_type_check(
        "struct Foo { private func foo() -> Int32 { return 42 } }
         func test(f: Foo) -> Int32 { return f.foo() }",
    );
    assert!(
        errors > 0,
        "Expected InaccessibleMember error for private method access from outside, got 0 errors"
    );
}

#[test]
fn test_public_struct_field_accessible_from_outside() {
    let errors = run_type_check(
        "struct Foo { public let x: Int32 }
         func test(f: Foo) -> Int32 { return f.x }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for public field access, got {}",
        errors
    );
}

#[test]
fn test_internal_struct_field_accessible_from_outside() {
    let errors = run_type_check(
        "struct Foo { let x: Int32 }
         func test(f: Foo) -> Int32 { return f.x }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for default internal field access, got {}",
        errors
    );
}

#[test]
fn test_public_struct_field_accessible_from_outside_2() {
    let errors = run_type_check(
        "struct Foo { public let x: Int32 }
         func test(f: Foo) -> Int32 { return f.x }",
    );
    assert_eq!(
        errors, 0,
        "Expected no errors for public field access, got {}",
        errors
    );
}

#[test]
fn test_closure_type_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { (x: Int32, y: Int32) -> Int32 in x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure { ty, parameters, .. } = &*init.borrow()
    {
        assert!(ty.is_some(), "Closure should have a type");
        let t = ty.as_ref().unwrap().borrow().clone();
        if let Type::Function(param_types, ret_type, is_vararg, None) = t {
            assert_eq!(param_types.len(), 2);
            assert_eq!(*param_types[0].borrow(), type_of("Int32"));
            assert_eq!(*param_types[1].borrow(), type_of("Int32"));
            assert_eq!(*ret_type.borrow(), type_of("Int32"));
            assert!(!is_vararg);
        } else {
            panic!("Expected Type::Function for closure, got {:?}", t);
        }
        assert_eq!(parameters.len(), 2);
    } else {
        panic!("Expected closure with type");
    }
}

#[test]
fn test_closure_no_params_no_return_type() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure { ty, .. } = &*init.borrow()
    {
        assert!(ty.is_some(), "Empty closure should have a type");
        let t = ty.as_ref().unwrap().borrow().clone();
        if let Type::Function(param_types, ret_type, is_vararg, None) = t {
            assert_eq!(param_types.len(), 0);
            assert_eq!(*ret_type.borrow(), Type::Void);
            assert!(!is_vararg);
        } else {
            panic!("Expected Type::Function for closure, got {:?}", t);
        }
    } else {
        panic!("Expected closure with type");
    }
}

#[test]
fn test_closure_body_variable_type_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { (x: Int32) -> Int32 in x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
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
        assert!(
            scope.is_some(),
            "Closure should have a scope from SymbolResolver"
        );
        if let Statement::ExpressionStatement { expression } = &*closure_body[0].borrow()
            && let Expression::Variable { name, ty, .. } = &*expression.borrow()
        {
            assert_eq!(name.value, "x");
            assert!(
                ty.is_some(),
                "Variable 'x' should have its type set by TypeResolver"
            );
            let t = ty.as_ref().unwrap().borrow().clone();
            assert_eq!(t, type_of("Int32"));
        } else {
            panic!("Expected closure body variable with type");
        }
    } else {
        panic!("Expected closure with scope");
    }
}

#[test]
fn test_function_type_expression_resolved() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f: (Int32) -> Bool }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl {
            type_expression, ..
        } = &*statements[0].borrow()
        && let Some(te) = type_expression
        && let Expression::FunctionType { ty, .. } = &*te.borrow()
    {
        assert!(ty.is_some(), "FunctionType should have a resolved type");
        let t = ty.as_ref().unwrap().borrow().clone();
        if let Type::Function(param_types, ret_type, is_vararg, None) = t {
            assert_eq!(param_types.len(), 1);
            assert_eq!(*param_types[0].borrow(), type_of("Int32"));
            assert_eq!(*ret_type.borrow(), type_of("Bool"));
            assert!(!is_vararg);
        } else {
            panic!("Expected Type::Function for function type, got {:?}", t);
        }
    } else {
        panic!("Expected FunctionType with resolved type");
    }
}

#[test]
fn test_closure_untyped_params_inferred() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f = { (x, y) in x } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure { ty, .. } = &*init.borrow()
    {
        assert!(ty.is_some(), "Closure should have a type");
        let t = ty.as_ref().unwrap().borrow().clone();
        if let Type::Function(param_types, ret_type, is_vararg, None) = t {
            assert_eq!(param_types.len(), 2);
            assert_eq!(*ret_type.borrow(), Type::Void);
            assert!(!is_vararg);
        } else {
            panic!("Expected Type::Function for closure, got {:?}", t);
        }
    } else {
        panic!("Expected closure with type");
    }
}

#[test]
fn test_closure_shorthand_type_default() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Expected error about missing type context for shorthand arg"
    );
    drop(engine_ref);
}

#[test]
fn test_closure_shorthand_type_binary() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(
        !errors.is_empty(),
        "Expected error about missing type context for shorthand args"
    );
    drop(engine_ref);
}

#[test]
fn test_closure_shorthand_with_context() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f: (Int32) -> Int32 = { $0 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let error_count = engine.borrow().get_errors().len();
    assert_eq!(error_count, 0, "Expected no errors");
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure { ty, .. } = &*init.borrow()
    {
        assert!(ty.is_some(), "Closure should have a type");
        let t = ty.as_ref().unwrap().borrow().clone();
        if let Type::Function(param_types, ret_type, _, None) = t {
            assert_eq!(param_types.len(), 1);
            assert_eq!(*param_types[0].borrow(), type_of("Int32"));
            assert_eq!(*ret_type.borrow(), type_of("Int32"));
        } else {
            panic!("Expected Type::Function for closure, got {:?}", t);
        }
    }
}

#[test]
fn test_closure_shorthand_binary_with_context() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let f: (Int32, Int32) -> Int32 = { $0 + $1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let error_count = engine.borrow().get_errors().len();
    assert_eq!(error_count, 0, "Expected no errors");
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { initializer, .. } = &*statements[0].borrow()
        && let Some(init) = initializer
        && let Expression::Closure { ty, .. } = &*init.borrow()
    {
        assert!(ty.is_some(), "Closure should have a type");
        let t = ty.as_ref().unwrap().borrow().clone();
        if let Type::Function(param_types, ret_type, _, None) = t {
            assert_eq!(param_types.len(), 2);
            assert_eq!(*param_types[0].borrow(), type_of("Int32"));
            assert_eq!(*param_types[1].borrow(), type_of("Int32"));
            assert_eq!(*ret_type.borrow(), type_of("Int32"));
        } else {
            panic!("Expected Type::Function for closure, got {:?}", t);
        }
    }
}

#[test]
fn test_assign_to_let_variable_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a = 1; a = 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        1,
        "Expected 1 error for assign to let, got {:?}",
        errors
    );
    let diag = &errors[0];
    assert_eq!(diag.code, TrussDiagnosticCode::AssignToImmutable);
}

#[test]
fn test_assign_to_var_variable_ok() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { var a = 1; a = 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected 0 errors for assign to var, got {:?}",
        errors
    );
}

#[test]
fn test_assign_to_uninited_let_variable_ok() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Int32; a = 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected 0 errors for first assign to uninited let, got {:?}",
        errors
    );
}

#[test]
fn test_double_assign_to_let_variable_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a: Int32; a = 1; a = 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        1,
        "Expected 1 error for double assign to let, got {:?}",
        errors
    );
    let diag = &errors[0];
    assert_eq!(diag.code, TrussDiagnosticCode::AssignToImmutable);
}

#[test]
fn test_inc_on_let_variable_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { let a = 1; a++ }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let assign_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.code == TrussDiagnosticCode::AssignToImmutable)
        .collect();
    assert_eq!(
        assign_errors.len(),
        1,
        "Expected 1 AssignToImmutable error for inc on let, got {:?}",
        errors
    );
}

#[test]
fn test_inc_on_var_variable_ok() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() { var a = 1; a++ }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected 0 errors for inc on var, got {:?}",
        errors
    );
}

#[test]
fn test_assign_to_let_property_outside_init_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { let x: Int32 }\nfunc test() { let f = Foo(x: 1); f.x = 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let assign_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.code == TrussDiagnosticCode::AssignToImmutable)
        .collect();
    assert_eq!(
        assign_errors.len(),
        1,
        "Expected 1 AssignToImmutable error for let property assign outside init, got {:?}",
        errors
    );
}

#[test]
fn test_assign_to_var_property_ok() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { var x: Int32 }\nfunc test() { let f = Foo(x: 1); f.x = 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let assign_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.code == TrussDiagnosticCode::AssignToImmutable)
        .collect();
    assert_eq!(
        assign_errors.len(),
        0,
        "Expected 0 AssignToImmutable for var property assign, got {:?}",
        errors
    );
}

#[test]
fn test_open_on_struct_member_errors() {
    let errors = run_type_check("struct Foo { open var x: Int32 }");
    assert_eq!(
        errors, 1,
        "Expected 1 error for open on struct member, got {}",
        errors
    );
}

#[test]
fn test_open_on_class_member_ok() {
    let errors = run_type_check("class Foo { open var x: Int32 }");
    assert_eq!(
        errors, 0,
        "Expected 0 errors for open on class member, got {}",
        errors
    );
}

#[test]
fn test_member_access_exceeds_container_level() {
    let errors = run_type_check("internal struct Foo { public var x: Int32 }");
    assert_eq!(
        errors, 1,
        "Expected 1 error for member access exceeding container, got {}",
        errors
    );
}

#[test]
fn test_member_access_equal_to_container_ok() {
    let errors = run_type_check("public struct Foo { public var x: Int32 }");
    assert_eq!(
        errors, 0,
        "Expected 0 errors for member access equal to container, got {}",
        errors
    );
}

#[test]
fn test_member_access_more_restrictive_ok() {
    let errors = run_type_check("public struct Foo { private var x: Int32 }");
    assert_eq!(
        errors, 0,
        "Expected 0 errors for member access more restrictive than container, got {}",
        errors
    );
}

#[test]
fn test_address_of_variable_type() {
    let code = "func test(v: Int32) -> Int32* { return &v }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for &v, got: {:?}",
        errors
    );
}

#[test]
fn test_address_of_deref_type() {
    let code = "func test(p: Int32*) -> Int32* { return &*p }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for &*p, got: {:?}",
        errors
    );
}

#[test]
fn test_struct_subscript_return_type() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Matrix { subscript(row: Int32, col: Int32) -> Int32 { get { return 0 } } }
             func test(m: Matrix) -> Int32 { return m[0, 1] }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Struct subscript should resolve, got: {:?}",
        errors
    );
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::Return {
            value: Some(ret_val),
            ..
        } = &*stmts[0].borrow()
        && let Expression::SubscriptAccess {
            ty: Some(ref_ty), ..
        } = &*ret_val.borrow()
    {
        assert_eq!(
            *ref_ty.borrow(),
            type_of("Int32"),
            "Subscript should return Int32"
        );
    } else {
        panic!("Expected subscript access with Int32 return type");
    }
}

#[test]
fn test_struct_subscript_implicit_get() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Array { subscript(index: Int32) -> Int32 { return 42 } }
             func test(a: Array) -> Int32 { return a[0] }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Implicit get subscript should resolve, got: {:?}",
        errors
    );
}

#[test]
fn test_class_subscript_return_type() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(
            "class MyArray { subscript(index: Int32) -> Int32 { get { return 0 } set { } } }
             func test(a: MyArray) -> Int32 { return a[0] }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Class subscript should resolve, got: {:?}",
        errors
    );
    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(stmts) = &*body.borrow()
        && let Statement::Return {
            value: Some(ret_val),
            ..
        } = &*stmts[0].borrow()
        && let Expression::SubscriptAccess {
            ty: Some(ref_ty), ..
        } = &*ret_val.borrow()
    {
        assert_eq!(
            *ref_ty.borrow(),
            type_of("Int32"),
            "Class subscript should return Int32"
        );
    } else {
        panic!("Expected subscript access with Int32 return type");
    }
}

#[test]
fn test_binary_operator_method_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyInt { var value: Int32; static func + (left: MyInt, right: MyInt) -> MyInt { return left } }"
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Static operator method declaration should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_binary_operator_method_member_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyInt { var value: Int32; func + (other: MyInt) -> MyInt { return self } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Member operator method declaration should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_binary_operator_method_call_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyInt { var value: Int32; static func + (left: MyInt, right: MyInt) -> MyInt { return left } }
             func test(a: MyInt, b: MyInt) -> MyInt { return a + b }"
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Binary operator method call should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_prefix_operator_method_call_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyInt { var value: Int32; static prefix func - (value: MyInt) -> MyInt { return value } }
             func test(a: MyInt) -> MyInt { return -a }"
            .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Prefix operator method call should resolve without errors, got: {:?}",
        errors
    );
}

#[test]
fn test_member_binary_operator_method_call_resolves() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct MyInt { var value: Int32; func + (other: MyInt) -> MyInt { return self } }
             func test(a: MyInt, b: MyInt) -> MyInt { return a + b }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Member binary operator method call should resolve without errors, got: {:?}",
        errors
    );
}

fn type_check(
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
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    (engine, program.statements)
}

#[test]
fn test_conditional_block_type_resolved() {
    let (engine, stmts) =
        type_check("#if DEBUG\nfunc foo() -> Int32 { 1 }\n#endif\nfunc bar() -> Int32 { 2 }");
    assert!(engine.borrow().get_errors().is_empty());
    assert_eq!(stmts.len(), 2);
}

#[test]
fn test_conditional_block_with_else_type_resolved() {
    let (engine, _stmts) = type_check("#if A\nlet x: Int32 = 1\n#else\nlet y: Int32 = 1\n#endif");
    assert!(engine.borrow().get_errors().is_empty());
}

#[test]
fn test_conditional_block_nested_type_resolved() {
    let (engine, _stmts) =
        type_check("#if A\nlet x: Int32 = 1\n#if B\nlet y: Int32 = 2\n#endif\n#endif");
    assert!(engine.borrow().get_errors().is_empty());
}

#[test]
fn test_pragma_directives_type_resolved() {
    let (engine, stmts) = type_check("#error \"test\"\n#warning \"test\"\nlet x: Int32 = 1");
    assert!(engine.borrow().get_errors().is_empty());
    assert_eq!(stmts.len(), 3);
}

#[test]
fn test_conditional_block_function_type_resolved() {
    let (engine, _stmts) =
        type_check("#if A\nfunc foo(x: Int32) -> Int32 { x }\n#endif\nlet y: Int32 = foo(x: 1)");
    let eng = engine.borrow();
    let errors = eng.get_errors();
    assert!(
        errors.is_empty(),
        "Conditional block function type resolve failed: {:?}",
        errors
    );
}

#[test]
fn test_sizeof_expression_type_resolved() {
    let (engine, stmts) = type_check("func test() -> UInt64 { return sizeof(Int32) }");
    let eng = engine.borrow();
    assert!(
        eng.get_errors().is_empty(),
        "SizeOf type resolve failed: {:?}",
        eng.get_errors()
    );
    drop(eng);
    if let Statement::FunctionDecl { body, .. } = &*stmts[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        let Statement::Return { value, .. } = &*statements[0].borrow() else {
            panic!("Expected return statement");
        };
        let value = value.clone().unwrap();
        assert!(matches!(*value.borrow(), Expression::SizeOf { .. }));
        if let Expression::SizeOf { argument, ty, .. } = &*value.borrow() {
            let resolved_ty = ty.clone().unwrap();
            assert_eq!(*resolved_ty.borrow(), type_of("UInt64"));
            if let Expression::Type {
                name, ty: arg_ty, ..
            } = &*argument.borrow()
            {
                assert_eq!(name.value, "Int32");
                let arg_resolved = arg_ty.clone().unwrap();
                assert_eq!(*arg_resolved.borrow(), type_of("Int32"));
            } else {
                panic!("Expected Type expression as sizeof argument");
            }
        }
    } else {
        panic!("Expected function decl");
    }
}

#[test]
fn test_asm_block_no_operands_type_check() {
    let errors = run_type_check(r#"func test() { asm { "nop" } }"#);
    assert_eq!(
        errors, 0,
        "asm block without operands should type-check without errors"
    );
}

#[test]
fn test_asm_block_with_operands_type_check() {
    let errors =
        run_type_check(r#"func test() { var x: Int32 = 10; asm { "nop" : : val = in(reg) x } }"#);
    assert_eq!(
        errors, 0,
        "asm block with input operand should type-check without errors"
    );
}

#[test]
fn test_asm_block_output_type_check() {
    let errors = run_type_check(
        r#"func test() { var x: Int32 = 0; asm { "mov {dst}, 42" : dst = out(reg) x } }"#,
    );
    assert_eq!(
        errors, 0,
        "asm block with output operand should type-check without errors"
    );
}

#[test]
fn test_asm_block_full_type_check() {
    let errors = run_type_check(
        r#"func test() { var x: Int32 = 0; asm { "add {dst}, {src}" : dst = out(reg) x : src = in(reg) 42 } }"#,
    );
    assert_eq!(
        errors, 0,
        "asm block with full operands should type-check without errors"
    );
}

fn run_type_check_do(code: &str) -> Type {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        for stmt in statements.iter().rev() {
            if let Statement::Return {
                value: Some(value), ..
            } = &*stmt.borrow()
            {
                if let Expression::Do { ty, .. } = &*value.borrow() {
                    return ty.as_ref().unwrap().borrow().clone();
                }
            }
            if let Statement::VariableDecl { initializer, .. } = &*stmt.borrow()
                && let Some(init) = initializer
                && let Expression::Do { ty, .. } = &*init.borrow()
            {
                return ty.as_ref().unwrap().borrow().clone();
            }
        }
    }
    panic!("No do expression type found");
}

#[test]
fn test_do_expression_type_void() {
    let errors = run_type_check("func test() { do {} }");
    assert_eq!(errors, 0, "do with empty body should type-check");
}

#[test]
fn test_do_expression_type_int() {
    let errors = run_type_check("func test() { let x = do { 1 } }");
    assert_eq!(errors, 0, "do with int literal should type-check");
}

#[test]
fn test_do_expression_variable_scope_type() {
    let ty = run_type_check_do("func test() { let x = do { let a = 1; a } }");
    assert_eq!(
        ty,
        type_of("Int32"),
        "do expression should infer Int32 from its body's last expression"
    );
}

#[test]
fn test_nested_do_expression_type() {
    let ty = run_type_check_do("func test() { let x = do { do { 1 } } }");
    assert_eq!(
        ty,
        type_of("Int32"),
        "nested do expressions should both resolve to Int32"
    );
}

#[test]
fn test_yield_in_function_type_check() {
    let errors = run_type_check("func test() -> Int32 { yield 42 }");
    assert_eq!(
        errors, 0,
        "yield Int32 in function returning Int32 should be valid"
    );
}

#[test]
fn test_yield_in_function_type_mismatch() {
    let errors = run_type_check("func test() -> Bool { yield 42 }");
    assert_eq!(
        errors, 1,
        "yield Int32 in function returning Bool should error"
    );
}

#[test]
fn test_non_null_pointer_type_annotation() {
    let code = "func test() { let p: Int32*! }";
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert!(
            matches!(ty.borrow().clone(), Type::NonNullPointer(inner) if matches!(*inner.borrow(), Type::Struct(ref n, _, _) if n == "Int32"))
        );
    } else {
        panic!("Expected variable declaration with non-null pointer type");
    }
}

#[test]
fn test_non_null_pointer_deref() {
    let errors = run_type_check("func test(p: Int32*!) -> Int32 { return *p }");
    assert_eq!(errors, 0, "dereference of non-null pointer should work");
}

#[test]
fn test_nullptr_rejected_for_non_null_pointer() {
    let errors = run_type_check("func test() { let p: Int32*! = nullptr }");
    assert_eq!(
        errors, 2,
        "assigning nullptr to non-null pointer should error"
    );
}

#[test]
fn test_non_null_ptr_to_ptr_conversion() {
    let errors = run_type_check("func foo(p: Int32*) {} func bar(q: Int32*!) { foo(p: q) }");
    assert_eq!(
        errors, 0,
        "passing non-null pointer to nullable pointer parameter should be allowed"
    );
}

#[test]
fn test_nullptr_in_return_type_non_null_error() {
    let errors = run_type_check("func test() -> Int32*! { return nullptr }");
    assert_eq!(
        errors, 1,
        "returning nullptr for non-null return type should error"
    );
}

#[test]
fn test_non_null_pointer_nullptr_init_regular_ptr() {
    let errors = run_type_check("func test() { let p: Int32* = nullptr }");
    assert_eq!(
        errors, 0,
        "assigning nullptr to regular pointer should be allowed"
    );
}

#[test]
fn test_yield_in_do_expression_type() {
    let errors = run_type_check("func test() -> Int32 { let x = do { yield 42 }; return x }");
    assert_eq!(
        errors, 0,
        "yield in do expression should produce correct type"
    );
}

#[test]
fn test_yield_in_if_branch_type() {
    let errors = run_type_check(
        "func test() -> Int32 { let x = if true { yield 42 } else { 10 }; return x }",
    );
    assert_eq!(errors, 0, "yield in if branch should produce correct type");
}

#[test]
fn test_yield_outside_function_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x = yield 42".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    assert!(
        engine.borrow().has_errors(),
        "yield at top level should error"
    );
}

#[test]
fn test_yield_to_void_function_no_value() {
    let errors = run_type_check("func test() { yield }");
    assert_eq!(
        errors, 0,
        "yield without value in void function should be valid"
    );
}

#[test]
fn test_yield_to_nonvoid_function_no_value_error() {
    let errors = run_type_check("func test() -> Int32 { yield }");
    assert_eq!(
        errors, 1,
        "yield without value in non-void function should error"
    );
}

#[test]
fn test_inline_type_class() {
    let errors = run_type_check("class Dog {} func test() -> Int32 { let _: inline Dog return 1 }");
    assert_eq!(errors, 0, "inline with class type should succeed");
}

#[test]
fn test_inline_type_class_with_size() {
    let errors =
        run_type_check("class Dog {} func test() -> Int32 { let _: inline<256> Dog return 1 }");
    assert_eq!(errors, 0, "inline<256> with class type should succeed");
}

#[test]
fn test_inline_type_empty_brackets() {
    let errors =
        run_type_check("class Dog {} func test() -> Int32 { let _: inline<> Dog return 1 }");
    assert_eq!(errors, 0, "inline<> with class type should succeed");
}

#[test]
fn test_inline_type_not_class_error() {
    let errors = run_type_check("func test() -> Int32 { let _: inline Int32 return 1 }");
    assert!(errors > 0, "inline with non-class type should error");
}

#[test]
fn test_const_generic_function_decl_resolves() {
    let errors = run_type_check("func foo<let N: Int32>(x: Int32) -> Int32 { return x }");
    assert_eq!(errors, 0, "const generic function decl should resolve");
}

#[test]
fn test_const_generic_struct_decl_resolves() {
    let errors = run_type_check(
        "struct Buffer<let N: Int32> { var data: Int32 }
         func test() -> Int32 { return 0 }",
    );
    assert_eq!(errors, 0, "const generic struct decl should resolve");
}

#[test]
fn test_const_generic_mixed_type_param_resolves() {
    let errors = run_type_check(
        "struct Pair<T, let N: Int32> { var first: T; var second: T }
         func test() -> Int32 { return 0 }",
    );
    assert_eq!(errors, 0, "mixed generic params should resolve");
}

#[test]
fn test_const_generic_missing_type_error() {
    let errors = run_type_check("func foo<let N>(x: Int32) -> Int32 { return x }");
    assert!(errors > 0, "missing const type should error");
}

#[test]
fn test_labeled_param_reorder_success() {
    let errors = run_type_check(
        "func f(a a: Int64, b b: Int64) -> Int64 { return a + b }
         func f2() { let r = f(b: 2, a: 1) }",
    );
    assert_eq!(errors, 0, "labeled params in any order should succeed");
}

#[test]
fn test_default_param_value_used() {
    let errors = run_type_check(
        "func foo(a: Int32 = 5) -> Int32 { return a }
         func bar() -> Int32 { return foo() }",
    );
    assert_eq!(errors, 0, "default param value should be used");
}

#[test]
fn test_default_param_with_label() {
    let errors = run_type_check(
        "func foo(from a: Int32 = 0, to b: Int32) -> Int32 { return a + b }
         func bar() -> Int32 { return foo(to: 1) }",
    );
    assert_eq!(errors, 0, "default param with label should work");
}

#[test]
fn test_default_param_out_of_order() {
    let errors = run_type_check(
        "func foo(from a: Int32 = 0, to b: Int32, by c: Int32 = 1) -> Int32 { return a + b + c }
         func bar() -> Int32 { return foo(by: 3, to: 2) }",
    );
    assert_eq!(
        errors, 0,
        "default params with out-of-order labels should work"
    );
}

#[test]
fn test_missing_required_param_error() {
    let errors = run_type_check(
        "func foo(a: Int32, b: Int32 = 0) -> Int32 { return a }
         func bar() -> Int32 { return foo() }",
    );
    assert!(errors > 0, "missing required param should produce error");
}

#[test]
fn test_default_value_wrong_type_error() {
    let errors = run_type_check(
        "func foo(a: Bool = 42) -> Bool { return a }
         func bar() -> Bool { return foo() }",
    );
    assert!(
        errors > 0,
        "default value with wrong type should produce error"
    );
}

#[test]
fn test_default_param_with_unlabeled() {
    let errors = run_type_check(
        "func foo(_ a: Int32 = 0, b: Int32) -> Int32 { return a + b }
         func bar() -> Int32 { return foo(b: 1) }",
    );
    assert_eq!(
        errors, 0,
        "unlabeled default param with labeled param should work"
    );
}

#[test]
fn test_function_call_missing_label_still_errors() {
    let errors = run_type_check("func f(a a: Int64) -> Int64 { return a } func f2() { f(1) }");
    assert!(errors > 0, "missing label for required param should error");
}

fn run_type_check_with_stdlib(code: &str, stdlib_decls: &[&str]) -> usize {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let tokens = lexer.parse();
    let mut parser = Parser::new(lexer.get_file(), tokens, engine.clone());
    let program = parser.parse();

    let mut packages: HashMap<String, Rc<RefCell<Package>>> = HashMap::new();
    let test_pkg = Rc::new(RefCell::new(Package::new("test".to_string())));
    packages.insert("test".to_string(), test_pkg.clone());
    let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
    packages.insert("Truss".to_string(), truss_pkg.clone());

    let combined_src = stdlib_decls.join("\n");
    if !combined_src.is_empty() {
        let mut std_lexer = Lexer::new(
            CharStream::new(combined_src, Rc::new("".to_string())),
            engine.clone(),
        );
        let std_tokens = std_lexer.parse();
        let mut std_parser = Parser::new(std_lexer.get_file(), std_tokens, engine.clone());
        let std_program = std_parser.parse();
        let mut std_resolver =
            SymbolResolver::new(packages.clone(), "Truss".to_string(), engine.clone());
        std_resolver.resolve(&std_program, "Truss".to_string());
        let truss_module = truss_pkg.borrow().modules.get("Truss").cloned().unwrap();
        let mut std_type_resolver =
            TypeResolver::new(packages.clone(), "Truss".to_string(), engine.clone());
        std_type_resolver.resolve(&std_program, truss_module);
    }

    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());

    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let binding = engine.borrow();
    binding.get_errors().len()
}

#[test]
fn test_auto_import_builtin_types() {
    let errors = run_type_check_with_stdlib(
        "func test() -> Int32 { return 42 }",
        &["#[builtintype] public struct Int32 {}"],
    );
    assert_eq!(errors, 0, "auto-imported Int32 should resolve");
}

#[test]
fn test_auto_import_struct_type() {
    let errors = run_type_check_with_stdlib(
        "func test() -> MyStruct { return MyStruct() }",
        &["public struct MyStruct { public init() {} }"],
    );
    assert_eq!(errors, 0, "auto-imported struct should resolve");
}

#[test]
fn test_auto_import_protocol_type() {
    let errors = run_type_check_with_stdlib(
        "func test(x: Number) -> Number { return x }",
        &["public protocol Number {}"],
    );
    assert_eq!(errors, 0, "auto-imported protocol should resolve");
}

#[test]
fn test_string_literal_type() {
    let errors = run_type_check("func test() { let s = \"hello\" }");
    assert_eq!(errors, 0, "string literal should type-check");
}

#[test]
fn test_null_with_optional_type() {
    let errors = run_type_check_with_stdlib(
        "func test() -> Optional<Box> { return null }",
        &[
            "public struct Box {}",
            "public enum Optional<T> { case None, Some(T) }",
        ],
    );
    assert_eq!(errors, 0, "null should work as Optional.None");
}

#[test]
fn test_failable_init_return_null() {
    let errors = run_type_check_with_stdlib(
        "struct Point { init?() { return null } }",
        &["public enum Optional<T> { case None, Some(T) }"],
    );
    assert_eq!(errors, 0, "return null should work in failable init");
}

#[test]
fn test_failable_init_call_returns_optional() {
    let errors = run_type_check_with_stdlib(
        "struct Point { init?() {} }
         func test() -> Optional<Point> { return Point() }",
        &["public enum Optional<T> { case None, Some(T) }"],
    );
    assert_eq!(
        errors, 0,
        "calling failable init should produce Optional type"
    );
}

#[test]
fn test_non_failable_init_return_null_error() {
    let errors = run_type_check_with_stdlib(
        "struct Point { init() { return null } }",
        &["public enum Optional<T> { case None, Some(T) }"],
    );
    assert!(errors > 0, "return null should error in non-failable init");
}

#[test]
fn test_subscript_public_accessible() {
    let errors = run_type_check(
        "struct Array { public subscript(index: Int32) -> Int32 { return 42 } }
         func test(a: Array) -> Int32 { return a[0] }",
    );
    assert_eq!(errors, 0, "public subscript should be accessible");
}

#[test]
fn test_subscript_private_inaccessible() {
    let errors = run_type_check(
        "struct Array { private subscript(index: Int32) -> Int32 { return 42 } }
         func test(a: Array) -> Int32 { return a[0] }",
    );
    assert!(
        errors > 0,
        "private subscript should be inaccessible from outside"
    );
}

#[test]
fn test_subscript_fileprivate_same_file_accessible() {
    let errors = run_type_check(
        "struct Array { fileprivate subscript(index: Int32) -> Int32 { return 42 } }
         func test(a: Array) -> Int32 { return a[0] }",
    );
    assert_eq!(
        errors, 0,
        "fileprivate subscript should be accessible in same file"
    );
}

#[test]
fn test_subscript_private_set_allows_read() {
    let errors = run_type_check(
        "struct Array { private(set) subscript(index: Int32) -> Int32 { get { return 42 } set { } } }
         func test(a: Array) -> Int32 { return a[0] }",
    );
    assert_eq!(errors, 0, "private(set) subscript should allow reading");
}

#[test]
fn test_subscript_private_set_disallows_write() {
    let errors = run_type_check(
        "struct Array { private(set) subscript(index: Int32) -> Int32 { get { return 42 } set { } } }
         func test(a: Array) { a[0] = 1 }",
    );
    assert!(
        errors > 0,
        "private(set) subscript should disallow writing from outside"
    );
}

#[test]
fn test_subscript_internal_default_accessible() {
    let errors = run_type_check(
        "struct Array { subscript(index: Int32) -> Int32 { return 42 } }
         func test(a: Array) -> Int32 { return a[0] }",
    );
    assert_eq!(
        errors, 0,
        "default (internal) subscript should be accessible"
    );
}

#[test]
fn test_array_literal_integer_elements() {
    let errors = run_type_check("func test() { let a = [1, 2, 3] }");
    assert_eq!(
        errors, 0,
        "array literal with integer elements should type-check"
    );
}

#[test]
fn test_array_literal_empty() {
    let errors = run_type_check("func test() { let a = [] }");
    assert_eq!(errors, 0, "empty array literal should type-check");
}

#[test]
fn test_array_literal_bool_elements() {
    let errors = run_type_check("func test() { let a = [true, false] }");
    assert_eq!(
        errors, 0,
        "array literal with bool elements should type-check"
    );
}

#[test]
fn test_array_literal_single_element() {
    let errors = run_type_check("func test() { let a = [42] }");
    assert_eq!(errors, 0, "single-element array literal should type-check");
}

#[test]
fn test_array_literal_nested() {
    let errors = run_type_check("func test() { let a = [[1], [2]] }");
    assert_eq!(errors, 0, "nested array literal should type-check");
}

#[test]
fn test_array_literal_as_variable() {
    let errors = run_type_check("func test() { let x = [1, 2, 3]; let y = x }");
    assert_eq!(
        errors, 0,
        "array literal assigned to variable should type-check"
    );
}

#[test]
fn test_array_literal_stdlib() {
    let errors = run_type_check_with_stdlib(
        "func test() { let a: Array<Int32> = [1, 2, 3] }",
        &[
            "#[builtintype] public struct Int32 {}",
            "public class Array<T> { public init() {} var count: UInt64 }",
        ],
    );
    assert_eq!(
        errors, 0,
        "array literal with explicit Array<Int32> annotation should type-check"
    );
}

#[test]
fn test_self_type_constructor_in_struct() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo {
    var x: Int32
    init(x: Int32) { self.x = x }
    func clone() -> Foo { Self(x: self.x) }
}
func test() -> Foo {
    let f = Foo(x: 42)
    f.clone()
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    assert!(
        !engine.borrow().has_errors(),
        "SymbolResolver should have no errors"
    );
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    assert!(
        !engine.borrow().has_errors(),
        "TypeResolver should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_self_type_constructor_in_class() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Foo {
    var x: Int32
    init(x: Int32) { self.x = x }
    func clone() -> Foo { Self(x: self.x) }
}
func test() -> Foo {
    let f = Foo(x: 42)
    f.clone()
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    assert!(
        !engine.borrow().has_errors(),
        "SymbolResolver should have no errors"
    );
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    assert!(
        !engine.borrow().has_errors(),
        "TypeResolver should have no errors: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_optional_type_display_shows_generic_params() {
    let ty = Type::Enum(
        "Optional".to_string(),
        WeakSymbol(std::rc::Weak::new()),
        vec![Rc::new(RefCell::new(type_of("Int32")))],
    );
    assert_eq!(format!("{}", ty), "Optional<Int32>");
}

#[test]
fn test_array_type_display_shows_generic_params() {
    let ty = Type::Struct(
        "Array".to_string(),
        WeakSymbol(std::rc::Weak::new()),
        vec![Rc::new(RefCell::new(type_of("Int32")))],
    );
    assert_eq!(format!("{}", ty), "Array<Int32>");
}

#[test]
fn test_non_generic_type_display_no_params() {
    let ty = Type::Struct(
        "Point".to_string(),
        WeakSymbol(std::rc::Weak::new()),
        vec![],
    );
    assert_eq!(format!("{}", ty), "Point");
}

#[test]
fn test_optional_type_sugar_in_type_resolver_no_crash() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: Int32? = 10".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

#[test]
fn test_override_missing_modifier_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Base { func foo() -> Int32 { return 1 } }
             class Derived: Base { func foo() -> Int32 { return 2 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let has_error = engine
        .borrow()
        .get_diagnostics()
        .iter()
        .any(|d| d.code == TrussDiagnosticCode::MissingOverrideModifier);
    assert!(has_error, "Should report missing override modifier");
}

#[test]
fn test_override_correct_modifier_no_error() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let has_missing = engine.borrow().get_diagnostics().iter().any(|d| {
        d.code == TrussDiagnosticCode::MissingOverrideModifier
            || d.code == TrussDiagnosticCode::OverrideWithoutOverride
    });
    assert!(
        !has_missing,
        "Should not report override errors when override modifier is used"
    );
}

#[test]
fn test_override_without_override_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Base { func foo() -> Int32 { return 1 } }
             class Unrelated: Base { override func bar() -> Int32 { return 2 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let has_error = engine
        .borrow()
        .get_diagnostics()
        .iter()
        .any(|d| d.code == TrussDiagnosticCode::OverrideWithoutOverride);
    assert!(has_error, "Should report override without purpose");
}

#[test]
fn test_override_final_method_error() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Base { final func foo() -> Int32 { return 1 } }
             class Derived: Base { override func foo() -> Int32 { return 2 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let has_error = engine
        .borrow()
        .get_diagnostics()
        .iter()
        .any(|d| d.code == TrussDiagnosticCode::CannotOverrideFinal);
    assert!(has_error, "Should report cannot override final method");
}

#[test]
fn test_override_in_final_class_no_override_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "class Base { func foo() -> Int32 { return 1 } }
             final class Derived: Base { override func foo() -> Int32 { return 2 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let errors: Vec<_> = {
        let engine_ref = engine.borrow();
        engine_ref
            .get_diagnostics()
            .iter()
            .filter(|d| {
                d.code == TrussDiagnosticCode::MissingOverrideModifier
                    || d.code == TrussDiagnosticCode::OverrideWithoutOverride
                    || d.code == TrussDiagnosticCode::CannotOverrideFinal
            })
            .map(|d| format!("{:?}", d.code))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        errors.len(),
        0,
        "Final class with override should not produce override errors: {:?}",
        errors
    );
}

#[test]
fn test_override_computed_property_no_error() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let errors: Vec<_> = {
        let engine_ref = engine.borrow();
        engine_ref
            .get_diagnostics()
            .iter()
            .filter(|d| {
                d.code == TrussDiagnosticCode::MissingOverrideModifier
                    || d.code == TrussDiagnosticCode::OverrideWithoutOverride
            })
            .map(|d| format!("{:?}", d.code))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        errors.len(),
        0,
        "Should not report override errors for correct property override: {:?}",
        errors
    );
}

#[test]
fn test_array_type_sugar_in_type_resolver_no_crash() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new("let x: [Int32] = {}".to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

// --- Exception handling type resolver tests ---

#[test]
fn test_throw_type_check_no_crash() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() throws { throw MyError.someCase }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

#[test]
fn test_throws_function_type_has_throws_field() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() throws -> Int32 { return 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { ty, .. } = &*program.statements[0].borrow() {
        let fn_type = ty.as_ref().unwrap().borrow();
        if let Type::Function(_, _, _, throws) = &*fn_type {
            assert!(throws.is_some(), "throws function should have Some throws");
        } else {
            panic!("Expected Type::Function");
        }
    }
}

#[test]
fn test_non_throws_function_type_no_throws() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { return 1 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { ty, .. } = &*program.statements[0].borrow() {
        let fn_type = ty.as_ref().unwrap().borrow();
        if let Type::Function(_, _, _, throws) = &*fn_type {
            assert!(
                throws.is_none(),
                "non-throws function should have None throws"
            );
        } else {
            panic!("Expected Type::Function");
        }
    }
}

#[test]
fn test_try_expression_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() throws -> Int32 { return try foo() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

#[test]
fn test_do_catch_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() throws { do { try foo() } catch { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

#[test]
fn test_do_catch_finally_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() throws { do { try foo() } catch { } finally { } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

#[test]
fn test_try_force_expression_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32 { return try! foo() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

#[test]
fn test_try_optional_expression_type_check() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "func test() -> Int32? { return try? foo() }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine);
    type_resolver.resolve(&program, module_id);
}

#[test]
fn test_protocol_throws_function_type() {
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
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        if let ProtocolMember::Method { decl, .. } = &members[0] {
            if let Statement::FunctionDecl { ty, .. } = &*decl.borrow() {
                let fn_type = ty.as_ref().unwrap().borrow();
                if let Type::Function(_, _, _, throws) = &*fn_type {
                    assert!(
                        throws.is_some(),
                        "Protocol throws method should have Some throws"
                    );
                } else {
                    panic!("Expected Type::Function");
                }
            }
        }
    }
}

#[test]
fn test_implicit_member_dot_enum_case_no_data() {
    let engine = create_engine();
    let code = r#"
enum TargetKind {
    case Executable
    case DynamicLibrary(Int32)
}

func getKind() -> TargetKind {
    .Executable
}
"#;
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Parser errors: {:?}", engine.borrow().get_errors());
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors(), "Symbol resolver errors: {:?}", engine.borrow().get_errors());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    assert!(!engine_ref.has_errors(), "Type resolver errors: {:?}", engine_ref.get_errors());

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
    {
        let expr = expression.borrow();
        match &*expr {
            Expression::ImplicitMemberAccess { member, ty } => {
                assert_eq!(member.value, "Executable");
                assert!(ty.is_some(), "ImplicitMemberAccess should have a type");
                if let Some(t) = ty {
                    assert_eq!(
                        t.borrow().to_string(),
                        "TargetKind",
                        "Type should be TargetKind enum"
                    );
                }
            }
            other => panic!("Expected ImplicitMemberAccess, got {:?}", other),
        }
    } else {
        panic!("Expected FunctionDecl with ExpressionStatement");
    }
}

#[test]
fn test_implicit_member_dot_enum_case_with_data() {
    let engine = create_engine();
    let code = r#"
enum TargetKind {
    case Executable
    case DynamicLibrary(Int32)
}

func getLib() -> TargetKind {
    .DynamicLibrary(42)
}
"#;
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Parser errors: {:?}", engine.borrow().get_errors());
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors(), "Symbol resolver errors: {:?}", engine.borrow().get_errors());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    assert!(!engine_ref.has_errors(), "Type resolver errors for .DynamicLibrary(\"test\"): {:?}", engine_ref.get_errors());

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::ExpressionStatement { expression } = &*statements[0].borrow()
    {
        let expr = expression.borrow();
        match &*expr {
            Expression::Call { callee, parameters, .. } => {
                let callee_expr = callee.borrow();
                match &*callee_expr {
                    Expression::ImplicitMemberAccess { member, ty } => {
                        assert_eq!(member.value, "DynamicLibrary");
                        assert!(ty.is_some(), "ImplicitMemberAccess callee should have a type");
                        if let Some(t) = ty {
                            assert!(
                                t.borrow().to_string().contains("Function"),
                                "Callee with data should have Function type, got: {}",
                                t.borrow()
                            );
                        }
                    }
                    other => panic!("Expected ImplicitMemberAccess as callee, got {:?}", other),
                }
            }
            other => panic!("Expected Call expression with ImplicitMemberAccess callee, got {:?}", other),
        }
    } else {
        panic!("Expected FunctionDecl with ExpressionStatement");
    }
}

#[test]
fn test_protocol_get_set_accessor_var_passes() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Settable { var value: Int32 { get set } }
             struct MyStruct: Settable { var value: Int32 }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "var property should satisfy {{ get set }}, got: {:?}", errors);
}

#[test]
fn test_protocol_get_set_accessor_let_fails() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Settable { var value: Int32 { get set } }
             struct MyStruct: Settable { let value: Int32 }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty(), "let property should NOT satisfy {{ get set }}");
    let found = errors.iter().any(|e| {
        e.code == TrussDiagnosticCode::ProtocolRequirementNotImplemented
            && e.message.contains("get set")
    });
    assert!(found, "Expected ProtocolRequirementNotImplemented about {{ get set }}, got: {:?}", errors);
}

#[test]
fn test_protocol_get_accessor_let_passes() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "protocol Readable { var value: Int32 { get } }
             struct MyStruct: Readable { let value: Int32 }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let (packages, _) = truss::krate::single_package_map("test");
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "{{ get }} should be satisfied by let property, got: {:?}", errors);
}

#[test]
fn test_mutating_method_can_assign_to_self_var_property() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { var x: Int32; mutating func foo() { self.x = 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Parser errors: {:?}", engine.borrow().get_errors());
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    assert!(!engine.borrow().has_errors(), "SymbolRes errors: {:?}", engine.borrow().get_errors());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let mutating_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.code == TrussDiagnosticCode::AssignToSelfInNonMutating)
        .collect();
    assert_eq!(
        mutating_errors.len(),
        0,
        "Expected 0 AssignToSelfInNonMutating errors for mutating method, got ALL errors: {:?}",
        errors
    );
}

#[test]
fn test_nonmutating_method_cannot_assign_to_self_var_property() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { var x: Int32; func foo() { self.x = 1 } }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let mutating_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.code == TrussDiagnosticCode::AssignToSelfInNonMutating)
        .collect();
    assert_eq!(
        mutating_errors.len(),
        1,
        "Expected 1 AssignToSelfInNonMutating error for non-mutating method, got {:?}",
        errors
    );
}

#[test]
fn test_nonmutating_method_can_assign_to_other_var_property() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { var x: Int32 }
             func test() { let f = Foo(x: 1); f.x = 2 }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let mutating_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.code == TrussDiagnosticCode::AssignToSelfInNonMutating)
        .collect();
    assert_eq!(
        mutating_errors.len(),
        0,
        "Expected 0 AssignToSelfInNonMutating errors for non-self var property, got {:?}",
        errors
    );
}

#[test]
fn test_mutating_on_free_function_errors() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "mutating func foo() {}".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    let modifier_errors: Vec<_> = errors
        .iter()
        .filter(|d| d.code == TrussDiagnosticCode::ModifierNotAllowedHere)
        .collect();
    assert_eq!(
        modifier_errors.len(),
        1,
        "Expected 1 ModifierNotAllowedHere error for mutating on free function, got {:?}",
        errors
    );
}

#[test]
fn test_self_init_delegation_in_init() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { let x: Int32; let y: Int32; init(x: Int32, y: Int32) { self.x = x; self.y = y } init(x: Int32) { self.init(x: x, y: 0) } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Parser errors: {:?}", engine.borrow().get_errors());
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        0,
        errors.len(),
        "Expected no errors for self.init() delegation, got: {:?}",
        errors
    );
}

#[test]
fn test_self_init_delegation_overload_resolution() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { let x: Int32; let y: Int32; init(x: Int32, y: Int32) { self.x = x; self.y = y } init(x: Int32) { self.init(x: x, y: 0) } init() { self.init(x: 10, y: 20) } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Parser errors: {:?}", engine.borrow().get_errors());
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        0,
        errors.len(),
        "Expected no errors for overloaded self.init(), got: {:?}",
        errors
    );
}

#[test]
fn test_init_call_with_param_count_overload() {
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(
            "struct Foo { let x: Int32; let y: Int32; init(x: Int32, y: Int32) { self.x = x; self.y = y } init(x: Int32) { self.x = x; self.y = 0 } }"
                .to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    assert!(!engine.borrow().has_errors(), "Parser errors: {:?}", engine.borrow().get_errors());
    let (packages, _krate) = truss::krate::single_package_map("test");
    let mut resolver = SymbolResolver::new(packages.clone(), "test".to_string(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(packages.clone(), "test".to_string(), engine.clone());
    type_resolver.resolve(&program, module);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        0,
        errors.len(),
        "Expected no errors for Foo(x: 10) overload resolution, got: {:?}",
        errors
    );
}
