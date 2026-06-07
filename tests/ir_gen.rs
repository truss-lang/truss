use std::{cell::RefCell, rc::Rc};

use inkwell::context::Context;
use truss::{
    diag::TrussDiagnosticEngine,
    ir_gen::IRGenerator,
    krate::Crate,
    lexer::{CharStream, Lexer},
    macro_expander::MacroExpander,
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("test"));
    assert!(llvm_ir.contains("load"));
}

#[test]
fn test_irgen_non_null_pointer_param() {
    let code = "func test(p: Int32*!) -> Int32 { return *p }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@Point.f"));
    assert!(llvm_ir.contains("call i64 @Point.f(ptr"));
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@Point.add"));
    assert!(llvm_ir.contains("call i32 @Point.add("));
    assert!(llvm_ir.contains("i32 3"));
    assert!(llvm_ir.contains("i32 4"));
}

#[test]
fn test_irgen_type_instantiation() {
    let code = r#"
        struct Point {
            let x: Int32
            init(x: Int32) {}
        }
        func test() {
            Point(x: 42)
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@Point.init"));
    assert!(llvm_ir.contains("call void @Point.init("));
    assert!(llvm_ir.contains("i32 42"));
}

#[test]
fn test_irgen_var_getter_shorthand() {
    let code = r#"
        func test() -> Int32 {
            var v: Int32 { return _v }
            return v
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@v.getter"));
    assert!(llvm_ir.contains("define i32 @v.getter(ptr"));
    assert!(llvm_ir.contains("call i32 @v.getter("));
}

#[test]
fn test_irgen_var_getter_explicit() {
    let code = r#"
        func test() -> Int32 {
            var v: Int32 { get { return _v } }
            return v
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@v.getter"));
    assert!(llvm_ir.contains("define i32 @v.getter(ptr"));
    assert!(llvm_ir.contains("call i32 @v.getter("));
}

#[test]
fn test_irgen_var_get_set() {
    let code = r#"
        func test() -> Int32 {
            var _v: Int32 = 10
            var v: Int32 { get { return _v } set { _v = newValue } }
            v = 20
            return v
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@v.getter"));
    assert!(llvm_ir.contains("@v.setter"));
    assert!(llvm_ir.contains("call i32 @v.getter("));
    assert!(llvm_ir.contains("call void @v.setter("));
}

#[test]
fn test_irgen_var_willset_didset() {
    let code = r#"
        func test() -> Int32 {
            var v: Int32 = 0 { willSet { } didSet { } }
            v = 42
            return v
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@v.willSet"));
    assert!(llvm_ir.contains("@v.didSet"));
    assert!(llvm_ir.contains("define void @v.willSet(ptr"));
    assert!(llvm_ir.contains("define void @v.didSet(ptr"));
    assert!(llvm_ir.contains("call void @v.willSet("));
    assert!(llvm_ir.contains("call void @v.didSet("));
}

#[test]
fn test_irgen_var_all_accessors() {
    let code = r#"
        func test() -> Int32 {
            var _v: Int32 = 0
            var v: Int32 { get { return _v } set(v) { _v = v } willSet { } didSet { } }
            let x = v
            v = 42
            return x
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@v.getter"));
    assert!(llvm_ir.contains("@v.setter"));
    assert!(llvm_ir.contains("@v.willSet"));
    assert!(llvm_ir.contains("@v.didSet"));
}

#[test]
fn test_irgen_var_set_no_param() {
    let code = r#"
        func test() -> Int32 {
            var _v: Int32 = 0
            var v: Int32 { get { return _v } set { _v = newValue } }
            v = 99
            return v
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@v.getter"));
    assert!(llvm_ir.contains("@v.setter"));
    assert!(llvm_ir.contains("call void @v.setter("));
}

#[test]
fn test_irgen_struct_getter_shorthand() {
    let code = r#"
        struct T {
            var i: Int32 { return 42 }
        }
        func test() -> Int32 {
            let t = T()
            return t.i
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@T.i.getter"));
    assert!(llvm_ir.contains("define i32 @T.i.getter(ptr"));
    assert!(llvm_ir.contains("call i32 @T.i.getter("));
}

#[test]
fn test_irgen_struct_getter_explicit() {
    let code = r#"
        struct T {
            var i: Int32 { get { return 1 } }
        }
        func test() -> Int32 {
            let t = T()
            return t.i
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("define i32 @T.i.getter(ptr"));
    assert!(llvm_ir.contains("call i32 @T.i.getter("));
}

#[test]
fn test_irgen_struct_get_set() {
    let code = r#"
        struct T {
            var _i: Int32
            var i: Int32 {
                get { return _i }
                set { _i = newValue }
            }
        }
        func test() {
            var t = T()
            t.i = 7
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("define i32 @T.i.getter(ptr"));
    assert!(llvm_ir.contains("define void @T.i.setter(ptr"));
    assert!(llvm_ir.contains("call void @T.i.setter("));
}

#[test]
fn test_irgen_struct_willset_didset() {
    let code = r#"
        struct T {
            var i: Int32 = 0 {
                willSet { }
                didSet { }
            }
        }
        func test() {
            var t = T()
            t.i = 1
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("define void @T.i.willSet(ptr"));
    assert!(llvm_ir.contains("define void @T.i.didSet(ptr"));
    assert!(llvm_ir.contains("call void @T.i.willSet("));
    assert!(llvm_ir.contains("call void @T.i.didSet("));
}

#[test]
fn test_irgen_struct_get_set_read_write() {
    let code = r#"
        struct T {
            var _i: Int32
            var i: Int32 {
                get { return _i }
                set { _i = newValue }
            }
        }
        func test() -> Int32 {
            var t = T()
            t.i = 7
            return t.i
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("define i32 @T.i.getter(ptr"));
    assert!(llvm_ir.contains("define void @T.i.setter(ptr"));
    assert!(llvm_ir.contains("call void @T.i.setter("));
    assert!(llvm_ir.contains("call i32 @T.i.getter("));
}

#[test]
fn test_irgen_enum_decl_simple_cases() {
    let code = r#"
        enum Option {
            case none
            case some
        }
        func test(e: Option) -> Option {
            return e
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("%enum.Option"));
    assert!(llvm_ir.contains("%enum.Option.payloads"));
}

#[test]
fn test_irgen_enum_case_construction_no_payload() {
    let code = r#"
        enum Option {
            case none
            case some
        }
        func test() -> Option {
            return Option.none
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("%enum.Option = type { i8"));
    assert!(llvm_ir.contains("test"));
}

#[test]
fn test_irgen_enum_case_construction_with_payload() {
    let code = r#"
        enum Option {
            case none
            case some(Int32)
        }
        func test() -> Option {
            return Option.some(42)
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("%enum.Option = type { i8"));
    assert!(llvm_ir.contains("i8 1"));
    assert!(llvm_ir.contains("i32 42"));
}

#[test]
fn test_irgen_enum_case_with_labeled_payload() {
    let code = r#"
        enum Either {
            case left(Int32)
            case right(Float64)
        }
        func test() -> Either {
            return Either.left(10)
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("%enum.Either"));
    assert!(llvm_ir.contains("i32 10"));
}

#[test]
fn test_irgen_enum_multiple_cases() {
    let code = r#"
        enum Status {
            case idle
            case loading
            case success(Int32)
            case error(Int32, Bool)
        }
        func test() -> Status {
            return Status.error(404, false)
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("%enum.Status"));
    assert!(llvm_ir.contains("i8 3"));
    assert!(llvm_ir.contains("i32 404"));
}

#[test]
fn test_irgen_enum_method() {
    let code = r#"
        enum Option {
            case none
            case some(Int32)
            func is_some() -> Bool {
                return true
            }
        }
        func test() -> Bool {
            return Option.is_some()
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("@Option.is_some"));
    assert!(llvm_ir.contains("true"));
}

#[test]
fn test_irgen_enum_variable() {
    let code = r#"
        enum Option {
            case none
            case some(Int32)
        }
        func test() -> Int32 {
            var x: Option = Option.some(99)
            return 0
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("%enum.Option"));
    assert!(llvm_ir.contains("i32 99"));
}

#[test]
fn test_irgen_if_case_no_bindings() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.none = x {}
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("if_then"));
    assert!(llvm_ir.contains("if_exit"));
    assert!(llvm_ir.contains("case_match"));
}

#[test]
fn test_irgen_if_case_with_bindings() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.some(val) = x {
                let _ = val
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(module) => {
            let llvm_ir = module.print_to_string().to_string();
            if !llvm_ir.contains("val") {
                panic!("IR does not contain 'val':\n{}", llvm_ir);
            }
        }
        Err(_) => {
            panic!("IR generation panicked");
        }
    }
}

#[test]
fn test_irgen_if_case_with_else() {
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
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("if_then"));
    assert!(llvm_ir.contains("if_else"));
    assert!(llvm_ir.contains("if_exit"));
    assert!(llvm_ir.contains("case_match"));
}

#[test]
fn test_irgen_if_case_else_if() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) {
            if case Option.none = x {
                let _ = 1
            } else if case Option.some(val) = x {
                let _ = val
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("case_match"));
    assert!(llvm_ir.contains("val"));
}

#[test]
fn test_irgen_class_decl() {
    let code = r#"
        class Point { let x: Int32 let y: Int32 }
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("class.Point"),
        "Expected class.Point in IR:\n{}",
        llvm_ir
    );
    assert!(llvm_ir.contains("load"));
}

#[test]
fn test_irgen_class_method_call() {
    let code = r#"
        class Point {
            let x: Int32
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Point.f"),
        "Expected Point.f in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_vtable_global() {
    let code = r#"
        class Animal {
            func speak() -> Int32 { return 1 }
            func eat() -> Int32 { return 2 }
        }
        func test() -> Int32 {
            var a: Animal
            return a.speak()
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("vtable.Animal"),
        "Expected vtable type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__vtable.Animal"),
        "Expected vtable global in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Animal.speak"),
        "Expected Animal.speak in vtable:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Animal.eat"),
        "Expected Animal.eat in vtable:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_vtable_method_call_is_indirect() {
    let code = r#"
        class Greeter {
            func greet() -> Int32 { return 42 }
        }
        func test() -> Int32 {
            var g: Greeter
            return g.greet()
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("vtable.Greeter"),
        "Expected vtable type:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__vtable.Greeter"),
        "Expected vtable global:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call"),
        "Expected a call instruction:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Greeter.greet"),
        "Expected Greeter.greet function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_inheritance_vtable_inherited_methods() {
    let code = r#"
        class Animal {
            func speak() -> Int32 { return 1 }
        }
        class Dog: Animal {
            func speak() -> Int32 { return 2 }
        }
        func test() -> Int32 {
            var d: Dog
            return d.speak()
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("vtable.Animal"),
        "Expected vtable.Animal:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("vtable.Dog"),
        "Expected vtable.Dog:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__vtable.Animal"),
        "Expected __vtable.Animal:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__vtable.Dog"),
        "Expected __vtable.Dog:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Dog.speak"),
        "Expected Dog.speak in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Animal.speak"),
        "Expected Animal.speak in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_inheritance_field_layout() {
    let code = r#"
        class Animal { let name: Int32 }
        class Dog: Animal { let breed: Int32 }
        func test() -> Int32 {
            var d: Dog
            return d.name
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("class.Dog"),
        "Expected class.Dog in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("{ ptr, i64, i32, i32 }"),
        "Expected Dog type [vtable_ptr, ref_count, name, breed] in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_inheritance_field_access() {
    let code = r#"
        class Animal { let name: Int32 }
        class Dog: Animal { let breed: Int32 }
        func test() -> Int32 {
            var d: Dog
            let val = d.name
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("class.Dog"),
        "Expected class.Dog in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("load"),
        "Expected load instruction in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_inheritance_multi_level() {
    let code = r#"
        class Animal { let a: Int32 }
        class Mammal: Animal { let b: Int32 }
        class Dog: Mammal { let c: Int32 }
        func test() -> Int32 {
            var d: Dog
            let v1 = d.a
            let v2 = d.b
            return v1
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("class.Dog"),
        "Expected class.Dog in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("{ ptr, i64, i32, i32, i32 }"),
        "Expected Dog type [vtable_ptr, ref_count, a, b, c] in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("getelementptr"),
        "Expected GEP instructions in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_tuple_type_annotation() {
    let code = "func test() { let a: (Int32, Bool) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Bool"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("{ i32, i1 }"),
        "Expected tuple layout (Int32, Bool) in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_self_return_in_struct_method() {
    let code = r#"
        struct Point {
            let x: Int32
            func identity() -> Point {
                return self
            }
        }
        func test() -> Point {
            var p: Point
            return p.identity()
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("@Point.identity"),
        "Expected Point.identity function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_self_in_init() {
    let code = r#"
        struct Point {
            let x: Int32
            init(x: Int32) {
            }
        }
        func test() -> Point {
            return Point(x: 42)
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("@Point.init"),
        "Expected Point.init function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_tuple_literal() {
    let code = "func test() -> Int32 { let a = (1, 2); return a.0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Int32"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("{ i32, i32 }"),
        "Expected tuple layout (Int32, Int32) in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("getelementptr"),
        "Expected GEP for field access in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_tuple_index_access() {
    let code = "func test() -> Bool { let a = (1, true); return a.1 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Bool"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("{ i32, i1 }"),
        "Expected tuple layout (Int32, Bool) in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("getelementptr"),
        "Expected GEP for field access in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_tuple_literal_as_return() {
    let code = "func test() -> (Int32, Bool) { return (1, true) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Bool"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("{ i32, i1 }"),
        "Expected tuple layout (Int32, Bool) in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_named_tuple_literal() {
    let code = "func test() -> (x: Int32, y: Bool) { return (x: 1, y: true) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Bool"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("{ i32, i1 }"),
        "Expected tuple layout (Int32, Bool) in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_named_tuple_member_access() {
    let code = "func test() -> Int32 { let t = (x: 42, y: true); return t.x }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Bool"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("getelementptr"),
        "Expected GEP for named field access in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_named_tuple_positional_access() {
    let code = "func test() -> Bool { let t = (x: 1, y: true); return t.1 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Bool"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("getelementptr"),
        "Expected GEP for positional access in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_named_tuple_type_annotation() {
    let code =
        "func test() -> Int32 { let t: (a: Int32, b: Bool) = (a: 10, b: false); return t.a }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("tuple.__tuple_Int32_Bool"),
        "Expected tuple struct type in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_protocol_empty() {
    let code = "protocol Empty {}";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("main"), "IR should contain module name");
}

#[test]
fn test_irgen_protocol_method_requirement_only() {
    let code = "protocol Drawable { func draw() -> Void }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        !llvm_ir.contains("Drawable.draw"),
        "Method requirement without body should not generate IR function"
    );
}

#[test]
fn test_irgen_protocol_default_implementation() {
    let code = "protocol Greeter { func greet() -> Int32 { return 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Greeter.greet"),
        "Default implementation should generate function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_protocol_default_impl_no_crash() {
    let code =
        "protocol Helper { func help() -> Int32 { return 42 } func need() -> Void { return } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Should not have errors, got: {:?}", errors);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Helper.help"),
        "Default impl 'help' should generate function:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Helper.need"),
        "Default impl 'need' should generate function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_protocol_only_requirement_no_default() {
    let code = "protocol Drawable { func draw() -> Void } protocol Helper { func help() -> Int32 { return 42 } func need() -> Int32 { return 0 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        !llvm_ir.contains("Drawable.draw"),
        "Requirement-only 'draw' should NOT generate function:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Helper.help"),
        "Default impl 'help' should generate function:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Helper.need"),
        "Default impl 'need' should generate function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_protocol_existential_container() {
    let code = "protocol Drawable { func draw() -> Int32 { return 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Drawable.draw"),
        "Default implementation should generate function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_protocol_witness_table_for_class() {
    let code = r#"
        protocol Drawable { func draw() -> Int32 }
        class Shape { }
        class Circle: Shape, Drawable { func draw() -> Int32 { return 42 } }
        func test() -> Int32 {
            let d: any Drawable = Circle()
            return d.draw()
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));

    let errors_before = engine.borrow().get_errors().len();
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    if errors.len() > errors_before {
        panic!("Type errors:\n{:#?}", errors);
    }
    drop(engine_ref);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("__protocol_wt.Drawable.Circle"),
        "Witness table for (Drawable, Circle) should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("existential.Drawable"),
        "Existential container type should exist:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_protocol_witness_table_for_struct() {
    let code = r#"
        protocol Drawable { func draw() -> Int32 }
        struct Circle: Drawable { func draw() -> Int32 { return 42 } }
        func test() -> Int32 {
            let d: any Drawable = Circle()
            return d.draw()
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));

    let errors_before = engine.borrow().get_errors().len();
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    if errors.len() > errors_before {
        panic!("Type errors:\n{:#?}", errors);
    }
    drop(engine_ref);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("__protocol_wt.Drawable.Circle"),
        "Witness table for (Drawable, Circle) struct should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("existential.Drawable"),
        "Existential container type should exist:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_compound_protocol_existential_dispatch() {
    let code = r#"
        protocol Drawable { func draw() -> Int32 }
        protocol Resettable { func reset() -> Void }
        struct Circle: Drawable, Resettable {
            var x: Int32
            func draw() -> Int32 { return 42 }
            func reset() -> Void { return }
        }
        func test() -> Int32 {
            let d: any Drawable & Resettable = Circle(x: 99)
            return d.draw()
        }
    "#;
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));

    let errors_before = engine.borrow().get_errors().len();
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    if errors.len() > errors_before {
        panic!("Type errors:\n{:#?}", errors);
    }
    drop(engine_ref);

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("existential.Drawable & Resettable"),
        "Existential container for compound type should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__protocol_wt.Drawable.Circle"),
        "Witness table for (Drawable, Circle) should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__protocol_wt.Resettable.Circle"),
        "Witness table for (Resettable, Circle) should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Circle.draw"),
        "Circle.draw function should exist:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_computed_property_getter_in_vtable() {
    let code = r#"
        class ViewModel {
            var _val: Int32
            var value: Int32 {
                get { return _val }
            }
        }
        func test() -> Int32 {
            var vm: ViewModel
            return vm.value
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("vtable.ViewModel"),
        "vtable type should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__vtable.ViewModel"),
        "vtable global should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ViewModel.value.getter"),
        "getter function should be in vtable:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_computed_property_setter_in_vtable() {
    let code = r#"
        class ViewModel {
            var _val: Int32
            var value: Int32 {
                get { return _val }
                set { _val = newValue }
            }
        }
        func test() -> Int32 {
            var vm: ViewModel
            vm.value = 42
            return vm.value
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("vtable.ViewModel"),
        "vtable type should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ViewModel.value.getter"),
        "getter should be in vtable:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ViewModel.value.setter"),
        "setter should be in vtable:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_computed_property_inheritance_override() {
    let code = r#"
        class Base {
            var value: Int32 {
                get { return 1 }
            }
        }
        class Derived: Base {
            var value: Int32 {
                get { return 2 }
            }
        }
        func test() -> Int32 {
            var d: Derived
            return d.value
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("vtable.Base"),
        "vtable.Base should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("vtable.Derived"),
        "vtable.Derived should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__vtable.Base"),
        "__vtable.Base should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__vtable.Derived"),
        "__vtable.Derived should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Base.value.getter"),
        "Base getter should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Derived.value.getter"),
        "Derived getter should exist:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_stored_property_auto_getter_setter() {
    let code = r#"
        class Data {
            let name: Int32
        }
        func test() -> Int32 {
            var d: Data
            return d.name
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("__vtable.Data"),
        "vtable should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Data.name.getter"),
        "stored property should have getter:\n{}",
        llvm_ir
    );
    assert!(
        !llvm_ir.contains("Data.name.setter"),
        "let property should not have setter:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call"),
        "should have indirect call:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_stored_var_auto_getter_and_setter() {
    let code = r#"
        class Data {
            var value: Int32
        }
        func test() -> Int32 {
            var d: Data
            d.value = 42
            return d.value
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Data.value.getter"),
        "var should have getter:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Data.value.setter"),
        "var should have setter:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_struct_auto_deinit() {
    let code = r#"
        struct Data {
            var value: Int32
        }
        func test() -> Int32 {
            var d = Data(value: 42)
            return d.value
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Data.deinit"),
        "struct should have deinit function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_enum_auto_deinit() {
    let code = r#"
        enum Option {
            case none
            case some(Int32)
        }
        func test() -> Int32 {
            var e = Option.some(42)
            return 0
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Option.deinit"),
        "enum should have deinit function:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_struct_deinit_called_on_scope_exit() {
    let code = r#"
        struct Counter {
            var value: Int32
        }
        func test() -> Int32 {
            var c = Counter(value: 42)
            return c.value
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Counter.deinit"),
        "struct deinit function should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call void @Counter.deinit"),
        "deinit should be called on scope exit:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_user_defined_struct_deinit() {
    let code = r#"
        struct Data {
            var value: Int32
            deinit {
                var dummy = 1
            }
        }
        func test() -> Int32 {
            var d: Data
            return 0
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Data.deinit"),
        "struct deinit function should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call void @Data.deinit"),
        "deinit should be called on scope exit:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_enum_deinit_called_on_scope_exit() {
    let code = r#"
        enum Option {
            case none
            case some(Int32)
        }
        func test() -> Int32 {
            var e = Option.some(42)
            return 0
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("Option.deinit"),
        "enum deinit function should exist:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call void @Option.deinit"),
        "deinit should be called on scope exit:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_method() {
    let code = "struct Foo {} extension Foo { func bar() -> Int32 { 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Foo.bar"),
        "extension method should generate Foo.bar:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_self_access() {
    let code = "struct Point { let x: Int32 } extension Point { func getX() -> Int32 { self.x } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Point.getX"),
        "extension method self access should generate Point.getX:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_protocol_witness_table() {
    let code = "protocol P { func req() -> Int32 } struct Foo {} extension Foo: P { func req() -> Int32 { 99 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Foo.req"),
        "extension method should generate Foo.req:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__protocol_wt.P.Foo"),
        "protocol witness table for P+Foo should exist:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_static_method_struct() {
    let code = "struct Foo {} extension Foo { static func bar() -> Int32 { 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Foo.bar"),
        "static method should generate Foo.bar:\n{}",
        llvm_ir
    );
    assert!(
        !llvm_ir.contains("define i32 @Foo.bar(i32"),
        "static method should NOT take i32 (self) as first parameter:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ret i32 42"),
        "static method should return 42:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_static_method_class() {
    let code = "class Foo {} extension Foo { static func bar() -> Int32 { 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Foo.bar"),
        "static method in class extension should generate Foo.bar:\n{}",
        llvm_ir
    );
    assert!(
        !llvm_ir.contains("define i32 @Foo.bar(ptr"),
        "static method in class extension should NOT take ptr (self) as first parameter:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_static_method_enum() {
    let code = "enum Foo { case a } extension Foo { static func bar() -> Int32 { 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Foo.bar"),
        "static method in enum extension should generate Foo.bar:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_static_method_protocol() {
    let code = "protocol Foo {} extension Foo { static func bar() -> Int32 { 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Foo.bar"),
        "static method in protocol extension should generate Foo.bar:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_instance_method_has_self_param() {
    let code = "struct Foo {} extension Foo { func bar() -> Int32 { 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Foo.bar(ptr"),
        "instance method in extension should have ptr (self) as first parameter:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_extension_static_method_with_params() {
    let code =
        "struct Calc {} extension Calc { static func add(a: Int32, b: Int32) -> Int32 { a + b } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("define i32 @Calc.add"),
        "static method with params should generate Calc.add:\n{}",
        llvm_ir
    );
    assert!(
        !llvm_ir.contains("define i32 @Calc.add(ptr"),
        "static method with params should NOT have ptr (self) as first parameter, got:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_guard_case_success() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) -> Int32 {
            guard case .some(val) = x else { return 0 }
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(_) => {}
        Err(_) => panic!("guard IR generation panicked"),
    }
}

