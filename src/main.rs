use std::{cell::RefCell, fs, rc::Rc};

use clap::Parser;
use truss::{
    diag::TrussDiagnosticEngine,
    id::CrateId,
    krate::Crate,
    lexer::{CharStream, Lexer},
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

    let content = fs::read_to_string(&cli.file).expect("Failed to read file");
    let file_rc = Rc::new(cli.file.clone());
    let char_stream = CharStream::new(content.clone(), file_rc.clone());
    let mut lexer = Lexer::new(char_stream);
    let tokens = lexer.parse();

    if cli.inspect || cli.tokens {
        println!("=== Tokens ===");
        for token in &tokens {
            println!("{:?}", token);
        }
        println!();
    }

    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let mut parser = TrussParser::new(file_rc.clone(), tokens, engine.clone());
    let program = parser.parse();

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after parse) ===");
        println!("{:#?}", program);
    }

    let krate = Rc::new(RefCell::new(Crate::new("main".to_string(), CrateId { id: 0 })));
    let mut symbol_resolver = SymbolResolver::new(krate.clone(), engine.clone());
    let module_id = symbol_resolver.resolve(&program, file_rc.to_string());

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after symbol resolve) ===");
        println!("{:#?}", program);
    }

    let mut type_resolver = TypeResolver::new(krate.clone(), engine.clone());
    type_resolver.resolve(&program, module_id);

    if emit_diagnostics(&engine.borrow(), &content) {
        return;
    }

    if cli.inspect || cli.ast {
        println!("=== AST (after type resolve) ===");
        println!("{:#?}", program);
    }
}
