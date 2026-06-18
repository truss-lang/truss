use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use clap::Parser;
use truss::{
    ast::{node::Program, statement::Statement},
    condition_eval::{TargetTriple, flatten_program},
    diag::TrussDiagnosticEngine,
    ir_gen::{IRGenerator, emit},
    krate::Package,
    lexer::{CharStream, Lexer},
    macro_expander::MacroExpander,
    parser::Parser as TrussParser,
    symbol_resolver::SymbolResolver,
    trusspm::manifest::TargetKind,
    type_resolver::TypeResolver,
};

#[derive(Parser)]
#[command(name = "trussc")]
#[command(about = "Truss compiler")]
#[command(long_about = None)]
#[command(infer_long_args = true)]
struct Cli {
    files: Vec<String>,
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
    #[arg(long)]
    shared: bool,
    #[arg(long)]
    r#static: bool,
    #[arg(long, short = 'o')]
    output: Option<String>,
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

fn get_target_kind(cli: &Cli) -> TargetKind {
    if cli.r#static {
        TargetKind::StaticLibrary
    } else if cli.shared {
        TargetKind::DynamicLibrary
    } else {
        TargetKind::Executable
    }
}

fn get_output_path(cli: &Cli, kind: TargetKind) -> String {
    if let Some(ref path) = cli.output {
        return path.clone();
    }
    let first_file = cli.files.first().map(|s| s.as_str()).unwrap_or("a");
    let stem = Path::new(first_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("a");
    match kind {
        TargetKind::Executable => format!("{}.out", stem),
        TargetKind::DynamicLibrary => format!("lib{}.so", stem),
        TargetKind::StaticLibrary => format!("lib{}.a", stem),
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.files.is_empty() {
        eprintln!("Error: No input files specified");
        return;
    }

    let kind = get_target_kind(&cli);
    let output_path = get_output_path(&cli, kind);

    let target_triple = match &cli.target {
        Some(t) => TargetTriple::parse(t).to_triple_string(),
        None => TargetTriple::host().to_triple_string(),
    };

    let main_pkg = Rc::new(RefCell::new(Package::new("main".to_string())));
    let mut packages: HashMap<String, Rc<RefCell<Package>>> = HashMap::new();
    packages.insert("main".to_string(), main_pkg.clone());

    let mut all_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
    let mut source_contents: Vec<(String, String)> = Vec::new();

    for file_path in &cli.files {
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error: Cannot read file '{}': {}", file_path, e);
                return;
            }
        };
        source_contents.push((file_path.clone(), content));
    }

    for (file_path, content) in &source_contents {
        let file_rc = Rc::new(file_path.clone());
        let char_stream = CharStream::new(content.clone(), file_rc.clone());
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();

        if emit_diagnostics(&engine.borrow(), content) {
            return;
        }

        if cli.inspect || cli.tokens {
            println!("=== Tokens ({}) ===", file_path);
            for token in &tokens {
                println!("{:?}", token);
            }
            println!();
        }

        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut parser = TrussParser::new(file_rc.clone(), tokens, engine.clone());
        let mut program = parser.parse();

        if emit_diagnostics(&engine.borrow(), content) {
            return;
        }

        if cli.inspect || cli.ast {
            println!("=== AST (after parse) ({}) ===", file_path);
            println!("{:#?}", program);
        }

        let mut expander = MacroExpander::new(engine.clone());
        expander.expand(&mut program);

        if emit_diagnostics(&engine.borrow(), content) {
            return;
        }

        if cli.inspect || cli.ast {
            println!("=== AST (after macro expansion) ({}) ===", file_path);
            println!("{:#?}", program);
        }

        let cond_triple = match &cli.target {
            Some(t) => TargetTriple::parse(t),
            None => TargetTriple::host(),
        };
        flatten_program(&mut program.statements, &cond_triple);

        if cli.inspect || cli.ast {
            println!("=== AST (after condition evaluation) ({}) ===", file_path);
            println!("{:#?}", program);
        }

        all_stmts.extend(program.statements);
    }

    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));

    let mut stdlib_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
    if let Some(ref stdlib_path) = cli.stdlib_path {
        let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
        packages.insert("Truss".to_string(), truss_pkg.clone());

        let std_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let (file_programs, _std_sources) = parse_std_lib(stdlib_path, std_engine.clone());

        let has_std_errors = {
            let engine = std_engine.borrow();
            if engine.has_errors() {
                let formatted = engine.format_all_compact_plain();
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
            let std_prog = Program {
                file: Rc::new("stdlib".to_string()),
                statements: stdlib_stmts.clone(),
            };
            let mut std_resolver =
                SymbolResolver::new(packages.clone(), "Truss".to_string(), std_engine.clone());
            let std_module = std_resolver.resolve(&std_prog, "Truss".to_string());

            let stdlib_ty_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
            let mut std_type_resolver =
                TypeResolver::new(packages.clone(), "Truss".to_string(), stdlib_ty_engine.clone());
            std_type_resolver.resolve(&std_prog, std_module);
        }
    }

    let src_content = source_contents.first().map(|(_, c)| c.as_str()).unwrap_or("");
    let first_file = cli.files.first().cloned().unwrap_or_default();
    let combined_prog = Program {
        file: Rc::new(first_file),
        statements: all_stmts.clone(),
    };

    let mut symbol_resolver =
        SymbolResolver::new(packages.clone(), "main".to_string(), engine.clone());
    let module = symbol_resolver.resolve(&combined_prog, "main".to_string());

    if emit_diagnostics(&engine.borrow(), src_content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after symbol resolve) ===");
        println!("{:#?}", combined_prog);
    }

    let mut type_resolver = TypeResolver::new(packages.clone(), "main".to_string(), engine.clone());
    type_resolver.resolve(&combined_prog, module.clone());

    if emit_diagnostics(&engine.borrow(), src_content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after type resolve) ===");
        println!("{:#?}", combined_prog);
    }

    let context = inkwell::context::Context::create();
    let ir_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let ir_generator = IRGenerator::new(&context, ir_engine.clone()).with_namespace("main", "main");
    let modules = ir_generator.generate_with_stdlib(
        &combined_prog,
        &stdlib_stmts,
        module.borrow().scope.clone().unwrap(),
    );

    if emit_diagnostics(&ir_engine.borrow(), src_content) {
        return;
    }

    if cli.ir || cli.inspect {
        if let Some(stdlib_mod) = &modules.stdlib {
            let ir = stdlib_mod.print_to_string().to_string();
            println!("=== LLVM IR (stdlib) ===");
            println!("{}", ir);
        }
        let ir = modules.main.print_to_string().to_string();
        println!("=== LLVM IR (main) ===");
        println!("{}", ir);
    }

    // Link stdlib module when available. Generic function bodies are skipped
    // during compilation, but vtables use null entries for them, so linking
    // non-generic stdlib functions is safe.
    let link_stdlib = modules.stdlib.as_deref();

    match emit::emit_output(
        &modules.main,
        link_stdlib,
        &target_triple,
        &output_path,
        kind,
    ) {
        Ok(()) => {
            if !cli.ir && !cli.inspect && !cli.tokens && !cli.ast {
                println!("Emitted: {}", output_path);
            }
        }
        Err(e) => {
            eprintln!("Emit failed: {}", e);
        }
    }
}

fn parse_std_lib(
    stdlib_path: &str,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
) -> (Vec<Vec<Rc<RefCell<Statement>>>>, Vec<(String, String)>) {
    let dir = Path::new(stdlib_path);
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "truss"))
            .collect(),
        Err(_) => return (Vec::new(), Vec::new()),
    };
    entries.sort_by_key(|e| e.file_name());

    let mut results = Vec::new();
    let mut sources = Vec::new();

    for entry in entries {
        let path = entry.path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let file_path = path.to_string_lossy().to_string();
        let file_rc = Rc::new(file_path.clone());

        let char_stream = CharStream::new(content.clone(), file_rc.clone());
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();

        if engine.borrow().has_errors() {
            return (results, sources);
        }

        let mut parser = TrussParser::new(file_rc.clone(), tokens, engine.clone());
        let program = parser.parse();

        if engine.borrow().has_errors() {
            return (results, sources);
        }

        results.push(program.statements);
        sources.push((file_path, content));
    }

    (results, sources)
}