#[test]
fn test_irgen_match_simple() {
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
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let program = parser.parse();
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(module) => {
            let llvm_ir = module.print_to_string().to_string();
            assert!(
                llvm_ir.contains("match_exit"),
                "match should generate match_exit block:\n{}",
                llvm_ir
            );
            assert!(
                llvm_ir.contains("case_body"),
                "match should generate case_body blocks:\n{}",
                llvm_ir
            );
        }
        Err(_) => panic!("match IR generation panicked"),
    }
}

#[test]
fn test_irgen_guard_dot_shorthand() {
    let code = r#"
        enum Option { case none case some(Int32) }
        func test(x: Option) -> Int32 {
            guard case .some(val) = x else { return 0 }
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
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(_) => {}
        Err(_) => panic!("guard dot shorthand IR generation panicked"),
    }
}

#[test]
fn test_irgen_associated_type_access_on_protocol() {
    let code = r#"
        protocol Container { associatedtype Item }
        func test(x: Container.Item) -> Container.Item { return x }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(module) => {
            let llvm_ir = module.print_to_string().to_string();
            assert!(
                llvm_ir.contains("test"),
                "LLVM IR should contain function test:\n{}",
                llvm_ir
            );
        }
        Err(_) => panic!("IR generation for associated type access panicked"),
    }
}

