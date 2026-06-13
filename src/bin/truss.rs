use std::{fs, path::Path};

use clap::{Parser, Subcommand};

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
    Init { name: String },
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
         \x20       Target(name: \"{name}\", kind: .Executable)\n\
         \x20   ]\n\
         )\n"
    );
    fs::write(root.join("Project.truss"), project_content).unwrap_or_else(|e| {
        eprintln!("Error: Failed to write Project.truss: {}", e);
        std::process::exit(1);
    });

    let main_content = "#[main]\nfunc main() {\n    \n}\n";
    fs::write(
        root.join("Sources").join(name).join("main.truss"),
        main_content,
    )
    .unwrap_or_else(|e| {
        eprintln!("Error: Failed to write main.truss: {}", e);
        std::process::exit(1);
    });

    println!("Created project '{}'", name);
}

fn cmd_build() {
    let mut orchestrator = match truss::trusspm::build::BuildOrchestrator::new(".") {
        Some(o) => o,
        None => {
            eprintln!("Error: Project.truss not found");
            std::process::exit(1);
        }
    };

    println!(
        "Building project '{}' v{}",
        orchestrator.manifest.name, orchestrator.manifest.version
    );

    orchestrator.run_all_passes(".");

    if orchestrator.has_errors() {
        eprintln!("Build failed");
        std::process::exit(1);
    }
}

fn cmd_run() {
    let mut orchestrator = match truss::trusspm::build::BuildOrchestrator::new(".") {
        Some(o) => o,
        None => {
            eprintln!("Error: Project.truss not found");
            std::process::exit(1);
        }
    };

    println!(
        "Building project '{}' v{}",
        orchestrator.manifest.name, orchestrator.manifest.version
    );

    orchestrator.run_all_passes(".");

    if orchestrator.has_errors() {
        eprintln!("Build failed");
        std::process::exit(1);
    }

    let output_path = match orchestrator.output_path {
        Some(ref p) => p.clone(),
        None => {
            eprintln!("Error: No output path from build");
            std::process::exit(1);
        }
    };

    println!("Running '{}'...", output_path);

    let status = std::process::Command::new(&output_path)
        .status()
        .unwrap_or_else(|e| {
            eprintln!("Error: Failed to run '{}': {}", output_path, e);
            std::process::exit(1);
        });

    std::process::exit(status.code().unwrap_or(0));
}
