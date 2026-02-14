use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct ObservabilityAddon;

impl Addon for ObservabilityAddon {
    fn name(&self) -> &str {
        "observability"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root
            .join("backend/src/middleware/request_id.rs")
            .exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_observability(project_root)
    }
}

fn install_observability(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing observability...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate tracing setup
    let content = engine.render("addon/observability/tracing_setup.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/middleware/tracing_setup.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/middleware/tracing_setup.rs",
        "create".green()
    );

    // Generate request ID middleware
    let content = engine.render("addon/observability/request_id.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/middleware/request_id.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/middleware/request_id.rs",
        "create".green()
    );

    // Ensure middleware/mod.rs exists
    let middleware_mod_path = project_root.join("backend/src/middleware/mod.rs");
    if middleware_mod_path.exists() {
        let content = std::fs::read_to_string(&middleware_mod_path)?;
        let mut new_content = content.clone();
        if !new_content.contains("mod tracing_setup;") {
            new_content = format!("pub mod tracing_setup;\n{}", new_content);
        }
        if !new_content.contains("mod request_id;") {
            new_content = format!("pub mod request_id;\n{}", new_content);
        }
        std::fs::write(&middleware_mod_path, new_content)?;
    } else {
        utils::write_file(
            &middleware_mod_path,
            "pub mod request_id;\npub mod tracing_setup;\n",
        )?;
    }

    // Add mod middleware to main.rs if not present
    super::add_mod_to_main(project_root, "middleware")?;

    // Replace scaffold's tracing init with observability module's init_tracing()
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("init_tracing()") {
        // Find and replace the existing tracing_subscriber block
        if main_content.contains("tracing_subscriber::registry()") {
            // Replace the block from "tracing_subscriber::registry()" through ".init();"
            let new_main = if let Some(start) = main_content.find("    tracing_subscriber::registry()") {
                if let Some(init_pos) = main_content[start..].find(".init();") {
                    let end = start + init_pos + ".init();".len();
                    format!(
                        "{}    crate::middleware::tracing_setup::init_tracing();{}",
                        &main_content[..start],
                        &main_content[end..]
                    )
                } else {
                    main_content.clone()
                }
            } else {
                main_content.clone()
            };
            // Also remove unused tracing_subscriber import if present
            let new_main = new_main.replace(
                "use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};\n",
                "",
            );
            std::fs::write(&main_path, new_main)?;
            println!("  {} backend/src/main.rs (replaced tracing init)", "update".green());
        }
    }

    // Inject trace layer into routes
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:MIDDLEWARE ===",
        "        .layer(crate::middleware::request_id::request_id_layer())",
    )?;

    // Add dependencies
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[
            ("tower-http", r#"{ version = "0.6", features = ["cors", "trace", "request-id", "propagate-header"] }"#),
        ],
    )?;

    // Add RUST_LOG to .env
    super::append_env_var(
        &project_root.join("backend/.env"),
        "RUST_LOG=info",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "RUST_LOG=info",
    )?;

    println!();
    println!(
        "{}",
        "Observability installed successfully!".green().bold()
    );
    println!("  Structured logging with request ID propagation enabled.");
    println!("  Set RUST_LOG=debug for verbose logging.");

    Ok(())
}
