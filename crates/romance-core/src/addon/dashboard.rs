use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct DashboardAddon;

impl Addon for DashboardAddon {
    fn name(&self) -> &str {
        "dashboard"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root
            .join("frontend/src/features/dev/DevDashboard.tsx")
            .exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_dashboard(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling dev dashboard...".bold());

        // Delete files
        if super::remove_file_if_exists(
            &project_root.join("backend/src/handlers/dev_dashboard.rs"),
        )? {
            println!(
                "  {} backend/src/handlers/dev_dashboard.rs",
                "delete".red()
            );
        }
        if super::remove_file_if_exists(
            &project_root.join("backend/src/routes/dev_dashboard.rs"),
        )? {
            println!(
                "  {} backend/src/routes/dev_dashboard.rs",
                "delete".red()
            );
        }
        if super::remove_file_if_exists(
            &project_root.join("frontend/src/features/dev/DevDashboard.tsx"),
        )? {
            println!(
                "  {} frontend/src/features/dev/DevDashboard.tsx",
                "delete".red()
            );
        }

        // Remove from handlers/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/handlers/mod.rs"),
            "pub mod dev_dashboard;",
        )?;

        // Remove from routes/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/routes/mod.rs"),
            "pub mod dev_dashboard;",
        )?;
        super::remove_line_from_file(
            &project_root.join("backend/src/routes/mod.rs"),
            ".merge(dev_dashboard::router())",
        )?;

        // Remove from frontend App.tsx (both import and Route)
        super::remove_line_from_file(
            &project_root.join("frontend/src/App.tsx"),
            "DevDashboard",
        )?;

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "Dev dashboard uninstalled successfully.".green().bold()
        );

        Ok(())
    }
}

fn install_dashboard(project_root: &Path) -> Result<()> {
    use crate::relation;
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use heck::{ToLowerCamelCase, ToPascalCase};
    use tera::Context;

    println!("{}", "Installing dev dashboard...".bold());

    let engine = TemplateEngine::new()?;

    // Discover entities
    let entity_names = relation::discover_entities(project_root)?;
    let entities_dir = project_root.join("backend/src/entities");
    let entities: Vec<serde_json::Value> = entity_names
        .iter()
        .filter(|n| {
            if *n == "user" || *n == "audit_entry" {
                return false;
            }
            let model_path = entities_dir.join(format!("{}.rs", n));
            if model_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&model_path) {
                    return content.contains("ROMANCE:CUSTOM");
                }
            }
            false
        })
        .map(|name| {
            serde_json::json!({
                "name": name.to_pascal_case(),
                "name_snake": name,
                "name_camel": name.to_lower_camel_case(),
            })
        })
        .collect();

    let has_auth = project_root.join("backend/src/auth.rs").exists();
    let has_audit = project_root.join("backend/src/audit.rs").exists();

    let mut ctx = Context::new();
    ctx.insert("entities", &entities);
    ctx.insert("has_auth", &has_auth);
    ctx.insert("has_audit", &has_audit);

    // Generate dev dashboard handler
    let content = engine.render("addon/dashboard/dev_handlers.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/handlers/dev_dashboard.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/handlers/dev_dashboard.rs",
        "create".green()
    );

    // Generate dev dashboard routes
    let content = engine.render("addon/dashboard/dev_routes.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/routes/dev_dashboard.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/routes/dev_dashboard.rs",
        "create".green()
    );

    // Generate frontend dashboard
    let content = engine.render("addon/dashboard/DevDashboard.tsx.tera", &ctx)?;
    utils::write_file(
        &project_root.join("frontend/src/features/dev/DevDashboard.tsx"),
        &content,
    )?;
    println!(
        "  {} frontend/src/features/dev/DevDashboard.tsx",
        "create".green()
    );

    // Register routes
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod dev_dashboard;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        mods_marker,
        "pub mod dev_dashboard;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:ROUTES ===",
        "        .merge(dev_dashboard::router())",
    )?;

    // Register frontend route
    utils::insert_at_marker(
        &project_root.join("frontend/src/App.tsx"),
        "// === ROMANCE:IMPORTS ===",
        "import DevDashboard from '@/features/dev/DevDashboard';",
    )?;
    utils::insert_at_marker(
        &project_root.join("frontend/src/App.tsx"),
        "// === ROMANCE:APP_ROUTES ===",
        "          <Route path=\"/dev\" element={<DevDashboard />} />",
    )?;

    println!();
    println!(
        "{}",
        "Dev dashboard installed successfully!".green().bold()
    );
    println!("  Visit /dev to see the developer dashboard.");

    Ok(())
}
