use crate::generator::context::markers;
use crate::relation;
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::fs;
use std::path::Path;
use tera::Context;

pub fn generate() -> Result<()> {
    let project_dir = Path::new(".");

    if !project_dir.join("romance.toml").exists() {
        anyhow::bail!("Not a Romance project (romance.toml not found)");
    }

    // Check auth is generated
    if !project_dir.join("backend/src/auth.rs").exists() {
        anyhow::bail!(
            "Auth must be generated first. Run: romance generate auth"
        );
    }

    // Check idempotency
    if project_dir.join("frontend/src/admin/AdminLayout.tsx").exists() {
        anyhow::bail!("Admin already generated (frontend/src/admin/AdminLayout.tsx exists)");
    }

    println!("{}", "Generating admin panel...".bold());

    let config = crate::config::RomanceConfig::load(project_dir)?;
    let engine = TemplateEngine::new()?;

    // Discover entities
    let entity_names = relation::discover_entities(project_dir)?;

    let mut ctx = Context::new();
    ctx.insert("project_name", &config.project.name);
    ctx.insert("project_name_snake", &config.project.name.to_snake_case());

    let entities_dir = project_dir.join("backend/src/entities");
    let entities: Vec<serde_json::Value> = entity_names
        .iter()
        .filter(|n| {
            // user is handled by auth
            if *n == "user" {
                return false;
            }
            // Skip junction tables (no ROMANCE:CUSTOM marker)
            let model_path = entities_dir.join(format!("{}.rs", n));
            if model_path.exists() {
                if let Ok(content) = fs::read_to_string(&model_path) {
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
    ctx.insert("entities", &entities);

    // Admin layout + dashboard
    let admin_dir = project_dir.join("frontend/src/admin");

    let content = engine.render("admin/frontend/AdminLayout.tsx.tera", &ctx)?;
    utils::write_file(&admin_dir.join("AdminLayout.tsx"), &content)?;
    println!("  {} frontend/src/admin/AdminLayout.tsx", "create".green());

    let content = engine.render("admin/frontend/Dashboard.tsx.tera", &ctx)?;
    utils::write_file(&admin_dir.join("Dashboard.tsx"), &content)?;
    println!("  {} frontend/src/admin/Dashboard.tsx", "create".green());

    let content = engine.render("admin/frontend/adminRoutes.tsx.tera", &ctx)?;
    utils::write_file(&admin_dir.join("routes.tsx"), &content)?;
    println!("  {} frontend/src/admin/routes.tsx", "create".green());

    // Backend admin routes + handlers
    let content = engine.render("admin/backend/admin_routes.rs.tera", &ctx)?;
    utils::write_file(
        &project_dir.join("backend/src/routes/admin.rs"),
        &content,
    )?;
    println!("  {} backend/src/routes/admin.rs", "create".green());

    let content = engine.render("admin/backend/admin_handlers.rs.tera", &ctx)?;
    utils::write_file(
        &project_dir.join("backend/src/handlers/admin.rs"),
        &content,
    )?;
    println!("  {} backend/src/handlers/admin.rs", "create".green());

    // Register via markers
    let base = project_dir.join("backend/src");
    utils::insert_at_marker(
        &base.join("routes/mod.rs"),
        markers::MODS,
        "pub mod admin;",
    )?;
    utils::insert_at_marker(
        &base.join("handlers/mod.rs"),
        markers::MODS,
        "pub mod admin;",
    )?;
    utils::insert_at_marker(
        &base.join("routes/mod.rs"),
        markers::ROUTES,
        "        .merge(admin::router())",
    )?;

    println!();
    println!("{}", "Admin panel generated successfully!".green().bold());

    Ok(())
}
