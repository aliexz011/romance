use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

#[allow(dead_code)]
struct CheckResult {
    name: String,
    passed: bool,
    message: String,
}

impl CheckResult {
    fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: true,
            message: message.into(),
        }
    }

    fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            passed: false,
            message: message.into(),
        }
    }
}

fn check_romance_toml() -> CheckResult {
    let path = Path::new("romance.toml");
    if !path.exists() {
        return CheckResult::fail("Config", "romance.toml not found");
    }
    match std::fs::read_to_string(path) {
        Ok(content) => match content.parse::<toml::Table>() {
            Ok(_) => CheckResult::pass("Config", "romance.toml found and valid"),
            Err(e) => CheckResult::fail("Config", format!("romance.toml has invalid TOML: {}", e)),
        },
        Err(e) => CheckResult::fail("Config", format!("romance.toml unreadable: {}", e)),
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
        CheckResult::pass("Backend", "Backend structure OK")
    } else {
        CheckResult::fail(
            "Backend",
            format!("Missing: {}", missing.join(", ")),
        )
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
        CheckResult::pass("Frontend", "Frontend structure OK")
    } else {
        CheckResult::fail(
            "Frontend",
            format!("Missing: {}", missing.join(", ")),
        )
    }
}

fn check_frontend_deps() -> CheckResult {
    if Path::new("frontend/node_modules").exists() {
        CheckResult::pass("Frontend Deps", "node_modules installed")
    } else {
        CheckResult::fail(
            "Frontend Deps",
            "node_modules not found (run: cd frontend && npm install)",
        )
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
        }
    }

    if broken.is_empty() {
        CheckResult::pass("Markers", "Code generation markers intact")
    } else {
        CheckResult::fail(
            "Markers",
            format!("Markers missing in: {}", broken.join(", ")),
        )
    }
}

fn check_tool(name: &str, args: &[&str], label: &str, install_hint: &str) -> CheckResult {
    match Command::new(name).args(args).output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let short = version
                    .split_whitespace()
                    .nth(1)
                    .or_else(|| version.split_whitespace().next())
                    .unwrap_or(&version);
                CheckResult::pass(label, format!("{} ({})", label, short))
            } else {
                CheckResult::fail(label, format!("{} found but returned error", label))
            }
        }
        Err(_) => CheckResult::fail(
            label,
            format!("{} not found (install: {})", label, install_hint),
        ),
    }
}

fn check_database_url() -> CheckResult {
    let env_path = Path::new("backend/.env");
    if !env_path.exists() {
        return CheckResult::fail("Database", "backend/.env not found (copy from .env.example)");
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
                CheckResult::pass("Database", "DATABASE_URL configured")
            } else {
                CheckResult::fail("Database", "DATABASE_URL not set in backend/.env")
            }
        }
        Err(e) => CheckResult::fail("Database", format!("Cannot read backend/.env: {}", e)),
    }
}

fn check_jwt_secret() -> CheckResult {
    let env_path = Path::new("backend/.env");
    if !env_path.exists() {
        return CheckResult::pass("JWT", "No .env file (auth not configured)");
    }

    match std::fs::read_to_string(env_path) {
        Ok(content) => {
            let jwt_line = content
                .lines()
                .find(|l| l.trim().starts_with("JWT_SECRET") && l.contains('='));
            match jwt_line {
                Some(line) => {
                    let value = line.split('=').nth(1).unwrap_or("").trim();
                    if value.is_empty() || value == "changeme" || value.contains("change-in-production") {
                        CheckResult::fail(
                            "JWT",
                            "JWT_SECRET is a placeholder — set a secure random value",
                        )
                    } else if value.len() < 32 {
                        CheckResult::fail("JWT", "JWT_SECRET is too short (use at least 32 chars)")
                    } else {
                        CheckResult::pass("JWT", "JWT_SECRET configured")
                    }
                }
                None => CheckResult::pass("JWT", "No JWT_SECRET (auth not configured)"),
            }
        }
        Err(_) => CheckResult::pass("JWT", "Could not read .env"),
    }
}

fn check_manifest() -> CheckResult {
    let path = Path::new(".romance/manifest.json");
    if !path.exists() {
        return CheckResult::fail(
            "Manifest",
            ".romance/manifest.json missing (run: romance update --init)",
        );
    }

    match std::fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(_) => CheckResult::pass("Manifest", "Manifest valid"),
            Err(e) => CheckResult::fail(
                "Manifest",
                format!("Invalid JSON: {}", e),
            ),
        },
        Err(e) => CheckResult::fail("Manifest", format!("Unreadable: {}", e)),
    }
}

fn check_git() -> CheckResult {
    match Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
    {
        Ok(output) if output.status.success() => {
            CheckResult::pass("Git", "Git repository initialized")
        }
        _ => CheckResult::fail("Git", "Not a git repository (run: git init)"),
    }
}

