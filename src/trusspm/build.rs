use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use crate::{
    ast::{node::Program, statement::Statement},
    diag::TrussDiagnosticEngine,
    ir_gen::IRGenerator,
    krate::Package,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    trusspm::{lock::LockManager, manifest::Manifest, resolver::DependencyResolver},
    type_resolver::TypeResolver,
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

        let _build_directives =
            crate::trusspm::cli::process_build_truss(project_path, &self.packages);

        for (pkg_name, _pkg) in self.packages.clone() {
            let is_main = pkg_name == self.manifest.name;

            let mut file_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();

            let files = DependencyResolver::discover_source_files(&pkg_name, project_path);
            for file in &files {
                let content = match std::fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let file_rc = Rc::new(file.to_string_lossy().to_string());
                let char_stream = CharStream::new(content, file_rc.clone());
                let mut lexer = Lexer::new(char_stream, self.engine.clone());
                let tokens = lexer.parse();
                if self.engine.borrow().has_errors() {
                    return;
                }
                let mut parser = Parser::new(file_rc, tokens, self.engine.clone());
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
                SymbolResolver::new(self.packages.clone(), pkg_name.clone(), self.engine.clone());
            let dummy_program = Program {
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
                TypeResolver::new(self.packages.clone(), pkg_name.clone(), self.engine.clone());
            let empty_prog = Program {
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

            let mut resolver2 =
                SymbolResolver::new(self.packages.clone(), pkg_name.clone(), self.engine.clone());
            let prog = Program {
                file: Rc::new(String::new()),
                statements: file_stmts.clone(),
            };
            let main_module = resolver2.resolve(&prog, pkg_name.clone());
            let mut type_resolver2 =
                TypeResolver::new(self.packages.clone(), pkg_name.clone(), self.engine.clone());
            type_resolver2.resolve(&prog, main_module.clone());

            if self.engine.borrow().has_errors() {
                return;
            }

            let context = inkwell::context::Context::create();
            let ir_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
            let scope = main_module.borrow().scope.clone();
            let main_scope = match scope {
                Some(s) => s,
                None => {
                    self.engine.borrow_mut().emit(crate::diag::new_diagnostic(
                        crate::diag::TrussDiagnosticCode::IRError,
                        "No scope found for main module",
                    ));
                    return;
                }
            };

            let files = DependencyResolver::discover_source_files(&pkg_name, project_path);
            let mut file_modules: Vec<(String, inkwell::module::Module<'_>)> = Vec::new();

            for file in &files {
                let content = match std::fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let file_rc = Rc::new(file.to_string_lossy().to_string());
                let char_stream = CharStream::new(content, file_rc.clone());
                let mut lexer = Lexer::new(char_stream, self.engine.clone());
                let tokens = lexer.parse();
                if self.engine.borrow().has_errors() {
                    return;
                }
                let mut parser = Parser::new(file_rc, tokens, self.engine.clone());
                let program = parser.parse();
                if self.engine.borrow().has_errors() {
                    return;
                }

                let file_ir_gen = IRGenerator::new(&context, ir_engine.clone());
                let file_modules_result =
                    file_ir_gen.generate_with_stdlib(&program, &[], main_scope.clone());
                file_modules.push((
                    file.to_string_lossy().to_string(),
                    (*file_modules_result.main).clone(),
                ));
            }

            if ir_engine.borrow().has_errors() {
                let formatted = duck_diagnostic::format_all_smart(&*ir_engine.borrow(), false);
                if !formatted.is_empty() {
                    eprintln!("{}", formatted);
                }
                return;
            }

            if file_modules.is_empty() {
                let single_ir_gen = IRGenerator::new(&context, ir_engine.clone());
                let modules = single_ir_gen.generate_with_stdlib(&prog, &[], main_scope);
                if ir_engine.borrow().has_errors() {
                    let formatted = duck_diagnostic::format_all_smart(&*ir_engine.borrow(), false);
                    if !formatted.is_empty() {
                        eprintln!("{}", formatted);
                    }
                    return;
                }
                let ir = modules.main.print_to_string().to_string();
                println!("{}", ir);
            } else if file_modules.len() == 1 {
                let ir = file_modules[0].1.print_to_string().to_string();
                println!("{}", ir);
            } else {
                let target = file_modules.remove(0);
                for (file_name, module) in &file_modules {
                    for func in module.get_functions() {
                        let name = func.get_name().to_str().unwrap_or("").to_string();
                        if !name.is_empty() && target.1.get_function(&name).is_none() {
                            let fn_type = func.get_type();
                            target.1.add_function(&name, fn_type, None);
                        }
                    }
                    println!("; LLVM Module: {}", file_name);
                    let ir = module.print_to_string().to_string();
                    println!("{}", ir);
                }
                println!("; LLVM Module: {} (linked)", target.0);
                let ir = target.1.print_to_string().to_string();
                println!("{}", ir);
            }
        }
    }

    pub fn has_errors(&self) -> bool {
        self.engine.borrow().has_errors()
    }
}
