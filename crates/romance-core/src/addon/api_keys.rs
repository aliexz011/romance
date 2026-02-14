use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct ApiKeysAddon;

impl Addon for ApiKeysAddon {
    fn name(&self) -> &str {
        "api-keys"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        if !project_root.join("romance.toml").exists() {
            anyhow::bail!("Not a Romance project (romance.toml not found)");
        }
        if !project_root.join("backend/src/auth.rs").exists() {
            anyhow::bail!("Auth must be generated first. Run: romance generate auth");
        }
        Ok(())
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/api_keys.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_api_keys(project_root)
    }
}

fn install_api_keys(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing API key authentication...".bold());

    let engine = TemplateEngine::new()?;
    let timestamp = crate::generator::migration::next_timestamp();

    let mut ctx = Context::new();
    ctx.insert("timestamp", &timestamp);

    // Generate api_keys module
    let content = engine.render("addon/api_keys/api_keys.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/api_keys.rs"), &content)?;
    println!("  {} backend/src/api_keys.rs", "create".green());

    // Generate migration
    let content = engine.render("addon/api_keys/migration.rs.tera", &ctx)?;
    let migration_module = format!("m{}_create_api_keys_table", timestamp);
    utils::write_file(
        &project_root.join(format!("backend/migration/src/{}.rs", migration_module)),
        &content,
    )?;
    println!(
        "  {} backend/migration/src/{}.rs",
        "create".green(),
        migration_module
    );

    // Register migration in lib.rs
    let lib_path = project_root.join("backend/migration/src/lib.rs");
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATION_MODS ===",
        &format!("mod {};", migration_module),
    )?;
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATIONS ===",
        &format!("            Box::new({}::Migration),", migration_module),
    )?;

    // Add mod api_keys to main.rs
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod api_keys;") {
        let new_content = main_content.replace("mod errors;", "mod api_keys;\nmod errors;");
        std::fs::write(&main_path, new_content)?;
    }

    // Add sha2 dependency (for hashing API keys)
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[("sha2", r#""0.10""#)],
    )?;

    println!();
    println!(
        "{}",
        "API key authentication installed successfully!".green().bold()
    );
    println!("  API keys are hashed with SHA-256 before storage.");
    println!("  Use X-API-Key header for machine-to-machine auth.");
    println!();
    println!("Next steps:");
    println!("  romance db migrate");

    Ok(())
}
