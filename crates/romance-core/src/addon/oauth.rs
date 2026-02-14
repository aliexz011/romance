use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct OauthAddon {
    pub provider: String,
}

impl Addon for OauthAddon {
    fn name(&self) -> &str {
        "oauth"
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
        project_root.join("backend/src/oauth.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_oauth(project_root, &self.provider)
    }
}

fn install_oauth(project_root: &Path, provider: &str) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use heck::ToPascalCase;
    use tera::Context;

    let valid_providers = ["google", "github", "discord"];
    if !valid_providers.contains(&provider) {
        anyhow::bail!(
            "Unsupported OAuth provider '{}'. Supported: {}",
            provider,
            valid_providers.join(", ")
        );
    }

    println!(
        "{}",
        format!("Installing OAuth ({})...", provider).bold()
    );

    let engine = TemplateEngine::new()?;
    let timestamp = crate::generator::migration::next_timestamp();

    let mut ctx = Context::new();
    ctx.insert("provider", provider);
    ctx.insert("provider_pascal", &provider.to_pascal_case());
    ctx.insert("timestamp", &timestamp);

    // Generate OAuth module
    let content = engine.render("addon/oauth/oauth.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/oauth.rs"), &content)?;
    println!("  {} backend/src/oauth.rs", "create".green());

    // Generate OAuth handlers
    let content = engine.render("addon/oauth/oauth_handlers.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/handlers/oauth.rs"),
        &content,
    )?;
    println!("  {} backend/src/handlers/oauth.rs", "create".green());

    // Generate OAuth routes
    let content = engine.render("addon/oauth/oauth_routes.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/routes/oauth.rs"),
        &content,
    )?;
    println!("  {} backend/src/routes/oauth.rs", "create".green());

    // Generate migration to add oauth columns to users table
    let content = engine.render("addon/oauth/oauth_migration.rs.tera", &ctx)?;
    let migration_module = format!("m{}_add_oauth_to_users", timestamp);
    utils::write_file(
        &project_root.join(format!("backend/migration/src/{}.rs", migration_module)),
        &content,
    )?;
    println!(
        "  {} backend/migration/src/{}.rs",
        "create".green(),
        migration_module
    );

    // Generate frontend OAuth button
    let content = engine.render("addon/oauth/OAuthButton.tsx.tera", &ctx)?;
    utils::write_file(
        &project_root.join("frontend/src/features/auth/OAuthButton.tsx"),
        &content,
    )?;
    println!(
        "  {} frontend/src/features/auth/OAuthButton.tsx",
        "create".green()
    );

    // Register modules
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod oauth;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        mods_marker,
        "pub mod oauth;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:ROUTES ===",
        "        .merge(oauth::router())",
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

    // Inject oauth fields into user entity model
    let user_model_path = project_root.join("backend/src/entities/user.rs");
    if user_model_path.exists() {
        let user_content = std::fs::read_to_string(&user_model_path)?;
        if !user_content.contains("oauth_provider") {
            // Insert oauth fields before the created_at field
            let new_content = if user_content.contains("pub created_at:") {
                user_content.replace(
                    "    pub created_at:",
                    "    pub oauth_provider: Option<String>,\n    pub oauth_id: Option<String>,\n    pub created_at:",
                )
            } else {
                // Fallback: insert before ROMANCE:RELATIONS marker
                user_content.replace(
                    "// === ROMANCE:RELATIONS ===",
                    "    pub oauth_provider: Option<String>,\n    pub oauth_id: Option<String>,\n}\n\n// === ROMANCE:RELATIONS ===",
                )
            };
            std::fs::write(&user_model_path, new_content)?;
            println!("  {} backend/src/entities/user.rs (added oauth fields)", "update".green());
        }
    }

    // Add mod oauth to main.rs
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    if !main_content.contains("mod oauth;") {
        let new_content = main_content.replace("mod errors;", "mod errors;\nmod oauth;");
        std::fs::write(&main_path, new_content)?;
    }

    // Add dependencies
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[
            ("oauth2", r#""4""#),
            ("reqwest", r#"{ version = "0.12", features = ["json"] }"#),
        ],
    )?;

    // Add env vars
    let provider_upper = provider.to_uppercase();
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        &format!("{}_CLIENT_ID=your-client-id", provider_upper),
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env"),
        &format!("{}_CLIENT_SECRET=your-client-secret", provider_upper),
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        &format!("{}_CLIENT_ID=your-client-id", provider_upper),
    )?;
    crate::generator::auth::append_env_var(
        &project_root.join("backend/.env.example"),
        &format!("{}_CLIENT_SECRET=your-client-secret", provider_upper),
    )?;

    println!();
    println!(
        "{}",
        format!("OAuth ({}) installed successfully!", provider)
            .green()
            .bold()
    );
    println!(
        "  Set {}_CLIENT_ID and {}_CLIENT_SECRET in backend/.env",
        provider_upper, provider_upper
    );

    Ok(())
}