#[test]
fn test_irgen_associated_type_access_typealias() {
    let code = r#"
        protocol P { typealias Item = Int32 }
        func test(x: P.Item) -> P.Item { return x }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(module) => {
            let llvm_ir = module.print_to_string().to_string();
            assert!(
                llvm_ir.contains("i32"),
                "Typealias to Int32 should produce i32 in IR:\n{}",
                llvm_ir
            );
        }
        Err(_) => panic!("IR generation for typealias access panicked"),
    }
}

#[test]
fn test_irgen_defer_basic() {
    let code = "func test(x: Int32) -> Int32 { defer { } return x }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    assert!(result.is_ok(), "Defer should not cause panic in IR gen");
}

#[test]
fn test_irgen_defer_lifo_order() {
    let code = r#"
        func test() -> Int32 {
            var result = 0
            defer { result = 1 }
            defer { result = 2 }
            return result
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    assert!(
        result.is_ok(),
        "Defer LIFO should not cause panic in IR gen"
    );
}

#[test]
fn test_irgen_defer_nested_scope() {
    let code = r#"
        func test() -> Int32 {
            let x = 1
            if x == 1 {
                defer { }
            }
            return x
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    assert!(
        result.is_ok(),
        "Defer in nested scope should not cause panic"
    );
}

#[test]
fn test_irgen_implicit_return_int_literal() {
    let code = "func test() -> Int32 { 42 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("ret i32 42"),
        "Expected implicit return with i32 42"
    );
}

#[test]
fn test_irgen_implicit_return_variable() {
    let code = "func test() -> Int32 { let x = 1 x }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("ret"),
        "Expected implicit return with ret instruction"
    );
}

