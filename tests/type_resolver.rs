use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::Expression,
        statement::{FunctionBody, ProtocolMember, Statement},
    },
    diag::{TrussDiagnosticCode, TrussDiagnosticEngine},
    krate::Crate,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    type_resolver::TypeResolver,
    types::Type,
};

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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int32);
        } else {
            panic!("VariableDecl should have Int32 type");
        }
        if let Statement::Return { value, .. } = &*statements[1].borrow()
            && let Some(value) = value
            && let Expression::Variable { ty, .. } = &*value.borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[1].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Expression(expr) = &*body.borrow()
        && let Expression::IntegerLiteral { ty, .. } = &*expr.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Bool);
        } else {
            panic!("VariableDecl should have Bool type");
        }
        if let Statement::Return { value, .. } = &*statements[1].borrow()
            && let Some(value) = value
            && let Expression::Variable { ty, .. } = &*value.borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Bool);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);
    if let Statement::FunctionDecl {
        parameters, body, ..
    } = &*program.statements[0].borrow()
    {
        if let Some(ty) = &parameters[0].borrow().ty {
            assert_eq!(ty.borrow().clone(), Type::Int32);
        } else {
            panic!("Parameter should have Int32 type");
        }
        if let FunctionBody::Statements(statements) = &*body.borrow()
            && let Statement::Return { value, .. } = &*statements[0].borrow()
            && let Some(value) = value
            && let Expression::Variable { ty, .. } = &*value.borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
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
                    Expression::BooleanLiteral { .. } => return Type::Bool,
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
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
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_zero() {
    let ty = run_type_check_with_return("func test() -> Int32 { return 0 }");
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_negative_integer() {
    let ty = run_type_check_with_return("func test() -> Int32 { return -100 }");
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_large_positive_integer() {
    let ty = run_type_check_with_return("func test() -> Int64 { return 5000000000 }");
    assert_eq!(ty, Type::Int64);
}

#[test]
fn test_very_large_integer() {
    let ty = run_type_check_with_return("func test() -> Int128 { return 20000000000000000000 }");
    assert_eq!(ty, Type::Int128);
}

#[test]
fn test_int8_annotation() {
    let ty = run_type_check_var("func test() { let a: Int8 = 100 }", "a");
    assert_eq!(ty, Type::Int8);
}

#[test]
fn test_int16_annotation() {
    let ty = run_type_check_var("func test() { let a: Int16 = 10000 }", "a");
    assert_eq!(ty, Type::Int16);
}

#[test]
fn test_int32_annotation() {
    let ty = run_type_check_var("func test() { let a: Int32 = 100000 }", "a");
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_int64_annotation() {
    let ty = run_type_check_var("func test() { let a: Int64 = 10000000000 }", "a");
    assert_eq!(ty, Type::Int64);
}

#[test]
fn test_int128_annotation() {
    let ty = run_type_check_var("func test() { let a: Int128 = 10000000000000000000 }", "a");
    assert_eq!(ty, Type::Int128);
}

#[test]
fn test_uint8_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt8 = 200 }", "a");
    assert_eq!(ty, Type::UInt8);
}

#[test]
fn test_uint16_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt16 = 50000 }", "a");
    assert_eq!(ty, Type::UInt16);
}

#[test]
fn test_uint32_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt32 = 3000000000 }", "a");
    assert_eq!(ty, Type::UInt32);
}

#[test]
fn test_uint64_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt64 = 100000000000000 }", "a");
    assert_eq!(ty, Type::UInt64);
}

#[test]
fn test_uint128_annotation() {
    let ty = run_type_check_var("func test() { let a: UInt128 = 10000000000000000000 }", "a");
    assert_eq!(ty, Type::UInt128);
}

#[test]
fn test_float_literal_default() {
    let ty = run_type_check_with_return("func test() -> Float64 { return 3.14 }");
    assert_eq!(ty, Type::Float64);
}

#[test]
fn test_float_with_exponent() {
    let ty = run_type_check_with_return("func test() -> Float64 { return 1.5e10 }");
    assert_eq!(ty, Type::Float64);
}

#[test]
fn test_float32_annotation() {
    let ty = run_type_check_var("func test() { let a: Float32 = 3.14 }", "a");
    assert_eq!(ty, Type::Float32);
}

#[test]
fn test_float64_annotation() {
    let ty = run_type_check_var("func test() { let a: Float64 = 3.14159265358979 }", "a");
    assert_eq!(ty, Type::Float64);
}

