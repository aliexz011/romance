use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct SecurityAddon;

impl Addon for SecurityAddon {
    fn name(&self) -> &str {
        "security"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root
            .join("backend/src/middleware/security_headers.rs")
            .exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_security(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling security middleware...".bold());

        // Delete files
        if super::remove_file_if_exists(
            &project_root.join("backend/src/middleware/security_headers.rs"),
        )? {
            println!(
                "  {} backend/src/middleware/security_headers.rs",
                "delete".red()
            );
        }
        if super::remove_file_if_exists(
            &project_root.join("backend/src/middleware/rate_limit.rs"),
        )? {
            println!("  {} backend/src/middleware/rate_limit.rs", "delete".red());
        }

        // Remove middleware lines from routes/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/routes/mod.rs"),
            "security_headers",
        )?;
        super::remove_line_from_file(
            &project_root.join("backend/src/routes/mod.rs"),
            "rate_limit_middleware",
        )?;

        // Remove lines from middleware/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/middleware/mod.rs"),
            "security_headers",
        )?;
        super::remove_line_from_file(
            &project_root.join("backend/src/middleware/mod.rs"),
            "rate_limit",
        )?;

        // Remove [security] section from romance.toml
        super::remove_toml_section(project_root, "security")?;

        // NOTE: Don't remove middleware/mod.rs or mod middleware; from main.rs
        // as observability may also use the middleware module.

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "Security middleware uninstalled successfully.".green().bold()
        );

        Ok(())
    }
}

fn install_security(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing security middleware...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate security headers middleware
    let content = engine.render("addon/security/security_headers.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/middleware/security_headers.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/middleware/security_headers.rs",
        "create".green()
    );

    // Generate rate limiter
    let content = engine.render("addon/security/rate_limit.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/middleware/rate_limit.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/middleware/rate_limit.rs",
        "create".green()
    );

    // Generate middleware mod.rs
    let content = engine.render("addon/security/middleware_mod.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/middleware/mod.rs"),
        &content,
    )?;
    println!("  {} backend/src/middleware/mod.rs", "create".green());

    // Add mod middleware to main.rs
    super::add_mod_to_main(project_root, "middleware")?;

    // Inject middleware into routes/mod.rs
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:MIDDLEWARE ===",
        "        .layer(axum::middleware::from_fn(crate::middleware::security_headers::security_headers))",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:MIDDLEWARE ===",
        "        .layer(axum::middleware::from_fn(crate::middleware::rate_limit::rate_limit_middleware))",
    )?;

    // Add dependencies
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[
            ("tower", r#"{ version = "0.5", features = ["limit", "timeout"] }"#),
            ("governor", r#""0.7""#),
            ("tower-governor", r#""0.5""#),
            ("base64", r#""0.22""#),
        ],
    )?;

    // Add per-user rate limit env vars (anonymous IP-based + authenticated user-based)
    super::append_env_var(
        &project_root.join("backend/.env"),
        "RATE_LIMIT_ANON_RPM=30",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env"),
        "RATE_LIMIT_AUTH_RPM=120",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "RATE_LIMIT_ANON_RPM=30",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "RATE_LIMIT_AUTH_RPM=120",
    )?;

    // Update romance.toml
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    if !content.contains("[security]") {
        let new_content = format!(
            "{}\n[security]\nrate_limit_anon_rpm = 30\nrate_limit_auth_rpm = 120\ncors_origins = [\"http://localhost:5173\"]\n",
            content.trim_end()
        );
        std::fs::write(&config_path, new_content)?;
    }

    println!();
    println!(
        "{}",
        "Security middleware installed successfully!".green().bold()
    );
    println!("  Security headers, per-user rate limiting, and CORS configured.");
    println!("  Anonymous: {} RPM (IP-based), Authenticated: {} RPM (user-based).", 30, 120);
    println!("  Configure in romance.toml under [security].");

    Ok(())
}
