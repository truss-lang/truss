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
fn test_annotated_param_type() {
    let mut lexer = Lexer::new(CharStream::new(
        "func test(a: Int32)->Int32 { return a }".to_string(),
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
