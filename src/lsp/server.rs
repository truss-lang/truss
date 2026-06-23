use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::rc::Rc;

use serde_json::{Value, json};

use duck_diagnostic::DiagnosticCode;

use crate::ast::node::Program;
use crate::ast::statement::{FunctionBody, ProtocolMember, Statement};
use crate::diag::TrussDiagnosticEngine;
use crate::krate::{Module, Package};
use crate::lexer::token::{KeywordType, SeparatorType, Token, TokenType};
use crate::lexer::{CharStream, Lexer};
use crate::parser::Parser;
use crate::scope::Scope;
use crate::symbol::Symbol;
use crate::symbol_resolver::SymbolResolver;
use crate::trusspm::manifest::Manifest;
use crate::trusspm::resolver::DependencyResolver;
use crate::type_resolver::TypeResolver;
use crate::types::Type;

fn read_message(reader: &mut BufReader<io::StdinLock<'_>>) -> Option<String> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
            content_length = len_str.trim().parse().ok();
        }
    }
    let len = content_length?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

fn write_message(writer: &mut dyn Write, content: &str) {
    let bytes = content.as_bytes();
    let header = format!("Content-Length: {}\r\n\r\n", bytes.len());
    let _ = writer.write_all(header.as_bytes());
    let _ = writer.write_all(bytes);
    let _ = writer.flush();
}

fn collect_diagnostics_from_engine(engine: &TrussDiagnosticEngine, content: &str) -> Vec<Value> {
    collect_diagnostics_filtered(engine, content, None)
}

fn collect_diagnostics_filtered(
    engine: &TrussDiagnosticEngine,
    _content: &str,
    filter_file: Option<&str>,
) -> Vec<Value> {
    let mut diagnostics = Vec::new();
    for diag in engine.get_diagnostics() {
        let severity = match diag.severity {
            duck_diagnostic::Severity::Error | duck_diagnostic::Severity::Bug => 1,
            duck_diagnostic::Severity::Warning => 2,
            duck_diagnostic::Severity::Note => 3,
            duck_diagnostic::Severity::Help => 4,
        };
        let (start_line, start_col, span_length, diag_file) = if let Some(label) = diag.primary_label() {
            (
                label.span.line,
                label.span.column,
                label.span.length,
                Some(label.span.file.clone()),
            )
        } else {
            (1, 1, 1, None)
        };
        if let Some(ref filter) = filter_file {
            if let Some(ref file_arc) = diag_file {
                if file_arc.as_ref() != *filter {
                    continue;
                }
            }
        }
        diagnostics.push(json!({
            "range": {
                "start": {
                    "line": (start_line - 1) as u64,
                    "character": (start_col - 1) as u64
                },
                "end": {
                    "line": (start_line - 1) as u64,
                    "character": (start_col - 1 + span_length) as u64
                }
            },
            "severity": severity,
            "message": format!("[{}] {}", diag.code.code(), diag.message)
        }));
    }
    diagnostics
}

