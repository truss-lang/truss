use std::{cell::RefCell, rc::Rc};

use inkwell::context::Context;
use truss::{
    diag::TrussDiagnosticEngine,
    id::CrateId,
    ir_gen::IRGenerator,
    krate::Crate,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    type_resolver::TypeResolver,
};

fn create_engine() -> Rc<RefCell<TrussDiagnosticEngine>> {
    Rc::new(RefCell::new(TrussDiagnosticEngine::new()))
}

#[test]
fn test_irgen_nullptr_literal() {
    let code = "func test() -> Void* { return nullptr }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program);
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("test"));
    assert!(llvm_ir.contains("ptr"));
}

#[test]
fn test_irgen_pointer_parameter() {
    let code = "func test(p: Int32*) -> Int32 { return *p }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program);
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("test"));
    assert!(llvm_ir.contains("load"));
}

#[test]
fn test_irgen_deref_in_variable() {
    let code = "func test(p: Int32*) -> Int32 { let val = *p; return val }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program);
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("test"));
    assert!(llvm_ir.contains("load"));
}

#[test]
fn test_irgen_void_pointer_variable() {
    let code = "func test() { var p: Void* = nullptr }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new(
        "test".to_string(),
        CrateId { id: 0 },
    )));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program);
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("test"));
}
