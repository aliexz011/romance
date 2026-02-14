use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct SoftDeleteAddon;

impl Addon for SoftDeleteAddon {
    fn name(&self) -> &str {
        "soft-delete"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        if !project_root.join("romance.toml").exists() {
            anyhow::bail!("Not a Romance project (romance.toml not found)");
        }
        Ok(())
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root
            .join("backend/src/soft_delete.rs")
            .exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_soft_delete(project_root)
    }
}

fn install_soft_delete(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing soft delete...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate soft_delete helper module
    let content = engine.render("addon/soft_delete/soft_delete.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/soft_delete.rs"),
        &content,
    )?;
    println!("  {} backend/src/soft_delete.rs", "create".green());

    // Add mod to main.rs
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod soft_delete;") {
        let new_content = main_content.replace("mod errors;", "mod errors;\nmod soft_delete;");
        std::fs::write(&main_path, new_content)?;
    }

    // Update romance.toml
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    if content.contains("[features]") {
        if !content.contains("soft_delete") {
            let new_content = content.replace("[features]", "[features]\nsoft_delete = true");
            std::fs::write(&config_path, new_content)?;
        }
    } else {
        let new_content =
            format!("{}\n[features]\nsoft_delete = true\n", content.trim_end());
        std::fs::write(&config_path, new_content)?;
    }

    println!();
    println!(
        "{}",
        "Soft delete installed successfully!".green().bold()
    );
    println!("  Future entities will use soft-delete by default.");
    println!("  Entities get: DELETE (soft), POST /:id/restore, DELETE /:id/permanent");

    Ok(())
}
