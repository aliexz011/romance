use crate::config::RomanceConfig;
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use heck::ToSnakeCase;
use std::path::Path;
use tera::Context;

pub fn generate() -> Result<()> {
    let project_dir = Path::new(".");

    if !project_dir.join("romance.toml").exists() {
        anyhow::bail!("Not a Romance project (romance.toml not found)");
    }

    // Check idempotency
    if project_dir.join("backend/src/auth.rs").exists() {
        anyhow::bail!("Auth already generated (backend/src/auth.rs exists)");
    }

    println!("{}", "Generating authentication...".bold());

    let config = RomanceConfig::load(project_dir)?;
    let engine = TemplateEngine::new()?;

    let timestamp = super::migration::next_timestamp();

    let mut ctx = Context::new();
    ctx.insert("project_name", &config.project.name);
    ctx.insert("project_name_snake", &config.project.name.to_snake_case());
    ctx.insert("timestamp", &timestamp);

    // Backend auth module
    let content = engine.render("auth/backend/auth.rs.tera", &ctx)?;
    utils::write_file(&project_dir.join("backend/src/auth.rs"), &content)?;
    println!("  {} backend/src/auth.rs", "create".green());

    // User entity model
    let content = engine.render("auth/backend/user_model.rs.tera", &ctx)?;
    utils::write_file(
        &project_dir.join("backend/src/entities/user.rs"),
        &content,
    )?;
    println!("  {} backend/src/entities/user.rs", "create".green());

    // Auth handlers
    let content = engine.render("auth/backend/auth_handlers.rs.tera", &ctx)?;
    utils::write_file(
        &project_dir.join("backend/src/handlers/auth.rs"),
        &content,
    )?;
    println!("  {} backend/src/handlers/auth.rs", "create".green());

    // Auth routes
    let content = engine.render("auth/backend/auth_routes.rs.tera", &ctx)?;
    utils::write_file(
        &project_dir.join("backend/src/routes/auth.rs"),
        &content,
    )?;
    println!("  {} backend/src/routes/auth.rs", "create".green());

    // User migration
    let content = engine.render("auth/backend/user_migration.rs.tera", &ctx)?;
    let migration_module = format!("m{}_create_users_table", timestamp);
    utils::write_file(
        &project_dir.join(format!("backend/migration/src/{}.rs", migration_module)),
        &content,
    )?;
    println!(
        "  {} backend/migration/src/{}.rs",
        "create".green(),
        migration_module
    );

    // Register modules via markers
    let base = project_dir.join("backend/src");
    let mods_marker = "// === ROMANCE:MODS ===";

    utils::insert_at_marker(
        &base.join("entities/mod.rs"),
        mods_marker,
        "pub mod user;",
    )?;
    utils::insert_at_marker(
        &base.join("handlers/mod.rs"),
        mods_marker,
        "pub mod auth;",
    )?;
    utils::insert_at_marker(
        &base.join("routes/mod.rs"),
        mods_marker,
        "pub mod auth;",
    )?;
    utils::insert_at_marker(
        &base.join("routes/mod.rs"),
        "// === ROMANCE:ROUTES ===",
        "        .merge(auth::router())",
    )?;

    // Register migration
    let lib_path = project_dir.join("backend/migration/src/lib.rs");
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

    // Add dependencies to Cargo.toml
    insert_cargo_dependency(
        &project_dir.join("backend/Cargo.toml"),
        &[
            ("argon2", r#""0.5""#),
            ("jsonwebtoken", r#""9""#),
        ],
    )?;

    // Add JWT_SECRET to .env and .env.example (random per project)
    let jwt_secret = generate_jwt_secret();
    append_env_var(
        &project_dir.join("backend/.env"),
        &format!("JWT_SECRET={}", jwt_secret),
    )?;
    append_env_var(
        &project_dir.join("backend/.env.example"),
        &format!("JWT_SECRET={}", jwt_secret),
    )?;

    // Add mod auth to main.rs
    let main_path = base.join("main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod auth;") {
        let new_content = main_content.replace("mod errors;", "mod auth;\nmod errors;");
        std::fs::write(&main_path, new_content)?;
    }

    // Frontend auth files
    let auth_dir = project_dir.join("frontend/src/features/auth");

    let frontend_files = vec![
        ("auth/frontend/types.ts.tera", "types.ts"),
        ("auth/frontend/api.ts.tera", "api.ts"),
        ("auth/frontend/hooks.ts.tera", "hooks.ts"),
        ("auth/frontend/AuthContext.tsx.tera", "AuthContext.tsx"),
        ("auth/frontend/LoginPage.tsx.tera", "LoginPage.tsx"),
        ("auth/frontend/RegisterPage.tsx.tera", "RegisterPage.tsx"),
        ("auth/frontend/ProtectedRoute.tsx.tera", "ProtectedRoute.tsx"),
    ];

    for (template, output) in &frontend_files {
        let content = engine.render(template, &ctx)?;
        utils::write_file(&auth_dir.join(output), &content)?;
        println!(
            "  {} frontend/src/features/auth/{}",
            "create".green(),
            output
        );
    }

    println!();
    println!("{}", "Authentication generated successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!("  cd backend && cargo check");
    println!("  romance db migrate");

    Ok(())
}

pub fn insert_cargo_dependency(path: &Path, deps: &[(&str, &str)]) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let mut new_content = content.clone();

    for (name, version) in deps {
        if new_content.contains(&format!("{} =", name)) {
            continue;
        }
        // Insert before the last line of [dependencies]
        // Find the end of [dependencies] section
        if let Some(pos) = new_content.rfind('\n') {
            let dep_line = format!("{} = {}\n", name, version);
            new_content.insert_str(pos + 1, &dep_line);
        }
    }

    std::fs::write(path, new_content)?;
    Ok(())
}

pub fn append_env_var(path: &Path, line: &str) -> Result<()> {
    if let Ok(content) = std::fs::read_to_string(path) {
        // Check by key (everything before '=') to avoid duplicates
        let key = line.split('=').next().unwrap_or(line);
        if content.lines().any(|l| l.starts_with(&format!("{}=", key))) {
            return Ok(());
        }
        let new_content = format!("{}\n{}\n", content.trim_end(), line);
        std::fs::write(path, new_content)?;
    }
    Ok(())
}

/// Generate a random 64-character hex string for use as a JWT secret.
pub fn generate_jwt_secret() -> String {
    format!(
        "{:032x}{:032x}",
        uuid::Uuid::new_v4().as_u128(),
        uuid::Uuid::new_v4().as_u128()
    )
}
