use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct EmailAddon;

impl Addon for EmailAddon {
    fn name(&self) -> &str {
        "email"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        if !project_root.join("romance.toml").exists() {
            anyhow::bail!("Not a Romance project (romance.toml not found)");
        }
        Ok(())
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
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod email;") {
        let new_content = main_content.replace("mod errors;", "mod email;\nmod errors;");
        std::fs::write(&main_path, new_content)?;
    }

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
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_HOST=smtp.example.com",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_PORT=587",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_USER=your_smtp_user",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        "SMTP_PASS=your_smtp_password",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        "FROM_EMAIL=noreply@example.com",
    )?;

    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_HOST=smtp.example.com",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_PORT=587",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_USER=",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        "SMTP_PASS=",
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        "FROM_EMAIL=noreply@example.com",
    )?;

    // Update romance.toml
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    if content.contains("[features]") {
        if !content.contains("email") {
            let new_content = content.replace("[features]", "[features]\nemail = true");
            std::fs::write(&config_path, new_content)?;
        }
    } else {
        let new_content = format!("{}\n[features]\nemail = true\n", content.trim_end());
        std::fs::write(&config_path, new_content)?;
    }

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