#[test]
fn test_irgen_if_as_expression_value() {
    let code = "func test() -> Int32 { if true { 1 } else { 2 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("if_then"), "Expected if_then basic block");
    assert!(llvm_ir.contains("if_else"), "Expected if_else basic block");
    assert!(llvm_ir.contains("if_exit"), "Expected if_exit basic block");
    assert!(
        llvm_ir.contains("ret i32"),
        "Expected return from if expression"
    );
}

#[test]
fn test_irgen_if_as_variable_initializer() {
    let code = "func test() { let x = if true { 1 } else { 2 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(llvm_ir.contains("if_then"), "Expected if_then basic block");
    assert!(llvm_ir.contains("if_else"), "Expected if_else basic block");
    assert!(llvm_ir.contains("if_exit"), "Expected if_exit basic block");
    assert!(
        llvm_ir.contains("if_result"),
        "Expected if_result alloca/load for if expression value"
    );
}

#[test]
fn test_irgen_if_elseif_chain_as_value() {
    let code = "func test() -> Int32 { if false { 1 } else if true { 2 } else { 3 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("ret"),
        "Expected return from if-else if chain"
    );
}

#[test]
fn test_irgen_match_multi_pattern_enum() {
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
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(module) => {
            let llvm_ir = module.print_to_string().to_string();
            assert!(
                llvm_ir.contains("match_exit"),
                "multi-pattern match should generate match_exit block:\n{}",
                llvm_ir
            );
            assert!(
                llvm_ir.contains("case_body"),
                "multi-pattern match should generate case_body blocks:\n{}",
                llvm_ir
            );
        }
        Err(_) => panic!("multi-pattern match IR generation panicked"),
    }
}

