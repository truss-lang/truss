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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("test"));
}

#[test]
fn test_irgen_cast_int_to_float() {
    let code = "func test(x: Int32) -> Float64 { return x as Float64 }";
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("sitofp"));
}

#[test]
fn test_irgen_cast_float_to_int() {
    let code = "func test(x: Float64) -> Int32 { return x as Int32 }";
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("fptosi"));
}

#[test]
fn test_irgen_cast_int_extend() {
    let code = "func test(x: Int32) -> Int64 { return x as Int64 }";
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("zext"));
}

#[test]
fn test_irgen_cast_force_bitcast() {
    let code = "func test(x: Float64) -> Int64 { return x as!! Int64 }";
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("bitcast"));
}

#[test]
fn test_irgen_struct_field_access() {
    let code = r#"
        struct Point { let x: Int32 let y: Int32 }
        func test() -> Int32 {
            var p: Point
            let val = p.x
            return val
        }
    "#;
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("struct.Point"));
    assert!(llvm_ir.contains("load"));
}

#[test]
fn test_irgen_struct_method_call() {
    let code = r#"
        struct Point {
            let x: Int32
            let y: Int32
            func f() -> Int64 {
                return 1
            }
        }
        func test() -> Int64 {
            var p: Point
            return p.f()
        }
    "#;
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@Point.f"));
    assert!(llvm_ir.contains("call i64 @Point.f()"));
}

#[test]
fn test_irgen_struct_method_call_with_params() {
    let code = r#"
        struct Point {
            let x: Int32
            func add(_ a: Int32, _ b: Int32) -> Int32 {
                return a + b
            }
        }
        func test() -> Int32 {
            var p: Point
            return p.add(3, 4)
        }
    "#;
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
    let module = ir_gen.generate(&program, symbol_resolver.get_module_scope(module_id));
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@Point.add"));
    assert!(llvm_ir.contains("call i32 @Point.add("));
    assert!(llvm_ir.contains("i32 3"));
    assert!(llvm_ir.contains("i32 4"));
}
