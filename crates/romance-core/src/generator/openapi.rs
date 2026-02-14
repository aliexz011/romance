use anyhow::Result;
use std::process::Command;

pub fn generate() -> Result<()> {
    println!("Generating OpenAPI spec...");
    let status = Command::new("cargo")
        .args(["run", "--bin", "openapi-export"])
        .current_dir("backend")
        .status()?;
    if !status.success() {
        anyhow::bail!("OpenAPI generation failed");
    }
    println!("OpenAPI spec generated at backend/openapi.json");
    Ok(())
}
