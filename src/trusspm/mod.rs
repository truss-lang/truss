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
    let current_file = std::path::Path::new(&home)
        .join(".trussup")
        .join("current.txt");
    let version = std::fs::read_to_string(current_file).ok()?;
    let version = version.trim().to_string();
    let std_dir = std::path::Path::new(&home)
        .join(".trussup")
        .join("toolchains")
        .join(&version)
        .join("stdlib");
    if std_dir.exists() {
        Some(std_dir.to_string_lossy().to_string())
    } else {
        None
    }
}