fn compute_hash(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

pub fn start_server() {
    let stdlib_path = crate::trusspm::find_stdlib_path();
    if let Some(ref path) = stdlib_path {
        eprintln!("truss-lsp: detected std library at {}", path);
    }
    let mut server = LanguageServer {
        documents: HashMap::new(),
        exit: false,
        stdlib_path,
        stdlib_scope: None,
        stdlib_cache: None,
        project_analyses: HashMap::new(),
        file_cache: HashMap::new(),
    };
    server.load_stdlib();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    while !server.exit {
        match read_message(&mut reader) {
            Some(body) => {
                let responses = server.handle_message(&body);
                for response in responses {
                    write_message(&mut io::stdout().lock(), &response);
                }
            }
            None => break,
        }
    }
}

struct ProjectAnalysis {
    scope: Rc<RefCell<Scope>>,
    #[allow(dead_code)]
    module: Rc<RefCell<Module>>,
}

struct CachedFile {
    hash: u64,
    statements: Vec<Rc<RefCell<Statement>>>,
}

struct StdlibCache {
    statements: Vec<Rc<RefCell<Statement>>>,
    sources: Vec<(String, String)>,
}

struct LanguageServer {
    documents: HashMap<String, String>,
    exit: bool,
    stdlib_path: Option<String>,
    stdlib_scope: Option<Rc<RefCell<Scope>>>,
    stdlib_cache: Option<StdlibCache>,
    project_analyses: HashMap<String, ProjectAnalysis>,
    file_cache: HashMap<String, CachedFile>,
}

impl LanguageServer {
    fn handle_message(&mut self, body: &str) -> Vec<String> {
        let msg: Value = match serde_json::from_str(body) {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };
        let method = match msg.get("method").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            None => return Vec::new(),
        };
        let id = msg.get("id").and_then(|v| v.as_u64());
        let params = msg.get("params");
        match method.as_str() {
            "initialize" => {
                vec![self.handle_initialize(id)]
            }
            "initialized" => Vec::new(),
            "textDocument/didOpen" => {
                self.handle_did_open(params);
                self.publish_diagnostics(params)
            }
            "textDocument/didChange" => {
                self.handle_did_change(params);
                self.publish_diagnostics(params)
            }
            "textDocument/didClose" => {
                self.handle_did_close(params);
                Vec::new()
            }
            "textDocument/didSave" => self.publish_diagnostics(params),
            "textDocument/completion" => {
                vec![self.handle_completion(id, params)]
            }
            "textDocument/hover" => {
                vec![self.handle_hover(id, params)]
            }
            "textDocument/definition" => {
                vec![self.handle_definition(id, params)]
            }
            "textDocument/signatureHelp" => {
                vec![self.handle_signature_help(id, params)]
            }
            "textDocument/documentSymbol" => {
                vec![self.handle_document_symbol(id, params)]
            }
            "textDocument/documentHighlight" => {
                vec![self.handle_document_highlight(id, params)]
            }
            "textDocument/foldingRange" => {
                vec![self.handle_folding_range(id, params)]
            }
            "textDocument/references" => {
                vec![self.handle_references(id, params)]
            }
            "textDocument/semanticTokens/full" => {
                vec![self.handle_semantic_tokens(id, params)]
            }
            "textDocument/inlayHint" => {
                vec![self.handle_inlay_hint(id, params)]
            }
            "shutdown" => {
                vec![json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string()]
            }
            "exit" => {
                self.exit = true;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    fn handle_initialize(&self, id: Option<u64>) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "capabilities": {
                    "textDocumentSync": 1,
                    "signatureHelpProvider": {
                        "triggerCharacters": ["("]
                    },
                    "completionProvider": {
                        "triggerCharacters": ["."]
                    },
                    "documentSymbolProvider": true,
                    "foldingRangeProvider": true,
                    "hoverProvider": true,
                    "definitionProvider": true,
                    "referencesProvider": true,
                    "inlayHintProvider": {
                        "resolveProvider": false
                    },
                    "semanticTokensProvider": {
                        "full": true,
                        "legend": {
                        "tokenTypes": [
                            "keyword", "type", "function", "variable",
                            "parameter", "string", "number", "comment",
                            "operator", "property", "macro"
                        ],
                            "tokenModifiers": ["declaration", "definition", "readonly", "static"]
                        }
                    }
                },
                "serverInfo": {
                    "name": "truss-lsp",
                    "version": "0.1.0",
                    "stdlibPath": self.stdlib_path
                }
            }
        })
        .to_string()
    }

    fn handle_did_open(&mut self, params: Option<&Value>) {
        if let Some(p) = params {
            if let Some(text_doc) = p.get("textDocument") {
                let uri = text_doc
                    .get("uri")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let text = text_doc
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.documents.insert(uri, text);
            }
        }
    }

    fn handle_did_change(&mut self, params: Option<&Value>) {
        if let Some(p) = params {
            let uri = p
                .get("textDocument")
                .and_then(|td| td.get("uri"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(content_changes) = p.get("contentChanges").and_then(|c| c.as_array()) {
                if let Some(change) = content_changes.first() {
                    if let Some(text) = change.get("text").and_then(|v| v.as_str()) {
                        self.documents.insert(uri, text.to_string());
                    }
                }
            }
        }
    }

    fn handle_did_close(&mut self, params: Option<&Value>) {
        if let Some(p) = params {
            let uri = p
                .get("textDocument")
                .and_then(|td| td.get("uri"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            self.documents.remove(&uri);
        }
    }

    fn publish_diagnostics(&mut self, params: Option<&Value>) -> Vec<String> {
        let uri = match params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
        {
            Some(u) => u.to_string(),
            None => return Vec::new(),
        };
        let content = match self.documents.get(&uri) {
            Some(c) => c.clone(),
            None => return Vec::new(),
        };
        let diagnostics = self.run_diagnostics(&uri, &content);
        let notification = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": diagnostics
            }
        });
        vec![notification.to_string()]
    }

    fn run_diagnostics(&mut self, uri: &str, content: &str) -> Vec<Value> {
        let file_path = uri.strip_prefix("file://").unwrap_or(uri);
        let file_rc = Rc::new(file_path.to_string());
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let char_stream = CharStream::new(content.to_string(), file_rc.clone());
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();
        let mut diagnostics = collect_diagnostics_from_engine(&engine.borrow(), content);
        if engine.borrow().has_errors() {
            return diagnostics;
        }
        let parser_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut parser = Parser::new(file_rc, tokens, parser_engine.clone());
        let program = parser.parse();
        let parser_diags = collect_diagnostics_from_engine(&parser_engine.borrow(), content);
        diagnostics.extend(parser_diags);
        if parser_engine.borrow().has_errors() {
            return diagnostics;
        }

        let analysis_diags = self.run_full_analysis(file_path, content, &program);
        diagnostics.extend(analysis_diags);
        diagnostics
    }

    fn run_full_analysis(&mut self, file_path: &str, content: &str, program: &Program) -> Vec<Value> {
        let analysis_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));

        let mut packages: HashMap<String, Rc<RefCell<Package>>> = HashMap::new();
        let main_pkg = Rc::new(RefCell::new(Package::new("main".to_string())));
        packages.insert("main".to_string(), main_pkg.clone());

        let stdlib_stmts: Vec<Rc<RefCell<Statement>>>;
        if let Some(ref cache) = self.stdlib_cache {
            let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
            packages.insert("Truss".to_string(), truss_pkg.clone());
            stdlib_stmts = cache.statements.clone();

            let std_prog = Program {
                file: Rc::new("stdlib".to_string()),
                statements: stdlib_stmts.clone(),
            };
            let std_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
            let mut std_resolver =
                SymbolResolver::new(packages.clone(), "Truss".to_string(), std_engine.clone());
            let std_module = std_resolver.resolve(&std_prog, "Truss".to_string());

            {
                let stdlib_ty_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                let mut std_type_resolver = TypeResolver::new(
                    packages.clone(),
                    "Truss".to_string(),
                    stdlib_ty_engine.clone(),
                );
                std_type_resolver.resolve(&std_prog, std_module);
            }
        } else if let Some(ref stdlib_path) = self.stdlib_path {
            let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
            packages.insert("Truss".to_string(), truss_pkg.clone());

            let std_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
            let (file_programs, _) = crate::trusspm::parse_std_lib(stdlib_path, std_engine.clone());

            let mut stdlib_stmts_mut = Vec::new();
            {
                let mut ordered: Vec<_> = file_programs.into_iter().map(|stmts| {
                    let name = stmts.first().and_then(|s| {
                        let file = &*s.borrow().token().file;
                        std::path::Path::new(file).file_stem().and_then(|n| n.to_str()).map(|n| n.to_string())
                    }).unwrap_or_default();
                    let priority = match name.as_str() {
                        "Truss" => 0,
                        "Iterator" => 1,
                        _ => 2,
                    };
                    (priority, stmts)
                }).collect();
                ordered.sort_by_key(|(p, _)| *p);
                for (_, stmts) in ordered {
                    for stmt in stmts {
                        stdlib_stmts_mut.push(stmt.clone());
                    }
                }
            }
            stdlib_stmts = stdlib_stmts_mut;

            let std_prog = Program {
                file: Rc::new("stdlib".to_string()),
                statements: stdlib_stmts.clone(),
            };
            let mut std_resolver =
                SymbolResolver::new(packages.clone(), "Truss".to_string(), std_engine.clone());
            let std_module = std_resolver.resolve(&std_prog, "Truss".to_string());

            {
                let stdlib_ty_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                let mut std_type_resolver = TypeResolver::new(
                    packages.clone(),
                    "Truss".to_string(),
                    stdlib_ty_engine.clone(),
                );
                std_type_resolver.resolve(&std_prog, std_module);
            }
        } else {
            stdlib_stmts = Vec::new();
        }

        let mut all_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
        for stmt in &program.statements {
            all_stmts.push(stmt.clone());
        }
        for stmt in &stdlib_stmts {
            all_stmts.push(stmt.clone());
        }

        if let Some(proj_dir) = self.find_project_dir(file_path) {
            if let Ok(manifest) = Manifest::from_project_dir(
                &proj_dir,
                Rc::new(RefCell::new(TrussDiagnosticEngine::new())),
            ) {
                let pkg_name = &manifest.name;
                if !packages.contains_key(pkg_name) {
                    let pkg = Rc::new(RefCell::new(Package::new(pkg_name.clone())));
                    packages.insert(pkg_name.clone(), pkg);
                }

                let source_files =
                    DependencyResolver::discover_source_files(pkg_name, Path::new(&proj_dir));
                for path in &source_files {
                    let path_str = path.to_string_lossy().to_string();
                    if path_str == file_path {
                        continue;
                    }
                    if let Ok(file_content) = std::fs::read_to_string(path) {
                        let hash = compute_hash(&file_content);
                        if let Some(cached) = self.file_cache.get(&path_str) {
                            if cached.hash == hash {
                                for stmt in &cached.statements {
                                    all_stmts.push(stmt.clone());
                                }
                                continue;
                            }
                        }
                        let f_rc = Rc::new(path_str.clone());
                        let f_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                        let cs = CharStream::new(file_content, f_rc.clone());
                        let mut lx = Lexer::new(cs, f_engine.clone());
                        let toks = lx.parse();
                        if f_engine.borrow().has_errors() {
                            continue;
                        }
                        let mut pp = Parser::new(f_rc, toks, f_engine.clone());
                        let prog = pp.parse();
                        if f_engine.borrow().has_errors() {
                            continue;
                        }
                        let stmts: Vec<Rc<RefCell<Statement>>> = prog.statements;
                        for stmt in &stmts {
                            all_stmts.push(stmt.clone());
                        }
                        self.file_cache.insert(path_str, CachedFile { hash, statements: stmts });
                    }
                }

                // Resolve dependency packages using DependencyResolver
                let dep_packages = DependencyResolver::resolve(&manifest, Path::new(&proj_dir), Rc::new(RefCell::new(TrussDiagnosticEngine::new())));
                for (dep_name, dep_pkg) in dep_packages {
                    if dep_name == "Truss" || dep_name == *pkg_name { continue; }
                    if packages.contains_key(&dep_name) { continue; }
                    packages.insert(dep_name.clone(), dep_pkg.clone());
                }

                // Parse and resolve dependency source files so their symbols are available
                for dep in &manifest.dependencies {
                    if dep.name == "Truss" || dep.name == *pkg_name { continue; }
                    let dep_src_dir = DependencyResolver::dependency_source_dir(dep, Path::new(&proj_dir));
                    if !dep_src_dir.exists() { continue; }

                    let mut dep_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
                    if let Ok(entries) = std::fs::read_dir(&dep_src_dir) {
                        let mut truss_files: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .filter(|e| e.path().extension().is_some_and(|ext| ext == "truss"))
                            .collect();
                        truss_files.sort_by_key(|e| e.file_name());
                        for entry in truss_files {
                            let path = entry.path();
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                let path_str = path.to_string_lossy().to_string();
                                let hash = compute_hash(&content);
                                if let Some(cached) = self.file_cache.get(&path_str) {
                                    if cached.hash == hash {
                                        for stmt in &cached.statements {
                                            dep_stmts.push(stmt.clone());
                                        }
                                        continue;
                                    }
                                }
                                let f_rc = Rc::new(path_str.clone());
                                let f_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                                let cs = CharStream::new(content, f_rc.clone());
                                let mut lx = Lexer::new(cs, f_engine.clone());
                                let toks = lx.parse();
                                if f_engine.borrow().has_errors() { continue; }
                                let mut pp = Parser::new(f_rc, toks, f_engine.clone());
                                let prog = pp.parse();
                                if f_engine.borrow().has_errors() { continue; }
                                let stmts: Vec<Rc<RefCell<Statement>>> = prog.statements;
                                for stmt in &stmts {
                                    dep_stmts.push(stmt.clone());
                                }
                                self.file_cache.insert(path_str, CachedFile { hash, statements: stmts });
                            }
                        }
                    }

                    if !dep_stmts.is_empty() {
                        let dep_prog = Program {
                            file: Rc::new(format!("dep:{}", dep.name)),
                            statements: dep_stmts,
                        };
                        let dep_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                        let mut dep_resolver = SymbolResolver::new(packages.clone(), dep.name.clone(), dep_engine.clone());
                        dep_resolver.resolve(&dep_prog, dep.name.clone());
                        // Also type resolve the dependency for full type info
                        let dep_ty_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                        let mut dep_type_resolver = TypeResolver::new(packages.clone(), dep.name.clone(), dep_ty_engine.clone());
                        if let Some(pkg) = packages.get(&dep.name) {
                            if let Some(mod_ref) = pkg.borrow().modules.get(&dep.name) {
                                dep_type_resolver.resolve(&dep_prog, mod_ref.clone());
                            }
                        }
                    }
                }
            }
        }

        let combined_prog = Program {
            file: Rc::new(file_path.to_string()),
            statements: all_stmts,
        };
        let mut symbol_resolver = SymbolResolver::new(
            packages.clone(),
            "main".to_string(),
            analysis_engine.clone(),
        );
        let module = symbol_resolver.resolve(&combined_prog, "main".to_string());
        let mut analysis_diags =
            collect_diagnostics_filtered(&analysis_engine.borrow(), content, Some(file_path));
        if analysis_engine.borrow().has_errors() {
            return analysis_diags;
        }
        eprintln!("TRUSS_LSP_ANALYSIS: symbol resolve passed");

        let mut type_resolver = TypeResolver::new(
            packages.clone(),
            "main".to_string(),
            analysis_engine.clone(),
        );
        type_resolver.resolve(&combined_prog, module.clone());
        let type_diags =
            collect_diagnostics_filtered(&analysis_engine.borrow(), content, Some(file_path));
        let type_diag_count = type_diags.len();
        analysis_diags.extend(type_diags);
        eprintln!("TRUSS_LSP_ANALYSIS: type resolve done, diags={}", type_diag_count);

        if !analysis_engine.borrow().has_errors() {
            self.project_analyses.insert(
                file_path.to_string(),
                ProjectAnalysis {
                    scope: module.borrow().scope.clone().unwrap_or_else(|| {
                        let s = Rc::new(RefCell::new(Scope::new(None)));
                        module.borrow_mut().scope = Some(s.clone());
                        s
                    }),
                    module: module.clone(),
                },
            );
        }

        analysis_diags
    }

    fn find_project_dir(&self, file_path: &str) -> Option<String> {
        let path = Path::new(file_path);
        let mut current = path.parent()?;
        loop {
            let project_file = current.join("Project.truss");
            if project_file.exists() {
                return Some(current.to_string_lossy().to_string());
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => return None,
            }
        }
    }

    fn load_stdlib(&mut self) {
        if let Some(ref stdlib_path) = self.stdlib_path.clone() {
            let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
            let (file_programs, sources) = crate::trusspm::parse_std_lib(&stdlib_path, engine.clone());
            let mut packages: HashMap<String, Rc<RefCell<Package>>> = HashMap::new();
            let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
            packages.insert("Truss".to_string(), truss_pkg.clone());

            let mut all_stmts = Vec::new();
            for stmts in &file_programs {
                for s in stmts {
                    all_stmts.push(s.clone());
                }
            }
            let std_prog = Program {
                file: Rc::new("stdlib".to_string()),
                statements: all_stmts,
            };

            let mut resolver =
                SymbolResolver::new(packages.clone(), "Truss".to_string(), engine.clone());
            let module = resolver.resolve(&std_prog, "Truss".to_string());

            let stdlib_ty_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
            let mut type_resolver =
                TypeResolver::new(packages.clone(), "Truss".to_string(), stdlib_ty_engine.clone());
            type_resolver.resolve(&std_prog, module.clone());

            self.stdlib_scope = module.borrow().scope.clone();

            let mut ordered_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
            let mut ordered: Vec<_> = file_programs.iter().map(|stmts| {
                let name = stmts.first().and_then(|s| {
                    let file = &*s.borrow().token().file;
                    std::path::Path::new(file).file_stem().and_then(|n| n.to_str()).map(|n| n.to_string())
                }).unwrap_or_default();
                let priority = match name.as_str() {
                    "Truss" => 0,
                    "Iterator" => 1,
                    _ => 2,
                };
                (priority, stmts)
            }).collect();
            ordered.sort_by_key(|(p, _)| *p);
            for (_, stmts) in ordered {
                for stmt in stmts {
                    ordered_stmts.push(stmt.clone());
                }
            }

            self.stdlib_cache = Some(StdlibCache {
                statements: ordered_stmts,
                sources,
            });
        }
    }

    fn word_at_position(&self, content: &str, line: usize, character: usize) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return None;
        }
        let current_line = lines[line];
        if character > current_line.len() {
            return None;
        }
        let chars: Vec<char> = current_line.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let mut start = character;
        let mut end = character;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    fn lookup_symbol_in_scopes(&self, name: &str) -> Option<(Rc<RefCell<Symbol>>, String)> {
        if let Some(ref scope) = self.stdlib_scope {
            if let Some(sym) = scope.borrow().get_symbol(name) {
                return Some((sym, "stdlib".to_string()));
            }
        }
        for (file_path, analysis) in &self.project_analyses {
            let sb = analysis.scope.borrow();
            if let Some(sym) = sb.get_symbol(name) {
                return Some((sym, file_path.clone()));
            }
        }
        None
    }

    fn lookup_all_symbols_in_scopes(&self, name: &str) -> Vec<Rc<RefCell<Symbol>>> {
        let mut result = Vec::new();
        if let Some(ref scope) = self.stdlib_scope {
            result.extend(scope.borrow().get_all_symbols(name));
        }
        for (_, analysis) in &self.project_analyses {
            let sb = analysis.scope.borrow();
            let mut symbols = sb.get_all_symbols(name);
            if !symbols.is_empty() {
                result.append(&mut symbols);
            }
        }
        result
    }

    fn signature_from_statement(&self, stmt: &Statement) -> Option<(String, Vec<Value>)> {
        match stmt {
            Statement::FunctionDecl { name, parameters, ty, .. } => {
                let mut label = format!("func {}", name.value);
                label.push('(');
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 { label.push_str(", "); }
                    let p = param.borrow();
                    if let Some(l) = &p.label {
                        label.push_str(&l.value);
                        label.push(' ');
                    }
                    label.push_str(&p.name.value);
                    label.push_str(": ");
                    if let Some(ref pty) = p.ty {
                        label.push_str(&pty.borrow().to_string());
                    }
                }
                label.push(')');
                if let Some(func_ty) = ty {
                    if let Type::Function(_, ret, _, _) = &*func_ty.borrow() {
                        label.push_str(" -> ");
                        label.push_str(&ret.borrow().to_string());
                    }
                }
                let param_info: Vec<Value> = parameters.iter().map(|p| {
                    let p = p.borrow();
                    let mut pl = String::new();
                    if let Some(l) = &p.label {
                        pl.push_str(&l.value);
                        pl.push(' ');
                    }
                    pl.push_str(&p.name.value);
                    pl.push_str(": ");
                    if let Some(ref pty) = p.ty {
                        pl.push_str(&pty.borrow().to_string());
                    }
                    json!({"label": pl, "documentation": ""})
                }).collect();
                Some((label, param_info))
            }
            Statement::InitDecl { parameters, is_failable, ty, .. } => {
                let mut label = if *is_failable { "init?".to_string() } else { "init".to_string() };
                label.push('(');
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 { label.push_str(", "); }
                    let p = param.borrow();
                    if let Some(l) = &p.label {
                        label.push_str(&l.value);
                        label.push(' ');
                    }
                    label.push_str(&p.name.value);
                    label.push_str(": ");
                    if let Some(ref pty) = p.ty {
                        label.push_str(&pty.borrow().to_string());
                    }
                }
                label.push(')');
                if let Some(func_ty) = ty {
                    if let Type::Function(_, ret, _, _) = &*func_ty.borrow() {
                        label.push_str(" -> ");
                        label.push_str(&ret.borrow().to_string());
                    }
                }
                let param_info: Vec<Value> = parameters.iter().map(|p| {
                    let p = p.borrow();
                    let mut pl = String::new();
                    if let Some(l) = &p.label {
                        pl.push_str(&l.value);
                        pl.push(' ');
                    }
                    pl.push_str(&p.name.value);
                    pl.push_str(": ");
                    if let Some(ref pty) = p.ty {
                        pl.push_str(&pty.borrow().to_string());
                    }
                    json!({"label": pl, "documentation": ""})
                }).collect();
                Some((label, param_info))
            }
            Statement::SubscriptDecl { parameters, ty, .. } => {
                let mut label = "subscript".to_string();
                label.push('(');
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 { label.push_str(", "); }
                    let p = param.borrow();
                    if let Some(l) = &p.label {
                        label.push_str(&l.value);
                        label.push(' ');
                    }
                    label.push_str(&p.name.value);
                    label.push_str(": ");
                    if let Some(ref pty) = p.ty {
                        label.push_str(&pty.borrow().to_string());
                    }
                }
                label.push(')');
                if let Some(func_ty) = ty {
                    if let Type::Function(_, ret, _, _) = &*func_ty.borrow() {
                        label.push_str(" -> ");
                        label.push_str(&ret.borrow().to_string());
                    }
                }
                let param_info: Vec<Value> = parameters.iter().map(|p| {
                    let p = p.borrow();
                    let mut pl = String::new();
                    if let Some(l) = &p.label {
                        pl.push_str(&l.value);
                        pl.push(' ');
                    }
                    pl.push_str(&p.name.value);
                    pl.push_str(": ");
                    if let Some(ref pty) = p.ty {
                        pl.push_str(&pty.borrow().to_string());
                    }
                    json!({"label": pl, "documentation": ""})
                }).collect();
                Some((label, param_info))
            }
            _ => None,
        }
    }

    fn lookup_type_in_scopes(&self, name: &str) -> Option<(String, String)> {
        if let Some(ref scope) = self.stdlib_scope {
            let sb = scope.borrow();
            if let Some(ty) = sb.get_type(name) {
                return Some((ty.borrow().to_string(), "stdlib".to_string()));
            }
            if sb.overloads.contains_key(name) {
                let overload_key = format!("{} (overloaded)", name);
                return Some((overload_key, "stdlib".to_string()));
            }
            let check_name_table = |n: &str| -> Option<(String, String)> {
                if let Some(sym) = sb.name_table.get(n) {
                    if let Some(desc) = self.symbol_type_string(&sym.borrow()) {
                        return Some((desc, "stdlib".to_string()));
                    }
                }
                None
            };
            if let Some(result) = check_name_table(name) {
                return Some(result);
            }
        }
        for (file_path, analysis) in &self.project_analyses {
            let sb = analysis.scope.borrow();
            if let Some(ty) = sb.get_type(name) {
                return Some((ty.borrow().to_string(), file_path.clone()));
            }
            if sb.overloads.contains_key(name) {
                let overload_key = format!("{} (overloaded)", name);
                return Some((overload_key, file_path.clone()));
            }
            let check_name_table = |n: &str| -> Option<(String, String)> {
                if let Some(sym) = sb.name_table.get(n) {
                    if let Some(desc) = self.symbol_type_string(&sym.borrow()) {
                        return Some((desc, file_path.clone()));
                    }
                }
                None
            };
            if let Some(result) = check_name_table(name) {
                return Some(result);
            }
        }
        None
    }

    fn symbol_type_string(&self, sym: &Symbol) -> Option<String> {
        // Handle enum case constructors directly from Symbol data
        if let Symbol::EnumCase { name, parent, parameter_types, .. } = sym {
            if !parameter_types.is_empty() {
                let enum_name = parent.0.upgrade().and_then(|p| p.borrow().name().ok());
                let mut sig = name.to_string();
                sig.push('(');
                for (i, pt) in parameter_types.iter().enumerate() {
                    if i > 0 { sig.push_str(", "); }
                    sig.push_str(&pt.borrow().to_string());
                }
                sig.push(')');
                if let Some(ref en) = enum_name {
                    sig.push_str(&format!(" -> {}", en));
                }
                return Some(sig);
            }
        }

        if let Ok(Some(decl)) = sym.get_decl() {
            let stmt = decl.borrow();
            match &*stmt {
                Statement::FunctionDecl { name, parameters, ty, .. } => {
                    let mut s = format!("func {}", name.value);
                    s.push('(');
                    for (i, param) in parameters.iter().enumerate() {
                        if i > 0 { s.push_str(", "); }
                        let p = param.borrow();
                        if let Some(label) = &p.label {
                            s.push_str(&label.value);
                            s.push(' ');
                        }
                        s.push_str(&p.name.value);
                        s.push_str(": ");
                        if let Some(ref pty) = p.ty {
                            s.push_str(&pty.borrow().to_string());
                        }
                    }
                    s.push(')');
                    if let Some(func_ty) = ty {
                        if let Type::Function(_, ret, _, _) = &*func_ty.borrow() {
                            s.push_str(" -> ");
                            s.push_str(&ret.borrow().to_string());
                        }
                    }
                    Some(s)
                }
                Statement::InitDecl { parameters, is_failable, .. } => {
                    let mut s = if *is_failable { String::from("init?") } else { String::from("init") };
                    s.push('(');
                    for (i, param) in parameters.iter().enumerate() {
                        if i > 0 { s.push_str(", "); }
                        let p = param.borrow();
                        if let Some(label) = &p.label {
                            s.push_str(&label.value);
                            s.push(' ');
                        }
                        s.push_str(&p.name.value);
                        s.push_str(": ");
                        if let Some(ref pty) = p.ty {
                            s.push_str(&pty.borrow().to_string());
                        }
                    }
                    s.push(')');
                    Some(s)
                }
                Statement::SubscriptDecl { parameters, ty, .. } => {
                    let mut s = String::from("subscript");
                    s.push('(');
                    for (i, param) in parameters.iter().enumerate() {
                        if i > 0 { s.push_str(", "); }
                        let p = param.borrow();
                        if let Some(label) = &p.label {
                            s.push_str(&label.value);
                            s.push(' ');
                        }
                        s.push_str(&p.name.value);
                        s.push_str(": ");
                        if let Some(ref pty) = p.ty {
                            s.push_str(&pty.borrow().to_string());
                        }
                    }
                    s.push(')');
                    if let Some(func_ty) = ty {
                        if let Type::Function(_, ret, _, _) = &*func_ty.borrow() {
                            s.push_str(" -> ");
                            s.push_str(&ret.borrow().to_string());
                        }
                    }
                    Some(s)
                }
                Statement::VariableDecl { ty, .. } => {
                    ty.as_ref().map(|t| t.borrow().to_string())
                }
                Statement::StructDecl { ty, .. } => {
                    ty.as_ref().map(|t| t.borrow().to_string())
                }
                Statement::ClassDecl { ty, .. } => {
                    ty.as_ref().map(|t| t.borrow().to_string())
                }
                Statement::EnumDecl { ty, .. } => {
                    ty.as_ref().map(|t| t.borrow().to_string())
                }
                Statement::ProtocolDecl { ty, .. } => {
                    ty.as_ref().map(|t| t.borrow().to_string())
                }
                Statement::DeinitDecl { .. } => {
                    Some(String::from("deinit"))
                }
                _ => None,
            }
        } else {
            None
        }
    }

    fn add_snippet_completions(&self, items: &mut Vec<Value>) {
        items.push(json!({"label": "fn", "kind": 15, "detail": "→ func", "insertText": "func", "insertTextFormat": 1, "sortText": "0"}));
        items.push(json!({"label": "func", "kind": 15, "detail": "function declaration", "insertText": "func ${1:name}(${2:params}) -> ${3:ReturnType} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "struct", "kind": 15, "detail": "struct declaration", "insertText": "struct ${1:Name}${2:: ${3:Protocol}} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "class", "kind": 15, "detail": "class declaration", "insertText": "class ${1:Name}${2:: ${3:SuperClass}} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "enum", "kind": 15, "detail": "enum declaration", "insertText": "enum ${1:Name}${2:: ${3:RawType}} {\n\tcase ${4:value}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "protocol", "kind": 15, "detail": "protocol declaration", "insertText": "protocol ${1:Name}${2:: ${3:ParentProtocol}} {\n\t${4:members}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "extension", "kind": 15, "detail": "type extension", "insertText": "extension ${1:Type}${2:: ${3:Protocol}} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "init", "kind": 15, "detail": "initializer declaration", "insertText": "init${1:?}(${2:params}) {\n\t${3:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "deinit", "kind": 15, "detail": "deinitializer", "insertText": "deinit {\n\t${1:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "subscript", "kind": 15, "detail": "subscript declaration", "insertText": "subscript(${1:params}) -> ${2:Type} {\n\tget {\n\t\treturn ${3:val}\n\t}\n\t${4:set {\n\t\t${5:newValue}\n\t}}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "typealias", "kind": 15, "detail": "type alias", "insertText": "typealias ${1:Name} = ${2:Type}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "if", "kind": 15, "detail": "if statement", "insertText": "if ${1:condition} {\n\t${2:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "ifelse", "kind": 15, "detail": "if-else statement", "insertText": "if ${1:condition} {\n\t${2:body}\n} else {\n\t${3:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "for", "kind": 15, "detail": "for-in loop", "insertText": "for ${1:item} in ${2:collection} {\n\t${3:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "while", "kind": 15, "detail": "while loop", "insertText": "while ${1:condition} {\n\t${2:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "repeat", "kind": 15, "detail": "repeat-while loop", "insertText": "repeat {\n\t${1:body}\n} while ${2:condition}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "match", "kind": 15, "detail": "match expression", "insertText": "match ${1:value} {\n\tcase ${2:pattern} =>\n\t\t${3:body}\n\tdefault =>\n\t\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "guard", "kind": 15, "detail": "guard statement", "insertText": "guard ${1:condition} else {\n\t${2:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "do", "kind": 15, "detail": "do-catch block", "insertText": "do {\n\t${1:body}\n} catch {\n\t${2:handler}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "defer", "kind": 15, "detail": "defer block", "insertText": "defer {\n\t${1:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "public", "kind": 14, "detail": "public access modifier", "insertText": "public ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "private", "kind": 14, "detail": "private access modifier", "insertText": "private ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "open", "kind": 14, "detail": "open access modifier", "insertText": "open ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "override", "kind": 14, "detail": "override modifier", "insertText": "override ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "static", "kind": 14, "detail": "static modifier", "insertText": "static ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "mutating", "kind": 14, "detail": "mutating modifier", "insertText": "mutating func ${1:name}($2) {\n\t$3\n}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "associatedtype", "kind": 15, "detail": "associated type declaration", "insertText": "associatedtype ${1:Name}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "prefix func", "kind": 15, "detail": "prefix operator function", "insertText": "prefix func ${1:name}(${2:params}) -> ${3:ReturnType} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "postfix func", "kind": 15, "detail": "postfix operator function", "insertText": "postfix func ${1:name}(${2:params}) -> ${3:ReturnType} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "infix func", "kind": 15, "detail": "infix operator function", "insertText": "infix func ${1:name}(${2:lhs: Type}, ${3:rhs: Type}) -> ${4:ReturnType} {\n\t${5:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "operator", "kind": 15, "detail": "operator declaration", "insertText": "operator ${1:...}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "precedencegroup", "kind": 15, "detail": "precedence group declaration", "insertText": "precedencegroup ${1:Name} {\n\tassociativity: ${2:left}\n\tprecedence: ${3:100}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "throws", "kind": 14, "detail": "throws keyword", "insertText": "throws", "insertTextFormat": 1, "sortText": "2"}));
        items.push(json!({"label": "catch", "kind": 15, "detail": "catch clause", "insertText": "catch ${1:error} {\n\t${2:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "fallthrough", "kind": 14, "detail": "fallthrough keyword", "insertText": "fallthrough", "insertTextFormat": 1, "sortText": "2"}));
        items.push(json!({"label": "where", "kind": 14, "detail": "where clause keyword", "insertText": "where ${1:condition}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "weak", "kind": 14, "detail": "weak modifier", "insertText": "weak", "insertTextFormat": 1, "sortText": "2"}));
        items.push(json!({"label": "unowned", "kind": 14, "detail": "unowned modifier", "insertText": "unowned", "insertTextFormat": 1, "sortText": "2"}));
        items.push(json!({"label": "indirect", "kind": 14, "detail": "indirect modifier", "insertText": "indirect", "insertTextFormat": 1, "sortText": "2"}));
        items.push(json!({"label": "rethrows", "kind": 14, "detail": "rethrows keyword", "insertText": "rethrows", "insertTextFormat": 1, "sortText": "2"}));
    }

    fn add_stdlib_completions(&self, items: &mut Vec<Value>) {
        if let Some(ref scope) = self.stdlib_scope {
            let sb = scope.borrow();
            for (name, _) in &sb.type_env {
                items.push(json!({"label": name, "kind": 22, "detail": "type", "sortText": "3"}));
            }
            for (name, symbol) in &sb.name_table {
                let (kind, detail) = match &*symbol.borrow() {
                    Symbol::Function { .. } => (3, "function"),
                    Symbol::Variable { .. } => (6, "variable"),
                    Symbol::Struct { .. } => (22, "struct"),
                    Symbol::Class { .. } => (7, "class"),
                    Symbol::Enum { .. } => (13, "enum"),
                    Symbol::Protocol { .. } => (8, "protocol"),
                    Symbol::StructProperty { .. } | Symbol::ClassProperty { .. } => (10, "property"),
                    Symbol::StructMethod { .. } | Symbol::ClassMethod { .. } => (2, "method"),
                    Symbol::EnumCase { .. } => (20, "enum case"),
                    Symbol::Module { .. } => (9, "module"),
                    Symbol::Macro { .. } => (14, "macro"),
                    _ => continue,
                };
                items.push(json!({"label": name, "kind": kind, "detail": format!("stdlib {}", detail), "sortText": "3"}));
            }
            for (name, _) in &sb.overloads {
                items.push(json!({"label": name, "kind": 3, "detail": "stdlib symbol (overloaded)", "sortText": "3"}));
            }
        }
    }

    fn add_scope_completions(&self, items: &mut Vec<Value>, _content: &str, uri: &str) {
        let file_path = uri.strip_prefix("file://").unwrap_or(uri);
        if let Some(analysis) = self.project_analyses.get(file_path) {
            let sb = analysis.scope.borrow();
            for (name, _) in &sb.type_env {
                items.push(json!({"label": name, "kind": 22, "detail": "type", "sortText": "4"}));
            }
            for (name, symbol) in &sb.name_table {
                let (kind, detail) = match &*symbol.borrow() {
                    Symbol::Function { .. } => (3, "function"),
                    Symbol::Variable { .. } => (6, "variable"),
                    Symbol::Struct { .. } => (22, "struct"),
                    Symbol::Class { .. } => (7, "class"),
                    Symbol::Enum { .. } => (13, "enum"),
                    Symbol::Protocol { .. } => (8, "protocol"),
                    Symbol::StructProperty { .. } | Symbol::ClassProperty { .. } => (10, "property"),
                    Symbol::StructMethod { .. } | Symbol::ClassMethod { .. } => (2, "method"),
                    Symbol::EnumCase { .. } => (20, "enum case"),
                    Symbol::Module { .. } => (9, "module"),
                    Symbol::Macro { .. } => (14, "macro"),
                    _ => continue,
                };
                items.push(json!({"label": name, "kind": kind, "detail": detail, "sortText": "4"}));
            }
        }
    }

    fn add_member_completions(&self, items: &mut Vec<Value>, content: &str, _uri: &str, line: usize, character: usize) {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return;
        }
        let current_line = lines[line];
        let before_dot = &current_line[..character.saturating_sub(1).min(current_line.len())];
        let obj_name = before_dot
            .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
            .filter(|s| !s.is_empty())
            .last()
            .and_then(|s| {
                if s.ends_with('.') {
                    s.trim_end_matches('.')
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .last()
                } else {
                    Some(s)
                }
            })
            .unwrap_or("");

        if obj_name.is_empty() {
            return;
        }

        if let Some((sym, _)) = self.lookup_symbol_in_scopes(obj_name) {
            let sym_borrow = sym.borrow();
            let (properties, methods) = match &*sym_borrow {
                Symbol::Struct { properties, methods, .. }
                | Symbol::Class { properties, methods, .. } => {
                    let props: Vec<_> = properties.iter().map(|s| {
                        let name = s.borrow().name().unwrap_or_default();
                        (name, 6, "property")
                    }).collect();
                    let meths: Vec<_> = methods.iter().map(|s| {
                        let name = s.borrow().name().unwrap_or_default();
                        (name, 3, "method")
                    }).collect();
                    (props, meths)
                }
                Symbol::Enum { cases, methods, .. } => {
                    let cases_items: Vec<_> = cases.iter().map(|s| {
                        let name = s.borrow().name().unwrap_or_default();
                        (name, 21, "enum case")
                    }).collect();
                    let meths: Vec<_> = methods.iter().map(|s| {
                        let name = s.borrow().name().unwrap_or_default();
                        (name, 3, "method")
                    }).collect();
                    (cases_items, meths)
                }
                Symbol::Protocol { methods, properties, subscripts, .. } => {
                    let props: Vec<_> = properties.iter().map(|s| {
                        let name = s.borrow().name().unwrap_or_default();
                        (name, 6, "property")
                    }).collect();
                    let mut meths: Vec<_> = methods.iter().map(|s| {
                        let name = s.borrow().name().unwrap_or_default();
                        (name, 3, "method")
                    }).collect();
                    let subs: Vec<_> = subscripts.iter().map(|s| {
                        let name = s.borrow().name().unwrap_or_default();
                        (name, 3, "subscript")
                    }).collect();
                    meths.extend(subs);
                    (props, meths)
                }
                _ => (vec![], vec![]),
            };
            for (name, kind, detail) in properties.iter().chain(methods.iter()) {
                if !name.is_empty() {
                    items.push(json!({"label": name, "kind": kind, "detail": detail, "sortText": "0"}));
                }
            }
        }
    }

    fn empty_completion(&self, id: Option<u64>) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "isIncomplete": false,
                "items": []
            }
        })
        .to_string()
    }

    fn handle_completion(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => return self.empty_completion(id),
        };

        let line = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("line"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let character = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("character"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let lines: Vec<&str> = content.lines().collect();
        let current_line = if line < lines.len() {
            lines[line]
        } else {
            ""
        };
        let before_cursor = &current_line[..character.min(current_line.len())];

        let is_member = before_cursor.trim_end().ends_with('.');

        let mut items: Vec<Value> = Vec::new();

        if is_member {
            self.add_member_completions(&mut items, &content, uri, line, character);
        } else {
            self.add_snippet_completions(&mut items);
            self.add_stdlib_completions(&mut items);
            self.add_scope_completions(&mut items, &content, uri);
        }

        // Prefix filtering: keep only items whose label starts with prefix
        let prefix = {
            let line_str = lines.get(line).unwrap_or(&"");
            let end = character.min(line_str.len());
            let bytes = line_str[..end].as_bytes();
            let mut start = end;
            while start > 0 {
                let b = bytes[start - 1];
                if !b.is_ascii_alphanumeric() && b != b'_' {
                    break;
                }
                start -= 1;
            }
            line_str[start..end].to_string()
        };
        if !prefix.is_empty() {
            items.retain(|item| {
                item.get("label")
                    .and_then(|v| v.as_str())
                    .map(|label| label.to_lowercase().starts_with(&prefix.to_lowercase()))
                    .unwrap_or(false)
            });
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "isIncomplete": false,
                "items": items
            }
        })
        .to_string()
    }

    fn handle_hover(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
            }
        };

        let line = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("line"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let character = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("character"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let word = match self.word_at_position(&content, line, character) {
            Some(w) => w,
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
            }
        };

        let result = if word == "self" || word == "Self" || word == "super" {
            let desc = match word.as_str() {
                "self" => "self (current instance)",
                "Self" => "Self (current type)",
                "super" => "super (parent instance)",
                _ => "",
            };
            Some(json!({"contents": {"kind": "markdown", "value": format!("```truss\n{}\nkeyword\n```", desc)}}))
        } else if let Some((sym, _)) = self.lookup_symbol_in_scopes(&word) {
            let sym_borrow = sym.borrow();
            let sym_name = sym_borrow.name().unwrap_or_default();
            let type_str = self.symbol_type_string(&sym_borrow);
            let is_func_like = matches!(&*sym_borrow,
                Symbol::Function { .. }
                | Symbol::StructMethod { .. }
                | Symbol::ClassMethod { .. }
                | Symbol::ProtocolMethod { .. }
                | Symbol::StructSubscript { .. }
                | Symbol::ClassSubscript { .. }
                | Symbol::ProtocolSubscript { .. }
            ) || matches!(&*sym_borrow, Symbol::EnumCase { parameter_types, .. } if !parameter_types.is_empty());
            let mut markdown = format!("```truss\n");
            if is_func_like {
                if let Some(ref sig) = type_str {
                    markdown.push_str(sig);
                } else {
                    markdown.push_str(&sym_name);
                }
            } else {
                markdown.push_str(&sym_name);
                if let Some(ref ty) = type_str {
                    markdown.push_str(&format!(": {}", ty));
                }
            }
            let decl_info = match &*sym_borrow {
                Symbol::Function { .. } => "func",
                Symbol::Variable { is_var, .. } => {
                    if *is_var { "var" } else { "let" }
                }
                Symbol::Struct { .. } => "struct",
                Symbol::Class { .. } => "class",
                Symbol::Enum { .. } => "enum",
                Symbol::Protocol { .. } => "protocol",
                Symbol::StructProperty { .. } => "property",
                Symbol::StructMethod { .. } => "method",
                Symbol::ClassProperty { .. } => "property",
                Symbol::ClassMethod { .. } => "method",
                Symbol::StructSubscript { .. } => "subscript",
                Symbol::ClassSubscript { .. } => "subscript",
                Symbol::ProtocolSubscript { .. } => "subscript",
                Symbol::EnumCase { .. } => "enum case",
                Symbol::Module { .. } => "module",
                Symbol::Macro { .. } => "macro",
                _ => "symbol",
            };
            markdown.push_str(&format!("\n{}", decl_info));
            markdown.push_str("\n```");
            Some(json!({"contents": {"kind": "markdown", "value": markdown}}))
        } else if let Some((type_desc, _)) = self.lookup_type_in_scopes(&word) {
            let markdown = format!("```truss\n{}\ntype\n```", type_desc);
            Some(json!({"contents": {"kind": "markdown", "value": markdown}}))
        } else {
            None
        };

        match result {
            Some(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}).to_string(),
            None => json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string(),
        }
    }

    fn handle_signature_help(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
            }
        };

        let line = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("line"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let character = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("character"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
        }
        let current_line = lines[line];
        let chars: Vec<char> = current_line.chars().collect();
        if chars.is_empty() || character > chars.len() {
            return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
        }

        let mut paren_pos = None;
        if character < chars.len() && chars[character] == '(' {
            paren_pos = Some(character);
        } else {
            let mut pos = character.min(chars.len());
            while pos > 0 {
                pos -= 1;
                if chars[pos] == '(' {
                    paren_pos = Some(pos);
                    break;
                }
            }
        }
        let paren_pos = match paren_pos {
            Some(p) => p,
            None => return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string(),
        };

        let mut name_end = paren_pos;
        while name_end > 0 && chars[name_end - 1].is_whitespace() {
            name_end -= 1;
        }
        let name_end = name_end;
        let mut name_start = name_end;
        while name_start > 0 && (chars[name_start - 1].is_alphanumeric() || chars[name_start - 1] == '_') {
            name_start -= 1;
        }
        if name_start >= name_end {
            return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
        }
        let func_name: String = chars[name_start..name_end].iter().collect();

        let mut member_sigs: Vec<(String, Vec<Value>)> = Vec::new();
        if name_start > 1 && chars[name_start - 1] == '.' {
            let obj_end = name_start - 1;
            let mut obj_start = obj_end;
            while obj_start > 0 && (chars[obj_start - 1].is_alphanumeric() || chars[obj_start - 1] == '_') {
                obj_start -= 1;
            }
            if obj_start < obj_end {
                let obj_name: String = chars[obj_start..obj_end].iter().collect();
                if let Some((sym, _)) = self.lookup_symbol_in_scopes(&obj_name) {
                    let binding = sym.borrow();
                    member_sigs = match &*binding {
                        Symbol::Enum { cases, .. } => {
                            let mut sigs = Vec::new();
                            for case in cases {
                                if case.borrow().name().ok().as_deref() == Some(&func_name) {
                                    if let Symbol::EnumCase { name, parameter_types, parent, .. } = &*case.borrow() {
                                        if !parameter_types.is_empty() {
                                            let enum_name = parent.0.upgrade().and_then(|p| p.borrow().name().ok()).unwrap_or_default();
                                            let mut label = name.to_string();
                                            label.push('(');
                                            let mut param_info = Vec::new();
                                            for (i, pt) in parameter_types.iter().enumerate() {
                                                if i > 0 { label.push_str(", "); }
                                                let type_str = pt.borrow().to_string();
                                                label.push_str(&format!("_: {}", type_str));
                                                param_info.push(json!({"label": format!("_: {}", type_str), "documentation": ""}));
                                            }
                                            label.push(')');
                                            if !enum_name.is_empty() {
                                                label.push_str(&format!(" -> {}", enum_name));
                                            }
                                            sigs.push((label, param_info));
                                        }
                                    }
                                }
                            }
                            sigs
                        }
                        _ => vec![],
                    };
                }
            }
        }
        if !member_sigs.is_empty() {
            let mut signatures = Vec::new();
            for (label, param_info) in member_sigs {
                signatures.push(json!({"label": label, "parameters": param_info}));
            }
            let active_param = {
                let end = character.min(chars.len());
                if end > paren_pos + 1 {
                    chars[paren_pos + 1..end].iter().filter(|&&c| c == ',').count() as u64
                } else { 0 }
            };
            return json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "signatures": signatures,
                    "activeSignature": 0,
                    "activeParameter": active_param
                }
            }).to_string();
        }

        let symbols = self.lookup_all_symbols_in_scopes(&func_name);
        if symbols.is_empty() {
            return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
        }

        let mut signatures = Vec::new();
        for sym in &symbols {
            let sym_borrow = sym.borrow();
            let constructor_sigs: Vec<(String, Vec<Value>)> = match &*sym_borrow {
                Symbol::Struct { constructors, .. }
                | Symbol::Class { constructors, .. } => {
                    let mut sigs = Vec::new();
                    for ctor in constructors {
                        if let Ok(Some(decl)) = ctor.borrow().get_decl() {
                            let stmt = decl.borrow();
                            if let Some(sig) = self.signature_from_statement(&stmt) {
                                sigs.push(sig);
                            }
                        }
                    }
                    sigs
                }
                Symbol::EnumCase { name, parameter_types, parent, .. } if !parameter_types.is_empty() => {
                    let enum_name = parent.0.upgrade().and_then(|p| p.borrow().name().ok()).unwrap_or_default();
                    let mut label = name.to_string();
                    label.push('(');
                    let mut param_info = Vec::new();
                    for (i, pt) in parameter_types.iter().enumerate() {
                        if i > 0 { label.push_str(", "); }
                        let type_str = pt.borrow().to_string();
                        label.push_str(&format!("_: {}", type_str));
                        param_info.push(json!({"label": format!("_: {}", type_str), "documentation": ""}));
                    }
                    label.push(')');
                    if !enum_name.is_empty() {
                        label.push_str(&format!(" -> {}", enum_name));
                    }
                    vec![(label, param_info)]
                }
                _ => vec![],
            };
            for (label, param_info) in constructor_sigs {
                signatures.push(json!({"label": label, "parameters": param_info}));
            }
            if let Ok(Some(decl)) = sym_borrow.get_decl() {
                let stmt = decl.borrow();
                if let Some((label, param_info)) = self.signature_from_statement(&stmt) {
                    signatures.push(json!({
                        "label": label,
                        "parameters": param_info
                    }));
                }
            }
        }
        if signatures.is_empty() {
            return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
        }

        let mut active_param = 0u64;
        let end = character.min(chars.len());
        if end > paren_pos + 1 {
            for c in &chars[paren_pos + 1..end] {
                if *c == ',' {
                    active_param += 1;
                }
            }
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "signatures": signatures,
                "activeSignature": 0,
                "activeParameter": active_param
            }
        })
        .to_string()
    }

    fn handle_definition(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
            }
        };

        let line = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("line"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let character = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("character"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let word = match self.word_at_position(&content, line, character) {
            Some(w) => w,
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
            }
        };

        // Check for member access (e.g., obj.method)
        let lines: Vec<&str> = content.lines().collect();
        if line < lines.len() {
            let current_line = lines[line];
            let chars: Vec<char> = current_line.chars().collect();
            let end = character.min(chars.len());
            let mut word_start = end;
            while word_start > 0 && (chars[word_start - 1].is_alphanumeric() || chars[word_start - 1] == '_') {
                word_start -= 1;
            }
            if word_start > 0 && chars[word_start - 1] == '.' {
                let dot_pos = word_start - 1;
                let mut obj_start = dot_pos;
                while obj_start > 0 && (chars[obj_start - 1].is_alphanumeric() || chars[obj_start - 1] == '_' || chars[obj_start - 1] == '.') {
                    obj_start -= 1;
                }
                let obj_expr: String = chars[obj_start..dot_pos].iter().collect();
                let obj_name = obj_expr.split('.').last().unwrap_or("").to_string();
                if !obj_name.is_empty() {
                    if let Some((sym, _)) = self.lookup_symbol_in_scopes(&obj_name) {
                        let sym_borrow = sym.borrow();
                        let members: Vec<Rc<RefCell<Symbol>>> = match &*sym_borrow {
                            Symbol::Struct { methods, properties, constructors, subscripts, .. } => {
                                let mut all = Vec::new();
                                all.extend(methods.iter().cloned());
                                all.extend(properties.iter().cloned());
                                all.extend(constructors.iter().cloned());
                                all.extend(subscripts.iter().cloned());
                                all
                            }
                            Symbol::Class { methods, properties, constructors, subscripts, .. } => {
                                let mut all = Vec::new();
                                all.extend(methods.iter().cloned());
                                all.extend(properties.iter().cloned());
                                all.extend(constructors.iter().cloned());
                                all.extend(subscripts.iter().cloned());
                                all
                            }
                            Symbol::Enum { methods, cases, .. } => {
                                let mut all = Vec::new();
                                all.extend(methods.iter().cloned());
                                all.extend(cases.iter().cloned());
                                all
                            }
                            Symbol::Protocol { methods, properties, subscripts, .. } => {
                                let mut all = Vec::new();
                                all.extend(methods.iter().cloned());
                                all.extend(properties.iter().cloned());
                                all.extend(subscripts.iter().cloned());
                                all
                            }
                            _ => vec![],
                        };
                        for member_sym in &members {
                            if let Ok(name) = member_sym.borrow().name() {
                                if name == word {
                                    if let Ok(Some(decl)) = member_sym.borrow().get_decl() {
                                        let decl_stmt = decl.borrow();
                                        let token = decl_stmt.token();
                                        let pos = token.position;
                                        let mut decl_file = token.file.as_str().to_string();
                                        if decl_file == "stdlib" || decl_file.is_empty() {
                                            if let Some(ref stdlib_path) = self.stdlib_path {
                                                decl_file = format!("file://{}/Sources/Truss/Truss.truss", stdlib_path);
                                            }
                                        } else if decl_file.starts_with('/') {
                                            decl_file = format!("file://{}", decl_file);
                                        }
                                        return json!({
                                            "jsonrpc": "2.0",
                                            "id": id,
                                            "result": {
                                                "uri": decl_file,
                                                "range": {
                                                    "start": {
                                                        "line": (pos.line.saturating_sub(1)) as u64,
                                                        "character": (pos.col.saturating_sub(1)) as u64
                                                    },
                                                    "end": {
                                                        "line": (pos.line.saturating_sub(1)) as u64,
                                                        "character": (pos.col.saturating_sub(1) + pos.len) as u64
                                                    }
                                                }
                                            }
                                        })
                                        .to_string();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((sym, _)) = self.lookup_symbol_in_scopes(&word) {
            if let Ok(Some(decl)) = sym.borrow().get_decl() {
                let stmt = decl.borrow();
                let token = stmt.token();
                let pos = token.position;
                let mut decl_file = token.file.as_str().to_string();
                if decl_file == "stdlib" || decl_file.is_empty() {
                    if let Some(ref stdlib_path) = self.stdlib_path {
                        decl_file = format!("file://{}/Sources/Truss/Truss.truss", stdlib_path);
                    }
                } else if decl_file.starts_with('/') {
                    decl_file = format!("file://{}", decl_file);
                }
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "uri": decl_file,
                        "range": {
                            "start": {
                                "line": (pos.line.saturating_sub(1)) as u64,
                                "character": (pos.col.saturating_sub(1)) as u64
                            },
                            "end": {
                                "line": (pos.line.saturating_sub(1)) as u64,
                                "character": (pos.col.saturating_sub(1) + pos.len) as u64
                            }
                        }
                    }
                })
                .to_string();
            }
        }

        json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string()
    }

    fn handle_document_symbol(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
            }
        };
        let file_path = uri.strip_prefix("file://").unwrap_or(uri);
        let file_rc = Rc::new(file_path.to_string());
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let char_stream = CharStream::new(content.clone(), file_rc.clone());
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();
        if engine.borrow().has_errors() {
            return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
        }
        let parser_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut parser = Parser::new(file_rc, tokens, parser_engine.clone());
        let program = parser.parse();
        let symbols = self.collect_document_symbols(&program.statements, &content);
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": symbols
        })
        .to_string()
    }

    fn handle_folding_range(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
            }
        };
        let file_path = uri.strip_prefix("file://").unwrap_or(uri);
        let file_rc = Rc::new(file_path.to_string());
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let char_stream = CharStream::new(content.clone(), file_rc.clone());
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();
        if engine.borrow().has_errors() {
            return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
        }
        let parser_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut parser = Parser::new(file_rc, tokens, parser_engine.clone());
        let program = parser.parse();
        let ranges = self.collect_folding_ranges(&program.statements, &content);
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": ranges
        })
        .to_string()
    }

    fn collect_folding_ranges(&self, stmts: &[Rc<RefCell<Statement>>], content: &str) -> Vec<Value> {
        let mut ranges = Vec::new();
        for stmt_rc in stmts {
            let child_stmts: Vec<Rc<RefCell<Statement>>> = {
                let stmt = stmt_rc.borrow();
                let decl_token = stmt.token();
                let start_line = decl_token.position.line;
                let start_col = decl_token.position.col;

                if matches!(&*stmt,
                    Statement::FunctionDecl { .. }
                    | Statement::StructDecl { .. }
                    | Statement::ClassDecl { .. }
                    | Statement::EnumDecl { .. }
                    | Statement::ProtocolDecl { .. }
                    | Statement::ExtensionDecl { .. }
                    | Statement::InitDecl { .. }
                    | Statement::DeinitDecl { .. }
                    | Statement::SubscriptDecl { .. }
                    | Statement::ModuleDecl { .. }
                    | Statement::Loop { .. }
                    | Statement::While { .. }
                    | Statement::RepeatWhile { .. }
                    | Statement::For { .. }
                    | Statement::Guard { .. }
                    | Statement::Defer { .. }
                    | Statement::ConditionalBlock { .. }
                    | Statement::AsmBlock { .. }
                ) {
                    if let Some((open_line, close_line)) = Self::find_brace_range(content, start_line, start_col) {
                        ranges.push(json!({
                            "startLine": (open_line - 1) as u64,
                            "endLine": (close_line - 1) as u64
                        }));
                    }
                }

                match &*stmt {
                    Statement::FunctionDecl { body, .. } => {
                        match &*body.borrow() {
                            FunctionBody::Statements(s) => s.clone(),
                            _ => vec![],
                        }
                    }
                    Statement::StructDecl { body, .. } => body.clone(),
                    Statement::ClassDecl { body, .. } => body.clone(),
                    Statement::EnumDecl { body, .. } => body.clone(),
                    Statement::ExtensionDecl { body, .. } => body.clone(),
                    Statement::ModuleDecl { body, .. } => body.clone(),
                    Statement::Loop { body, .. } => body.clone(),
                    Statement::While { body, .. } => body.clone(),
                    Statement::RepeatWhile { body, .. } => body.clone(),
                    Statement::For { body, .. } => body.clone(),
                    Statement::Guard { else_body, .. } => else_body.clone(),
                    Statement::Defer { body, .. } => body.clone(),
                    Statement::ConditionalBlock { clauses } => {
                        clauses.iter().flat_map(|c| c.body.clone()).collect()
                    }
                    Statement::InitDecl { body, .. } => {
                        match &*body.borrow() {
                            FunctionBody::Statements(s) => s.clone(),
                            _ => vec![],
                        }
                    }
                    Statement::DeinitDecl { body, .. } => {
                        match &*body.borrow() {
                            FunctionBody::Statements(s) => s.clone(),
                            _ => vec![],
                        }
                    }
                    Statement::SubscriptDecl { accessors, .. } => {
                        accessors.iter().flat_map(|a| a.body.clone()).collect()
                    }
                    _ => vec![],
                }
            };
            if !child_stmts.is_empty() {
                ranges.extend(self.collect_folding_ranges(&child_stmts, content));
            }
        }
        ranges
    }

    fn handle_document_highlight(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.as_str(),
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
            }
        };

        let line = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("line"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let character = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("character"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let word = match self.word_at_position(content, line, character) {
            Some(w) => w,
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string();
            }
        };

        let mut highlights: Vec<Value> = Vec::new();

        for (line_idx, line_str) in content.lines().enumerate() {
            let mut search_start = 0usize;
            let line_bytes = line_str.as_bytes();
            while search_start + word.len() <= line_str.len() {
                match line_str[search_start..].find(&word) {
                    Some(pos) => {
                        let abs_pos = search_start + pos;
                        let has_boundary_before = if abs_pos == 0 {
                            true
                        } else {
                            let b = line_bytes[abs_pos - 1];
                            !b.is_ascii_alphanumeric() && b != b'_'
                        };
                        let has_boundary_after = if abs_pos + word.len() >= line_bytes.len() {
                            true
                        } else {
                            let b = line_bytes[abs_pos + word.len()];
                            !b.is_ascii_alphanumeric() && b != b'_'
                        };
                        if has_boundary_before && has_boundary_after {
                            highlights.push(json!({
                                "range": {
                                    "start": {
                                        "line": line_idx as u64,
                                        "character": abs_pos as u64
                                    },
                                    "end": {
                                        "line": line_idx as u64,
                                        "character": (abs_pos + word.len()) as u64
                                    }
                                },
                                "kind": 1
                            }));
                        }
                        search_start = abs_pos + word.len();
                    }
                    None => break,
                }
            }
        }

        json!({"jsonrpc": "2.0", "id": id, "result": highlights}).to_string()
    }

    fn handle_references(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
            }
        };

        let line = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("line"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let character = params
            .and_then(|p| p.get("position"))
            .and_then(|pos| pos.get("character"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        let word = match self.word_at_position(&content, line, character) {
            Some(w) => w,
            None => {
                return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
            }
        };

        let search_word = if let Some((sym, _)) = self.lookup_symbol_in_scopes(&word) {
            sym.borrow().name().unwrap_or(word)
        } else {
            word
        };

        let mut locations: Vec<Value> = Vec::new();

        for (doc_uri, doc_content) in &self.documents {
            let matches = find_word_occurrences(doc_content, &search_word, doc_uri);
            locations.extend(matches);
        }

        for (file_path, _) in &self.project_analyses {
            let file_uri = format!("file://{}", file_path);
            if !self.documents.contains_key(&file_uri) {
                if let Ok(file_content) = std::fs::read_to_string(file_path) {
                    let matches = find_word_occurrences(&file_content, &search_word, &file_uri);
                    locations.extend(matches);
                }
            }
        }

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": locations
        })
        .to_string()
    }

    fn collect_document_symbols(&self, stmts: &[Rc<RefCell<Statement>>], content: &str) -> Vec<Value> {
        stmts.iter().filter_map(|stmt_rc| {
            self.statement_to_document_symbol(&stmt_rc.borrow(), content)
        }).collect()
    }

    fn statement_to_document_symbol(&self, stmt: &Statement, content: &str) -> Option<Value> {
        let (name, kind, children, name_token): (String, u64, Vec<Value>, Token) = match stmt {
            Statement::FunctionDecl { name, body, .. } => {
                let children = match &*body.borrow() {
                    FunctionBody::Statements(stmts) => self.collect_document_symbols(stmts, content),
                    _ => vec![],
                };
                (name.value.clone(), 12, children, (**name).clone())
            }
            Statement::StructDecl { name, body, .. } => {
                let children = self.collect_document_symbols(body, content);
                (name.value.clone(), 23, children, (**name).clone())
            }
            Statement::ClassDecl { name, body, .. } => {
                let children = self.collect_document_symbols(body, content);
                (name.value.clone(), 5, children, (**name).clone())
            }
            Statement::EnumDecl { name, body, cases, .. } => {
                let mut children = self.collect_document_symbols(body, content);
                children.extend(self.collect_enum_cases(cases));
                (name.value.clone(), 10, children, (**name).clone())
            }
            Statement::ProtocolDecl { name, members, .. } => {
                let children = self.collect_protocol_members(members, content);
                (name.value.clone(), 14, children, (**name).clone())
            }
            Statement::ExtensionDecl { type_name, body, .. } => {
                let children = self.collect_document_symbols(body, content);
                (type_name.value.clone(), 5, children, (**type_name).clone())
            }
            Statement::InitDecl { token, .. } => {
                ("init".to_string(), 12, vec![], (**token).clone())
            }
            Statement::DeinitDecl { token, .. } => {
                ("deinit".to_string(), 12, vec![], (**token).clone())
            }
            Statement::SubscriptDecl { token, .. } => {
                ("subscript".to_string(), 12, vec![], (**token).clone())
            }
            Statement::VariableDecl { name, .. } => {
                (name.value.clone(), 13, vec![], (**name).clone())
            }
            Statement::TypeAlias { name, .. } => {
                (name.value.clone(), 17, vec![], (**name).clone())
            }
            Statement::OperatorDecl { token, symbol, .. } => {
                (symbol.clone(), 12, vec![], (**token).clone())
            }
            Statement::PrecedenceGroupDecl { name, .. } => {
                (name.value.clone(), 24, vec![], (**name).clone())
            }
            Statement::ModuleDecl { name, body, .. } => {
                let children = self.collect_document_symbols(body, content);
                (name.value.clone(), 2, children, (**name).clone())
            }
            Statement::MacroDecl { name, .. } => {
                (name.value.clone(), 14, vec![], (**name).clone())
            }
            _ => return None,
        };
        let decl_token = stmt.token();
        let start_pos = decl_token.position;
        let name_pos = name_token.position;
        let has_brace_body = matches!(stmt,
            Statement::FunctionDecl { .. }
            | Statement::StructDecl { .. }
            | Statement::ClassDecl { .. }
            | Statement::EnumDecl { .. }
            | Statement::ProtocolDecl { .. }
            | Statement::ExtensionDecl { .. }
            | Statement::ModuleDecl { .. }
            | Statement::InitDecl { .. }
            | Statement::DeinitDecl { .. }
            | Statement::SubscriptDecl { .. }
            | Statement::MacroDecl { .. }
        );
        let name_end = name_pos.col + name_pos.len;
        let (end_line, end_col) = if has_brace_body {
            LanguageServer::find_brace_end(content, start_pos.line, start_pos.col)
                .unwrap_or((name_pos.line, name_end))
        } else {
            (name_pos.line, name_end)
        };
        Some(json!({
            "name": name,
            "kind": kind,
            "range": {
                "start": lsp_pos(start_pos.line, start_pos.col),
                "end": lsp_pos(end_line, end_col)
            },
            "selectionRange": {
                "start": lsp_pos(name_pos.line, name_pos.col),
                "end": lsp_pos(name_pos.line, name_pos.col + name_pos.len)
            },
            "children": children
        }))
    }

    fn collect_protocol_members(&self, members: &[ProtocolMember], content: &str) -> Vec<Value> {
        members.iter().filter_map(|member| match member {
            ProtocolMember::Method { decl, .. } => {
                self.statement_to_document_symbol(&decl.borrow(), content)
            }
            ProtocolMember::Property { name, .. } => {
                let pos = name.position;
                Some(json!({
                    "name": name.value.clone(),
                    "kind": 7,
                    "range": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "selectionRange": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "children": []
                }))
            }
            ProtocolMember::AssociatedType { name, .. } => {
                let pos = name.position;
                Some(json!({
                    "name": name.value.clone(),
                    "kind": 26,
                    "range": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "selectionRange": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "children": []
                }))
            }
            ProtocolMember::StaticVar { name, .. } => {
                let pos = name.position;
                Some(json!({
                    "name": name.value.clone(),
                    "kind": 13,
                    "range": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "selectionRange": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "children": []
                }))
            }
            ProtocolMember::TypeAlias { name, .. } => {
                let pos = name.position;
                Some(json!({
                    "name": name.value.clone(),
                    "kind": 17,
                    "range": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "selectionRange": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "children": []
                }))
            }
            ProtocolMember::Subscript { token, .. } => {
                let pos = token.position;
                Some(json!({
                    "name": "subscript",
                    "kind": 12,
                    "range": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "selectionRange": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "children": []
                }))
            }
            ProtocolMember::Init { token, .. } => {
                let pos = token.position;
                Some(json!({
                    "name": "init",
                    "kind": 12,
                    "range": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "selectionRange": {
                        "start": lsp_pos(pos.line, pos.col),
                        "end": lsp_pos(pos.line, pos.col + pos.len)
                    },
                    "children": []
                }))
            }
        }).collect()
    }

    fn collect_enum_cases(&self, cases: &[crate::ast::statement::EnumCase]) -> Vec<Value> {
        cases.iter().map(|ec| {
            let pos = ec.name.position;
            json!({
                "name": ec.name.value.clone(),
                "kind": 22,
                "range": {
                    "start": lsp_pos(pos.line, pos.col),
                    "end": lsp_pos(pos.line, pos.col + pos.len)
                },
                "selectionRange": {
                    "start": lsp_pos(pos.line, pos.col),
                    "end": lsp_pos(pos.line, pos.col + pos.len)
                },
                "children": []
            })
        }).collect()
    }

    fn find_brace_range(content: &str, start_line: usize, start_col: usize) -> Option<(usize, usize)> {
        let mut open_line: Option<usize> = None;
        let mut depth: i32 = 0;
        for (i, line) in content.lines().enumerate() {
            let line_num = i + 1;
            if line_num < start_line {
                continue;
            }
            let col_start = if line_num == start_line { start_col.saturating_sub(1) as usize } else { 0 };
            for (j, ch) in line.char_indices() {
                if line_num == start_line && j < col_start {
                    continue;
                }
                if ch == '{' {
                    if open_line.is_none() {
                        open_line = Some(line_num);
                    }
                    depth += 1;
                } else if ch == '}' && depth > 0 {
                    depth -= 1;
                    if depth == 0 && open_line.is_some() {
                        return Some((open_line.unwrap(), line_num));
                    }
                }
            }
        }
        None
    }

    fn find_brace_end(content: &str, start_line: usize, start_col: usize) -> Option<(usize, usize)> {
        let mut depth = 0i32;
        for (i, line) in content.lines().enumerate() {
            let line_num = i + 1;
            if line_num < start_line {
                continue;
            }
            let col_start = if line_num == start_line { start_col.saturating_sub(1) as usize } else { 0 };
            for (j, ch) in line.char_indices() {
                if line_num == start_line && j < col_start {
                    continue;
                }
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    if depth == 1 {
                        return Some((line_num, j + 1));
                    }
                    if depth > 0 {
                        depth -= 1;
                    }
                }
            }
        }
        None
    }

    fn handle_semantic_tokens(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "data": [] }
                })
                .to_string();
            }
        };
        let file_rc = Rc::new(uri.to_string());
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let char_stream = CharStream::new(content, file_rc);
        let mut lexer = Lexer::new(char_stream, engine);
        let tokens = lexer.parse();
        let data = encode_semantic_tokens(&tokens);
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "data": data }
        })
        .to_string()
    }

    fn handle_inlay_hint(&self, id: Option<u64>, params: Option<&Value>) -> String {
        let uri = params
            .and_then(|p| p.get("textDocument"))
            .and_then(|td| td.get("uri"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let content = match self.documents.get(uri) {
            Some(c) => c.clone(),
            None => {
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": []
                })
                .to_string();
            }
        };

        let file_path = uri.strip_prefix("file://").unwrap_or(uri);
        let file_rc = Rc::new(file_path.to_string());

        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let char_stream = CharStream::new(content.clone(), file_rc.clone());
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();
        if engine.borrow().has_errors() {
            return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
        }

        let parser_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut parser = Parser::new(file_rc.clone(), tokens, parser_engine.clone());
        let program = parser.parse();
        if parser_engine.borrow().has_errors() {
            return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
        }

        let analysis_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut packages: HashMap<String, Rc<RefCell<Package>>> = HashMap::new();
        let main_pkg = Rc::new(RefCell::new(Package::new("main".to_string())));
        packages.insert("main".to_string(), main_pkg.clone());

        let mut stdlib_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
        if let Some(ref stdlib_path) = self.stdlib_path {
            let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
            packages.insert("Truss".to_string(), truss_pkg.clone());

            let std_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
            let (file_programs, _) = crate::trusspm::parse_std_lib(stdlib_path, std_engine.clone());

            if !std_engine.borrow().has_errors() {
                for file_stmts in &file_programs {
                    for stmt in file_stmts {
                        stdlib_stmts.push(stmt.clone());
                    }
                }

                let std_prog = Program {
                    file: Rc::new("stdlib".to_string()),
                    statements: stdlib_stmts.clone(),
                };
                let mut std_resolver =
                    SymbolResolver::new(packages.clone(), "Truss".to_string(), std_engine.clone());
                let std_module = std_resolver.resolve(&std_prog, "Truss".to_string());

                if !std_engine.borrow().has_errors() {
                    let mut std_type_resolver = TypeResolver::new(
                        packages.clone(),
                        "Truss".to_string(),
                        std_engine.clone(),
                    );
                    std_type_resolver.resolve(&std_prog, std_module);
                }
            }
        }

        let all_stmts: Vec<Rc<RefCell<Statement>>> = program.statements.iter().cloned().collect();

        let combined_prog = Program {
            file: file_rc.clone(),
            statements: all_stmts,
        };
        let mut symbol_resolver = SymbolResolver::new(
            packages.clone(),
            "main".to_string(),
            analysis_engine.clone(),
        );
        let module = symbol_resolver.resolve(&combined_prog, "main".to_string());
        if analysis_engine.borrow().has_errors() {
            return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
        }

        let mut type_resolver = TypeResolver::new(
            packages.clone(),
            "main".to_string(),
            analysis_engine.clone(),
        );
        type_resolver.resolve(&combined_prog, module.clone());
        if analysis_engine.borrow().has_errors() {
            return json!({"jsonrpc": "2.0", "id": id, "result": []}).to_string();
        }

        let mut hints: Vec<Value> = Vec::new();
        Self::collect_inlay_hints(&program.statements, &mut hints);

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": hints
        })
        .to_string()
    }

    fn collect_inlay_hints(statements: &[Rc<RefCell<Statement>>], hints: &mut Vec<Value>) {
        for stmt_ref in statements {
            let stmt = stmt_ref.borrow();
            match &*stmt {
                Statement::VariableDecl {
                    name,
                    type_expression,
                    ty,
                    ..
                } => {
                    if type_expression.is_none() {
                        if let Some(type_ref) = ty {
                            let type_name = type_ref.borrow().to_string();
                            let line = name.position.line;
                            let col = name.position.col + name.value.len();
                            hints.push(json!({
                                "position": {
                                    "line": line as u64,
                                    "character": col as u64
                                },
                                "label": format!(": {}", type_name),
                                "kind": 2,
                                "paddingLeft": true,
                                "paddingRight": false
                            }));
                        }
                    }
                }
                Statement::StructDecl { body, .. }
                | Statement::ClassDecl { body, .. }
                | Statement::EnumDecl { body, .. }
                | Statement::ExtensionDecl { body, .. } => {
                    Self::collect_inlay_hints(body, hints);
                }
                Statement::ModuleDecl { body, .. } => {
                    Self::collect_inlay_hints(body, hints);
                }
                Statement::FunctionDecl { body, .. } => {
                    if let FunctionBody::Statements(body_stmts) = &*body.borrow() {
                        Self::collect_inlay_hints(body_stmts, hints);
                    }
                }
                Statement::InitDecl { body, .. } => {
                    if let FunctionBody::Statements(body_stmts) = &*body.borrow() {
                        Self::collect_inlay_hints(body_stmts, hints);
                    }
                }
                Statement::DeinitDecl { body, .. } => {
                    if let FunctionBody::Statements(body_stmts) = &*body.borrow() {
                        Self::collect_inlay_hints(body_stmts, hints);
                    }
                }
                Statement::For { body, .. }
                | Statement::While { body, .. }
                | Statement::RepeatWhile { body, .. }
                | Statement::Loop { body, .. }
                | Statement::Defer { body, .. } => {
                    Self::collect_inlay_hints(body, hints);
                }
                Statement::Guard { else_body, .. } => {
                    Self::collect_inlay_hints(else_body, hints);
                }
                Statement::ConditionalBlock { clauses } => {
                    for clause in clauses {
                        Self::collect_inlay_hints(&clause.body, hints);
                    }
                }
                _ => {}
            }
        }
    }
}

