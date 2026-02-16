use anyhow::Result;
use colored::Colorize;
use std::process::Command;

pub fn migrate() -> Result<()> {
    println!("{}", "Running migrations...".bold());
    let status = Command::new("cargo")
        .args(["run", "-p", "migration", "--", "up"])
        .current_dir("backend")
        .status()?;
    if !status.success() {
        anyhow::bail!("Migration failed");
    }
    println!("{}", "Migrations applied successfully!".green());
    Ok(())
}

pub fn rollback() -> Result<()> {
    println!("{}", "Rolling back last migration...".bold());
    let status = Command::new("cargo")
        .args(["run", "-p", "migration", "--", "down"])
        .current_dir("backend")
        .status()?;
    if !status.success() {
        anyhow::bail!("Rollback failed");
    }
    println!("{}", "Rollback completed!".green());
    Ok(())
}

pub fn status() -> Result<()> {
    println!("{}", "Migration status:".bold());
    let status = Command::new("cargo")
        .args(["run", "-p", "migration", "--", "status"])
        .current_dir("backend")
        .status()?;
    if !status.success() {
        anyhow::bail!("Failed to get migration status");
    }
    Ok(())
}

pub fn seed() -> Result<()> {
    println!("{}", "Running seed data...".bold());

    let project_root = std::path::Path::new(".");

    // Generate seed file if it doesn't exist
    romance_core::seed::generate_seed_file(project_root)?;

    let status = Command::new("cargo")
        .args(["run", "--bin", "seed"])
        .current_dir("backend")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{}", "Seed data applied successfully!".green());
            Ok(())
        }
        _ => {
            // Fallback: try running as a test since seed might not be a binary
            println!("  Trying seed via cargo test...");
            let status = Command::new("cargo")
                .args(["test", "seed", "--", "--ignored"])
                .current_dir("backend")
                .status()?;
            if !status.success() {
                anyhow::bail!("Seed failed. Ensure backend/src/seed.rs is configured.");
            }
            println!("{}", "Seed data applied successfully!".green());
            Ok(())
        }
    }
}