#[test]
fn test_irgen_match_multi_pattern_with_guard() {
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
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap())
    }));
    match result {
        Ok(module) => {
            let llvm_ir = module.print_to_string().to_string();
            assert!(
                llvm_ir.contains("match_exit"),
                "multi-pattern match with guard should generate match_exit:\n{}",
                llvm_ir
            );
            assert!(
                llvm_ir.contains("case_body"),
                "multi-pattern match with guard should generate case_body:\n{}",
                llvm_ir
            );
        }
        Err(_) => panic!("multi-pattern match with guard IR generation panicked"),
    }
}

#[test]
fn test_irgen_module_with_func() {
    let code = "module foo { func bar() -> Int32 { 42 } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("bar"),
        "module func should generate IR for function 'bar':\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("42"),
        "module func IR should contain return value 42:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_module_with_variable() {
    let code = "module foo { func test() -> Int32 { let x: Int32 = 10 return x } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("test"),
        "module func 'test' should generate IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_empty_module() {
    let code = "module foo { }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("main"),
        "empty module should still generate valid IR"
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_module_with_nested_func_call() {
    let code = "module foo { func bar() -> Int32 { 42 } func baz() -> Int32 { bar() } }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("bar"),
        "module func 'bar' should generate IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("baz"),
        "module func 'baz' should generate IR:\n{}",
        llvm_ir
    );
    assert!(llvm_ir.contains("call"), "baz should call bar");
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_overloaded_functions_mangled_names() {
    let code = "func foo(x: Int32) -> Int32 { x } func foo(y: Float64) -> Float64 { y } func caller() -> Float64 { foo(y: 3.0) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains(r#""foo$x$I32""#),
        "expected mangled name for foo(x: Int32), got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains(r#""foo$y$F64""#),
        "expected mangled name for foo(y: Float64), got:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_overloaded_struct_methods_mangled_names() {
    let code = "struct S { func foo(x: Int32) -> Int32 { x } func foo(y: Float64) -> Float64 { y } } func test() -> Float64 { let s = S() return s.foo(y: 3.0) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains(r#""S.foo$x$I32""#),
        "expected mangled name for S.foo(x: Int32), got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains(r#""S.foo$y$F64""#),
        "expected mangled name for S.foo(y: Float64), got:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains(r#""S.foo$y$F64""#),
        "expected call to mangled name S.foo$y$F64, got:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

fn run_ir_gen(code: &str) -> (String, Rc<RefCell<TrussDiagnosticEngine>>) {
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    (llvm_ir, engine)
}

#[test]
fn test_irgen_import_module_call() {
    let (llvm_ir, engine) = run_ir_gen(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo
         func test() -> Int32 { return Foo.bar() }",
    );
    assert!(
        llvm_ir.contains("bar"),
        "Expected 'bar' function in IR, got:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_import_wildcard_call() {
    let (llvm_ir, engine) = run_ir_gen(
        "module Foo { func bar() -> Int32 { return 42 } }
         import Foo.*
         func test() -> Int32 { return bar() }",
    );
    assert!(
        llvm_ir.contains("bar"),
        "Expected 'bar' function in IR, got:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_import_member_call() {
    let (llvm_ir, engine) = run_ir_gen(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import Foo.Bar.baz
         func test() -> Int32 { return baz() }",
    );
    assert!(
        llvm_ir.contains("baz"),
        "Expected 'baz' function in IR, got:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_import_nested_module_call() {
    let (llvm_ir, engine) = run_ir_gen(
        "module Foo { module Bar { func baz() -> Int32 { return 99 } } }
         import Foo
         func test() -> Int32 { return Foo.Bar.baz() }",
    );
    assert!(
        llvm_ir.contains("baz"),
        "Expected 'baz' function in IR, got:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_overloaded_function_call() {
    let code = "func foo(x: Int32) -> Int32 { return x } func foo(y: Float64) -> Float64 { return y } func caller() -> Float64 { return foo(y: 3.0) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert!(
        llvm_ir.contains("call"),
        "expected a call instruction in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains(r#""foo$y$F64""#),
        "expected call to mangled name foo$y$F64, got:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_generic_function_decl() {
    let code = "func identity<T>(x: T) -> T { return x }
                 func test() -> Int32 { return identity(x: 42) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert!(
        llvm_ir.contains("define ptr @identity"),
        "expected generic function identity with ptr return type, got:\n{}",
        llvm_ir
    );
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "no errors expected, got: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_irgen_generic_call_with_ptr_arg() {
    let code = "func identity<T>(x: T) -> T { return x }
                 func test() -> Int32 { return identity(x: 42) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert!(
        llvm_ir.contains("call ptr @identity"),
        "expected call to generic function identity, got:\n{}",
        llvm_ir
    );
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "no errors expected, got: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_irgen_generic_bool_arg() {
    let code = "func identity<T>(x: T) -> T { return x }
                 func test() -> Bool { return identity(x: true) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert!(
        llvm_ir.contains("call ptr @identity"),
        "expected call to generic identity, got:\n{}",
        llvm_ir
    );
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "no errors expected, got: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_irgen_closure_simple() {
    let code = "func test() -> Int32 { let f = { (x: Int32) -> Int32 in x }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("define i32 @__closure_0(i32 %"),
        "Closure should have i32 parameter and return i32"
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_no_params() {
    let code = "func test() -> Int32 { let f = { in 42 }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_multi_param() {
    let code = "func test() -> Int32 { let f = { (x: Int32, y: Int32) -> Int32 in x }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("define i32 @__closure_0(i32 %"),
        "Closure should have i32 params"
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_with_return() {
    let code = "func test() -> Int32 { let f = { in 42 }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "Closure body should have return instruction"
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_call() {
    let code = "func test() -> Int32 { let f = { (x: Int32) -> Int32 in x }; return f(42) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert!(
        llvm_ir.contains("call i32"),
        "Should have a call i32 instruction, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function"
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_higher_order_call() {
    let code = "func apply(fn: (Int32) -> Int32, x: Int32) -> Int32 { return fn(x) }
                 func test() -> Int32 { return apply(fn: { (y: Int32) -> Int32 in y }, x: 42) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert!(
        llvm_ir.contains("define i32 @apply"),
        "Should define apply function"
    );
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function"
    );
    assert!(
        llvm_ir.contains("call i32"),
        "Should have call instructions"
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_function_ref_assignment() {
    let code = "func addOne(x: Int32) -> Int32 { return x + 1 }
                 func test() -> Int32 { let f: (Int32)->Int32 = addOne; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert!(
        llvm_ir.contains("store ptr @addOne"),
        "Should have store instruction for function pointer, IR:\n{}",
        llvm_ir
    );
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "no errors expected, got: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_irgen_fn_ref_call_through_variable() {
    let code = "func addOne(x: Int32) -> Int32 { return x + 1 }
                 func test() -> Int32 { let f: (Int32)->Int32 = addOne; return f(41) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert!(
        llvm_ir.contains("call i32"),
        "Should have call instruction, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@addOne"),
        "Should reference addOne function, IR:\n{}",
        llvm_ir
    );
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "no errors expected, got: {:?}",
        engine.borrow().get_diagnostics()
    );
}

#[test]
fn test_irgen_closure_capture_outer_variable() {
    let code = "func test() -> Int32 { let x = 42; let f = { (y: Int32) -> Int32 in x + y }; return f(10) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i32 "),
        "Should call the closure, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_multi_statement_body() {
    let code = "func test() -> Int32 { let f = { (x: Int32) -> Int32 in let y = x + 1; return y }; return f(41) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i32 "),
        "Should call the closure, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "Closure should return i32, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_trailing_syntax() {
    let code = "func apply(fn: () -> Int32) -> Int32 { return fn() }
                 func test() -> Int32 { return apply(fn: { in 42 }) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("define i32 @apply"),
        "Should define apply function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i32 @apply"),
        "Should call apply (which calls the closure), IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_expression_only_body() {
    let code = "func test() -> Int32 { let f = { 42 }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_multiple_in_function() {
    let code = "func test() -> Int32 { let f1 = { (x: Int32) -> Int32 in x }; let f2 = { (y: Int32) -> Int32 in y + 1 }; return f1(1) + f2(2) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("__closure_1"),
        "Should define __closure_1, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i32 "),
        "Should have call instructions, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_implicit_return_last_expression() {
    let code = "func test() -> Int32 { let f = { (x: Int32) -> Int32 in let y = x + 1; y }; return f(41) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "Closure should have ret i32 for implicit return, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i32 "),
        "Should call the closure, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_void_return_type() {
    let code = "func test() -> Int32 { let f = { () -> Void in }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("void @__closure_0"),
        "Closure should return void, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("ret void"),
        "Should have ret void, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_shorthand_argument() {
    let code = "func test() -> Int32 { let f: (Int32) -> Int32 = { $0 }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("define i32 @__closure_0(i32 %"),
        "Closure should take one i32 param and return i32, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_shorthand_binary() {
    let code = "func test() -> Int32 { let f: (Int32, Int32) -> Int32 = { $0 + $1 }; return 0 }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("define i32 @__closure_0(i32 %"),
        "Closure should take two i32 params and return i32, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("add i32"),
        "Should have add instruction, IR:\n{}",
        llvm_ir
    );
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_shorthand_with_type_annotation() {
    let code = "func test() -> Int32 { let f: (Int32) -> Int32 = { (x: Int32) -> Int32 in x }; return f(42) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("define i32 @__closure_0(i32 %"),
        "IR:\n{}",
        llvm_ir
    );
    assert!(llvm_ir.contains("call i32 "), "IR:\n{}", llvm_ir);
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_closure_shorthand_multi_args() {
    let code = "func test() -> Int32 { let f = { (x: Int32, y: Int32) -> Int32 in x + y }; return f(10, 20) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    eprintln!("=== LLVM IR ===\n{}\n=== END ===", llvm_ir);
    eprintln!("Errors: {:?}", engine.borrow().get_errors());
    assert!(
        llvm_ir.contains("__closure_0"),
        "Should define __closure_0 function, IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("define i32 @__closure_0(i32 %"),
        "Closure should take i32 params, IR:\n{}",
        llvm_ir
    );
    assert!(llvm_ir.contains("call i32 "), "IR:\n{}", llvm_ir);
    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
}

#[test]
fn test_irgen_super_method_call() {
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
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert_eq!(engine.borrow().get_errors().len(), 0, "no errors expected");
    assert!(
        llvm_ir.contains("Animal.speak"),
        "Expected Animal.speak definition:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Dog.speak"),
        "Expected Dog.speak definition:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("Dog.call_super"),
        "Expected Dog.call_super definition:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("call i32 @Animal.speak"),
        "Expected direct call to Animal.speak (not through vtable):\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_address_of_variable() {
    let code = "func test(v: Int32) -> Int32* { return &v }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors: {:?}",
        engine.borrow().get_errors()
    );
    assert!(
        llvm_ir.contains("test"),
        "Expected function test in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_struct_subscript_getter() {
    let code = r#"
        struct Array {
            subscript(idx: Int32) -> Int32 {
                get { return 42 }
            }
        }
        func test(a: Array) -> Int32 {
            return a[0]
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors: {:?}",
        engine.borrow().get_errors()
    );
    assert!(
        llvm_ir.contains("@Array.subscript.getter"),
        "Expected subscript.getter in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_struct_subscript_get_set() {
    let code = r#"
        struct Array {
            subscript(idx: Int32) -> Int32 {
                get { return 42 }
                set(newValue) { }
            }
        }
        func test(a: Array) -> Int32 {
            a[0] = 1
            return a[0]
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors: {:?}",
        engine.borrow().get_errors()
    );
    assert!(
        llvm_ir.contains("@Array.subscript.getter"),
        "Expected subscript.getter in IR:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("@Array.subscript.setter"),
        "Expected subscript.setter in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_class_subscript_getter() {
    let code = r#"
        class Array {
            subscript(idx: Int32) -> Int32 {
                get { return 42 }
            }
        }
        func test(a: Array) -> Int32 {
            return a[0]
        }
    "#;
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors: {:?}",
        engine.borrow().get_errors()
    );
    assert!(
        llvm_ir.contains("subscript.getter"),
        "Expected subscript getter in vtable:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_address_of_deref() {
    let code = "func test(p: Int32*) -> Int32* { return &*p }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());

    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();

    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors: {:?}",
        engine.borrow().get_errors()
    );
    assert!(
        llvm_ir.contains("test"),
        "Expected function test in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_macro_declaration() {
    let code = "macro id { ($x:expr) => { $x } }\nfunc test() -> Int32 { return 42 }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let mut program = parser.parse();
    let mut expander = MacroExpander::new(engine.clone());
    expander.expand(&mut program);
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors: {:?}",
        engine.borrow().get_errors()
    );
    assert!(llvm_ir.contains("test"));
}

#[test]
fn test_irgen_macro_expanded() {
    let code = "macro id { ($x:expr) => { $x } }\nfunc test() -> Int32 { return id!(42) }";
    let engine = create_engine();
    let mut lexer = Lexer::new(
        CharStream::new(code.to_string(), Rc::new("".to_string())),
        engine.clone(),
    );
    let mut parser = Parser::new(lexer.get_file(), lexer.parse(), engine.clone());
    let mut program = parser.parse();
    let mut expander = MacroExpander::new(engine.clone());
    expander.expand(&mut program);
    let krate = Rc::new(RefCell::new(Crate::new("test".to_string())));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, "test".to_string());
    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "Expected no errors: {:?}",
        engine.borrow().get_errors()
    );
    assert!(llvm_ir.contains("test"));
}

#[test]
fn test_irgen_static_binary_operator_method() {
    let code = "struct MyInt { var value: Int32; static func + (left: MyInt, right: MyInt) -> MyInt { return left } } func use_op(a: MyInt, b: MyInt) -> MyInt { return a + b }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(
        llvm_ir.contains("MyInt.+"),
        "Expected mangled operator +, got:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_member_binary_operator_method() {
    let code = "struct MyInt { var value: Int32; func + (other: MyInt) -> MyInt { return other } } func use_op(a: MyInt, b: MyInt) -> MyInt { return a + b }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(
        llvm_ir.contains("MyInt.+"),
        "Expected mangled operator +, got:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_conditional_block_basic() {
    let (llvm_ir, engine) = run_ir_gen(
        "#if A
func foo() -> Int32 { 1 }
#endif
func bar() -> Int32 { 2 }",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("foo"), "Expected foo function in IR");
    assert!(llvm_ir.contains("bar"), "Expected bar function in IR");
}

#[test]
fn test_irgen_conditional_block_with_else() {
    let (llvm_ir, engine) = run_ir_gen(
        "#if A
func foo() -> Int32 { 1 }
#else
func bar() -> Int32 { 2 }
#endif",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("foo"), "Expected foo function in IR");
    assert!(llvm_ir.contains("bar"), "Expected bar function in IR");
}

#[test]
fn test_irgen_conditional_block_nested() {
    let (llvm_ir, engine) = run_ir_gen(
        "#if A
#if B
func inner() -> Int32 { 1 }
#endif
#endif
func outer() -> Int32 { 2 }",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("inner"));
    assert!(llvm_ir.contains("outer"));
}

#[test]
fn test_irgen_conditional_block_in_function_body() {
    let (llvm_ir, engine) = run_ir_gen(
        "func test() -> Int32 {
    var x: Int32 = 1
#if A
    x = x + 1
#endif
    return x
}",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("test"));
}

#[test]
fn test_irgen_pragma_directives_noop() {
    let (llvm_ir, engine) = run_ir_gen(
        "#error \"test error\"
#warning \"test warning\"
func foo() -> Int32 { 1 }",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("foo"));
}

#[test]
fn test_irgen_conditional_block_function_call() {
    let (llvm_ir, engine) = run_ir_gen(
        "#if A
func foo() -> Int32 { 42 }
#endif
func test() -> Int32 { foo() }",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("foo"));
    assert!(llvm_ir.contains("test"));
}

#[test]
fn test_irgen_sizeof_int32() {
    let (llvm_ir, engine) = run_ir_gen("func test() -> UInt64 { return sizeof(Int32) }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("test"));
}

#[test]
fn test_irgen_sizeof_pointer() {
    let (llvm_ir, engine) =
        run_ir_gen("struct Foo { let x: Int32 } func test() -> UInt64 { return sizeof(Foo) }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(llvm_ir.contains("test"));
    assert!(llvm_ir.contains("Foo"));
}

#[test]
fn test_irgen_asm_block_no_operands() {
    let (llvm_ir, engine) = run_ir_gen("func test() { asm { \"nop\" } }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(
        llvm_ir.contains("asm sideeffect"),
        "Expected inline asm in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_asm_block_with_input() {
    let (llvm_ir, engine) =
        run_ir_gen(r#"func test() { var x: Int32 = 10; asm { "nop" : : val = in(reg) x } }"#);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(
        llvm_ir.contains("asm sideeffect"),
        "Expected inline asm in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_asm_block_with_output() {
    let (llvm_ir, engine) = run_ir_gen(
        r#"func test() { var x: Int32 = 0; asm { "mov {dst}, 42" : dst = out(reg) x } }"#,
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(
        llvm_ir.contains("asm sideeffect"),
        "Expected inline asm in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_asm_block_with_clobbers() {
    let (llvm_ir, engine) =
        run_ir_gen(r#"func test() { var x: Int32 = 0; asm { "nop" : : : "rax", "rbx" } }"#);
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(errors.len(), 0, "Expected no errors, got: {:?}", errors);
    assert!(
        llvm_ir.contains("asm sideeffect"),
        "Expected inline asm in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_do_expression_void() {
    let (_llvm_ir, engine) = run_ir_gen("func test() { do {} }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for do with empty body, got: {:?}",
        errors
    );
}

#[test]
fn test_irgen_do_expression_int_value() {
    let (llvm_ir, engine) = run_ir_gen("func test() -> Int32 { do { 1 } }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for do returning Int32, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "Expected return of i32 from do expression:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_do_expression_as_initializer() {
    let (_llvm_ir, engine) = run_ir_gen("func test() { let x = do { 42 } }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for do as initializer, got: {:?}",
        errors
    );
}

#[test]
fn test_irgen_nested_do_expression() {
    let (llvm_ir, engine) = run_ir_gen("func test() -> Int32 { do { do { 1 } } }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for nested do, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "Nested do should return i32:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_do_expression_with_variables() {
    let (llvm_ir, engine) =
        run_ir_gen("func test() -> Int32 { do { let a = 10 let b = 20 a + b } }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for do with variables, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "Do with variables should return i32:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_yield_in_function_returns() {
    let (llvm_ir, engine) = run_ir_gen("func test() -> Int32 { yield 42 }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for yield in function, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "yield in function should generate ret i32:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_yield_in_do_expression() {
    let (llvm_ir, engine) = run_ir_gen("func test() -> Int32 { do { yield 42 } }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for yield in do, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "yield in do should generate ret i32:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_yield_in_do_with_early_exit() {
    let (llvm_ir, engine) = run_ir_gen("func test() -> Int32 { do { if true { yield 42 }; 10 } }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for yield with early exit in do, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "yield with early exit should generate ret i32:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_yield_as_variable_initializer() {
    let (llvm_ir, engine) =
        run_ir_gen("func test() -> Int32 { let x = do { yield 42 }; return x }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for yield as variable initializer, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret i32"),
        "yield as initializer should generate ret i32:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_yield_in_function_void() {
    let (llvm_ir, engine) = run_ir_gen("func test() { yield }");
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for yield in void function, got: {:?}",
        errors
    );
    assert!(
        llvm_ir.contains("ret void"),
        "yield in void function should generate ret void:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_inline_class_init() {
    let (llvm_ir, engine) = run_ir_gen(
        "class Point { let x: Int32 }
         func test() -> Int32 { let p: inline Point = Point(1) return p.x }",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for inline class init, got: {:?}",
        errors
    );
    assert!(
        !llvm_ir.contains("malloc"),
        "Inline class should not use malloc:\n{}",
        llvm_ir
    );
    assert!(
        llvm_ir.contains("class.Point"),
        "Expected class.Point struct in IR:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_inline_class_no_size() {
    let (llvm_ir, engine) = run_ir_gen(
        "class Point { let x: Int32 }
         func test() -> Int32 { let p: inline Point = Point(1) return p.x }",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for inline class without size, got: {:?}",
        errors
    );
    assert!(
        !llvm_ir.contains("malloc"),
        "Inline class should not use malloc:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_inline_class_explicit_size() {
    let (llvm_ir, engine) = run_ir_gen(
        "class Point { let x: Int32 }
         func test() -> Int32 { let p: inline<256> Point = Point(1) return p.x }",
    );
    let engine_ref = engine.borrow();
    let errors = engine_ref.get_errors();
    assert_eq!(
        errors.len(),
        0,
        "Expected no errors for inline<256> class, got: {:?}",
        errors
    );
    assert!(
        !llvm_ir.contains("malloc"),
        "Inline class should not use malloc:\n{}",
        llvm_ir
    );
}

#[test]
fn test_irgen_const_generic_function_decl() {
    let code = "func foo<let N: Int32>(x: Int32) -> Int32 { return x }
                 func test() -> Int32 { return foo(x: 42) }";
    let engine = create_engine();
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
    type_resolver.resolve(&program, module_id.clone());
    let context = Context::create();
    let ir_gen = IRGenerator::new(&context, engine.clone());
    let module = ir_gen.generate(&program, module_id.borrow().scope.clone().unwrap());
    let llvm_ir = module.print_to_string().to_string();
    assert_eq!(
        engine.borrow().get_errors().len(),
        0,
        "no errors expected, got: {:?}",
        engine.borrow().get_diagnostics()
    );
}
