use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct SearchAddon;

impl Addon for SearchAddon {
    fn name(&self) -> &str {
        "search"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
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
    super::add_mod_to_main(project_root, "search")?;

    // Register search handler
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod search;",
    )?;

    // Update romance.toml
    super::update_feature_flag(project_root, "search", true)?;

    println!();
    println!(
        "{}",
        "Full-text search installed successfully!".green().bold()
    );
    println!("  Use [searchable] annotation on fields: title:string[searchable]");
    println!("  Search endpoint: GET /api/{{entities}}/search?q=term");

    Ok(())
}
