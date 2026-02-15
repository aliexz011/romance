use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct AuditLogAddon;

impl Addon for AuditLogAddon {
    fn name(&self) -> &str {
        "audit-log"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)?;
        super::check_auth_exists(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/audit.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_audit_log(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling audit log...".bold());

        // Delete files
        if super::remove_file_if_exists(&project_root.join("backend/src/audit.rs"))? {
            println!("  {} backend/src/audit.rs", "delete".red());
        }
        if super::remove_file_if_exists(
            &project_root.join("backend/src/entities/audit_entry.rs"),
        )? {
            println!(
                "  {} backend/src/entities/audit_entry.rs",
                "delete".red()
            );
        }
        if super::remove_file_if_exists(
            &project_root.join("backend/src/handlers/audit_log.rs"),
        )? {
            println!("  {} backend/src/handlers/audit_log.rs", "delete".red());
        }
        if super::remove_file_if_exists(
            &project_root.join("frontend/src/features/admin/AuditLog.tsx"),
        )? {
            println!(
                "  {} frontend/src/features/admin/AuditLog.tsx",
                "delete".red()
            );
        }

        // Remove mod declaration from main.rs
        super::remove_mod_from_main(project_root, "audit")?;

        // Remove from entities/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/entities/mod.rs"),
            "pub mod audit_entry;",
        )?;

        // Remove from handlers/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/handlers/mod.rs"),
            "pub mod audit_log;",
        )?;

        // Remove feature flag
        super::remove_feature_flag(project_root, "audit_log")?;

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "Audit log uninstalled successfully.".green().bold()
        );

        Ok(())
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["auth"]
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
    super::add_mod_to_main(project_root, "audit")?;

    // Update romance.toml
    super::update_feature_flag(project_root, "audit_log", true)?;

    println!();
    println!(
        "{}",
        "Audit log installed successfully!".green().bold()
    );
    println!("  All create/update/delete operations will be logged.");
    println!("  View at /admin/audit-log");

    Ok(())
}
