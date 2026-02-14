use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct ValidationAddon;

impl Addon for ValidationAddon {
    fn name(&self) -> &str {
        "validation"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root
            .join("backend/src/validation.rs")
            .exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_validation(project_root)
    }
}

fn install_validation(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing validation...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate validation middleware
    let content = engine.render("addon/validation/validate_middleware.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/validation.rs"),
        &content,
    )?;
    println!("  {} backend/src/validation.rs", "create".green());

    // Add mod declaration to main.rs
    super::add_mod_to_main(project_root, "validation")?;

    // Add validator dependencies to Cargo.toml
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[
            ("validator", r#"{ version = "0.19", features = ["derive"] }"#),
        ],
    )?;

    // Update romance.toml features
    super::update_feature_flag(project_root, "validation", true)?;

    println!();
    println!("{}", "Validation installed successfully!".green().bold());
    println!("  Entity fields now support validation rules: name:string[min=3,max=100]");

    Ok(())
}
