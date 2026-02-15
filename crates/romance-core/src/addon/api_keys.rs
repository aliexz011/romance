use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct ApiKeysAddon;

impl Addon for ApiKeysAddon {
    fn name(&self) -> &str {
        "api-keys"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)?;
        super::check_auth_exists(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/api_keys.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_api_keys(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling API key authentication...".bold());

        // Delete files
        if super::remove_file_if_exists(&project_root.join("backend/src/api_keys.rs"))? {
            println!("  {} backend/src/api_keys.rs", "delete".red());
        }

        // Remove mod declaration from main.rs
        super::remove_mod_from_main(project_root, "api_keys")?;

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "API key authentication uninstalled successfully."
                .green()
                .bold()
        );

        Ok(())
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["auth"]
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
    super::add_mod_to_main(project_root, "api_keys")?;

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
