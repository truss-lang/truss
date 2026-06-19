use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc};

use crate::{
    ast::{node::Program, statement::Statement},
    condition_eval::TargetTriple,
    diag::TrussDiagnosticEngine,
    ir_gen::{IRGenerator, IRModules, emit},
    krate::Package,
    lexer::{CharStream, Lexer},
    parser::Parser,
    symbol_resolver::SymbolResolver,
    trusspm::{
        lock::LockManager,
        manifest::{LibraryType, Manifest, ProductType},
        resolver::DependencyResolver,
    },
    type_resolver::TypeResolver,
};

pub struct BuildOrchestrator {
    pub packages: HashMap<String, Rc<RefCell<Package>>>,
    pub manifest: Manifest,
    engine: Rc<RefCell<TrussDiagnosticEngine>>,
    pub output_path: Option<String>,
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
            output_path: None,
        })
    }

    pub fn run_all_passes(&mut self, project_dir: &str) {
        let project_path = Path::new(project_dir);

        let _build_directives =
            crate::trusspm::cli::process_build_truss(project_path, &self.packages);

        // Auto-detect and load standard library from active toolchain
        let mut stdlib_stmts: Vec<Rc<RefCell<Statement>>> = Vec::new();
        if let Some(std_path) = crate::trusspm::find_stdlib_path() {
            let truss_pkg = Rc::new(RefCell::new(Package::new("Truss".to_string())));
            self.packages.insert("Truss".to_string(), truss_pkg.clone());

            let std_dir = std::path::Path::new(&std_path);
            let project_file = std_dir.join("Project.truss");

            // Discover std source files (project structure or flat)
            let mut std_files: Vec<std::path::PathBuf> = Vec::new();
            if project_file.exists() {
                let std_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                if let Ok(manifest) = Manifest::from_project_dir(&std_path, std_engine) {
                    std_files = DependencyResolver::discover_source_files(&manifest.name, std_dir);
                }
            }
            if std_files.is_empty() {
                let mut entries: Vec<_> = match std_dir.read_dir() {
                    Ok(entries) => entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().extension().is_some_and(|ext| ext == "truss"))
                        .collect(),
                    Err(_) => Vec::new(),
                };
                entries.sort_by_key(|e| e.file_name());
                std_files = entries.into_iter().map(|e| e.path()).collect();
            }

            if !std_files.is_empty() {
                // Parse std files
                let mut file_programs: Vec<Vec<Rc<RefCell<Statement>>>> = Vec::new();
                for path in &std_files {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let file_path = path.to_string_lossy().to_string();
                        let file_rc = Rc::new(file_path);
                        let file_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                        let char_stream = CharStream::new(content, file_rc.clone());
                        let mut lexer = Lexer::new(char_stream, file_engine.clone());
                        let tokens = lexer.parse();
                        if file_engine.borrow().has_errors() {
                            continue;
                        }
                        let mut parser = Parser::new(file_rc, tokens, file_engine.clone());
                        let program = parser.parse();
                        if file_engine.borrow().has_errors() {
                            continue;
                        }
                        self.engine.borrow_mut().extend(file_engine.take());
                        file_programs.push(program.statements);
                    }
                }

                if !file_programs.is_empty() {
                    // Collect all std statements
                    for file_stmts in &file_programs {
                        for stmt in file_stmts {
                            stdlib_stmts.push(stmt.clone());
                        }
                    }

                    // Symbol resolve std library
                    let std_prog = Program {
                        file: Rc::new("stdlib".to_string()),
                        statements: stdlib_stmts.clone(),
                    };
                    let std_engine2 = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                    let mut std_resolver = SymbolResolver::new(
                        self.packages.clone(),
                        "Truss".to_string(),
                        std_engine2.clone(),
                    );
                    let std_module = std_resolver.resolve(&std_prog, "Truss".to_string());

                    // Type resolve std library
                    let std_ty_engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
                    let mut std_type_resolver = TypeResolver::new(
                        self.packages.clone(),
                        "Truss".to_string(),
                        std_ty_engine.clone(),
                    );
                    std_type_resolver.resolve(&std_prog, std_module);
                }
            }
        }

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

                let file_ir_gen = IRGenerator::new(&context, ir_engine.clone())
                    .with_namespace(&pkg_name, &pkg_name);
                let file_modules_result =
                    file_ir_gen.generate_with_stdlib(&program, &stdlib_stmts, main_scope.clone());
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

            let combined_module = if file_modules.is_empty() {
                let single_ir_gen = IRGenerator::new(&context, ir_engine.clone())
                    .with_namespace(&pkg_name, &pkg_name);
                let modules = single_ir_gen.generate_with_stdlib(&prog, &stdlib_stmts, main_scope);
                if ir_engine.borrow().has_errors() {
                    let formatted = duck_diagnostic::format_all_smart(&*ir_engine.borrow(), false);
                    if !formatted.is_empty() {
                        eprintln!("{}", formatted);
                    }
                    return;
                }
                modules
            } else if file_modules.len() == 1 {
                IRModules {
                    main: Rc::new(file_modules[0].1.clone()),
                    stdlib: None,
                }
            } else {
                let target_module = file_modules.remove(0);
                for (_, module) in &file_modules {
                    for func in module.get_functions() {
                        let name = func.get_name().to_str().unwrap_or("").to_string();
                        if !name.is_empty() && target_module.1.get_function(&name).is_none() {
                            let fn_type = func.get_type();
                            target_module.1.add_function(&name, fn_type, None);
                        }
                    }
                }
                IRModules {
                    main: Rc::new(target_module.1),
                    stdlib: None,
                }
            };

            let triple = TargetTriple::host().to_triple_string();

            let kind = self
                .manifest
                .products
                .first()
                .map(|p| p.product_type)
                .unwrap_or(ProductType::Executable);

            let build_dir = project_path.join("build");
            std::fs::create_dir_all(&build_dir).ok();

            let output_name = match kind {
                ProductType::Executable => self.manifest.name.clone(),
                ProductType::Library(LibraryType::Dynamic) => {
                    format!("lib{}.so", self.manifest.name)
                }
                ProductType::Library(LibraryType::Static) => format!("lib{}.a", self.manifest.name),
            };
            let output_path = build_dir.join(&output_name);
            let output_str = output_path.to_string_lossy().to_string();

            match emit::emit_output(
                &combined_module.main,
                combined_module.stdlib.as_deref(),
                &triple,
                &output_str,
                kind,
            ) {
                Ok(()) => {
                    self.output_path = Some(output_str);
                    println!("Build succeeded: {}", output_path.display());
                }
                Err(e) => {
                    eprintln!("Emit failed: {}", e);
                    self.engine.borrow_mut().emit(crate::diag::new_diagnostic(
                        crate::diag::TrussDiagnosticCode::IRError,
                        format!("Failed to emit output: {}", e),
                    ));
                }
            }
        }
    }

    pub fn has_errors(&self) -> bool {
        self.engine.borrow().has_errors()
    }
}
