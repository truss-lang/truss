use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;

#[derive(Parser)]
#[command(name = "trussup")]
#[command(about = "Truss toolchain manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install { version: String },
    InstallStd,
    UpdateStd,
    List,
    ListRemote,
    Current,
    Remove { version: String },
    RemoveStd,
    Use { version: String },
    Update,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Install { version } => cmd_install(&version),
        Commands::InstallStd => cmd_install_std(),
        Commands::UpdateStd => cmd_update_std(),
        Commands::List => cmd_list(),
        Commands::ListRemote => cmd_list_remote(),
        Commands::Current => cmd_current(),
        Commands::Remove { version } => cmd_remove(&version),
        Commands::RemoveStd => cmd_remove_std(),
        Commands::Use { version } => cmd_use(&version),
        Commands::Update => cmd_update(),
    }
}

fn home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else {
        PathBuf::from(".")
    }
}

fn get_trussup_dir() -> PathBuf {
    let dir = home_dir().join(".trussup");
    std::fs::create_dir_all(&dir).ok();
    dir
}

fn get_current_version() -> Option<String> {
    let current_file = get_trussup_dir().join("current.txt");
    std::fs::read_to_string(current_file)
        .ok()
        .map(|s| s.trim().to_string())
}

fn set_current_version(version: &str) {
    let current_file = get_trussup_dir().join("current.txt");
    std::fs::write(&current_file, version).unwrap_or_else(|e| {
        eprintln!("Error: failed to write current version: {}", e);
        std::process::exit(1);
    });
}

