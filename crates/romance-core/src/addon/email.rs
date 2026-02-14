use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct EmailAddon;

impl Addon for EmailAddon {
    fn name(&self) -> &str {
        "email"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/email.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_email(project_root)
    }
}

fn install_email(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing email system...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate email service module
    let content = engine.render("addon/email/email.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/email.rs"), &content)?;
    println!("  {} backend/src/email.rs", "create".green());

    // Generate password reset handler
    let content = engine.render("addon/email/password_reset.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/handlers/password_reset.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/handlers/password_reset.rs",
        "create".green()
    );

    // Add mod email to main.rs
    super::add_mod_to_main(project_root, "email")?;

    // Register password_reset handler
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod password_reset;",
    )?;

    // Add dependencies
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[(
            "lettre",
            r#"{ version = "0.11", features = ["tokio1-native-tls"] }"#,
        )],
    )?;

    // Add env vars
    super::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_HOST=smtp.example.com",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_PORT=587",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_USER=your_smtp_user",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_PASS=your_smtp_password",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env"),
        "FROM_EMAIL=noreply@example.com",
    )?;

    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_HOST=smtp.example.com",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_PORT=587",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_USER=",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_PASS=",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "FROM_EMAIL=noreply@example.com",
    )?;

    // Update romance.toml
    super::update_feature_flag(project_root, "email", true)?;

    println!();
    println!(
        "{}",
        "Email system installed successfully!".green().bold()
    );
    println!("  Configure SMTP settings in backend/.env");
    println!("  Use EmailService::new() to create an instance.");
    println!("  Password reset handler available at /api/auth/password-reset");

    Ok(())
}
