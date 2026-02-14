use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct AuditLogAddon;

impl Addon for AuditLogAddon {
    fn name(&self) -> &str {
        "audit-log"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        if !project_root.join("romance.toml").exists() {
            anyhow::bail!("Not a Romance project (romance.toml not found)");
        }
        if !project_root.join("backend/src/auth.rs").exists() {
            anyhow::bail!("Auth must be generated first. Run: romance generate auth");
        }
        Ok(())
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/audit.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_audit_log(project_root)
    }
}

fn install_audit_log(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing audit log...".bold());

    let engine = TemplateEngine::new()?;
    let timestamp = crate::generator::migration::next_timestamp();

    let mut ctx = Context::new();
    ctx.insert("timestamp", &timestamp);

    // Generate audit module
    let content = engine.render("addon/audit_log/audit.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/audit.rs"), &content)?;
    println!("  {} backend/src/audit.rs", "create".green());

    // Generate audit_entry entity model
    let content = engine.render("addon/audit_log/model.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/entities/audit_entry.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/entities/audit_entry.rs",
        "create".green()
    );

    // Generate migration
    let content = engine.render("addon/audit_log/migration.rs.tera", &ctx)?;
    let migration_module = format!("m{}_create_audit_entries_table", timestamp);
    utils::write_file(
        &project_root.join(format!("backend/migration/src/{}.rs", migration_module)),
        &content,
    )?;
    println!(
        "  {} backend/migration/src/{}.rs",
        "create".green(),
        migration_module
    );

    // Generate audit log handler for admin
    let content = engine.render("addon/audit_log/handlers.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/handlers/audit_log.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/handlers/audit_log.rs",
        "create".green()
    );

    // Generate frontend audit log viewer
    let content = engine.render("addon/audit_log/AuditLog.tsx.tera", &ctx)?;
    utils::write_file(
        &project_root.join("frontend/src/features/admin/AuditLog.tsx"),
        &content,
    )?;
    println!(
        "  {} frontend/src/features/admin/AuditLog.tsx",
        "create".green()
    );

    // Register modules
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/entities/mod.rs"),
        mods_marker,
        "pub mod audit_entry;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod audit_log;",
    )?;

    // Register migration
    let lib_path = project_root.join("backend/migration/src/lib.rs");
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATION_MODS ===",
        &format!("mod {};", migration_module),
    )?;
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATIONS ===",
        &format!("            Box::new({}::Migration),", migration_module),
    )?;

    // Add mod audit to main.rs
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod audit;") {
        let new_content = main_content.replace("mod errors;", "mod audit;\nmod errors;");
        std::fs::write(&main_path, new_content)?;
    }

    // Update romance.toml
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    if content.contains("[features]") {
        if !content.contains("audit_log") {
            let new_content = content.replace("[features]", "[features]\naudit_log = true");
            std::fs::write(&config_path, new_content)?;
        }
    } else {
        let new_content = format!("{}\n[features]\naudit_log = true\n", content.trim_end());
        std::fs::write(&config_path, new_content)?;
    }

    println!();
    println!(
        "{}",
        "Audit log installed successfully!".green().bold()
    );
    println!("  All create/update/delete operations will be logged.");
    println!("  View at /admin/audit-log");

    Ok(())
}
