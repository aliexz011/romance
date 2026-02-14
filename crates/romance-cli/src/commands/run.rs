use anyhow::Result;
use colored::Colorize;
use std::process::Command;

pub fn run(command: &str, args: &[String]) -> Result<()> {
    println!(
        "{}",
        format!("Running management command: {}", command).bold()
    );

    // Build arguments for cargo run
    let mut cargo_args = vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--".to_string(),
        "run-command".to_string(),
        command.to_string(),
    ];
    cargo_args.extend(args.iter().cloned());

    let status = Command::new("cargo")
        .args(&cargo_args)
        .current_dir("backend")
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{}", format!("Command '{}' completed.", command).green());
            Ok(())
        }
        Ok(s) => {
            anyhow::bail!(
                "Command '{}' failed with exit code: {}",
                command,
                s.code().unwrap_or(-1)
            );
        }
        Err(e) => {
            anyhow::bail!("Failed to execute command '{}': {}", command, e);
        }
    }
}
