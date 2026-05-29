use std::{cell::RefCell, rc::Rc};

use inkwell::context::Context;
use truss::{
    diag::TrussDiagnosticEngine,
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

    assert!(llvm_ir.contains("class.Point"), "Expected class.Point in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Point.f"), "Expected Point.f in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("vtable.Animal"), "Expected vtable type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__vtable.Animal"), "Expected vtable global in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Animal.speak"), "Expected Animal.speak in vtable:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Animal.eat"), "Expected Animal.eat in vtable:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("vtable.Greeter"), "Expected vtable type:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__vtable.Greeter"), "Expected vtable global:\n{}", llvm_ir);
    assert!(llvm_ir.contains("call"), "Expected a call instruction:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Greeter.greet"), "Expected Greeter.greet function:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("vtable.Animal"), "Expected vtable.Animal:\n{}", llvm_ir);
    assert!(llvm_ir.contains("vtable.Dog"), "Expected vtable.Dog:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__vtable.Animal"), "Expected __vtable.Animal:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__vtable.Dog"), "Expected __vtable.Dog:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Dog.speak"), "Expected Dog.speak in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Animal.speak"), "Expected Animal.speak in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("class.Dog"), "Expected class.Dog in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("{ ptr, i64, i32, i32 }"), "Expected Dog type [vtable_ptr, ref_count, name, breed] in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("class.Dog"), "Expected class.Dog in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("load"), "Expected load instruction in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("class.Dog"), "Expected class.Dog in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("{ ptr, i64, i32, i32, i32 }"), "Expected Dog type [vtable_ptr, ref_count, a, b, c] in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("getelementptr"), "Expected GEP instructions in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Bool"), "Expected tuple struct type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("{ i32, i1 }"), "Expected tuple layout (Int32, Bool) in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("@Point.identity"), "Expected Point.identity function:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("@Point.init"), "Expected Point.init function:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Int32"), "Expected tuple struct type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("{ i32, i32 }"), "Expected tuple layout (Int32, Int32) in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("getelementptr"), "Expected GEP for field access in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Bool"), "Expected tuple struct type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("{ i32, i1 }"), "Expected tuple layout (Int32, Bool) in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("getelementptr"), "Expected GEP for field access in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Bool"), "Expected tuple struct type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("{ i32, i1 }"), "Expected tuple layout (Int32, Bool) in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Bool"), "Expected tuple struct type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("{ i32, i1 }"), "Expected tuple layout (Int32, Bool) in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Bool"), "Expected tuple struct type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("getelementptr"), "Expected GEP for named field access in IR:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Bool"), "Expected tuple struct type in IR:\n{}", llvm_ir);
    assert!(llvm_ir.contains("getelementptr"), "Expected GEP for positional access in IR:\n{}", llvm_ir);
}

#[test]
fn test_irgen_named_tuple_type_annotation() {
    let code = "func test() -> Int32 { let t: (a: Int32, b: Bool) = (a: 10, b: false); return t.a }";
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

    assert!(llvm_ir.contains("tuple.__tuple_Int32_Bool"), "Expected tuple struct type in IR:\n{}", llvm_ir);
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

    assert!(!llvm_ir.contains("Drawable.draw"), "Method requirement without body should not generate IR function");
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

    assert!(llvm_ir.contains("Greeter.greet"), "Default implementation should generate function:\n{}", llvm_ir);
}

#[test]
fn test_irgen_protocol_default_impl_no_crash() {
    let code = "protocol Helper { func help() -> Int32 { return 42 } func need() -> Void { return } }";
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

    assert!(llvm_ir.contains("Helper.help"), "Default impl 'help' should generate function:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Helper.need"), "Default impl 'need' should generate function:\n{}", llvm_ir);
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

    assert!(!llvm_ir.contains("Drawable.draw"), "Requirement-only 'draw' should NOT generate function:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Helper.help"), "Default impl 'help' should generate function:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Helper.need"), "Default impl 'need' should generate function:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Drawable.draw"), "Default implementation should generate function:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("__protocol_wt.Drawable.Circle"), "Witness table for (Drawable, Circle) should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("existential.Drawable"), "Existential container type should exist:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("__protocol_wt.Drawable.Circle"), "Witness table for (Drawable, Circle) struct should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("existential.Drawable"), "Existential container type should exist:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("existential.Drawable & Resettable"), "Existential container for compound type should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__protocol_wt.Drawable.Circle"), "Witness table for (Drawable, Circle) should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__protocol_wt.Resettable.Circle"), "Witness table for (Resettable, Circle) should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Circle.draw"), "Circle.draw function should exist:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("vtable.ViewModel"), "vtable type should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__vtable.ViewModel"), "vtable global should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("ViewModel.value.getter"), "getter function should be in vtable:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("vtable.ViewModel"), "vtable type should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("ViewModel.value.getter"), "getter should be in vtable:\n{}", llvm_ir);
    assert!(llvm_ir.contains("ViewModel.value.setter"), "setter should be in vtable:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("vtable.Base"), "vtable.Base should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("vtable.Derived"), "vtable.Derived should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__vtable.Base"), "__vtable.Base should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__vtable.Derived"), "__vtable.Derived should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Base.value.getter"), "Base getter should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Derived.value.getter"), "Derived getter should exist:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("__vtable.Data"), "vtable should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Data.name.getter"), "stored property should have getter:\n{}", llvm_ir);
    assert!(!llvm_ir.contains("Data.name.setter"), "let property should not have setter:\n{}", llvm_ir);
    assert!(llvm_ir.contains("call"), "should have indirect call:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Data.value.getter"), "var should have getter:\n{}", llvm_ir);
    assert!(llvm_ir.contains("Data.value.setter"), "var should have setter:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Data.deinit"), "struct should have deinit function:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Option.deinit"), "enum should have deinit function:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Counter.deinit"), "struct deinit function should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("call void @Counter.deinit"), "deinit should be called on scope exit:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Data.deinit"), "struct deinit function should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("call void @Data.deinit"), "deinit should be called on scope exit:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("Option.deinit"), "enum deinit function should exist:\n{}", llvm_ir);
    assert!(llvm_ir.contains("call void @Option.deinit"), "deinit should be called on scope exit:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("define i32 @Foo.bar"), "extension method should generate Foo.bar:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("define i32 @Point.getX"), "extension method self access should generate Point.getX:\n{}", llvm_ir);
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

    assert!(llvm_ir.contains("define i32 @Foo.req"), "extension method should generate Foo.req:\n{}", llvm_ir);
    assert!(llvm_ir.contains("__protocol_wt.P.Foo"), "protocol witness table for P+Foo should exist:\n{}", llvm_ir);
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
            assert!(llvm_ir.contains("match_exit"), "match should generate match_exit block:\n{}", llvm_ir);
            assert!(llvm_ir.contains("case_body"), "match should generate case_body blocks:\n{}", llvm_ir);
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
            assert!(llvm_ir.contains("test"), "LLVM IR should contain function test:\n{}", llvm_ir);
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
            assert!(llvm_ir.contains("i32"), "Typealias to Int32 should produce i32 in IR:\n{}", llvm_ir);
        }
        Err(_) => panic!("IR generation for typealias access panicked"),
    }
}
