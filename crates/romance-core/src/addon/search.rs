use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct SearchAddon;

impl Addon for SearchAddon {
    fn name(&self) -> &str {
        "search"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        if !project_root.join("romance.toml").exists() {
            anyhow::bail!("Not a Romance project (romance.toml not found)");
        }
        Ok(())
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/search.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_search(project_root)
    }
}

fn install_search(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing full-text search...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate search module
    let content = engine.render("addon/search/search.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/search.rs"), &content)?;
    println!("  {} backend/src/search.rs", "create".green());

    // Generate search handler template
    let content = engine.render("addon/search/search_handler.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/handlers/search.rs"),
        &content,
    )?;
    println!("  {} backend/src/handlers/search.rs", "create".green());

    // Generate frontend search component
    let content = engine.render("addon/search/SearchBar.tsx.tera", &ctx)?;
    utils::write_file(
        &project_root.join("frontend/src/components/SearchBar.tsx"),
        &content,
    )?;
    println!(
        "  {} frontend/src/components/SearchBar.tsx",
        "create".green()
    );

    // Add mod search to main.rs
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod search;") {
        let new_content = main_content.replace("mod errors;", "mod errors;\nmod search;");
        std::fs::write(&main_path, new_content)?;
    }

    // Register search handler
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod search;",
    )?;

    // Update romance.toml
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    if content.contains("[features]") {
        if !content.contains("search") {
            let new_content = content.replace("[features]", "[features]\nsearch = true");
            std::fs::write(&config_path, new_content)?;
        }
    } else {
        let new_content = format!("{}\n[features]\nsearch = true\n", content.trim_end());
        std::fs::write(&config_path, new_content)?;
    }

    println!();
    println!(
        "{}",
        "Full-text search installed successfully!".green().bold()
    );
    println!("  Use [searchable] annotation on fields: title:string[searchable]");
    println!("  Search endpoint: GET /api/{{entities}}/search?q=term");

    Ok(())
}