#[test]
fn test_return_type_context_int32() {
    let ty = run_type_check_with_return("func test() -> Int32 { return 42 }");
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_return_type_context_int64() {
    let ty = run_type_check_with_return("func test() -> Int64 { return 42 }");
    assert_eq!(ty, Type::Int64);
}

#[test]
fn test_return_type_context_uint32() {
    let ty = run_type_check_with_return("func test() -> UInt32 { return 42 }");
    assert_eq!(ty, Type::UInt32);
}

#[test]
fn test_return_type_context_float32() {
    let ty = run_type_check_with_return("func test() -> Float32 { return 3.14 }");
    assert_eq!(ty, Type::Float32);
}

#[test]
fn test_return_type_context_float64() {
    let ty = run_type_check_with_return("func test() -> Float64 { return 3.14 }");
    assert_eq!(ty, Type::Float64);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { parameters, .. } = &*program.statements[0].borrow() {
        let param_ty = parameters[0].borrow().ty.as_ref().unwrap().borrow().clone();
        assert_eq!(param_ty, Type::Int64);
    } else {
        panic!("Expected FunctionDecl");
    }
}

#[test]
fn test_binary_expression_same_type() {
    let ty = run_type_check_with_return("func test() -> Int32 { let a = 10 + 20 return a }");
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_unary_expression() {
    let ty = run_type_check_with_return("func test() -> Int32 { let a = -42 return a }");
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_variable_propagation() {
    let ty = run_type_check_with_return("func test() -> Int32 { let a = 42 return a }");
    assert_eq!(ty, Type::Int32);
}

#[test]
fn test_i8_max() {
    let ty = run_type_check_var("func test() { let a: Int8 = 127 }", "a");
    assert_eq!(ty, Type::Int8);
}

#[test]
fn test_i8_min() {
    let ty = run_type_check_var("func test() { let a: Int8 = -128 }", "a");
    assert_eq!(ty, Type::Int8);
}

#[test]
fn test_u8_max() {
    let ty = run_type_check_var("func test() { let a: UInt8 = 255 }", "a");
    assert_eq!(ty, Type::UInt8);
}

#[test]
fn test_u16_max() {
    let ty = run_type_check_var("func test() { let a: UInt16 = 65535 }", "a");
    assert_eq!(ty, Type::UInt16);
}

#[test]
fn test_u32_max() {
    let ty = run_type_check_var("func test() { let a: UInt32 = 4294967295 }", "a");
    assert_eq!(ty, Type::UInt32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int8);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int64);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Float32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine);
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[1].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
    {
        if let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
            && let Some(ty) = ty
        {
            assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(!errors.is_empty());
    assert_eq!(errors[0].code, TrussDiagnosticCode::ArgumentLabelMismatch);
    assert!(
        errors[0]
            .message
            .contains("Expected argument label 'a' but found 'b'")
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert!(
            matches!(ty.borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Int32))
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());

    if let Statement::FunctionDecl { return_type, .. } = &*program.statements[0].borrow()
        && let Some(return_type) = return_type
        && let Expression::Type { ty, .. } = &*return_type.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.is_empty());

    if let Statement::FunctionDecl { return_type, .. } = &*program.statements[0].borrow()
        && let Some(return_type) = return_type
        && let Expression::Type { ty, .. } = &*return_type.borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Int64);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
            matches!(ty.borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Int32))
        );
        if let Expression::NullptrLiteral { ty: init_ty, .. } = &*init.borrow() {
            assert!(init_ty.is_some());
            assert!(
                matches!(init_ty.as_ref().unwrap().borrow().clone(), Type::Pointer(inner) if matches!(*inner.borrow(), Type::Int32))
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
        assert_eq!(ty.borrow().clone(), Type::Float64);
        if let Expression::Cast { ty: cast_ty, .. } = &*init.borrow() {
            assert!(cast_ty.is_some());
            assert_eq!(cast_ty.as_ref().unwrap().borrow().clone(), Type::Float64);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Int64);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0);

    if let Statement::FunctionDecl { body, .. } = &*program.statements[0].borrow()
        && let FunctionBody::Statements(statements) = &*body.borrow()
        && let Statement::VariableDecl { ty, .. } = &*statements[0].borrow()
        && let Some(ty) = ty
    {
        assert_eq!(ty.borrow().clone(), Type::Int32);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::StructDecl { body, .. } = &*program.statements[0].borrow() {
        assert_eq!(body.len(), 3);
        if let Statement::InitDecl { ty, .. } = &*body[1].borrow() {
            assert!(ty.is_some());
            let fn_type = ty.as_ref().unwrap().borrow().clone();
            if let Type::Function(param_tys, ret_ty, is_vararg) = &fn_type {
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
            if let Type::Function(param_tys, ret_ty, is_vararg) = &fn_type {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_type_resolved() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.some(val) = x {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_binding_type_inference() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.some(val) = x {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_no_bindings_type() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.none = x {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
}

#[test]
fn test_if_case_enum_type_not_found() {
    let code = r#"
        func test(x: Int32) {
            if case Option.some(val) = x {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
        enum Option { case none case some(Int32) }
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.some(val) = x {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
                Type::Struct(name, _) => assert_eq!(name, "Point"),
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
                Type::Class(name, _) => assert_eq!(name, "Animal"),
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
                if let Type::Function(params, ret, _) = &*ty.as_ref().unwrap().borrow() {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    if let Statement::ProtocolDecl { members, .. } = &*program.statements[0].borrow() {
        if let ProtocolMember::Method { decl, .. } = &members[0] {
            if let Statement::FunctionDecl { ty, .. } = &*decl.borrow() {
                assert!(ty.is_some());
                if let Type::Function(params, ret, _) = &*ty.as_ref().unwrap().borrow() {
                    assert_eq!(params.len(), 1);
                    assert_eq!(*params[0].borrow(), Type::Int32);
                    assert_eq!(*ret.borrow(), Type::Int64);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
            matches!(&*resolved, Type::Protocol(name, _) if name == "MyProtocol"),
            "Expected Protocol type, got {:?}",
            resolved
        );
    } else {
        panic!("Expected VariableDecl");
    }
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
            Type::Function(vec![], Rc::new(RefCell::new(Type::Int32)), false)
        );
    } else {
        panic!("Extension method should have function type");
    }
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
    type_resolver.resolve(&program, module);

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
            "enum Option<T> { case none case some(T) }".to_string(),
            Rc::new("".to_string()),
        ),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
    type_resolver.resolve(&program, module_id);
    let _ = engine.borrow();
}

#[test]
fn test_if_case_dot_shorthand_type_resolved() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case .some(val) = x {
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
        enum Option { case none case some(Int32) }
        func test(x: Option) -> Int32 {
            guard case .some(val) = x else { return 0 }
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate, engine.clone());
    type_resolver.resolve(&program, module_id);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert!(errors.len() > 0, "Expected type errors in defer body");
}
