use std::{cell::RefCell, fs, path::Path, rc::Rc};

use clap::{Parser, Subcommand};
use truss::diag::TrussDiagnosticEngine;

#[derive(Parser)]
#[command(name = "truss")]
#[command(about = "Truss package manager")]
#[command(long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        name: String,
    },
    Build,
    Run,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { name } => cmd_init(&name),
        Commands::Build => cmd_build(),
        Commands::Run => cmd_run(),
    }
}

fn cmd_init(name: &str) {
    let root = Path::new(name);
    if root.exists() {
        eprintln!("Error: Directory '{}' already exists", name);
        return;
    }
    fs::create_dir_all(root.join("Sources").join(name)).unwrap_or_else(|e| {
        eprintln!("Error: Failed to create project structure: {}", e);
        std::process::exit(1);
    });

    let project_content = format!(
        "let project = Project(\n\
         \x20   name: \"{name}\",\n\
         \x20   version: \"0.1.0\",\n\
         \x20   targets: [\n\
         \x20       Target(name: \"{name}\", kind: \"executable\")\n\
         \x20   ]\n\
         )\n"
    );
    fs::write(root.join("Project.truss"), project_content).unwrap_or_else(|e| {
        eprintln!("Error: Failed to write Project.truss: {}", e);
        std::process::exit(1);
    });

    let main_content = "#[main]\nfunc main() {\n    \n}\n";
    fs::write(root.join("Sources").join(name).join("main.truss"), main_content).unwrap_or_else(
        |e| {
            eprintln!("Error: Failed to write main.truss: {}", e);
            std::process::exit(1);
        },
    );

    println!("Created project '{}'", name);
}

fn cmd_build() {
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let manifest = match truss::trusspm::manifest::Manifest::from_project_dir(".", engine) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    println!("Building project '{}' v{}", manifest.name, manifest.version);
}

fn cmd_run() {
    cmd_build();
    println!("Running...");
}
