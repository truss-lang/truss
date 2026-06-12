use std::{cell::RefCell, collections::HashMap, fs, rc::Rc};

use clap::Parser;
use truss::{
    ast::statement::Statement,
    condition_eval::{TargetTriple, flatten_program},
    diag::TrussDiagnosticEngine,
    ir_gen::IRGenerator,
    krate::Package,
    lexer::{CharStream, Lexer},
    macro_expander::MacroExpander,
    parser::Parser as TrussParser,
    symbol_resolver::SymbolResolver,
    type_resolver::TypeResolver,
};

#[derive(Parser)]
#[command(name = "truss")]
#[command(about = "Truss compiler")]
#[command(long_about = None)]
#[command(infer_long_args = true)]
struct Cli {
    file: String,
    #[arg(long, short, default_value_t = false)]
    tokens: bool,
    #[arg(long, short, default_value_t = false)]
    ast: bool,
    #[arg(long, short, default_value_t = false)]
    inspect: bool,
    #[arg(long, default_value_t = false)]
    ir: bool,
    #[arg(long)]
    target: Option<String>,
    #[arg(long)]
    stdlib_path: Option<String>,
}

fn emit_diagnostics(engine: &TrussDiagnosticEngine, content: &str) -> bool {
    let formatted = engine.format_all_plain(content);
    if !formatted.is_empty() {
        println!("{}", formatted);
        true
    } else {
        false
    }
}

fn main() {
    let cli = Cli::parse();

    let content = match fs::read_to_string(&cli.file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Cannot read file '{}': {}", cli.file, e);
            return;
        }
    };
    let file_rc = Rc::new(cli.file.clone());
    let char_stream = CharStream::new(content.clone(), file_rc.clone());
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut lexer = Lexer::new(char_stream, engine.clone());
    let tokens = lexer.parse();

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.tokens {
        println!("=== Tokens ===");
        for token in &tokens {
            println!("{:?}", token);
        }
        println!();
    }

    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut parser = TrussParser::new(file_rc.clone(), tokens, engine.clone());
    let mut program = parser.parse();

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after parse) ===");
        println!("{:#?}", program);
    }

    let mut expander = MacroExpander::new(engine.clone());
    expander.expand(&mut program);

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after macro expansion) ===");
        println!("{:#?}", program);
    }

    let target_triple = match &cli.target {
        Some(t) => TargetTriple::parse(t),
        None => TargetTriple::host(),
    };
    flatten_program(&mut program.statements, &target_triple);

    if cli.inspect || cli.ast {
        println!("=== AST (after condition evaluation) ===");
        println!("{:#?}", program);
    }

    let main_pkg = Rc::new(RefCell::new(Package::new("main".to_string())));
    let mut packages: HashMap<String, Rc<RefCell<Package>>> = HashMap::new();
    packages.insert("main".to_string(), main_pkg.clone());

    let mut stdlib_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();

    if let Some(ref stdlib_path) = cli.stdlib_path {
        let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
        packages.insert("Truss".to_string(), truss_pkg.clone());

        let std_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let (file_programs, _std_sources) =
            truss::std_lib::parse_std_lib(stdlib_path, std_engine.clone());

        let has_std_errors = {
            let engine = std_engine.borrow();
            if engine.has_errors() {
                let formatted = duck_diagnostic::format_all_smart(&*engine, false);
                if !formatted.is_empty() {
                    println!("{}", formatted);
                }
                true
            } else {
                false
            }
        };

        for file_stmts in file_programs {
            for stmt in &file_stmts {
                stdlib_stmts.push(stmt.clone());
            }
        }

        if !has_std_errors {
            let mut std_resolver =
                SymbolResolver::new(packages.clone(), "Truss".to_string(), engine.clone());
            let dummy_program = truss::ast::node::Program {
                file: Rc::new("".to_string()),
                statements: Vec::new(),
            };
            let std_module = std_resolver.resolve(&dummy_program, "Truss".to_string());

            if let Some(scope) = std_module.borrow().scope.clone() {
                std_resolver.enter_scope(Some(scope));
            }

            for stmt in &stdlib_stmts {
                std_resolver.register_symbols(stmt.clone());
            }

            let mut std_type_resolver =
                TypeResolver::new(packages.clone(), "Truss".to_string(), engine.clone());
            let empty_prog = truss::ast::node::Program {
                file: Rc::new("".to_string()),
                statements: vec![],
            };
            std_type_resolver.resolve(&empty_prog, std_module);
        }
    }

    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "main".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&program, file_rc.to_string());

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after symbol resolve) ===");
        println!("{:#?}", program);
    }

    let mut type_resolver = TypeResolver::new(packages.clone(), "main".to_string(), engine.clone());
    type_resolver.resolve(&program, module.clone());

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after type resolve) ===");
        println!("{:#?}", program);
    }

    let context = inkwell::context::Context::create();
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let ir_generator = IRGenerator::new(&context, engine.clone());
    let module = ir_generator.generate_with_stdlib(
        &program,
        &stdlib_stmts,
        module.borrow().scope.clone().unwrap(),
    );

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.ir || cli.inspect {
        let ir_content = module.print_to_string().to_string();
        println!("=== LLVM IR ===");
        println!("{}", ir_content);
    }
}
