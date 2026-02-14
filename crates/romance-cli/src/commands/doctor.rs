use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

struct CheckResult {
    passed: bool,
    message: String,
}

impl CheckResult {
    fn pass(message: impl Into<String>) -> Self {
        Self {
            passed: true,
            message: message.into(),
        }
    }

    fn fail(message: impl Into<String>) -> Self {
        Self {
            passed: false,
            message: message.into(),
        }
    }
}

fn check_romance_toml() -> CheckResult {
    let path = Path::new("romance.toml");
    if !path.exists() {
        return CheckResult::fail("romance.toml not found");
    }
    match std::fs::read_to_string(path) {
        Ok(content) => match content.parse::<toml::Table>() {
            Ok(_) => CheckResult::pass("romance.toml found and valid"),
            Err(e) => CheckResult::fail(format!("romance.toml has invalid TOML: {}", e)),
        },
        Err(e) => CheckResult::fail(format!("romance.toml unreadable: {}", e)),
    }
}

fn check_backend_structure() -> CheckResult {
    let required = [
        "backend/src/main.rs",
        "backend/src/entities/mod.rs",
        "backend/src/handlers/mod.rs",
        "backend/src/routes/mod.rs",
    ];

    let missing: Vec<&str> = required
        .iter()
        .filter(|p| !Path::new(p).exists())
        .copied()
        .collect();

    if missing.is_empty() {
        CheckResult::pass("Backend structure OK")
    } else {
        CheckResult::fail(format!(
            "Backend structure incomplete, missing: {}",
            missing.join(", ")
        ))
    }
}

fn check_frontend_structure() -> CheckResult {
    let required = ["frontend/package.json", "frontend/src/App.tsx"];

    let missing: Vec<&str> = required
        .iter()
        .filter(|p| !Path::new(p).exists())
        .copied()
        .collect();

    if missing.is_empty() {
        CheckResult::pass("Frontend structure OK")
    } else {
        CheckResult::fail(format!(
            "Frontend structure incomplete, missing: {}",
            missing.join(", ")
        ))
    }
}

fn check_markers() -> CheckResult {
    let marker_files = [
        ("backend/src/entities/mod.rs", "// === ROMANCE:MODS ==="),
        ("backend/src/handlers/mod.rs", "// === ROMANCE:MODS ==="),
        ("backend/src/routes/mod.rs", "// === ROMANCE:MODS ==="),
    ];

    let mut broken = Vec::new();

    for (file, marker) in &marker_files {
        let path = Path::new(file);
        if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => {
                    if !content.contains(marker) {
                        broken.push(*file);
                    }
                }
                Err(_) => {
                    broken.push(*file);
                }
            }
        } else {
            // File missing is caught by backend structure check, skip here
            continue;
        }
    }

    if broken.is_empty() {
        CheckResult::pass("Markers intact (entities, handlers, routes)")
    } else {
        CheckResult::fail(format!(
            "Markers missing in: {}",
            broken.join(", ")
        ))
    }
}

fn check_cargo() -> CheckResult {
    match Command::new("cargo").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
                // Extract version number from "cargo X.Y.Z (hash date)"
                let version_str = version
                    .strip_prefix("cargo ")
                    .and_then(|s| s.split_whitespace().next())
                    .unwrap_or(&version);
                CheckResult::pass(format!("Cargo installed ({})", version_str))
            } else {
                CheckResult::fail("Cargo found but returned error")
            }
        }
        Err(_) => CheckResult::fail("Cargo not found (install Rust: https://rustup.rs)"),
    }
}

fn check_node() -> CheckResult {
    match Command::new("node").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
                CheckResult::pass(format!("Node.js installed ({})", version))
            } else {
                CheckResult::fail("Node.js found but returned error")
            }
        }
        Err(_) => CheckResult::fail("Node.js not found (install: https://nodejs.org)"),
    }
}

fn check_database_url() -> CheckResult {
    let env_path = Path::new("backend/.env");
    if !env_path.exists() {
        return CheckResult::fail("backend/.env not found (copy from .env.example)");
    }

    match std::fs::read_to_string(env_path) {
        Ok(content) => {
            let has_db_url = content.lines().any(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with('#')
                    && trimmed.starts_with("DATABASE_URL")
                    && trimmed.contains('=')
            });
            if has_db_url {
                CheckResult::pass("DATABASE_URL configured")
            } else {
                CheckResult::fail("DATABASE_URL not set in backend/.env")
            }
        }
        Err(e) => CheckResult::fail(format!("Cannot read backend/.env: {}", e)),
    }
}

fn check_manifest() -> CheckResult {
    let path = Path::new(".romance/manifest.json");
    if !path.exists() {
        return CheckResult::fail(".romance/manifest.json missing (run `romance update --init`)");
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => CheckResult::pass(".romance/manifest.json valid"),
            Err(e) => CheckResult::fail(format!(".romance/manifest.json invalid JSON: {}", e)),
        },
        Err(e) => CheckResult::fail(format!(".romance/manifest.json unreadable: {}", e)),
    }
}

pub fn run() -> Result<()> {
    println!("{}", "Romance Doctor".bold());

    let checks: Vec<CheckResult> = vec![
        check_romance_toml(),
        check_backend_structure(),
        check_frontend_structure(),
        check_markers(),
        check_cargo(),
        check_node(),
        check_database_url(),
        check_manifest(),
    ];

    let total = checks.len();
    let passed = checks.iter().filter(|c| c.passed).count();

    for check in &checks {
        if check.passed {
            println!("  {} {}", "\u{2713}".green(), check.message);
        } else {
            println!("  {} {}", "\u{2717}".red(), check.message);
        }
    }

    println!();
    let summary = format!("{}/{} checks passed", passed, total);
    if passed == total {
        println!("{}", summary.green().bold());
    } else {
        println!("{}", summary.yellow().bold());
    }

    Ok(())
}
