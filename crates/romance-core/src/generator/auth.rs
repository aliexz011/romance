use crate::config::RomanceConfig;
use crate::generator::context::{self, markers};
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
    ctx.insert("has_multitenancy", &config.has_feature("multitenancy"));

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

    utils::insert_at_marker(
        &base.join("entities/mod.rs"),
        markers::MODS,
        "pub mod user;",
    )?;
    utils::insert_at_marker(
        &base.join("handlers/mod.rs"),
        markers::MODS,
        "pub mod auth;",
    )?;
    utils::insert_at_marker(
        &base.join("routes/mod.rs"),
        markers::MODS,
        "pub mod auth;",
    )?;
    utils::insert_at_marker(
        &base.join("routes/mod.rs"),
        markers::ROUTES,
        "        .merge(auth::router())",
    )?;

    // Register migration
    context::register_migration(project_dir, &migration_module)?;

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

    // Create a minimal user API module for FK dropdown support
    // (entities with FK references to User need userApi.list())
    let user_api_path = project_dir.join("frontend/src/features/user/api.ts");
    if !user_api_path.exists() {
        let user_api_content = r#"import { apiFetch, apiFetchPaginated } from '@/lib/utils';

export interface User {
  id: string;
  email: string;
  role: string;
}

export interface UserListParams {
  page?: number;
  perPage?: number;
  [key: string]: string | number | undefined;
}

export const userApi = {
  list: (params: UserListParams = {}) => {
    const { page = 1, perPage = 100 } = params;
    const searchParams = new URLSearchParams();
    searchParams.set('page', String(page));
    searchParams.set('per_page', String(perPage));
    return apiFetchPaginated<User[]>(`/auth/users?${searchParams.toString()}`);
  },

  get: (id: string) =>
    apiFetch<User>(`/auth/users/${id}`),
};
"#;
        utils::write_file(&user_api_path, user_api_content)?;
        println!(
            "  {} frontend/src/features/user/api.ts",
            "create".green()
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
            // Dependency exists — merge features if the new spec has features
            if let Some(new_features) = extract_features(version) {
                new_content = merge_features_into_dep(&new_content, name, &new_features);
            }
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

/// Extract feature names from a dependency spec like `{ version = "0.8", features = ["json", "multipart"] }`
fn extract_features(spec: &str) -> Option<Vec<String>> {
    let start = spec.find("features = [")?;
    let bracket_start = start + "features = [".len();
    let bracket_end = spec[bracket_start..].find(']')? + bracket_start;
    let features_str = &spec[bracket_start..bracket_end];
    let features: Vec<String> = features_str
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if features.is_empty() {
        None
    } else {
        Some(features)
    }
}

/// Merge new features into an existing dependency line in Cargo.toml content
fn merge_features_into_dep(content: &str, dep_name: &str, new_features: &[String]) -> String {
    let dep_prefix = format!("{} =", dep_name);
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    for line in &mut lines {
        if !line.trim_start().starts_with(&dep_prefix) {
            continue;
        }
        // Parse existing features from this line
        if let Some(existing) = extract_features(line) {
            let mut all_features: Vec<String> = existing;
            for f in new_features {
                if !all_features.contains(f) {
                    all_features.push(f.clone());
                }
            }
            // Rebuild the features array in the line
            let old_start = line.find("features = [").unwrap();
            let bracket_end = line[old_start..].find(']').unwrap() + old_start + 1;
            let new_features_str = format!(
                "features = [{}]",
                all_features
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            line.replace_range(old_start..bracket_end, &new_features_str);
        } else if line.contains("features") {
            // features key exists but couldn't parse — skip
        } else if line.contains('{') && line.contains('}') {
            // Inline table without features — add features before closing brace
            let close = line.rfind('}').unwrap();
            let features_str = format!(
                ", features = [{}] ",
                new_features
                    .iter()
                    .map(|f| format!("\"{}\"", f))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            line.insert_str(close, &features_str);
        }
        break;
    }

    // Preserve trailing newline
    let result = lines.join("\n");
    if content.ends_with('\n') && !result.ends_with('\n') {
        format!("{}\n", result)
    } else {
        result
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_features_basic() {
        let spec = r#"{ version = "0.8", features = ["json", "multipart"] }"#;
        let features = extract_features(spec).unwrap();
        assert_eq!(features, vec!["json", "multipart"]);
    }

    #[test]
    fn extract_features_single() {
        let spec = r#"{ version = "0.6", features = ["cors"] }"#;
        let features = extract_features(spec).unwrap();
        assert_eq!(features, vec!["cors"]);
    }

    #[test]
    fn extract_features_none() {
        let spec = r#""0.8""#;
        assert!(extract_features(spec).is_none());
    }

    #[test]
    fn merge_features_adds_new() {
        let content = r#"[dependencies]
axum = { version = "0.8", features = ["json"] }
"#;
        let result = merge_features_into_dep(content, "axum", &["multipart".to_string()]);
        assert!(result.contains(r#""json""#));
        assert!(result.contains(r#""multipart""#));
    }

    #[test]
    fn merge_features_no_duplicates() {
        let content = r#"[dependencies]
axum = { version = "0.8", features = ["json", "multipart"] }
"#;
        let result = merge_features_into_dep(content, "axum", &["json".to_string()]);
        // Should not have duplicate "json"
        let count = result.matches(r#""json""#).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn merge_features_multiple_deps() {
        let content = r#"[dependencies]
serde = { version = "1", features = ["derive"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }
"#;
        let result = merge_features_into_dep(
            content,
            "tower-http",
            &["request-id".to_string(), "propagate-header".to_string()],
        );
        assert!(result.contains(r#""cors""#));
        assert!(result.contains(r#""trace""#));
        assert!(result.contains(r#""request-id""#));
        assert!(result.contains(r#""propagate-header""#));
        // serde should be unchanged
        assert!(result.contains(r#"serde = { version = "1", features = ["derive"] }"#));
    }

    #[test]
    fn insert_cargo_dependency_merges_features() {
        let dir = tempfile::tempdir().unwrap();
        let cargo_path = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_path,
            r#"[dependencies]
axum = { version = "0.8", features = ["json"] }
"#,
        )
        .unwrap();

        insert_cargo_dependency(
            &cargo_path,
            &[("axum", r#"{ version = "0.8", features = ["json", "multipart"] }"#)],
        )
        .unwrap();

        let result = std::fs::read_to_string(&cargo_path).unwrap();
        assert!(result.contains(r#""json""#));
        assert!(result.contains(r#""multipart""#));
    }
}
