use std::process::Command;

fn compile(args: &[&str]) -> (bool, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_trussc"))
        .args(args)
        .output()
        .expect("Failed to run trussc");
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    (output.status.success(), stdout, stderr)
}

#[test]
fn test_trussc_single_file() {
    let dir = std::env::temp_dir().join(format!("trussc_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("hello.truss");
    std::fs::write(&src, "#[main]\nfunc main() -> Int32 { 0 }\n").unwrap();

    let out = dir.join("hello.out");
    let (ok, stdout, stderr) = compile(&[&src.to_string_lossy(), "-o", &out.to_string_lossy(), "--target", "x86_64-unknown-linux-gnu"]);
    assert!(ok, "trussc should succeed\nstdout:{}\nstderr:{}", stdout, stderr);
    assert!(out.exists(), "Output file should exist: {:?}\nstdout:{}\nstderr:{}", out, stdout, stderr);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_trussc_single_file_ir_only() {
    let dir = std::env::temp_dir().join(format!("trussc_ir_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let src = dir.join("hello.truss");
    std::fs::write(&src, "#[main]\nfunc main() -> Int32 { 0 }\n").unwrap();

    let (ok, stdout, stderr) = compile(&[&src.to_string_lossy(), "--ir"]);
    // IR output is valid even without cc
    assert!(ok, "trussc --ir should succeed\nstdout:{}\nstderr:{}", stdout, stderr);
    assert!(stdout.contains("LLVM IR"), "Should contain LLVM IR output: {}", stdout);

    let _ = std::fs::remove_dir_all(&dir);
}