fn cmd_install(version: &str) {
    let toolchains_dir = get_trussup_dir().join("toolchains").join(version);

    if toolchains_dir.exists() {
        eprintln!("Error: toolchain '{}' is already installed", version);
        std::process::exit(1);
    }

    std::fs::create_dir_all(&toolchains_dir).unwrap_or_else(|e| {
        eprintln!("Error: failed to create toolchain directory: {}", e);
        std::process::exit(1);
    });

    let status = Command::new("cargo")
        .args(["build", "--release", "--bin", "truss", "--bin", "trussc"])
        .status();

    match status {
        Ok(s) if s.success() => {
            let target_dir = PathBuf::from("target/release");
            let truss_bin = target_dir.join("truss");
            let trussc_bin = target_dir.join("trussc");

            std::fs::copy(&truss_bin, toolchains_dir.join("truss")).unwrap_or_else(|e| {
                eprintln!("Error: failed to copy truss binary: {}", e);
                std::process::exit(1);
            });
            std::fs::copy(&trussc_bin, toolchains_dir.join("trussc")).unwrap_or_else(|e| {
                eprintln!("Error: failed to copy trussc binary: {}", e);
                std::process::exit(1);
            });

            // Download standard library
            let std_dir = toolchains_dir.join("stdlib");
            let std_url = "https://github.com/truss-lang/truss-std.git";
            let std_status = Command::new("git")
                .args(["clone", "--depth", "1", std_url, &std_dir.to_string_lossy()])
                .status();
            match std_status {
                Ok(s) if s.success() => {
                    println!("Downloaded std library");
                }
                _ => {
                    eprintln!("Warning: failed to download std library from {}", std_url);
                }
            }

            println!("Installed toolchain '{}'", version);
            set_current_version(version);
        }
        Ok(s) => {
            eprintln!("Error: cargo build failed with status: {}", s);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: failed to run cargo build: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_install_std() {
    let std_dir = home_dir().join(".trussup").join("stdlib");
    if std_dir.exists() {
        eprintln!(
            "Error: std library is already installed at {}",
            std_dir.display()
        );
        std::process::exit(1);
    }
    std::fs::create_dir_all(std_dir.parent().unwrap()).ok();
    let url = "https://github.com/truss-lang/truss-std.git";
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url, &std_dir.to_string_lossy()])
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("Installed std library to {}", std_dir.display());
        }
        Ok(s) => {
            eprintln!("Error: git clone failed with status: {}", s);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: failed to run git: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_update_std() {
    let std_dir = home_dir().join(".trussup").join("stdlib");
    if !std_dir.exists() {
        eprintln!("Error: std library is not installed. Use 'install-std' first.");
        std::process::exit(1);
    }
    let status = Command::new("git")
        .args(["-C", &std_dir.to_string_lossy(), "pull"])
        .status();
    match status {
        Ok(s) if s.success() => {
            println!("Updated std library at {}", std_dir.display());
        }
        Ok(s) => {
            eprintln!("Error: git pull failed with status: {}", s);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: failed to run git: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_remove_std() {
    let std_dir = home_dir().join(".trussup").join("stdlib");
    if !std_dir.exists() {
        eprintln!("Error: std library is not installed");
        std::process::exit(1);
    }
    std::fs::remove_dir_all(&std_dir).unwrap_or_else(|e| {
        eprintln!("Error: failed to remove std library: {}", e);
        std::process::exit(1);
    });
    println!("Removed std library");
}

fn cmd_list() {
    let toolchains_dir = get_trussup_dir().join("toolchains");
    if !toolchains_dir.exists() {
        println!("No toolchains installed");
        return;
    }

    let current = get_current_version();
    let mut entries: Vec<_> = match std::fs::read_dir(&toolchains_dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => Vec::new(),
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if current.as_deref() == Some(&name) {
            println!("{} (current)", name);
        } else {
            println!("{}", name);
        }
    }
}

fn cmd_list_remote() {
    let url = "https://github.com/truss-lang/truss.git";
    let output = Command::new("git")
        .args(["ls-remote", "--tags", "--refs", url])
        .output();
    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let mut versions: Vec<&str> = stdout
                .lines()
                .filter_map(|line| line.split('\t').nth(1))
                .filter_map(|refname| refname.strip_prefix("refs/tags/"))
                .collect();
            if versions.is_empty() {
                println!("No remote versions available");
                return;
            }
            versions.sort_by(|a, b| b.cmp(a));
            for v in &versions {
                println!("{}", v);
            }
        }
        _ => {
            eprintln!("Error: failed to fetch remote versions");
            std::process::exit(1);
        }
    }
}

fn cmd_current() {
    match get_current_version() {
        Some(v) => println!("{}", v),
        None => {
            eprintln!("Error: no active toolchain set");
            std::process::exit(1);
        }
    }
}

fn cmd_remove(version: &str) {
    let toolchain_dir = get_trussup_dir().join("toolchains").join(version);
    if !toolchain_dir.exists() {
        eprintln!("Error: toolchain '{}' is not installed", version);
        std::process::exit(1);
    }

    std::fs::remove_dir_all(&toolchain_dir).unwrap_or_else(|e| {
        eprintln!("Error: failed to remove toolchain: {}", e);
        std::process::exit(1);
    });

    if get_current_version().as_deref() == Some(version) {
        let current_file = get_trussup_dir().join("current.txt");
        let _ = std::fs::remove_file(&current_file);
    }

    println!("Removed toolchain '{}'", version);
}

fn cmd_use(version: &str) {
    let toolchain_dir = get_trussup_dir().join("toolchains").join(version);
    if !toolchain_dir.exists() {
        eprintln!("Error: toolchain '{}' is not installed", version);
        std::process::exit(1);
    }

    set_current_version(version);
    println!("Using toolchain '{}'", version);
}

fn cmd_update() {
    let repo_url = "https://github.com/truss-lang/truss.git";
    let temp_dir = std::env::temp_dir().join("truss-update");

    let _ = std::fs::remove_dir_all(&temp_dir);

    println!("Cloning truss repository from {}...", repo_url);
    let clone_status = Command::new("git")
        .args(["clone", "--depth", "1", repo_url, &temp_dir.to_string_lossy()])
        .status();
    match clone_status {
        Ok(s) if s.success() => {
            println!("Repository cloned successfully");
        }
        Ok(s) => {
            eprintln!("Error: git clone failed with status: {}", s);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: failed to run git: {}", e);
            std::process::exit(1);
        }
    }

    println!("Building truss toolchain (release mode)...");
    let build_status = Command::new("cargo")
        .args([
            "build",
            "--release",
            "--bin",
            "truss",
            "--bin",
            "trussc",
            "--bin",
            "truss-lsp",
            "--bin",
            "trussup",
        ])
        .current_dir(&temp_dir)
        .status();
    match build_status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!("Error: cargo build failed with status: {}", s);
            let _ = std::fs::remove_dir_all(&temp_dir);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: failed to run cargo: {}", e);
            let _ = std::fs::remove_dir_all(&temp_dir);
            std::process::exit(1);
        }
    }

    let version = "latest";
    let toolchains_dir = get_trussup_dir().join("toolchains").join(version);
    std::fs::create_dir_all(&toolchains_dir).unwrap_or_else(|e| {
        eprintln!("Error: failed to create toolchain directory: {}", e);
        std::process::exit(1);
    });

    let target_dir = temp_dir.join("target").join("release");
    for bin in &["truss", "trussc", "truss-lsp", "trussup"] {
        let src = target_dir.join(bin);
        let dst = toolchains_dir.join(bin);
        std::fs::copy(&src, &dst).unwrap_or_else(|e| {
            eprintln!("Error: failed to copy {}: {}", bin, e);
            std::process::exit(1);
        });
    }
    println!("Copied binaries to {}", toolchains_dir.display());

    let std_url = "https://github.com/truss-lang/truss-std.git";
    let std_dir = toolchains_dir.join("stdlib");
    if std_dir.exists() {
        let _ = std::fs::remove_dir_all(&std_dir);
    }
    let std_status = Command::new("git")
        .args(["clone", "--depth", "1", std_url, &std_dir.to_string_lossy()])
        .status();
    match std_status {
        Ok(s) if s.success() => {
            println!("Downloaded std library");
        }
        _ => {
            eprintln!("Warning: failed to download std library from {}", std_url);
        }
    }

    set_current_version(version);

    let _ = std::fs::remove_dir_all(&temp_dir);

    println!("Update complete! Active toolchain: {} (latest)", version);
}
