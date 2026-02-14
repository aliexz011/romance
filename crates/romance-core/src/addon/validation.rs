use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct ValidationAddon;

impl Addon for ValidationAddon {
    fn name(&self) -> &str {
        "validation"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        if !project_root.join("romance.toml").exists() {
            anyhow::bail!("Not a Romance project (romance.toml not found)");
        }
        Ok(())
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
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod validation;") {
        let new_content = main_content.replace("mod errors;", "mod errors;\nmod validation;");
        std::fs::write(&main_path, new_content)?;
    }

    // Add validator dependencies to Cargo.toml
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[
            ("validator", r#"{ version = "0.19", features = ["derive"] }"#),
        ],
    )?;

    // Update romance.toml features
    update_features_config(project_root, "validation = true")?;

    println!();
    println!("{}", "Validation installed successfully!".green().bold());
    println!("  Entity fields now support validation rules: name:string[min=3,max=100]");

    Ok(())
}

fn update_features_config(project_root: &Path, line: &str) -> Result<()> {
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;

    if content.contains("[features]") {
        if !content.contains(line) {
            let new_content = content.replace("[features]", &format!("[features]\n{}", line));
            std::fs::write(&config_path, new_content)?;
        }
    } else {
        let new_content = format!("{}\n[features]\n{}\n", content.trim_end(), line);
        std::fs::write(&config_path, new_content)?;
    }

    Ok(())
}
