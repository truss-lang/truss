pub mod build;
pub mod cli;
pub mod extractor;
pub mod lock;
pub mod manifest;
pub mod resolver;

use std::{cell::RefCell, path::Path, rc::Rc};

use crate::{
    ast::statement::Statement,
    diag::TrussDiagnosticEngine,
    lexer::{CharStream, Lexer},
    parser::Parser,
    trusspm::manifest::Manifest,
    trusspm::resolver::DependencyResolver,
};

/// Find the path to the standard library from the active toolchain.
/// Returns `Some(path)` if the toolchain version is set and its stdlib directory exists.
pub fn find_stdlib_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let trussup_dir = std::path::Path::new(&home).join(".trussup");

    let current_file = trussup_dir.join("current.txt");
    if let Ok(version) = std::fs::read_to_string(&current_file) {
        let version = version.trim().to_string();
        let toolchain_std = trussup_dir.join("toolchains").join(&version).join("stdlib");
        if toolchain_std.exists() {
            return Some(toolchain_std.to_string_lossy().to_string());
        }
    }

    let standalone_std = trussup_dir.join("stdlib");
    if standalone_std.exists() {
        return Some(standalone_std.to_string_lossy().to_string());
    }

    None
}

/// Parse standard library files from the given directory path.
/// Returns parsed statements and source contents.
pub fn parse_std_lib(
    stdlib_path: &str,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
) -> (Vec<Vec<Rc<RefCell<Statement>>>>, Vec<(String, String)>) {
    let dir = Path::new(stdlib_path);
    let project_file = dir.join("Project.truss");

    // Check for project structure (Project.truss + Sources/<name>/)
    if project_file.exists() {
        if let Ok(manifest) = Manifest::from_project_dir(stdlib_path, engine.clone()) {
            let source_files = DependencyResolver::discover_source_files(&manifest.name, dir);
            if !source_files.is_empty() {
                let mut results = Vec::new();
                let mut sources = Vec::new();

                for path in &source_files {
                    let content = match std::fs::read_to_string(path) {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    let file_path = path.to_string_lossy().to_string();
                    let file_rc = Rc::new(file_path.clone());

                    let file_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                    let char_stream = CharStream::new(content.clone(), file_rc.clone());
                    let mut lexer = Lexer::new(char_stream, file_engine.clone());
                    let tokens = lexer.parse();
                    if file_engine.borrow().has_errors() {
                        let formatted = file_engine.borrow().format_all_plain(&content);
                        if !formatted.is_empty() {
                            println!("{}", formatted);
                        }
                        engine.borrow_mut().extend(file_engine.take());
                        return (results, sources);
                    }

                    let mut parser = Parser::new(file_rc.clone(), tokens, file_engine.clone());
                    let program = parser.parse();
                    if file_engine.borrow().has_errors() {
                        let formatted = file_engine.borrow().format_all_plain(&content);
                        if !formatted.is_empty() {
                            println!("{}", formatted);
                        }
                        engine.borrow_mut().extend(file_engine.take());
                        return (results, sources);
                    }

                    engine.borrow_mut().extend(file_engine.take());
                    results.push(program.statements);
                    sources.push((file_path, content));
                }

                return (results, sources);
            }
        }
    }

    // Legacy flat directory fallback (read all .truss files directly)
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

        let file_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let char_stream = CharStream::new(content.clone(), file_rc.clone());
        let mut lexer = Lexer::new(char_stream, file_engine.clone());
        let tokens = lexer.parse();
        if file_engine.borrow().has_errors() {
            let formatted = file_engine.borrow().format_all_plain(&content);
            if !formatted.is_empty() {
                println!("{}", formatted);
            }
            engine.borrow_mut().extend(file_engine.take());
            return (results, sources);
        }

        let mut parser = Parser::new(file_rc.clone(), tokens, file_engine.clone());
        let program = parser.parse();
        if file_engine.borrow().has_errors() {
            let formatted = file_engine.borrow().format_all_plain(&content);
            if !formatted.is_empty() {
                println!("{}", formatted);
            }
            engine.borrow_mut().extend(file_engine.take());
            return (results, sources);
        }

        engine.borrow_mut().extend(file_engine.take());
        results.push(program.statements);
        sources.push((file_path, content));
    }

    (results, sources)
}
