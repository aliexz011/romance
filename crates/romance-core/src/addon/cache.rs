use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct CacheAddon;

impl Addon for CacheAddon {
    fn name(&self) -> &str {
        "cache"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        if !project_root.join("romance.toml").exists() {
            anyhow::bail!("Not a Romance project (romance.toml not found)");
        }
        Ok(())
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/cache.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_cache(project_root)
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
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod cache;") {
        let new_content = main_content.replace("mod errors;", "mod cache;\nmod errors;");
        std::fs::write(&main_path, new_content)?;
    }

    // Add dependencies
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[(
            "redis",
            r#"{ version = "0.27", features = ["tokio-comp", "connection-manager"] }"#,
        )],
    )?;

    // Add env vars
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        "REDIS_URL=redis://127.0.0.1:6379",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        "REDIS_URL=redis://127.0.0.1:6379",
    )?;

    // Update romance.toml
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    if content.contains("[features]") {
        if !content.contains("cache") {
            let new_content = content.replace("[features]", "[features]\ncache = true");
            std::fs::write(&config_path, new_content)?;
        }
    } else {
        let new_content = format!("{}\n[features]\ncache = true\n", content.trim_end());
        std::fs::write(&config_path, new_content)?;
    }

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
