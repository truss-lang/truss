use std::{cell::RefCell, fs, path::Path, rc::Rc};

use truss::diag::TrussDiagnosticEngine;
use truss::trusspm::manifest::Manifest;
use truss::trusspm::resolver::DependencyResolver;

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
    Check,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { name } => cmd_init(&name),
        Commands::Build => cmd_build(),
        Commands::Run => cmd_run(),
        Commands::Check => cmd_check(),
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
         \x20       Target(name: \"{name}\")\n\
         \x20   ],\n\
         \x20   products: [\n\
         \x20       Product(name: \"{name}\", type: .Executable, targets: [\"{name}\"])\n\
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
        orchestrator.manifest.name,
        orchestrator.manifest.version
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
        orchestrator.manifest.name,
        orchestrator.manifest.version
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

fn cmd_check() {
    let root = Path::new(".");
    let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
    let manifest = match Manifest::from_project_dir(".", engine) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut has_errors = false;

    if manifest.name.is_empty() {
        eprintln!("Error: project name is empty");
        has_errors = true;
    }

    if manifest.version.is_empty() {
        eprintln!("Error: project version is empty");
        has_errors = true;
    }

    for target in &manifest.targets {
        let src_dir = root.join("Sources").join(&target.name);
        if !src_dir.exists() {
            eprintln!(
                "Error: Target '{}' has no source directory Sources/{}",
                target.name, target.name
            );
            has_errors = true;
            continue;
        }
        let files = DependencyResolver::discover_source_files(&target.name, root);
        if files.is_empty() {
            eprintln!(
                "Error: Target '{}' has no source files in Sources/{}",
                target.name, target.name
            );
            has_errors = true;
        }
    }

    for product in &manifest.products {
        for target_name in &product.targets {
            if !manifest.targets.iter().any(|t| &t.name == target_name) {
                eprintln!(
                    "Error: Product '{}' references unknown target '{}'",
                    product.name, target_name
                );
                has_errors = true;
            }
        }
    }

    for target in &manifest.targets {
        for dep in &target.dependencies {
            let is_target = manifest.targets.iter().any(|t| &t.name == dep);
            let is_dependency = manifest.dependencies.iter().any(|d| &d.name == dep);
            if !is_target && !is_dependency {
                eprintln!(
                    "Error: Target '{}' has unknown dependency '{}'",
                    target.name, dep
                );
                has_errors = true;
            }
        }
    }

    if has_errors {
        std::process::exit(1);
    }

    println!("Project check passed");
}
