use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

/// Run the test suite with a temporary database.
pub fn run_tests(project_root: &Path) -> Result<()> {
    println!("{}", "Running tests...".bold());

    let backend_dir = project_root.join("backend");
    if !backend_dir.exists() {
        anyhow::bail!("Backend directory not found");
    }

    // Generate test_helpers.rs if not present
    let test_helpers_path = backend_dir.join("src/test_helpers.rs");
    if !test_helpers_path.exists() {
        if let Ok(engine) = crate::template::TemplateEngine::new() {
            let ctx = tera::Context::new();
            if let Ok(content) = engine.render("addon/test/test_helpers.rs.tera", &ctx) {
                let _ = crate::utils::write_file(&test_helpers_path, &content);
                // Add mod test_helpers to main.rs
                let main_path = backend_dir.join("src/main.rs");
                if let Ok(main_content) = std::fs::read_to_string(&main_path) {
                    if !main_content.contains("mod test_helpers;") {
                        let new_content = main_content.replace(
                            "mod errors;",
                            "mod errors;\n#[cfg(test)]\nmod test_helpers;",
                        );
                        let _ = std::fs::write(&main_path, new_content);
                    }
                }
                println!("  {} backend/src/test_helpers.rs", "create".green());
            }
        }
    }

    // Generate a unique test database name
    let db_name = format!(
        "romance_test_{}",
        uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string()
    );

    // Try to read the base DATABASE_URL to derive the test URL
    let env_path = backend_dir.join(".env");
    let base_url = if env_path.exists() {
        let content = std::fs::read_to_string(&env_path)?;
        content
            .lines()
            .find(|l| l.starts_with("DATABASE_URL="))
            .and_then(|l| l.strip_prefix("DATABASE_URL="))
            .map(|s| s.to_string())
    } else {
        None
    };

    let test_db_url = if let Some(ref url) = base_url {
        // Replace the database name at the end of the URL
        if let Some(pos) = url.rfind('/') {
            format!("{}/{}", &url[..pos], db_name)
        } else {
            format!("postgres://localhost/{}", db_name)
        }
    } else {
        format!("postgres://localhost/{}", db_name)
    };

    println!("  Creating test database: {}", db_name);

    // Create test database
    let create_status = Command::new("createdb")
        .arg(&db_name)
        .status();

    let db_created = match create_status {
        Ok(s) if s.success() => true,
        _ => {
            println!(
                "{}",
                "  Warning: Could not create test database (createdb not available)".yellow()
            );
            println!("  Running tests with existing DATABASE_URL...");
            false
        }
    };

    // Run migrations on test database
    if db_created {
        println!("  Running migrations on test database...");
        let migration_status = Command::new("cargo")
            .args(["run", "--bin", "migration", "--", "up"])
            .current_dir(&backend_dir)
            .env("DATABASE_URL", &test_db_url)
            .status()?;

        if !migration_status.success() {
            // Clean up test database
            let _ = Command::new("dropdb").arg(&db_name).status();
            anyhow::bail!("Test migrations failed");
        }
    }

    // Run tests
    let env_var = if db_created {
        test_db_url.clone()
    } else if let Some(url) = base_url {
        url
    } else {
        "postgres://localhost/romance_test".to_string()
    };

    println!("  Running cargo test...");
    let test_status = Command::new("cargo")
        .args(["test"])
        .current_dir(&backend_dir)
        .env("DATABASE_URL", &env_var)
        .status()?;

    // Clean up test database
    if db_created {
        println!("  Dropping test database: {}", db_name);
        let _ = Command::new("dropdb").arg(&db_name).status();
    }

    if !test_status.success() {
        anyhow::bail!("Tests failed");
    }

    println!("{}", "All tests passed!".green().bold());
    Ok(())
}
