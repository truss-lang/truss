use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use std::rc::Rc;

use serde_json::{Value, json};

use duck_diagnostic::DiagnosticCode;

use crate::ast::node::Program;
use crate::ast::statement::Statement;
use crate::diag::TrussDiagnosticEngine;
use crate::krate::{Module, Package};
use crate::lexer::token::{KeywordType, Token, TokenType};
use crate::lexer::{CharStream, Lexer};
use crate::parser::Parser;
use crate::scope::Scope;
use crate::symbol::Symbol;
use crate::symbol_resolver::SymbolResolver;
use crate::trusspm::manifest::Manifest;
use crate::trusspm::resolver::DependencyResolver;
use crate::type_resolver::TypeResolver;

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
        let (start_line, start_col, diag_file) = if let Some(label) = diag.primary_label() {
            (
                label.span.line,
                label.span.column,
                Some(label.span.file.clone()),
            )
        } else {
            (1, 1, None)
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
                    "character": start_col as u64
                }
            },
            "severity": severity,
            "message": format!("[{}] {}", diag.code.code(), diag.message)
        }));
    }
    diagnostics
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
        project_analyses: HashMap::new(),
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

struct LanguageServer {
    documents: HashMap<String, String>,
    exit: bool,
    stdlib_path: Option<String>,
    stdlib_scope: Option<Rc<RefCell<Scope>>>,
    project_analyses: HashMap<String, ProjectAnalysis>,
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
            "textDocument/semanticTokens/full" => {
                vec![self.handle_semantic_tokens(id, params)]
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
                    "completionProvider": {
                        "triggerCharacters": ["."]
                    },
                    "hoverProvider": true,
                    "definitionProvider": true,
                    "semanticTokensProvider": {
                        "full": true,
                        "legend": {
                            "tokenTypes": [
                                "keyword", "type", "function", "variable",
                                "parameter", "string", "number", "comment",
                                "operator", "property"
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

        let mut all_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
        for stmt in &program.statements {
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
                        let f_rc = Rc::new(path_str);
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
                        for stmt in prog.statements {
                            all_stmts.push(stmt);
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

        let mut type_resolver = TypeResolver::new(
            packages.clone(),
            "main".to_string(),
            analysis_engine.clone(),
        );
        type_resolver.resolve(&combined_prog, module.clone());
        let type_diags =
            collect_diagnostics_filtered(&analysis_engine.borrow(), content, Some(file_path));
        analysis_diags.extend(type_diags);

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
            let (file_programs, _) = crate::trusspm::parse_std_lib(&stdlib_path, engine.clone());
            if engine.borrow().has_errors() {
                return;
            }
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

            if engine.borrow().has_errors() {
                return;
            }

            let mut type_resolver =
                TypeResolver::new(packages.clone(), "Truss".to_string(), engine.clone());
            type_resolver.resolve(&std_prog, module.clone());

            if !engine.borrow().has_errors() {
                self.stdlib_scope = module.borrow().scope.clone();
            }
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
            if let Some(sym) = analysis.scope.borrow().get_symbol(name) {
                return Some((sym, file_path.clone()));
            }
        }
        None
    }

    fn symbol_type_string(&self, sym: &Symbol) -> Option<String> {
        if let Ok(Some(decl)) = sym.get_decl() {
            let stmt = decl.borrow();
            let ty = match &*stmt {
                Statement::FunctionDecl { ty, .. } => ty.clone(),
                Statement::VariableDecl { ty, .. } => ty.clone(),
                Statement::StructDecl { ty, .. } => ty.clone(),
                Statement::ClassDecl { ty, .. } => ty.clone(),
                Statement::EnumDecl { ty, .. } => ty.clone(),
                Statement::ProtocolDecl { ty, .. } => ty.clone(),
                Statement::InitDecl { ty, .. } => ty.clone(),
                Statement::DeinitDecl { ty, .. } => ty.clone(),
                Statement::SubscriptDecl { ty, .. } => ty.clone(),
                _ => None,
            };
            ty.map(|t| t.borrow().to_string())
        } else {
            None
        }
    }

    fn add_snippet_completions(&self, items: &mut Vec<Value>) {
        items.push(json!({"label": "fn", "kind": 14, "detail": "→ func", "insertText": "func", "insertTextFormat": 1, "sortText": "0"}));
        items.push(json!({"label": "func", "kind": 14, "detail": "function declaration", "insertText": "func ${1:name}(${2:params}) -> ${3:ReturnType} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "struct", "kind": 14, "detail": "struct declaration", "insertText": "struct ${1:Name}${2:: ${3:Protocol}} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "class", "kind": 14, "detail": "class declaration", "insertText": "class ${1:Name}${2:: ${3:SuperClass}} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "enum", "kind": 14, "detail": "enum declaration", "insertText": "enum ${1:Name}${2:: ${3:RawType}} {\n\tcase ${4:value}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "protocol", "kind": 14, "detail": "protocol declaration", "insertText": "protocol ${1:Name}${2:: ${3:ParentProtocol}} {\n\t${4:members}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "extension", "kind": 14, "detail": "type extension", "insertText": "extension ${1:Type}${2:: ${3:Protocol}} {\n\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "init", "kind": 14, "detail": "initializer declaration", "insertText": "init${1:?}(${2:params}) {\n\t${3:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "deinit", "kind": 14, "detail": "deinitializer", "insertText": "deinit {\n\t${1:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "subscript", "kind": 14, "detail": "subscript declaration", "insertText": "subscript(${1:params}) -> ${2:Type} {\n\tget {\n\t\treturn ${3:val}\n\t}\n\t${4:set {\n\t\t${5:newValue}\n\t}}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "typealias", "kind": 14, "detail": "type alias", "insertText": "typealias ${1:Name} = ${2:Type}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "if", "kind": 14, "detail": "if statement", "insertText": "if ${1:condition} {\n\t${2:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "ifelse", "kind": 14, "detail": "if-else statement", "insertText": "if ${1:condition} {\n\t${2:body}\n} else {\n\t${3:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "for", "kind": 14, "detail": "for-in loop", "insertText": "for ${1:item} in ${2:collection} {\n\t${3:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "while", "kind": 14, "detail": "while loop", "insertText": "while ${1:condition} {\n\t${2:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "repeat", "kind": 14, "detail": "repeat-while loop", "insertText": "repeat {\n\t${1:body}\n} while ${2:condition}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "match", "kind": 14, "detail": "match expression", "insertText": "match ${1:value} {\n\tcase ${2:pattern} =>\n\t\t${3:body}\n\tdefault =>\n\t\t${4:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "guard", "kind": 14, "detail": "guard statement", "insertText": "guard ${1:condition} else {\n\t${2:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "do", "kind": 14, "detail": "do-catch block", "insertText": "do {\n\t${1:body}\n} catch {\n\t${2:handler}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "defer", "kind": 14, "detail": "defer block", "insertText": "defer {\n\t${1:body}\n}", "insertTextFormat": 2, "sortText": "1"}));
        items.push(json!({"label": "public", "kind": 14, "detail": "public access modifier", "insertText": "public ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "private", "kind": 14, "detail": "private access modifier", "insertText": "private ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "open", "kind": 14, "detail": "open access modifier", "insertText": "open ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "override", "kind": 14, "detail": "override modifier", "insertText": "override ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "static", "kind": 14, "detail": "static modifier", "insertText": "static ${1:...}", "insertTextFormat": 2, "sortText": "2"}));
        items.push(json!({"label": "mutating", "kind": 14, "detail": "mutating modifier", "insertText": "mutating func ${1:name}($2) {\n\t$3\n}", "insertTextFormat": 2, "sortText": "2"}));
    }

    fn add_stdlib_completions(&self, items: &mut Vec<Value>) {
        if let Some(ref scope) = self.stdlib_scope {
            let sb = scope.borrow();
            for (name, _) in &sb.type_env {
                items.push(json!({"label": name, "kind": 5, "detail": "builtin type", "sortText": "3"}));
            }
            for (name, symbol) in &sb.name_table {
                let kind = match &*symbol.borrow() {
                    Symbol::Function { .. } => 3,
                    Symbol::Variable { .. } => 6,
                    _ => continue,
                };
                items.push(json!({"label": name, "kind": kind, "detail": "stdlib symbol", "sortText": "3"}));
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
                items.push(json!({"label": name, "kind": 5, "detail": "type", "sortText": "4"}));
            }
            for (name, symbol) in &sb.name_table {
                let (kind, detail) = match &*symbol.borrow() {
                    Symbol::Function { .. } => (3, "function"),
                    Symbol::Variable { .. } => (6, "variable"),
                    Symbol::Struct { .. } => (5, "struct"),
                    Symbol::Class { .. } => (5, "class"),
                    Symbol::Enum { .. } => (10, "enum"),
                    Symbol::Protocol { .. } => (8, "protocol"),
                    Symbol::StructProperty { .. } | Symbol::ClassProperty { .. } => (6, "property"),
                    Symbol::StructMethod { .. } | Symbol::ClassMethod { .. } => (3, "method"),
                    Symbol::EnumCase { .. } => (21, "enum case"),
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

        let result = if let Some((sym, _)) = self.lookup_symbol_in_scopes(&word) {
            let sym_borrow = sym.borrow();
            let sym_name = sym_borrow.name().unwrap_or_default();
            let type_str = self.symbol_type_string(&sym_borrow);
            let mut markdown = format!("```truss\n{}", sym_name);
            if let Some(ref ty) = type_str {
                markdown.push_str(&format!(": {}", ty));
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
                Symbol::EnumCase { .. } => "enum case",
                Symbol::Module { .. } => "module",
                Symbol::Macro { .. } => "macro",
                _ => "symbol",
            };
            markdown.push_str(&format!("\n{}", decl_info));
            markdown.push_str("\n```");
            Some(json!({"contents": {"kind": "markdown", "value": markdown}}))
        } else {
            None
        };

        match result {
            Some(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}).to_string(),
            None => json!({"jsonrpc": "2.0", "id": id, "result": null}).to_string(),
        }
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

        if let Some((sym, _)) = self.lookup_symbol_in_scopes(&word) {
            if let Ok(Some(decl)) = sym.borrow().get_decl() {
                let stmt = decl.borrow();
                let token = stmt.token();
                let pos = token.position;
                let decl_file = token.file.as_str().to_string();
                let decl_uri = if decl_file.starts_with('/') {
                    format!("file://{}", decl_file)
                } else {
                    decl_file
                };
                return json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "uri": decl_uri,
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
}

fn encode_semantic_tokens(tokens: &[Token]) -> Vec<u64> {
    let mut encoded = Vec::new();
    let mut prev_line = 0u64;
    let mut prev_col = 0u64;
    let mut prev_keyword: Option<KeywordType> = None;
    for token in tokens {
        if let Some((type_idx, modifier_bits)) = semantic_token_info(token, prev_keyword) {
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
        if let TokenType::Keyword { keyword } = &token.ty {
            prev_keyword = Some(*keyword);
        } else {
            prev_keyword = None;
        }
    }
    encoded
}

fn semantic_token_info(token: &Token, prev_keyword: Option<KeywordType>) -> Option<(u64, u64)> {
    match &token.ty {
        TokenType::Keyword { keyword } => {
            let type_idx = match keyword {
                KeywordType::SelfType => 1,
                _ => 0,
            };
            Some((type_idx, 0))
        }
        TokenType::Identifier => {
            let first_char = token.value.chars().next().unwrap_or(' ');
            if first_char.is_uppercase() {
                Some((1, 0))
            } else if prev_keyword == Some(KeywordType::Func) {
                Some((2, 0))
            } else {
                Some((3, 0))
            }
        }
        TokenType::StringLiteral { .. } | TokenType::CharLiteral { .. } => Some((5, 0)),
        TokenType::IntegerLiteral { .. } | TokenType::DecimalLiteral { .. } => Some((6, 0)),
        TokenType::BooleanLiteral { .. } | TokenType::NullLiteral | TokenType::NullptrLiteral => {
            Some((0, 0))
        }
        TokenType::Operator { .. } => Some((8, 0)),
        TokenType::Separator { .. } => None,
    }
}
