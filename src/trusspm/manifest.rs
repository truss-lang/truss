use std::{cell::RefCell, fmt, rc::Rc};

use crate::diag::TrussDiagnosticEngine;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TargetKind {
    Executable,
    DynamicLibrary,
    StaticLibrary,
}

impl TargetKind {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Executable" => Some(TargetKind::Executable),
            "DynamicLibrary" => Some(TargetKind::DynamicLibrary),
            "StaticLibrary" => Some(TargetKind::StaticLibrary),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            TargetKind::Executable => "Executable",
            TargetKind::DynamicLibrary => "DynamicLibrary",
            TargetKind::StaticLibrary => "StaticLibrary",
        }
    }
}

impl fmt::Display for TargetKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub target_triple: Option<String>,
    pub targets: Vec<ManifestTarget>,
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestTarget {
    pub name: String,
    pub kind: TargetKind,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManifestDependency {
    pub name: String,
    pub url: Option<String>,
    pub path: Option<String>,
    pub version: Option<String>,
}

impl Manifest {
    pub fn from_project_dir(
        project_dir: &str,
        _engine: Rc<RefCell<TrussDiagnosticEngine>>,
    ) -> Result<Manifest, String> {
        let project_path = std::path::Path::new(project_dir).join("Project.truss");
        let content = std::fs::read_to_string(&project_path)
            .map_err(|_| format!("Project.truss not found in '{}'", project_dir))?;

        let file_rc = Rc::new(project_path.to_string_lossy().to_string());
        let char_stream = crate::lexer::CharStream::new(content.clone(), file_rc.clone());
        let engine = Rc::new(RefCell::new(TrussDiagnosticEngine::new()));
        let mut lexer = crate::lexer::Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();

        if engine.borrow().has_errors() {
            return Err("Failed to lex Project.truss".to_string());
        }

        let mut parser = crate::parser::Parser::new(file_rc.clone(), tokens, engine.clone());
        let program = parser.parse();

        if engine.borrow().has_errors() {
            return Err("Failed to parse Project.truss".to_string());
        }

        crate::trusspm::extractor::extract_manifest(&program)
            .ok_or_else(|| "Project.truss: expected 'let project = Project(...)'".to_string())
    }
}
