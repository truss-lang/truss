use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use crate::{
    ast::node::Program,
    diag::TrussDiagnosticEngine,
    ir_gen::IRGenerator,
    krate::Package,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    type_resolver::TypeResolver,
};

pub fn process_build_truss(
    project_dir: &Path,
    packages: &HashMap<String, Rc<RefCell<Package>>>,
) -> Option<Vec<String>> {
    let build_path = project_dir.join("Build.truss");
    if !build_path.exists() {
        return Some(Vec::new());
    }

    let content = std::fs::read_to_string(&build_path).ok()?;
    let file_rc = Rc::new(build_path.to_string_lossy().to_string());
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));

    let char_stream = CharStream::new(content, file_rc.clone());
    let mut lexer = Lexer::new(char_stream, engine.clone());
    let tokens = lexer.parse();
    if engine.borrow().has_errors() {
        return None;
    }

    let mut parser = Parser::new(file_rc.clone(), tokens, engine.clone());
    let program = parser.parse();
    if engine.borrow().has_errors() {
        return None;
    }

    let mut packages = packages.clone();
    let build_pkg = Rc::new(RefCell::new(Package::new("Build".to_string())));
    packages.insert("Build".to_string(), build_pkg.clone());

    let mut resolver = SymbolResolver::new(packages.clone(), "Build".to_string(), engine.clone());
    let dummy_prog = Program {
        file: Rc::new(String::new()),
        statements: Vec::new(),
    };
    let module = resolver.resolve(&dummy_prog, "Build".to_string());

    if let Some(scope) = module.borrow().scope.clone() {
        resolver.enter_scope(Some(scope));
    }

    for stmt in &program.statements {
        resolver.register_symbols(stmt.clone());
    }

    let mut type_resolver =
        TypeResolver::new(packages.clone(), "Build".to_string(), engine.clone());
    let empty_prog = Program {
        file: Rc::new(String::new()),
        statements: vec![],
    };
    type_resolver.resolve(&empty_prog, module.clone());

    if engine.borrow().has_errors() {
        return None;
    }

    let mut resolver2 = SymbolResolver::new(packages.clone(), "Build".to_string(), engine.clone());
    let main_module = resolver2.resolve(&program, "Build".to_string());
    let mut type_resolver2 =
        TypeResolver::new(packages.clone(), "Build".to_string(), engine.clone());
    type_resolver2.resolve(&program, main_module.clone());

    if engine.borrow().has_errors() {
        return None;
    }

    let context = inkwell::context::Context::create();
    let ir_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let scope = main_module.borrow().scope.clone();
    if let Some(s) = scope {
        let ir_generator =
            IRGenerator::new(&context, ir_engine.clone()).with_namespace("Build", "");
        let modules = ir_generator.generate_with_stdlib(&program, &[], s);
        if ir_engine.borrow().has_errors() {
            return None;
        }

        if let Some(func) = modules.main.get_function("main") {
            let engine = modules
                .main
                .create_jit_execution_engine(inkwell::OptimizationLevel::None);
            if let Ok(exec_engine) = engine {
                unsafe {
                    let _ = exec_engine.run_function(func, &[]);
                }
            }
        }
    }

    Some(Vec::new())
}