fn lsp_pos(line: usize, col: usize) -> Value {
    json!({
        "line": line as u64,
        "character": col as u64
    })
}

fn find_word_occurrences(content: &str, word: &str, uri: &str) -> Vec<Value> {
    let mut locations = Vec::new();
    for (line_idx, line) in content.lines().enumerate() {
        let mut search_start = 0;
        let line_bytes = line.as_bytes();
        while let Some(pos) = line[search_start..].find(word) {
            let abs_pos = search_start + pos;
            let before_ok = if abs_pos > 0 {
                !line_bytes[abs_pos - 1].is_ascii_alphanumeric() && line_bytes[abs_pos - 1] != b'_'
            } else {
                true
            };
            let after_end = abs_pos + word.len();
            let after_ok = if after_end < line_bytes.len() {
                !line_bytes[after_end].is_ascii_alphanumeric() && line_bytes[after_end] != b'_'
            } else {
                true
            };
            if before_ok && after_ok {
                locations.push(json!({
                    "uri": uri,
                    "range": {
                        "start": { "line": line_idx as u64, "character": abs_pos as u64 },
                        "end": { "line": line_idx as u64, "character": after_end as u64 }
                    }
                }));
            }
            search_start = abs_pos + word.len();
        }
    }
    locations
}

fn encode_semantic_tokens(tokens: &[Token]) -> Vec<u64> {
    let mut encoded = Vec::new();
    let mut prev_line = 0u64;
    let mut prev_col = 0u64;
    let mut prev_keyword: Option<KeywordType> = None;
    let mut prev_was_hash = false;
    let mut inside_attribute = false;
    let mut pending_macro_identifier = false;
    let mut pending_function_params = false;
    let mut inside_function_params = false;
    for token in tokens {
        let is_hash = matches!(&token.ty, TokenType::Separator { separator: SeparatorType::Hash });
        if prev_was_hash
            && matches!(&token.ty, TokenType::Separator { separator: SeparatorType::OpenBracket })
        {
            inside_attribute = true;
        }
        if matches!(&token.ty, TokenType::Separator { separator: SeparatorType::OpenParen }) {
            if pending_function_params || prev_keyword == Some(KeywordType::Init) {
                inside_function_params = true;
                pending_function_params = false;
            }
        }
        if matches!(&token.ty, TokenType::Identifier)
            && (prev_keyword == Some(KeywordType::Func) || prev_keyword == Some(KeywordType::Init))
        {
            pending_function_params = true;
        }
        let is_directive = prev_was_hash
            && matches!(
                token.value.as_str(),
                "define" | "undef" | "if" | "ifdef" | "ifndef" | "else" | "elseif" | "endif"
                    | "error" | "warning"
            );
        if let Some((type_idx, modifier_bits)) = semantic_token_info(
            token,
            prev_keyword,
            prev_was_hash,
            inside_attribute,
            is_directive,
            pending_macro_identifier,
            inside_function_params,
        ) {
            let line = token.position.line as u64;
            let col = token.position.col as u64;
            let length = token.value.len() as u64;
            let delta_line = line - prev_line;
            let delta_col = if delta_line == 0 { col - prev_col } else { col };
            encoded.push(delta_line);
            encoded.push(delta_col);
            encoded.push(length);
            encoded.push(type_idx);
            encoded.push(modifier_bits);
            prev_line = line;
            prev_col = col;
        }
        if is_hash {
            prev_was_hash = true;
        } else if prev_was_hash {
            prev_was_hash = false;
        }
        if is_directive {
            pending_macro_identifier = matches!(token.value.as_str(), "define" | "ifdef" | "ifndef");
        } else {
            pending_macro_identifier = false;
        }
        if let TokenType::Separator { separator: SeparatorType::CloseBracket } = &token.ty {
            if inside_attribute {
                inside_attribute = false;
            }
        }
        if matches!(&token.ty, TokenType::Separator { separator: SeparatorType::CloseParen }) {
            inside_function_params = false;
        }
        if let TokenType::Keyword { keyword } = &token.ty {
            prev_keyword = Some(*keyword);
        } else {
            prev_keyword = None;
        }
    }
    encoded
}

