use std::path::Path;

use crate::trusspm::manifest::Manifest;

pub struct LockManager;

impl LockManager {
    pub fn read(project_dir: &Path) -> Option<Vec<LockedDepJson>> {
        let lock_path = project_dir.join("Project.lock");
        let content = std::fs::read_to_string(lock_path).ok()?;
        parse_lock_json(&content)
    }

    pub fn write(manifest: &Manifest, project_dir: &Path) {
        let lock_path = project_dir.join("Project.lock");
        let mut content = String::from("{\n  \"version\": 1,\n  \"dependencies\": [\n");
        for (i, dep) in manifest.dependencies.iter().enumerate() {
            if i > 0 {
                content.push_str(",\n");
            }
            content.push_str(&format!("    {{\n      \"name\": \"{}\"", dep.name));
            if let Some(ref url) = dep.url {
                content.push_str(&format!(",\n      \"url\": \"{}\"", url));
            }
            if let Some(ref path) = dep.path {
                content.push_str(&format!(",\n      \"path\": \"{}\"", path));
            }
            if let Some(ref ver) = dep.version {
                content.push_str(&format!(",\n      \"version\": \"{}\"", ver));
            }
            content.push_str("\n    }");
        }
        content.push_str("\n  ]\n}\n");
        std::fs::write(lock_path, content).ok();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LockedDepJson {
    pub name: String,
    pub url: Option<String>,
    pub path: Option<String>,
    pub version: Option<String>,
}

fn parse_lock_json(content: &str) -> Option<Vec<LockedDepJson>> {
    let content = content.trim();
    if !content.starts_with('{') || !content.ends_with('}') {
        return None;
    }

    let deps_start = content.find("\"dependencies\"")?;
    let arr_start = content[deps_start..].find('[')? + deps_start;
    let arr_end = content[arr_start..].find(']')? + arr_start;
    let arr_content = &content[arr_start + 1..arr_end];

    let mut deps = Vec::new();
    let mut depth = 0;
    let mut obj_start = None;

    for (i, ch) in arr_content.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    obj_start = Some(i);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = obj_start {
                        let obj_str = &arr_content[start..=i];
                        if let Some(dep) = parse_dep_obj(obj_str) {
                            deps.push(dep);
                        }
                    }
                    obj_start = None;
                }
            }
            _ => {}
        }
    }

    Some(deps)
}

fn parse_dep_obj(obj: &str) -> Option<LockedDepJson> {
    let name = extract_json_str(obj, "name")?;
    let url = extract_json_str(obj, "url");
    let path = extract_json_str(obj, "path");
    let version = extract_json_str(obj, "version");
    Some(LockedDepJson {
        name,
        url,
        path,
        version,
    })
}

fn extract_json_str(content: &str, key: &str) -> Option<String> {
    let search = &format!("\"{}\"", key);
    let key_pos = content.find(search)?;
    let after_key = &content[key_pos + search.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim();
    if after_colon.starts_with('"') {
        let start = 1;
        let end = after_colon[1..].find('"')? + 1;
        Some(after_colon[start..end].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lock_json() {
        let json = r#"{
            "version": 1,
            "dependencies": [
                { "name": "http", "url": "https://github.com/truss-lang/http", "version": "0.1.0" },
                { "name": "json", "path": "../json" }
            ]
        }"#;
        let deps = parse_lock_json(json).expect("should parse");
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "http");
        assert_eq!(
            deps[0].url.as_deref(),
            Some("https://github.com/truss-lang/http")
        );
        assert_eq!(deps[1].name, "json");
        assert_eq!(deps[1].path.as_deref(), Some("../json"));
    }

    #[test]
    fn test_parse_lock_empty() {
        let json = r#"{"version": 1, "dependencies": []}"#;
        let deps = parse_lock_json(json).expect("should parse");
        assert!(deps.is_empty());
    }

    #[test]
    fn test_parse_lock_no_deps_key() {
        let json = r#"{"version": 1}"#;
        assert!(parse_lock_json(json).is_none());
    }
}
