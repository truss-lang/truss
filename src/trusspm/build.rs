use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use crate::{
    diag::TrussDiagnosticEngine,
    krate::Package,
    trusspm::{
        lock::LockManager,
        manifest::Manifest,
        resolver::DependencyResolver,
    },
};

pub struct BuildOrchestrator {
    pub packages: HashMap<String, Rc<RefCell<Package>>>,
    pub manifest: Manifest,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
}

impl BuildOrchestrator {
    pub fn new(project_dir: &str) -> Option<Self> {
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));

        let manifest = Manifest::from_project_dir(project_dir, engine.clone()).ok()?;
        let project_path = Path::new(project_dir);

        if LockManager::read(project_path).is_none() {
            LockManager::write(&manifest, project_path);
        }

        let packages = DependencyResolver::resolve(&manifest, project_path, engine.clone());

        Some(BuildOrchestrator {
            packages,
            manifest,
            engine,
        })
    }

    pub fn run_all_passes(&mut self, project_dir: &str) {
        let project_path = Path::new(project_dir);

        for (pkg_name, _pkg) in self.packages.clone() {
            let is_main = pkg_name == self.manifest.name;
            let _src_dir = project_path.join("Sources").join(&pkg_name);

            let mut file_stmts: Vec<Rc<RefCell<crate::ast::statement::Statement>>> = Vec::new();

            let files = DependencyResolver::discover_source_files(&pkg_name, project_path);
            for file in &files {
                let content = match std::fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let file_rc = Rc::new(file.to_string_lossy().to_string());
                let char_stream = crate::lexer::CharStream::new(content, file_rc.clone());
                let mut lexer = crate::lexer::Lexer::new(char_stream, self.engine.clone());
                let tokens = lexer.parse();
                if self.engine.borrow().has_errors() {
                    return;
                }
                let mut parser = crate::parser::Parser::new(file_rc, tokens, self.engine.clone());
                let program = parser.parse();
                if self.engine.borrow().has_errors() {
                    return;
                }
                for stmt in program.statements {
                    file_stmts.push(stmt);
                }
            }

            if file_stmts.is_empty() {
                continue;
            }

            let mut resolver =
                crate::symbol_resolver::SymbolResolver::new(self.packages.clone(), pkg_name.clone(), self.engine.clone());
            let dummy_program = crate::ast::node::Program {
                file: Rc::new(String::new()),
                statements: Vec::new(),
            };
            let module = resolver.resolve(&dummy_program, pkg_name.clone());

            if let Some(scope) = module.borrow().scope.clone() {
                resolver.enter_scope(Some(scope));
            }

            for stmt in &file_stmts {
                resolver.register_symbols(stmt.clone());
            }

            let mut type_resolver =
                crate::type_resolver::TypeResolver::new(self.packages.clone(), pkg_name.clone(), self.engine.clone());
            let empty_prog = crate::ast::node::Program {
                file: Rc::new(String::new()),
                statements: vec![],
            };
            type_resolver.resolve(&empty_prog, module);

            if self.engine.borrow().has_errors() {
                return;
            }

            if !is_main {
                continue;
            }

            let mut resolver2 = crate::symbol_resolver::SymbolResolver::new(
                self.packages.clone(),
                pkg_name.clone(),
                self.engine.clone(),
            );
            let main_program = crate::ast::node::Program {
                file: Rc::new(String::new()),
                statements: file_stmts.clone(),
            };
            let main_module = resolver2.resolve(&main_program, pkg_name.clone());
            let mut type_resolver2 = crate::type_resolver::TypeResolver::new(
                self.packages.clone(),
                pkg_name.clone(),
                self.engine.clone(),
            );
            type_resolver2.resolve(&main_program, main_module);
        }
    }

    pub fn has_errors(&self) -> bool {
        self.engine.borrow().has_errors()
    }
}
