use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use clap::Parser;
use truss::{
    ast::{node::Program, statement::Statement},
    condition_eval::{TargetTriple, flatten_program, predefined_symbols},
    diag::TrussDiagnosticEngine,
    ir_gen::{IRGenerator, emit},
    krate::Package,
    lexer::{CharStream, Lexer},
    macro_expander::MacroExpander,
    parser::Parser as TrussParser,
    symbol_resolver::SymbolResolver,
    trusspm::{manifest::ProductType, parse_std_lib},
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
    shared: bool,
    #[arg(long)]
    r#static: bool,
    #[arg(long, short = 'o')]
    output: Option<String>,
    #[arg(long, short = 'D', value_name = "DEFINE")]
    define: Vec<String>,
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

fn get_product_type(cli: &Cli) -> ProductType {
    if cli.r#static {
        ProductType::Library(truss::trusspm::manifest::LibraryType::Static)
    } else if cli.shared {
        ProductType::Library(truss::trusspm::manifest::LibraryType::Dynamic)
    } else {
        ProductType::Executable
    }
}

fn get_output_path(cli: &Cli, kind: ProductType) -> String {
    if let Some(ref path) = cli.output {
        return path.clone();
    }
    let first_file = cli.files.first().map(|s| s.as_str()).unwrap_or("a");
    let stem = Path::new(first_file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("a");
    match kind {
        ProductType::Executable => format!("{}.out", stem),
        ProductType::Library(truss::trusspm::manifest::LibraryType::Dynamic) => {
            format!("lib{}.so", stem)
        }
        ProductType::Library(truss::trusspm::manifest::LibraryType::Static) => {
            format!("lib{}.a", stem)
        }
    }
}

fn main() {
    let cli = Cli::parse();

    if cli.files.is_empty() {
        eprintln!("Error: No input files specified");
        return;
    }

    let kind = get_product_type(&cli);
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
        let mut symbols = predefined_symbols(file_path);
        for d in &cli.define {
            if let Some((name, value)) = d.split_once('=') {
                symbols.insert(name.to_string(), Some(value.to_string()));
            } else {
                symbols.insert(d.clone(), None);
            }
        }
        flatten_program(&mut program.statements, &cond_triple, &mut symbols);

        if cli.inspect || cli.ast {
            println!("=== AST (after condition evaluation) ({}) ===", file_path);
            println!("{:#?}", program);
        }

        all_stmts.extend(program.statements);
    }

    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));

    let mut stdlib_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
    let stdlib_path = truss::trusspm::find_stdlib_path();
    if let Some(ref stdlib_path) = stdlib_path {
        let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
        packages.insert("Truss".to_string(), truss_pkg.clone());

        let (file_programs, _std_sources) = parse_std_lib(stdlib_path, engine.clone());

        for file_stmts in file_programs {
            for stmt in &file_stmts {
                stdlib_stmts.push(stmt.clone());
            }
        }

        let std_prog = Program {
            file: Rc::new("stdlib".to_string()),
            statements: stdlib_stmts.clone(),
        };
        let mut std_resolver =
            SymbolResolver::new(packages.clone(), "Truss".to_string(), engine.clone());
        let std_module = std_resolver.resolve(&std_prog, "Truss".to_string());

        let mut std_type_resolver = TypeResolver::new(
            packages.clone(),
            "Truss".to_string(),
            engine.clone(),
        );
        std_type_resolver.resolve(&std_prog, std_module);
    }

    let src_content = source_contents
        .first()
        .map(|(_, c)| c.as_str())
        .unwrap_or("");
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
