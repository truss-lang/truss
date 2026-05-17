use std::{cell::RefCell, fs, rc::Rc};

use clap::Parser;
use truss::{
    diag::TrussDiagnosticEngine,
    lexer::{CharStream, Lexer},
    parser::Parser as TrussParser,
};

#[derive(Parser)]
#[command(name = "truss")]
#[command(about = "Truss compiler")]
#[command(long_about = None)]
#[command(infer_long_args = true)]
struct Cli {
    /// Input file to compile
    file: String,

    /// Print tokens
    #[arg(long, short = 't', default_value_t = false)]
    tokens: bool,

    /// Print AST
    #[arg(long, short = 'a', default_value_t = false)]
    ast: bool,

    /// Print all inspection info (tokens, ast)
    #[arg(long, short, default_value_t = false)]
    inspect: bool,
}

fn main() {
    let cli = Cli::parse();

    let content = fs::read_to_string(&cli.file).expect("Failed to read file");
    let file_rc = Rc::new(cli.file.clone());
    let char_stream = CharStream::new(content, file_rc.clone());
    let mut lexer = Lexer::new(char_stream);
    let tokens = lexer.parse();

    let inspect = cli.inspect || (cli.tokens && cli.ast);

    if inspect || cli.tokens {
        println!("=== Tokens ===");
        for token in &tokens {
            println!("{:?}", token);
        }
        println!();
    }

    if inspect || cli.ast {
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut parser = TrussParser::new(file_rc, tokens, engine);
        let program = parser.parse();
        println!("=== AST ===");
        println!("{:#?}", program);
    }
}