fn check_external_services() -> Vec<CheckResult> {
    let mut results = Vec::new();
    let env_path = Path::new("backend/.env");

    let env_content = if env_path.exists() {
        std::fs::read_to_string(env_path).unwrap_or_default()
    } else {
        return results;
    };

    let has_env_with_value = |key: &str| -> bool {
        env_content.lines().any(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return false;
            }
            if let Some((k, v)) = trimmed.split_once('=') {
                k.trim() == key && !v.trim().is_empty() && !v.contains("your-") && !v.contains("your_")
            } else {
                false
            }
        })
    };

    let has_env = |key: &str| -> bool {
        env_content.lines().any(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return false;
            }
            if let Some(k) = trimmed.split('=').next() {
                k.trim() == key
            } else {
                false
            }
        })
    };

    // Check Redis if cache addon is installed
    if Path::new("backend/src/cache.rs").exists() {
        if has_env_with_value("REDIS_URL") {
            results.push(CheckResult::pass("Redis", "REDIS_URL configured"));
        } else {
            results.push(CheckResult::fail(
                "Redis",
                "REDIS_URL not configured (required by cache addon)",
            ));
        }
    }

    // Check SMTP if email addon is installed
    if Path::new("backend/src/email.rs").exists() {
        if has_env_with_value("SMTP_HOST") && has_env_with_value("SMTP_USER") {
            results.push(CheckResult::pass("SMTP", "SMTP credentials configured"));
        } else {
            results.push(CheckResult::fail(
                "SMTP",
                "SMTP_HOST/SMTP_USER not configured (required by email addon)",
            ));
        }
    }

    // Check OAuth if oauth addon is installed
    if Path::new("backend/src/oauth.rs").exists() {
        let has_any_client_id = env_content.lines().any(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#')
                && trimmed.contains("_CLIENT_ID=")
                && !trimmed.ends_with("your-client-id")
        });
        if has_any_client_id {
            results.push(CheckResult::pass("OAuth", "OAuth client credentials configured"));
        } else {
            results.push(CheckResult::fail(
                "OAuth",
                "OAuth CLIENT_ID not configured (set *_CLIENT_ID in .env)",
            ));
        }
    }

    // Check storage if storage addon is installed
    if Path::new("backend/src/storage.rs").exists() {
        if has_env("UPLOAD_DIR") {
            results.push(CheckResult::pass("Storage", "Upload directory configured"));
        } else {
            results.push(CheckResult::fail(
                "Storage",
                "UPLOAD_DIR not configured (required by storage addon)",
            ));
        }
    }

    results
}

fn check_installed_addons() -> Vec<CheckResult> {
    let addons: Vec<(&str, &str)> = vec![
        ("validation", "backend/src/validation.rs"),
        ("soft-delete", "backend/src/soft_delete.rs"),
        ("security", "backend/src/middleware/security_headers.rs"),
        ("observability", "backend/src/middleware/request_id.rs"),
        ("storage", "backend/src/storage.rs"),
        ("search", "backend/src/search.rs"),
        ("cache", "backend/src/cache.rs"),
        ("email", "backend/src/email.rs"),
        ("tasks", "backend/src/tasks.rs"),
        ("websocket", "backend/src/ws.rs"),
        ("i18n", "backend/src/i18n.rs"),
        ("dashboard", "frontend/src/features/dev/DevDashboard.tsx"),
        ("audit-log", "backend/src/audit.rs"),
        ("oauth", "backend/src/oauth.rs"),
        ("api-keys", "backend/src/api_keys.rs"),
    ];

    let installed: Vec<&str> = addons
        .iter()
        .filter(|(_, path)| Path::new(path).exists())
        .map(|(name, _)| *name)
        .collect();

    if installed.is_empty() {
        vec![CheckResult::pass("Addons", "No addons installed")]
    } else {
        vec![CheckResult::pass(
            "Addons",
            format!("{} installed: {}", installed.len(), installed.join(", ")),
        )]
    }
}

pub fn run() -> Result<()> {
    println!("{}", "Romance Doctor".bold());
    println!();

    // === Project Structure ===
    println!("{}", "Project Structure".bold().underline());
    let structure_checks = vec![
        check_romance_toml(),
        check_backend_structure(),
        check_frontend_structure(),
        check_frontend_deps(),
        check_markers(),
        check_manifest(),
        check_git(),
    ];
    print_checks(&structure_checks);

    // === Tools ===
    println!();
    println!("{}", "Development Tools".bold().underline());
    let tool_checks = vec![
        check_tool("cargo", &["--version"], "Cargo", "https://rustup.rs"),
        check_tool("node", &["--version"], "Node.js", "https://nodejs.org"),
        check_tool("npm", &["--version"], "npm", "https://nodejs.org"),
        check_tool("psql", &["--version"], "PostgreSQL", "https://postgresql.org"),
    ];
    print_checks(&tool_checks);

    // === Configuration ===
    println!();
    println!("{}", "Configuration".bold().underline());
    let config_checks = vec![check_database_url(), check_jwt_secret()];
    print_checks(&config_checks);

    // === External Services ===
    let service_checks = check_external_services();
    if !service_checks.is_empty() {
        println!();
        println!("{}", "External Services".bold().underline());
        print_checks(&service_checks);
    }

    // === Addons ===
    println!();
    println!("{}", "Addons".bold().underline());
    let addon_checks = check_installed_addons();
    print_checks(&addon_checks);

    // === Summary ===
    let all_checks: Vec<&CheckResult> = structure_checks
        .iter()
        .chain(tool_checks.iter())
        .chain(config_checks.iter())
        .chain(service_checks.iter())
        .chain(addon_checks.iter())
        .collect();

    let total = all_checks.len();
    let passed = all_checks.iter().filter(|c| c.passed).count();
    let failed = total - passed;

    println!();
    let summary = format!("{}/{} checks passed", passed, total);
    if failed == 0 {
        println!("{}", summary.green().bold());
    } else {
        println!("{}", summary.yellow().bold());
        println!(
            "{}",
            format!("{} issue(s) found — see above for details", failed).yellow()
        );
    }

    Ok(())
}

fn print_checks(checks: &[CheckResult]) {
    for check in checks {
        if check.passed {
            println!("  {} {}", "\u{2713}".green(), check.message);
        } else {
            println!("  {} {}", "\u{2717}".red(), check.message);
        }
    }
}
