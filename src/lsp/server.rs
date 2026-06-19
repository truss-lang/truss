use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::rc::Rc;

use serde_json::{json, Value};

use duck_diagnostic::DiagnosticCode;

use crate::diag::TrussDiagnosticEngine;
use crate::lexer::token::{KeywordType, Token, TokenType};
use crate::lexer::{CharStream, Lexer};
use crate::parser::Parser;

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

fn collect_diagnostics_from_engine(
    engine: &TrussDiagnosticEngine,
    _content: &str,
) -> Vec<Value> {
    let mut diagnostics = Vec::new();
    for diag in engine.get_diagnostics() {
        let severity = match diag.severity {
            duck_diagnostic::Severity::Error | duck_diagnostic::Severity::Bug => 1,
            duck_diagnostic::Severity::Warning => 2,
            duck_diagnostic::Severity::Note => 3,
            duck_diagnostic::Severity::Help => 4,
        };
        let (start_line, start_col) = if let Some(label) = diag.primary_label() {
            (label.span.line, label.span.column)
        } else {
            (1, 1)
        };
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
    let mut server = LanguageServer {
        documents: HashMap::new(),
        exit: false,
    };
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

struct LanguageServer {
    documents: HashMap<String, String>,
    exit: bool,
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
                vec![self.handle_completion(id)]
            }
            "textDocument/hover" => {
                vec![self.handle_hover(id, params)]
            }
            "textDocument/definition" => {
                vec![self.handle_definition(id)]
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
                    "version": "0.1.0"
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

    fn publish_diagnostics(&self, params: Option<&Value>) -> Vec<String> {
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

    fn run_diagnostics(&self, uri: &str, content: &str) -> Vec<Value> {
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
        let _program = parser.parse();
        let parser_diags = collect_diagnostics_from_engine(&parser_engine.borrow(), content);
        diagnostics.extend(parser_diags);
        diagnostics
    }

    fn handle_completion(&self, id: Option<u64>) -> String {
        let items = vec![
            json!({"label": "func", "kind": 14, "detail": "keyword"}),
            json!({"label": "let", "kind": 14, "detail": "keyword"}),
            json!({"label": "var", "kind": 14, "detail": "keyword"}),
            json!({"label": "return", "kind": 14, "detail": "keyword"}),
            json!({"label": "if", "kind": 14, "detail": "keyword"}),
            json!({"label": "else", "kind": 14, "detail": "keyword"}),
            json!({"label": "for", "kind": 14, "detail": "keyword"}),
            json!({"label": "while", "kind": 14, "detail": "keyword"}),
            json!({"label": "struct", "kind": 14, "detail": "keyword"}),
            json!({"label": "class", "kind": 14, "detail": "keyword"}),
            json!({"label": "enum", "kind": 14, "detail": "keyword"}),
            json!({"label": "protocol", "kind": 14, "detail": "keyword"}),
            json!({"label": "extension", "kind": 14, "detail": "keyword"}),
            json!({"label": "public", "kind": 14, "detail": "keyword"}),
            json!({"label": "private", "kind": 14, "detail": "keyword"}),
            json!({"label": "internal", "kind": 14, "detail": "keyword"}),
            json!({"label": "import", "kind": 14, "detail": "keyword"}),
            json!({"label": "match", "kind": 14, "detail": "keyword"}),
            json!({"label": "true", "kind": 21, "detail": "literal"}),
            json!({"label": "false", "kind": 21, "detail": "literal"}),
            json!({"label": "null", "kind": 21, "detail": "literal"}),
            json!({"label": "Self", "kind": 15, "detail": "type"}),
            json!({"label": "self", "kind": 21, "detail": "keyword"}),
            json!({"label": "super", "kind": 21, "detail": "keyword"}),
            json!({"label": "init", "kind": 15, "detail": "keyword"}),
            json!({"label": "deinit", "kind": 15, "detail": "keyword"}),
        ];
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

    fn handle_hover(&self, id: Option<u64>, _params: Option<&Value>) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null
        })
        .to_string()
    }

    fn handle_definition(&self, id: Option<u64>) -> String {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": null
        })
        .to_string()
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
            let delta_col = if delta_line == 0 {
                col - prev_col
            } else {
                col
            };
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

fn semantic_token_info(
    token: &Token,
    prev_keyword: Option<KeywordType>,
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
        TokenType::BooleanLiteral { .. }
        | TokenType::NullLiteral
        | TokenType::NullptrLiteral => Some((0, 0)),
        TokenType::Operator { .. } => Some((8, 0)),
        TokenType::Separator { .. } => None,
    }
}
