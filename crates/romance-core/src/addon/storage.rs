use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct StorageAddon;

impl Addon for StorageAddon {
    fn name(&self) -> &str {
        "storage"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/storage.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_storage(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling file storage...".bold());

        // Delete files
        if super::remove_file_if_exists(&project_root.join("backend/src/storage.rs"))? {
            println!("  {} backend/src/storage.rs", "delete".red());
        }
        if super::remove_file_if_exists(&project_root.join("backend/src/handlers/upload.rs"))? {
            println!("  {} backend/src/handlers/upload.rs", "delete".red());
        }
        if super::remove_file_if_exists(&project_root.join("backend/src/routes/upload.rs"))? {
            println!("  {} backend/src/routes/upload.rs", "delete".red());
        }
        if super::remove_file_if_exists(
            &project_root.join("frontend/src/components/FileUpload.tsx"),
        )? {
            println!(
                "  {} frontend/src/components/FileUpload.tsx",
                "delete".red()
            );
        }

        // Remove mod declaration from main.rs
        super::remove_mod_from_main(project_root, "storage")?;

        // Remove from handlers/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/handlers/mod.rs"),
            "pub mod upload;",
        )?;

        // Remove from routes/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/routes/mod.rs"),
            "pub mod upload;",
        )?;
        super::remove_line_from_file(
            &project_root.join("backend/src/routes/mod.rs"),
            ".merge(upload::router())",
        )?;

        // Remove [storage] section from romance.toml
        super::remove_toml_section(project_root, "storage")?;

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "File storage uninstalled successfully.".green().bold()
        );

        Ok(())
    }
}

fn install_storage(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing file storage...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate storage backend trait + impls
    let content = engine.render("addon/storage/storage.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/storage.rs"), &content)?;
    println!("  {} backend/src/storage.rs", "create".green());

    // Generate upload handler
    let content = engine.render("addon/storage/upload_handler.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/handlers/upload.rs"),
        &content,
    )?;
    println!("  {} backend/src/handlers/upload.rs", "create".green());

    // Generate upload routes
    let content = engine.render("addon/storage/upload_routes.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/routes/upload.rs"),
        &content,
    )?;
    println!("  {} backend/src/routes/upload.rs", "create".green());

    // Generate frontend upload component
    let content = engine.render("addon/storage/FileUpload.tsx.tera", &ctx)?;
    utils::write_file(
        &project_root.join("frontend/src/components/FileUpload.tsx"),
        &content,
    )?;
    println!(
        "  {} frontend/src/components/FileUpload.tsx",
        "create".green()
    );

    // Add mod storage to main.rs
    super::add_mod_to_main(project_root, "storage")?;

    // Register upload routes
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod upload;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        mods_marker,
        "pub mod upload;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:ROUTES ===",
        "        .merge(upload::router())",
    )?;

    // Add dependencies
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[
            ("axum", r#"{ version = "0.8", features = ["json", "multipart"] }"#),
            ("mime", r#""0.3""#),
        ],
    )?;

    // Add env vars for storage configuration
    super::append_env_var(
        &project_root.join("backend/.env"),
        "UPLOAD_DIR=./uploads",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env"),
        "UPLOAD_URL=/uploads",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env"),
        "MAX_FILE_SIZE=10MB",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "UPLOAD_DIR=./uploads",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "UPLOAD_URL=/uploads",
    )?;
    super::append_env_var(
        &project_root.join("backend/.env.example"),
        "MAX_FILE_SIZE=10MB",
    )?;

    // Create uploads directory
    std::fs::create_dir_all(project_root.join("backend/uploads"))?;
    println!("  {} backend/uploads/", "create".green());

    // Update romance.toml
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    if !content.contains("[storage]") {
        let new_content = format!(
            "{}\n[storage]\nbackend = \"local\"\nupload_dir = \"./uploads\"\nmax_file_size = \"10MB\"\n",
            content.trim_end()
        );
        std::fs::write(&config_path, new_content)?;
    }

    println!();
    println!("{}", "File storage installed successfully!".green().bold());
    println!("  Use `avatar:image` or `document:file` field types in entity generation.");
    println!("  Configure storage in romance.toml under [storage].");

    Ok(())
}
