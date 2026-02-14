use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct SoftDeleteAddon;

impl Addon for SoftDeleteAddon {
    fn name(&self) -> &str {
        "soft-delete"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
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
    super::add_mod_to_main(project_root, "soft_delete")?;

    // Update romance.toml
    super::update_feature_flag(project_root, "soft_delete", true)?;

    println!();
    println!(
        "{}",
        "Soft delete installed successfully!".green().bold()
    );
    println!("  Future entities will use soft-delete by default.");
    println!("  Entities get: DELETE (soft), POST /:id/restore, DELETE /:id/permanent");

    Ok(())
}
