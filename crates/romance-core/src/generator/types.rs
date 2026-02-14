use anyhow::Result;
use std::process::Command;

pub fn generate() -> Result<()> {
    println!("Generating TypeScript types from Rust structs...");
    let status = Command::new("cargo")
        .args(["test", "--", "export_bindings"])
        .current_dir("backend")
        .status()?;
    if !status.success() {
        anyhow::bail!("TypeScript type generation failed");
    }
    println!("TypeScript types exported to frontend/src/types/");
    Ok(())
}
