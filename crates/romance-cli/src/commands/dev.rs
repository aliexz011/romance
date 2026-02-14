use anyhow::Result;
use std::process::Command;

pub fn run() -> Result<()> {
    println!("Starting development servers...");

    let mut backend = Command::new("cargo")
        .args(["watch", "-x", "run"])
        .current_dir("backend")
        .spawn()?;

    let mut frontend = Command::new("npm")
        .args(["run", "dev"])
        .current_dir("frontend")
        .spawn()?;

    // Wait for either to exit
    let _ = backend.wait();
    let _ = frontend.kill();
    let _ = frontend.wait();

    Ok(())
}