fn semantic_token_info(
    token: &Token,
    prev_keyword: Option<KeywordType>,
    prev_was_hash: bool,
    inside_attribute: bool,
    is_directive: bool,
    pending_macro_identifier: bool,
    inside_function_params: bool,
) -> Option<(u64, u64)> {
    match &token.ty {
        TokenType::Keyword { keyword } => {
            let type_idx = match keyword {
                KeywordType::SelfType => 1,
                _ => 0,
            };
            Some((type_idx, 0))
        }
        TokenType::Identifier => {
            if is_directive {
                Some((0, 0))
            } else if prev_was_hash {
                Some((10, 0))
            } else if pending_macro_identifier {
                Some((10, 0))
            } else if inside_attribute {
                Some((9, 0))
            } else {
                let first_char = token.value.chars().next().unwrap_or(' ');
                if first_char.is_uppercase() {
                    Some((1, 0))
                } else if prev_keyword == Some(KeywordType::Func) {
                    Some((2, 0))
                } else if inside_function_params {
                    Some((4, 0))
                } else {
                    Some((3, 0))
                }
            }
        }
        TokenType::StringLiteral { .. } | TokenType::CharLiteral { .. } => Some((5, 0)),
        TokenType::IntegerLiteral { .. } | TokenType::DecimalLiteral { .. } => Some((6, 0)),
        TokenType::BooleanLiteral { .. } | TokenType::NullLiteral | TokenType::NullptrLiteral => {
            Some((0, 0))
        }
        TokenType::Operator { .. } => Some((8, 0)),
        TokenType::Separator { separator } => match separator {
            SeparatorType::Hash => Some((0, 0)),
            SeparatorType::OpenBracket if inside_attribute => Some((0, 0)),
            SeparatorType::CloseBracket if inside_attribute => Some((0, 0)),
            _ => None,
        },
    }
}
