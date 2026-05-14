use std::{cell::RefCell, rc::Rc};

use truss::{
    ast::{
        expression::Expression,
        statement::{FunctionBody, Statement},
    },
    id::CrateId,
    krate::Crate,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    type_resolver::TypeResolver,
    types::Type,
};

#[test]
fn test_infer_variable_decl() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Int32 { let a = 1 return a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Int32 { let a: Int32 = 1 return a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Bool { return true }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Int32 { let a = 1 let b = a + 1 return b }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Int32 = 42".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Bool { let a: Bool = true return a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
#[should_panic(expected = "Type mismatch")]
fn test_type_annotation_mismatch() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Int32 { let a: Bool = 1 return a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
}

#[test]
fn test_never_type_annotation() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test()->Never { let a: Never return a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
    let mut lexer = Lexer::new(CharStream::new(
        "func test(_ a: Int32)->Int32 { return a }".to_string(),
        Rc::new("".to_string()),
    ));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
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
    let mut lexer = Lexer::new(CharStream::new(code.to_string(), Rc::new("".to_string())));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();

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
    let mut lexer = Lexer::new(CharStream::new(code.to_string(), Rc::new("".to_string())));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();

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
    let mut lexer = Lexer::new(CharStream::new(code.to_string(), Rc::new("".to_string())));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();

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
    // Currently unable to infer type through unary expression
    // let ty = run_type_check_var("func test() { let a: Int8 = -128 }", "a");
    // assert_eq!(ty, Type::Int8);
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
#[should_panic(expected = "Type mismatch")]
fn test_type_mismatch_int_float() {
    let code = "func test() -> Int32 { let a: Float64 = 3.14 return a }";
    let mut lexer = Lexer::new(CharStream::new(code.to_string(), Rc::new("".to_string())));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
}

#[test]
#[should_panic(expected = "Type mismatch")]
fn test_type_mismatch_different_int_sizes() {
    let code = "func test() -> Int32 { let a: Int64 = 42 return a }";
    let mut lexer = Lexer::new(CharStream::new(code.to_string(), Rc::new("".to_string())));
    let mut parser = Parser::new(lexer.get_file(), lexer.parse());
    let program = parser.parse().unwrap();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone());
    let module_id = symbol_resolver
        .resolve(&program, "test".to_string())
        .unwrap();
    let mut type_resolver = TypeResolver::new(krate.clone());
    type_resolver.resolve(&program, module_id).unwrap();
}
