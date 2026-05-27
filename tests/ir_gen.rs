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
    assert!(llvm_ir.contains("{ i64, i32, i32 }"), "Expected Dog type [ref_count, name, breed] in IR:\n{}", llvm_ir);
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
    assert!(llvm_ir.contains("{ i64, i32, i32, i32 }"), "Expected Dog type [ref_count, a, b, c] in IR:\n{}", llvm_ir);
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

