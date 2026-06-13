use std::{cell::RefCell, rc::Rc};

use crate::{
    ast::{expression::Expression, node::Program, statement::Statement},
    trusspm::manifest::{Manifest, ManifestDependency, ManifestTarget, TargetKind},
};

pub fn extract_manifest(program: &Program) -> Option<Manifest> {
    let stmt = program.statements.iter().find_map(|s| {
        let s_borrow = s.borrow();
        match &*s_borrow {
            Statement::VariableDecl {
                name, initializer, ..
            } => {
                if name.value == "project" {
                    if let Some(init) = initializer {
                        let init_borrow = init.borrow();
                        if let Expression::Call {
                            callee, parameters, ..
                        } = &*init_borrow
                        {
                            let callee_borrow = callee.borrow();
                            if let Expression::Variable {
                                name: callee_name, ..
                            } = &*callee_borrow
                            {
                                if callee_name.value == "Project" {
                                    return Some(parameters.clone());
                                }
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    })?;

    extract_project_call(&stmt)
}

fn extract_project_call(parameters: &[crate::ast::expression::CallParameter]) -> Option<Manifest> {
    let mut name = None;
    let mut version = None;
    let mut target_triple = None;
    let mut targets = Vec::new();
    let mut dependencies = Vec::new();

    for param in parameters {
        let label = param.label.as_ref().map(|t| t.value.as_str());
        match label {
            Some("name") => {
                name = extract_string(param);
            }
            Some("version") => {
                version = extract_string(param);
            }
            Some("target_triple") | Some("target") => {
                target_triple = extract_string(param);
            }
            Some("targets") => {
                if let Some(items) = extract_array(param) {
                    for item in items {
                        if let Some(t) = extract_target(&item) {
                            targets.push(t);
                        }
                    }
                }
            }
            Some("dependencies") => {
                if let Some(items) = extract_array(param) {
                    for item in items {
                        if let Some(d) = extract_dependency(&item) {
                            dependencies.push(d);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let name = name?;
    let version = version.unwrap_or_else(|| "0.1.0".to_string());

    Some(Manifest {
        name,
        version,
        target_triple,
        targets,
        dependencies,
    })
}

fn extract_string(param: &crate::ast::expression::CallParameter) -> Option<String> {
    let expr = param.expression.borrow();
    if let Expression::StringLiteral { value, .. } = &*expr {
        Some(value.clone())
    } else {
        None
    }
}

fn extract_array(
    param: &crate::ast::expression::CallParameter,
) -> Option<Vec<Rc<RefCell<Expression>>>> {
    let expr = param.expression.borrow();
    if let Expression::ArrayLiteral { elements, .. } = &*expr {
        Some(elements.clone())
    } else {
        None
    }
}

fn extract_target(expr: &Rc<RefCell<Expression>>) -> Option<ManifestTarget> {
    let e = expr.borrow();
    if let Expression::Call {
        callee, parameters, ..
    } = &*e
    {
        let callee_borrow = callee.borrow();
        if let Expression::Variable { name, .. } = &*callee_borrow {
            if name.value == "Target" {
                return extract_target_call(parameters);
            }
        }
    }
    None
}

fn extract_target_call(
    parameters: &[crate::ast::expression::CallParameter],
) -> Option<ManifestTarget> {
    let mut name = None;
    let mut kind = None;
    let mut deps = Vec::new();

    for param in parameters {
        let label = param.label.as_ref().map(|t| t.value.as_str());
        match label {
            Some("name") => {
                name = extract_string(param);
            }
            Some("kind") => {
                kind = extract_target_kind(param);
            }
            Some("dependencies") => {
                if let Some(items) = extract_array(param) {
                    for item in items {
                        let item_borrow = item.borrow();
                        if let Expression::StringLiteral { value, .. } = &*item_borrow {
                            deps.push(value.clone());
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Some(ManifestTarget {
        name: name?,
        kind: kind.unwrap_or(TargetKind::Executable),
        dependencies: deps,
    })
}

fn extract_target_kind(param: &crate::ast::expression::CallParameter) -> Option<TargetKind> {
    let expr = param.expression.borrow();
    if let Expression::ImplicitMemberAccess { member, .. } = &*expr {
        TargetKind::from_str(&member.value)
    } else {
        None
    }
}

fn extract_dependency(expr: &Rc<RefCell<Expression>>) -> Option<ManifestDependency> {
    let e = expr.borrow();
    if let Expression::Call {
        callee, parameters, ..
    } = &*e
    {
        let callee_borrow = callee.borrow();
        if let Expression::Variable { name, .. } = &*callee_borrow {
            if name.value == "Dependency" {
                return extract_dependency_call(parameters);
            }
        }
    }
    None
}

fn extract_dependency_call(
    parameters: &[crate::ast::expression::CallParameter],
) -> Option<ManifestDependency> {
    let mut dep_name = None;
    let mut url = None;
    let mut path = None;
    let mut version = None;

    for param in parameters {
        let label = param.label.as_ref().map(|t| t.value.as_str());
        match label {
            Some("name") => {
                dep_name = extract_string(param);
            }
            Some("url") => {
                url = extract_string(param);
            }
            Some("path") => {
                path = extract_string(param);
            }
            Some("version") => {
                version = extract_string(param);
            }
            _ => {}
        }
    }

    let dep_name = dep_name?;

    if path.is_none() && url.is_none() {
        path = Some(format!("../{}", dep_name));
    }

    Some(ManifestDependency {
        name: dep_name,
        url,
        path,
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{CharStream, Lexer};
    use crate::parser::Parser;
    use std::rc::Rc;

    fn parse_project(code: &str) -> Option<Manifest> {
        let engine = Rc::new(RefCell::new(crate::diag::TrussDiagnosticEngine::new()));
        let file_rc = Rc::new("Project.truss".to_string());
        let char_stream = CharStream::new(code.to_string(), file_rc.clone());
        let mut lexer = Lexer::new(char_stream, engine.clone());
        let tokens = lexer.parse();
        let mut parser = Parser::new(file_rc, tokens, engine);
        let program = parser.parse();
        extract_manifest(&program)
    }

    #[test]
    fn test_extract_full() {
        let code = r#"let project = Project(
            name: "my-app",
            version: "0.1.0",
            targets: [
                Target(name: "my-app", kind: .Executable)
            ],
            dependencies: [
                Dependency(name: "http", url: "https://github.com/truss-lang/http", version: "0.1.0")
            ]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.name, "my-app");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.targets.len(), 1);
        assert_eq!(m.targets[0].name, "my-app");
        assert_eq!(m.targets[0].kind, TargetKind::Executable);
        assert_eq!(m.dependencies.len(), 1);
        assert_eq!(m.dependencies[0].name, "http");
        assert_eq!(
            m.dependencies[0].url.as_deref(),
            Some("https://github.com/truss-lang/http")
        );
    }

    #[test]
    fn test_extract_minimal() {
        let code = r#"let project = Project(
            name: "my-app",
            version: "0.1.0",
            targets: [
                Target(name: "my-app", kind: .Executable)
            ]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.name, "my-app");
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn test_extract_no_deps() {
        let code = r#"let project = Project(
            name: "my-app",
            targets: [
                Target(name: "my-app", kind: .Executable)
            ]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.version, "0.1.0");
        assert!(m.dependencies.is_empty());
    }

    #[test]
    fn test_extract_no_version() {
        let code = r#"let project = Project(
            name: "my-app",
            targets: [
                Target(name: "my-app", kind: .Executable)
            ]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.version, "0.1.0");
    }

    #[test]
    fn test_extract_target_with_deps() {
        let code = r#"let project = Project(
            name: "my-app",
            targets: [
                Target(name: "my-app", kind: .Executable, dependencies: ["http", "json"])
            ],
            dependencies: [
                Dependency(name: "http", url: "https://github.com/truss-lang/http"),
                Dependency(name: "json")
            ]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.targets[0].dependencies, vec!["http", "json"]);
        assert_eq!(m.dependencies[1].name, "json");
        assert_eq!(m.dependencies[1].path.as_deref(), Some("../json"));
    }

    #[test]
    fn test_extract_dep_auto_path() {
        let code = r#"let project = Project(
            name: "my-app",
            targets: [Target(name: "my-app", kind: .Executable)],
            dependencies: [Dependency(name: "json")]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.dependencies[0].path.as_deref(), Some("../json"));
        assert!(m.dependencies[0].url.is_none());
    }

    #[test]
    fn test_extract_dynamic_library() {
        let code = r#"let project = Project(
            name: "my-lib",
            targets: [Target(name: "my-lib", kind: .DynamicLibrary)]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.targets[0].kind, TargetKind::DynamicLibrary);
    }

    #[test]
    fn test_extract_static_library() {
        let code = r#"let project = Project(
            name: "my-lib",
            targets: [Target(name: "my-lib", kind: .StaticLibrary)]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.targets[0].kind, TargetKind::StaticLibrary);
    }

    #[test]
    fn test_extract_default_kind() {
        let code = r#"let project = Project(
            name: "my-app",
            targets: [Target(name: "my-app")]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.targets[0].kind, TargetKind::Executable);
    }

    #[test]
    fn test_extract_target_triple() {
        let code = r#"let project = Project(
            name: "my-app",
            target_triple: "x86_64-unknown-linux-gnu",
            targets: [Target(name: "my-app", kind: .Executable)]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(
            m.target_triple.as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
    }

    #[test]
    fn test_extract_target_alias() {
        let code = r#"let project = Project(
            name: "my-app",
            target: "aarch64-apple-darwin",
            targets: [Target(name: "my-app", kind: .Executable)]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert_eq!(m.target_triple.as_deref(), Some("aarch64-apple-darwin"));
    }

    #[test]
    fn test_extract_target_triple_default() {
        let code = r#"let project = Project(
            name: "my-app",
            targets: [Target(name: "my-app", kind: .Executable)]
        )"#;
        let m = parse_project(code).expect("should parse");
        assert!(m.target_triple.is_none());
    }
}
