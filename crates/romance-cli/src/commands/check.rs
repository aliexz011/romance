use anyhow::{bail, Result};
use colored::Colorize;
use std::process::Command;

pub fn run() -> Result<()> {
    println!("{}", "Running checks...".bold());

    // cargo check
    print!("  cargo check... ");
    let status = Command::new("cargo")
        .args(["check"])
        .current_dir("backend")
        .status()?;
    if !status.success() {
        println!("{}", "FAILED".red());
        bail!("cargo check failed");
    }
    println!("{}", "OK".green());

    // cargo test
    print!("  cargo test... ");
    let status = Command::new("cargo")
        .args(["test"])
        .current_dir("backend")
        .status()?;
    if !status.success() {
        println!("{}", "FAILED".red());
        bail!("cargo test failed");
    }
    println!("{}", "OK".green());

    // tsc --noEmit
    print!("  tsc --noEmit... ");
    let status = Command::new("npx")
        .args(["tsc", "--noEmit"])
        .current_dir("frontend")
        .status()?;
    if !status.success() {
        println!("{}", "FAILED".red());
        bail!("tsc --noEmit failed");
    }
    println!("{}", "OK".green());

    println!("{}", "All checks passed!".green().bold());
    Ok(())
}
