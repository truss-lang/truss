use std::{cell::RefCell, collections::HashMap, path::Path, rc::Rc, fs};

use crate::{
    diag::TrussDiagnosticEngine,
    krate::{Package, Module},
    trusspm::manifest::{Manifest, ManifestDependency},
};

pub struct DependencyResolver;

impl DependencyResolver {
    pub fn resolve(
        manifest: &Manifest,
        project_dir: &Path,
        _engine: Rc<RefCell<TrussDiagnosticEngine>>,
    ) -> HashMap<String, Rc<RefCell<Package>>> {
        let mut packages: HashMap<String, Rc<RefCell<Package>>> = HashMap::new();

        let main_pkg = Rc::new(RefCell::new(Package::new(manifest.name.clone())));
        packages.insert(manifest.name.clone(), main_pkg.clone());

        let src_dir = project_dir.join("Sources").join(&manifest.name);
        if src_dir.exists() {
            let root_module = Rc::new(RefCell::new(Module::new(manifest.name.clone())));
            main_pkg.borrow_mut().modules.insert(manifest.name.clone(), root_module.clone());
            let scope = Rc::new(RefCell::new(crate::scope::Scope::new(None)));
            root_module.borrow_mut().scope = Some(scope);
        }

        for dep in &manifest.dependencies {
            let dep_pkg = Self::resolve_dependency(dep, project_dir);
            packages.insert(dep.name.clone(), Rc::new(RefCell::new(dep_pkg)));
        }

        packages
    }

    fn resolve_dependency(dep: &ManifestDependency, project_dir: &Path) -> Package {
        let source_dir = if let Some(ref url) = dep.url {
            let cache_dir = project_dir.join(".truss-cache").join(&dep.name);
            if !cache_dir.exists() {
                let status = std::process::Command::new("git")
                    .args(["clone", "--depth", "1", url, &cache_dir.to_string_lossy()])
                    .status();
                match status {
                    Ok(s) if s.success() => {}
                    Ok(s) => {
                        eprintln!("Warning: git clone failed for '{}' (exit: {}), using cached if available", dep.name, s);
                    }
                    Err(e) => {
                        eprintln!("Warning: git not available for '{}': {}", dep.name, e);
                    }
                }
            }
            cache_dir.join("Sources").join(&dep.name)
        } else {
            let default_path = format!("../{}", dep.name);
            let path = dep.path.as_deref().unwrap_or(&default_path);
            project_dir.join(path).join("Sources").join(&dep.name)
        };

        let mut pkg = Package::new(dep.name.clone());

        if source_dir.exists() {
            let root_module = Rc::new(RefCell::new(Module::new(dep.name.clone())));
            pkg.modules.insert(dep.name.clone(), root_module.clone());
            let scope = Rc::new(RefCell::new(crate::scope::Scope::new(None)));
            root_module.borrow_mut().scope = Some(scope);
        }

        pkg
    }

    pub fn discover_source_files(package_name: &str, project_dir: &Path) -> Vec<std::path::PathBuf> {
        let src_dir = project_dir.join("Sources").join(package_name);
        if !src_dir.exists() {
            return Vec::new();
        }
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "truss") {
                    files.push(path);
                }
            }
        }
        files.sort();
        files
    }
}
