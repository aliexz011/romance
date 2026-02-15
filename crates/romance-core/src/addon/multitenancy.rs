use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct MultitenancyAddon;

impl Addon for MultitenancyAddon {
    fn name(&self) -> &str {
        "multitenancy"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)?;
        super::check_auth_exists(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/tenant.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_multitenancy(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        uninstall_multitenancy(project_root)
    }

    fn dependencies(&self) -> Vec<&str> {
        vec!["auth"]
    }
}

fn install_multitenancy(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing multitenancy...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // 1. Generate tenant extractor module
    let content = engine.render("addon/multitenancy/tenant.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/tenant.rs"), &content)?;
    println!("  {} backend/src/tenant.rs", "create".green());

    // 2. Generate tenant entity model
    let content = engine.render("addon/multitenancy/tenant_model.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/entities/tenant.rs"),
        &content,
    )?;
    println!("  {} backend/src/entities/tenant.rs", "create".green());

    // 3. Generate tenant handlers
    let content = engine.render("addon/multitenancy/tenant_handlers.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/handlers/tenant.rs"),
        &content,
    )?;
    println!("  {} backend/src/handlers/tenant.rs", "create".green());

    // 4. Generate tenant routes
    let content = engine.render("addon/multitenancy/tenant_routes.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/routes/tenant.rs"),
        &content,
    )?;
    println!("  {} backend/src/routes/tenant.rs", "create".green());

    // 5. Generate tenants table migration
    let ts1 = crate::generator::migration::next_timestamp();
    let content = engine.render("addon/multitenancy/tenant_migration.rs.tera", &ctx)?;
    let migration1_module = format!("m{}_create_tenants_table", ts1);
    utils::write_file(
        &project_root.join(format!("backend/migration/src/{}.rs", migration1_module)),
        &content,
    )?;
    println!(
        "  {} backend/migration/src/{}.rs",
        "create".green(),
        migration1_module
    );

    // 6. Generate add_tenant_to_users migration (1 second later to avoid collision)
    let ts2 = crate::generator::migration::next_timestamp();
    let content = engine.render(
        "addon/multitenancy/add_tenant_to_users_migration.rs.tera",
        &ctx,
    )?;
    let migration2_module = format!("m{}_add_tenant_id_to_users", ts2);
    utils::write_file(
        &project_root.join(format!("backend/migration/src/{}.rs", migration2_module)),
        &content,
    )?;
    println!(
        "  {} backend/migration/src/{}.rs",
        "create".green(),
        migration2_module
    );

    // Register modules via markers
    let mods_marker = "// === ROMANCE:MODS ===";

    utils::insert_at_marker(
        &project_root.join("backend/src/entities/mod.rs"),
        mods_marker,
        "pub mod tenant;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/handlers/mod.rs"),
        mods_marker,
        "pub mod tenant;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        mods_marker,
        "pub mod tenant;",
    )?;
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:ROUTES ===",
        "        .merge(tenant::router())",
    )?;

    // Register migrations
    let lib_path = project_root.join("backend/migration/src/lib.rs");
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATION_MODS ===",
        &format!("mod {};", migration1_module),
    )?;
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATIONS ===",
        &format!("            Box::new({}::Migration),", migration1_module),
    )?;
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATION_MODS ===",
        &format!("mod {};", migration2_module),
    )?;
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATIONS ===",
        &format!("            Box::new({}::Migration),", migration2_module),
    )?;

    // Add mod tenant to main.rs
    super::add_mod_to_main(project_root, "tenant")?;

    // Patch existing auth files in-place
    patch_auth_for_multitenancy(project_root)?;

    // Update romance.toml
    super::update_feature_flag(project_root, "multitenancy", true)?;

    println!();
    println!(
        "{}",
        "Multitenancy installed successfully!".green().bold()
    );
    println!("  All future entities will include tenant_id column.");
    println!("  Existing entities need manual migration to add tenant_id.");
    println!("  Tenant admin API: POST/GET /api/tenants (admin-only)");
    println!();
    println!("Next steps:");
    println!("  romance db migrate");

    Ok(())
}

