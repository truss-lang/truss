use std::path::Path;

use anyhow::{Context, Result};

use crate::trusspm::manifest::{LibraryType, ProductType};

pub fn initialize_targets() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        inkwell::targets::Target::initialize_all(
            &inkwell::targets::InitializationConfig::default(),
        );
    });
}

fn create_target_machine(triple: &str) -> Result<inkwell::targets::TargetMachine> {
    initialize_targets();
    let target =
        inkwell::targets::Target::from_triple(&inkwell::targets::TargetTriple::create(triple))
            .with_context(|| format!("Failed to get target for triple '{}'", triple))?;
    let tm = target
        .create_target_machine(
            &inkwell::targets::TargetTriple::create(triple),
            "generic",
            "",
            inkwell::OptimizationLevel::None,
            inkwell::targets::RelocMode::Default,
            inkwell::targets::CodeModel::Default,
        )
        .with_context(|| format!("Failed to create target machine for '{}'", triple))?;
    Ok(tm)
}

fn emit_object_file(
    module: &inkwell::module::Module,
    target_machine: &inkwell::targets::TargetMachine,
    output_path: &str,
) -> Result<()> {
    let buffer = target_machine
        .write_to_memory_buffer(module, inkwell::targets::FileType::Object)
        .with_context(|| "Failed to write object code via TargetMachine")?;
    let data = buffer.as_slice();
    std::fs::write(output_path, data)
        .with_context(|| format!("Failed to write object file to '{}'", output_path))?;
    Ok(())
}

pub fn emit_output(
    main_module: &inkwell::module::Module,
    stdlib_module: Option<&inkwell::module::Module>,
    triple: &str,
    output_path: &str,
    kind: ProductType,
) -> Result<()> {
    initialize_targets();
    let target_machine = create_target_machine(triple)?;
    let temp_dir = std::env::temp_dir().join(format!("truss_build_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).with_context(|| "Failed to create temp build directory")?;

    let mut object_files = Vec::new();

    let main_obj = temp_dir.join("main.o");
    emit_object_file(main_module, &target_machine, &main_obj.to_string_lossy())?;
    object_files.push(main_obj.to_string_lossy().to_string());

    if let Some(stdlib) = stdlib_module {
        let stdlib_obj = temp_dir.join("stdlib.o");
        emit_object_file(stdlib, &target_machine, &stdlib_obj.to_string_lossy())?;
        object_files.push(stdlib_obj.to_string_lossy().to_string());
    }

    let output = Path::new(output_path);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory '{}'", parent.display()))?;
    }

    match kind {
        ProductType::Executable => link_executable(&object_files, output_path, triple)?,
        ProductType::Library(LibraryType::Dynamic) => {
            link_dynamic_library(&object_files, output_path, triple)?
        }
        ProductType::Library(LibraryType::Static) => {
            create_static_library(&object_files, output_path)?
        }
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
    Ok(())
}

fn link_executable(object_files: &[String], output_path: &str, triple: &str) -> Result<()> {
    if triple.contains("darwin") || triple.contains("apple") {
        let mut cmd = std::process::Command::new("ld");
        cmd.arg("-o").arg(output_path);
        cmd.arg("-lc");
        cmd.arg("-lSystem");
        cmd.arg("-syslibroot")
            .arg("/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk");
        for obj in object_files {
            cmd.arg(obj);
        }
        let status = cmd.status().with_context(|| "Failed to run linker (ld)")?;
        if !status.success() {
            anyhow::bail!("Linker exited with status: {}", status);
        }
    } else if triple.contains("windows") || triple.contains("win32") {
        let mut cmd = std::process::Command::new("ld");
        cmd.arg("-o").arg(output_path);
        cmd.arg("/entry:main");
        cmd.arg("/defaultlib:libcmt");
        for obj in object_files {
            cmd.arg(obj);
        }
        let status = cmd.status().with_context(|| "Failed to run linker (ld)")?;
        if !status.success() {
            anyhow::bail!("Linker exited with status: {}", status);
        }
    } else {
        let mut cmd = std::process::Command::new("cc");
        cmd.arg("-o").arg(output_path);
        cmd.arg("-no-pie");
        for obj in object_files {
            cmd.arg(obj);
        }
        let status = cmd.status().with_context(|| "Failed to run linker (cc)")?;
        if !status.success() {
            anyhow::bail!("Linker exited with status: {}", status);
        }
    }
    Ok(())
}

fn link_dynamic_library(object_files: &[String], output_path: &str, triple: &str) -> Result<()> {
    if triple.contains("darwin") || triple.contains("apple") {
        let mut cmd = std::process::Command::new("ld");
        cmd.arg("-o").arg(output_path);
        cmd.arg("-dylib");
        cmd.arg("-lc");
        cmd.arg("-lSystem");
        cmd.arg("-syslibroot")
            .arg("/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk");
        for obj in object_files {
            cmd.arg(obj);
        }
        let status = cmd
            .status()
            .with_context(|| "Failed to link shared library")?;
        if !status.success() {
            anyhow::bail!("Linker (shared lib) exited with status: {}", status);
        }
    } else {
        let mut cmd = std::process::Command::new("cc");
        cmd.arg("-shared").arg("-o").arg(output_path);
        for obj in object_files {
            cmd.arg(obj);
        }
        let status = cmd
            .status()
            .with_context(|| "Failed to link shared library")?;
        if !status.success() {
            anyhow::bail!("Linker (shared lib) exited with status: {}", status);
        }
    }
    Ok(())
}

fn create_static_library(object_files: &[String], output_path: &str) -> Result<()> {
    let mut cmd = std::process::Command::new("ar");
    cmd.arg("rcs").arg(output_path);
    for obj in object_files {
        cmd.arg(obj);
    }
    let status = cmd.status().with_context(|| "Failed to run ar")?;
    if !status.success() {
        anyhow::bail!("ar exited with status: {}", status);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_targets() {
        initialize_targets();
    }

    #[test]
    fn test_create_target_machine() {
        initialize_targets();
        let tm = create_target_machine("x86_64-unknown-linux-gnu");
        assert!(tm.is_ok(), "Should create target machine for x86_64 linux");
    }
}
