use std::{cell::RefCell, path::Path, rc::Rc};

use crate::{
    ast::statement::Statement,
    diag::TrussDiagnosticEngine,
    lexer::{CharStream, Lexer},
    parser::Parser,
};

pub fn parse_std_lib(
    stdlib_path: &str,
    file_rc: Rc<String>,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
) -> Vec<Vec<Rc<RefCell<Statement>>>> {
    let dir = Path::new(stdlib_path);
    let mut entries: Vec<_> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "truss"))
            .collect(),
        Err(_) => return Vec::new(),
    };
    entries.sort_by_key(|e| e.file_name());

    let mut results = Vec::new();

    for entry in entries {
        let path = entry.path();
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let char_stream = CharStream::new(content, file_rc.clone());
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();

        if engine.borrow().has_errors() {
            return results;
        }

        let mut parser = Parser::new(file_rc.clone(), tokens, engine.clone());
        let program = parser.parse();

        if engine.borrow().has_errors() {
            return results;
        }

        results.push(program.statements);
    }

    results
}