/// Patch existing auth files to add tenant_id support.
/// This is needed when multitenancy is installed AFTER `romance generate auth`.
fn patch_auth_for_multitenancy(project_root: &Path) -> Result<()> {
    use colored::Colorize;

    // Patch backend/src/auth.rs — add tenant_id to Claims and create_token
    let auth_path = project_root.join("backend/src/auth.rs");
    if auth_path.exists() {
        let content = std::fs::read_to_string(&auth_path)?;

        // Only patch if not already patched
        if !content.contains("tenant_id") {
            let mut patched = content;

            // Add tenant_id field to Claims struct
            patched = patched.replace(
                "    pub role: String,\n    pub exp: usize,",
                "    pub role: String,\n    pub tenant_id: Option<String>,\n    pub exp: usize,",
            );

            // Update create_token signature
            patched = patched.replace(
                "pub fn create_token(user_id: Uuid, email: &str, role: &str) -> Result<String>",
                "pub fn create_token(user_id: Uuid, email: &str, role: &str, tenant_id: Option<Uuid>) -> Result<String>",
            );

            // Add tenant_id to Claims construction
            patched = patched.replace(
                "        role: role.to_string(),\n        exp,",
                "        role: role.to_string(),\n        tenant_id: tenant_id.map(|t| t.to_string()),\n        exp,",
            );

            std::fs::write(&auth_path, patched)?;
            println!("  {} backend/src/auth.rs (added tenant_id)", "patch".yellow());
        }
    }

    // Patch backend/src/entities/user.rs — add tenant_id field
    let user_model_path = project_root.join("backend/src/entities/user.rs");
    if user_model_path.exists() {
        let content = std::fs::read_to_string(&user_model_path)?;

        if !content.contains("tenant_id") {
            let mut patched = content;

            // Add tenant_id to Model struct
            patched = patched.replace(
                "    pub role: String,\n    pub created_at:",
                "    pub role: String,\n    pub tenant_id: Uuid,\n    pub created_at:",
            );

            // Add tenant_id to UserPublic
            patched = patched.replace(
                "    pub role: String,\n    pub created_at: DateTimeWithTimeZone,\n}\n\n#[derive(Debug, Serialize, Deserialize)]\npub struct UpdateUserRole",
                "    pub role: String,\n    pub tenant_id: Uuid,\n    pub created_at: DateTimeWithTimeZone,\n}\n\n#[derive(Debug, Serialize, Deserialize)]\npub struct UpdateUserRole",
            );

            // Add tenant_id to CreateUser
            patched = patched.replace(
                "pub struct CreateUser {\n    pub email: String,\n    pub password: String,\n}",
                "pub struct CreateUser {\n    pub email: String,\n    pub password: String,\n    pub tenant_id: Option<Uuid>,\n}",
            );

            std::fs::write(&user_model_path, patched)?;
            println!(
                "  {} backend/src/entities/user.rs (added tenant_id)",
                "patch".yellow()
            );
        }
    }

    // Patch backend/src/handlers/auth.rs — wire tenant_id through register/login
    let auth_handlers_path = project_root.join("backend/src/handlers/auth.rs");
    if auth_handlers_path.exists() {
        let content = std::fs::read_to_string(&auth_handlers_path)?;

        if !content.contains("tenant_id") {
            let mut patched = content;

            // Update register: add tenant resolution + set tenant_id on model
            // Replace the ActiveModel construction
            patched = patched.replace(
                "    let model = user::ActiveModel {\n        id: Set(user_id),\n        email: Set(input.email.clone()),\n        password_hash: Set(password_hash),\n        role: Set(\"user\".to_string()),\n        created_at: Set(now),\n        updated_at: Set(now),\n    };",
                "    // Resolve tenant: use provided tenant_id or create a default tenant\n    let tenant_id = if let Some(tid) = input.tenant_id {\n        crate::entities::tenant::Entity::find_by_id(tid)\n            .one(&state.db)\n            .await?\n            .ok_or_else(|| AppError::NotFound(\"Tenant not found\".into()))?;\n        tid\n    } else {\n        let tid = Uuid::new_v4();\n        let tenant_model = crate::entities::tenant::ActiveModel {\n            id: Set(tid),\n            name: Set(format!(\"{}'s Organization\", input.email)),\n            slug: Set(format!(\"org-{}\", user_id)),\n            created_at: Set(now),\n            updated_at: Set(now),\n        };\n        tenant_model.insert(&state.db).await?;\n        tid\n    };\n\n    let model = user::ActiveModel {\n        id: Set(user_id),\n        email: Set(input.email.clone()),\n        password_hash: Set(password_hash),\n        role: Set(\"user\".to_string()),\n        tenant_id: Set(tenant_id),\n        created_at: Set(now),\n        updated_at: Set(now),\n    };",
            );

            // Update create_token calls in register
            patched = patched.replace(
                "    let token = auth::create_token(created.id, &created.email, &created.role)\n        .map_err(|e| AppError::Internal(e))?;\n\n    let user_public = UserPublic {\n        id: created.id,\n        email: created.email,\n        role: created.role,\n        created_at: created.created_at,\n    };",
                "    let token = auth::create_token(created.id, &created.email, &created.role, Some(created.tenant_id))\n        .map_err(|e| AppError::Internal(e))?;\n\n    let user_public = UserPublic {\n        id: created.id,\n        email: created.email,\n        role: created.role,\n        tenant_id: created.tenant_id,\n        created_at: created.created_at,\n    };",
            );

            // Update create_token call in login
            patched = patched.replace(
                "    let token = auth::create_token(user.id, &user.email, &user.role)\n        .map_err(|e| AppError::Internal(e))?;\n\n    let user_public = UserPublic {\n        id: user.id,\n        email: user.email,\n        role: user.role,\n        created_at: user.created_at,\n    };",
                "    let token = auth::create_token(user.id, &user.email, &user.role, Some(user.tenant_id))\n        .map_err(|e| AppError::Internal(e))?;\n\n    let user_public = UserPublic {\n        id: user.id,\n        email: user.email,\n        role: user.role,\n        tenant_id: user.tenant_id,\n        created_at: user.created_at,\n    };",
            );

            // Update me() handler UserPublic
            patched = patched.replace(
                "    let user_public = UserPublic {\n        id: user.id,\n        email: user.email,\n        role: user.role,\n        created_at: user.created_at,\n    };\n\n    Ok(ok(user_public))\n}\n\n/// Update a user's role",
                "    let user_public = UserPublic {\n        id: user.id,\n        email: user.email,\n        role: user.role,\n        tenant_id: user.tenant_id,\n        created_at: user.created_at,\n    };\n\n    Ok(ok(user_public))\n}\n\n/// Update a user's role",
            );

            std::fs::write(&auth_handlers_path, patched)?;
            println!(
                "  {} backend/src/handlers/auth.rs (added tenant_id)",
                "patch".yellow()
            );
        }
    }

    Ok(())
}

