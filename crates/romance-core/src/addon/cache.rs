use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct CacheAddon;

impl Addon for CacheAddon {
    fn name(&self) -> &str {
        "cache"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/cache.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_cache(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling caching layer...".bold());

        // Delete files
        if super::remove_file_if_exists(&project_root.join("backend/src/cache.rs"))? {
            println!("  {} backend/src/cache.rs", "delete".red());
        }

        // Remove mod declaration from main.rs
        super::remove_mod_from_main(project_root, "cache")?;

        // Remove feature flag
        super::remove_feature_flag(project_root, "cache")?;

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "Caching layer uninstalled successfully.".green().bold()
        );

        Ok(())
    }
}

fn install_cache(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing caching layer...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate cache service module
    let content = engine.render("addon/cache/cache.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/cache.rs"), &content)?;
    println!("  {} backend/src/cache.rs", "create".green());

    // Add mod cache to main.rs
    super::add_mod_to_main(project_root, "cache")?;

    // Add dependencies
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[(
            "redis",
            r#"{ version = "0.27", features = ["tokio-comp", "connection-manager"] }"#,
        )],
    )?;

    // Add env vars
    super::append_env_var(
        &project_root.join("backend/.env"),
        "REDIS_URL=redis://127.0.0.1:6379",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "REDIS_URL=redis://127.0.0.1:6379",
    )?;

    // Update romance.toml
    super::update_feature_flag(project_root, "cache", true)?;

    println!();
    println!(
        "{}",
        "Caching layer installed successfully!".green().bold()
    );
    println!("  Configure Redis connection in backend/.env");
    println!("  Use CacheService::new()? to create an instance.");
    println!("  Example: cache.set(\"key\", &value, 300).await?");

    Ok(())
}
