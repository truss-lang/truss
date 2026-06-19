pub mod build;
pub mod cli;
pub mod extractor;
pub mod lock;
pub mod manifest;
pub mod resolver;

/// Find the path to the standard library from the active toolchain.
/// Returns `Some(path)` if the toolchain version is set and its stdlib directory exists.
pub fn find_stdlib_path() -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let trussup_dir = std::path::Path::new(&home).join(".trussup");

    let current_file = trussup_dir.join("current.txt");
    if let Ok(version) = std::fs::read_to_string(&current_file) {
        let version = version.trim().to_string();
        let toolchain_std = trussup_dir.join("toolchains").join(&version).join("stdlib");
        if toolchain_std.exists() {
            return Some(toolchain_std.to_string_lossy().to_string());
        }
    }

    let standalone_std = trussup_dir.join("stdlib");
    if standalone_std.exists() {
        return Some(standalone_std.to_string_lossy().to_string());
    }

    None
}