fn uninstall_multitenancy(project_root: &Path) -> Result<()> {
    use colored::Colorize;

    println!("{}", "Uninstalling multitenancy...".bold());

    // Delete generated files
    let files_to_remove = [
        "backend/src/tenant.rs",
        "backend/src/entities/tenant.rs",
        "backend/src/handlers/tenant.rs",
        "backend/src/routes/tenant.rs",
    ];

    for file in &files_to_remove {
        if super::remove_file_if_exists(&project_root.join(file))? {
            println!("  {} {}", "delete".red(), file);
        }
    }

    // Remove mod declarations
    super::remove_mod_from_main(project_root, "tenant")?;

    super::remove_line_from_file(
        &project_root.join("backend/src/entities/mod.rs"),
        "pub mod tenant;",
    )?;
    super::remove_line_from_file(
        &project_root.join("backend/src/handlers/mod.rs"),
        "pub mod tenant;",
    )?;
    super::remove_line_from_file(
        &project_root.join("backend/src/routes/mod.rs"),
        "pub mod tenant;",
    )?;
    super::remove_line_from_file(
        &project_root.join("backend/src/routes/mod.rs"),
        ".merge(tenant::router())",
    )?;

    // Unpatch auth.rs — remove tenant_id from Claims
    let auth_path = project_root.join("backend/src/auth.rs");
    if auth_path.exists() {
        let content = std::fs::read_to_string(&auth_path)?;
        if content.contains("tenant_id") {
            let mut patched = content;
            // Remove tenant_id field from Claims
            patched = patched.replace("    pub tenant_id: Option<String>,\n", "");
            // Revert create_token signature
            patched = patched.replace(
                "pub fn create_token(user_id: Uuid, email: &str, role: &str, tenant_id: Option<Uuid>) -> Result<String>",
                "pub fn create_token(user_id: Uuid, email: &str, role: &str) -> Result<String>",
            );
            // Remove tenant_id from Claims construction
            patched = patched.replace(
                "        tenant_id: tenant_id.map(|t| t.to_string()),\n",
                "",
            );
            std::fs::write(&auth_path, patched)?;
            println!("  {} backend/src/auth.rs (removed tenant_id)", "patch".yellow());
        }
    }

    // Unpatch user model
    let user_model_path = project_root.join("backend/src/entities/user.rs");
    if user_model_path.exists() {
        let content = std::fs::read_to_string(&user_model_path)?;
        if content.contains("    pub tenant_id: Uuid,") {
            let patched = content
                .replace("    pub tenant_id: Uuid,\n", "")
                .replace("    pub tenant_id: Option<Uuid>,\n", "");
            std::fs::write(&user_model_path, patched)?;
            println!(
                "  {} backend/src/entities/user.rs (removed tenant_id)",
                "patch".yellow()
            );
        }
    }

    // Unpatch auth handlers — revert create_token calls
    let auth_handlers_path = project_root.join("backend/src/handlers/auth.rs");
    if auth_handlers_path.exists() {
        let content = std::fs::read_to_string(&auth_handlers_path)?;
        if content.contains("tenant_id") {
            let mut patched = content;
            // Revert create_token calls (remove tenant_id argument)
            patched = patched.replace(
                ", Some(created.tenant_id))",
                ")",
            );
            patched = patched.replace(
                ", Some(user.tenant_id))",
                ")",
            );
            // Remove tenant_id from UserPublic constructions
            patched = patched.replace("        tenant_id: created.tenant_id,\n", "");
            patched = patched.replace("        tenant_id: user.tenant_id,\n", "");
            patched = patched.replace("        tenant_id: updated.tenant_id,\n", "");
            // Remove the tenant resolution block in register
            // This is harder to do cleanly with string replace, so we remove line by line
            let lines: Vec<&str> = patched.lines().collect();
            let mut result = Vec::new();
            let mut skip_block = false;
            for line in &lines {
                if line.contains("// Resolve tenant:") {
                    skip_block = true;
                    continue;
                }
                if skip_block {
                    if line.trim() == "};" && !line.contains("ActiveModel") {
                        // End of tenant resolution block
                        skip_block = false;
                        continue;
                    }
                    continue;
                }
                // Remove tenant_id: Set(tenant_id) from ActiveModel
                if line.contains("tenant_id: Set(tenant_id)") {
                    continue;
                }
                result.push(*line);
            }
            patched = result.join("\n");
            if !patched.ends_with('\n') {
                patched.push('\n');
            }

            std::fs::write(&auth_handlers_path, patched)?;
            println!(
                "  {} backend/src/handlers/auth.rs (removed tenant_id)",
                "patch".yellow()
            );
        }
    }

    // Remove feature flag
    super::remove_feature_flag(project_root, "multitenancy")?;

    // Note: migration files are left in place (can't safely remove without risking DB state)

    // Regenerate AI context
    crate::ai_context::regenerate(project_root).ok();

    println!();
    println!(
        "{}",
        "Multitenancy uninstalled successfully.".green().bold()
    );
    println!(
        "  {}",
        "Note: Migration files were left in place. Create a new migration to remove tenant_id columns."
            .dimmed()
    );

    Ok(())
}
