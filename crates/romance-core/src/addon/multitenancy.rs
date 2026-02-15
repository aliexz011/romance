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

/// Insert `new_line` after the first line that contains `needle`.
fn insert_after_first(content: &str, needle: &str, new_line: &str) -> String {
    let mut result = String::with_capacity(content.len() + new_line.len() + 2);
    let mut found = false;
    for line in content.lines() {
        result.push_str(line);
        result.push('\n');
        if !found && line.contains(needle) {
            result.push_str(new_line);
            result.push('\n');
            found = true;
        }
    }
    result
}

/// Insert `block` before the first line that contains `needle`.
fn insert_before_first(content: &str, needle: &str, block: &str) -> String {
    let mut result = String::with_capacity(content.len() + block.len());
    let mut found = false;
    for line in content.lines() {
        if !found && line.contains(needle) {
            result.push_str(block);
            if !block.ends_with('\n') {
                result.push('\n');
            }
            found = true;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Patch existing auth files to add tenant_id support.
/// This is needed when multitenancy is installed AFTER `romance generate auth`.
/// Uses line-based operations instead of multi-line string matching for robustness.
fn patch_auth_for_multitenancy(project_root: &Path) -> Result<()> {
    use colored::Colorize;

    // === 1. Patch backend/src/auth.rs ===
    let auth_path = project_root.join("backend/src/auth.rs");
    if auth_path.exists() {
        let content = std::fs::read_to_string(&auth_path)?;
        if !content.contains("tenant_id") {
            let mut patched = content;

            // Add tenant_id field to Claims (after "pub role: String," — unique in auth.rs)
            patched = insert_after_first(
                &patched,
                "pub role: String,",
                "    pub tenant_id: Option<String>,",
            );

            // Update create_token signature (single-line match — reliable)
            patched = patched.replace(
                "pub fn create_token(user_id: Uuid, email: &str, role: &str)",
                "pub fn create_token(user_id: Uuid, email: &str, role: &str, tenant_id: Option<Uuid>)",
            );

            // Add tenant_id to Claims construction (after "role: role.to_string()," — unique in auth.rs)
            patched = insert_after_first(
                &patched,
                "role: role.to_string(),",
                "        tenant_id: tenant_id.map(|t| t.to_string()),",
            );

            std::fs::write(&auth_path, patched)?;
            println!(
                "  {} backend/src/auth.rs (added tenant_id)",
                "patch".yellow()
            );
        }
    }

    // === 2. Patch backend/src/entities/user.rs ===
    let user_model_path = project_root.join("backend/src/entities/user.rs");
    if user_model_path.exists() {
        let content = std::fs::read_to_string(&user_model_path)?;
        if !content.contains("tenant_id") {
            let mut lines: Vec<String> = content.lines().map(String::from).collect();
            let mut insertions: Vec<(usize, String)> = Vec::new();
            let mut in_update_user_role = false;

            for (i, line) in lines.iter().enumerate() {
                if line.contains("pub struct UpdateUserRole") {
                    in_update_user_role = true;
                }
                if in_update_user_role && line.trim() == "}" {
                    in_update_user_role = false;
                }
                // Add tenant_id: Uuid to Model and UserPublic (skip UpdateUserRole)
                if line.contains("pub role: String,") && !in_update_user_role {
                    insertions.push((i + 1, "    pub tenant_id: Uuid,".to_string()));
                }
                // Add tenant_id: Option<Uuid> to CreateUser
                if line.contains("pub password: String,") {
                    insertions.push((i + 1, "    pub tenant_id: Option<Uuid>,".to_string()));
                }
            }

            for (idx, new_line) in insertions.into_iter().rev() {
                lines.insert(idx, new_line);
            }

            let patched = lines.join("\n") + "\n";
            std::fs::write(&user_model_path, patched)?;
            println!(
                "  {} backend/src/entities/user.rs (added tenant_id)",
                "patch".yellow()
            );
        }
    }

    // === 3. Patch backend/src/handlers/auth.rs ===
    let auth_handlers_path = project_root.join("backend/src/handlers/auth.rs");
    if auth_handlers_path.exists() {
        let content = std::fs::read_to_string(&auth_handlers_path)?;
        if !content.contains("tenant_id") {
            let mut patched = content;

            // Update create_token calls (unique single-line matches — reliable)
            patched = patched.replace(
                "auth::create_token(created.id, &created.email, &created.role)",
                "auth::create_token(created.id, &created.email, &created.role, Some(created.tenant_id))",
            );
            patched = patched.replace(
                "auth::create_token(user.id, &user.email, &user.role)",
                "auth::create_token(user.id, &user.email, &user.role, Some(user.tenant_id))",
            );

            // Add tenant_id: Set(tenant_id) to register's ActiveModel
            patched = insert_after_first(
                &patched,
                "role: Set(\"user\".to_string()),",
                "        tenant_id: Set(tenant_id),",
            );

            // Add tenant resolution block before "let model = user::ActiveModel {"
            let tenant_block = "\
    // Resolve tenant: use provided tenant_id or create a default tenant
    let tenant_id = if let Some(tid) = input.tenant_id {
        crate::entities::tenant::Entity::find_by_id(tid)
            .one(&state.db)
            .await?
            .ok_or_else(|| AppError::NotFound(\"Tenant not found\".into()))?;
        tid
    } else {
        let tid = Uuid::new_v4();
        let tenant_model = crate::entities::tenant::ActiveModel {
            id: Set(tid),
            name: Set(format!(\"{}\'s Organization\", input.email)),
            slug: Set(format!(\"org-{}\", user_id)),
            created_at: Set(now),
            updated_at: Set(now),
        };
        tenant_model.insert(&state.db).await?;
        tid
    };

";
            patched = insert_before_first(
                &patched,
                "let model = user::ActiveModel {",
                tenant_block,
            );

            // Add tenant_id to all UserPublic constructions via line-based insertion
            let mut lines: Vec<String> = patched.lines().map(String::from).collect();
            let mut insertions: Vec<(usize, String)> = Vec::new();
            for (i, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                // Match "role: VAR.role," patterns in UserPublic constructions
                if trimmed.starts_with("role:") && trimmed.ends_with(".role,") {
                    let indent: String =
                        line.chars().take_while(|c| c.is_whitespace()).collect();
                    if let Some(var) = trimmed
                        .strip_prefix("role: ")
                        .and_then(|s| s.strip_suffix(".role,"))
                    {
                        insertions.push((
                            i + 1,
                            format!("{}tenant_id: {}.tenant_id,", indent, var),
                        ));
                    }
                }
            }

            for (idx, new_line) in insertions.into_iter().rev() {
                lines.insert(idx, new_line);
            }

            let patched = lines.join("\n") + "\n";
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

    // Unpatch auth handlers — revert create_token calls and remove tenant_id
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
            // Remove line by line: tenant resolution block, tenant_id fields, Set(tenant_id)
            let lines: Vec<&str> = patched.lines().collect();
            let mut result = Vec::new();
            let mut skip_block = false;
            for line in &lines {
                if line.contains("// Resolve tenant:") {
                    skip_block = true;
                    continue;
                }
                if skip_block {
                    // Skip until we reach the ActiveModel that we want to keep
                    if line.contains("let model = user::ActiveModel") || line.contains("let model: user::ActiveModel") {
                        skip_block = false;
                        // Don't skip — keep this line
                    } else {
                        continue;
                    }
                }
                // Remove tenant_id: Set(tenant_id) from ActiveModel
                if line.contains("tenant_id: Set(tenant_id)") {
                    continue;
                }
                // Remove tenant_id from UserPublic constructions (any indentation)
                let trimmed = line.trim();
                if trimmed.starts_with("tenant_id:") && trimmed.ends_with(".tenant_id,") {
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

    // Clean up tenant references in all generated entity files
    cleanup_entity_tenant_references(project_root)?;

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

/// Clean up tenant_id references from all generated entity files.
/// This handles entity model and handler files that were generated while multitenancy was active.
fn cleanup_entity_tenant_references(project_root: &Path) -> Result<()> {
    use colored::Colorize;

    // Clean up entity handler files
    let handlers_dir = project_root.join("backend/src/handlers");
    if handlers_dir.exists() {
        for entry in std::fs::read_dir(&handlers_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                // Skip files already handled by other uninstall steps
                if matches!(
                    name.as_str(),
                    "auth" | "tenant" | "mod" | "upload" | "search" | "audit_log" | "dev_dashboard"
                ) {
                    continue;
                }
                let content = std::fs::read_to_string(&path)?;
                if !content.contains("TenantGuard") {
                    continue;
                }
                let mut patched = content;
                // Replace TenantGuard import with AuthUser
                patched = patched.replace(
                    "use crate::tenant::TenantGuard;\n",
                    "use crate::auth::AuthUser;\n",
                );
                // Replace parameter name
                patched = patched.replace("tenant: TenantGuard,", "_auth: AuthUser,");
                // Replace audit log / claims references
                patched = patched.replace("tenant.claims.", "_auth.0.");

                // Remove tenant-specific lines
                let lines: Vec<&str> = patched.lines().collect();
                let mut result = Vec::new();
                for line in &lines {
                    let trimmed = line.trim();
                    if trimmed.contains("Column::TenantId.eq(tenant.tenant_id)") {
                        continue;
                    }
                    if trimmed.contains("tenant_id: Set(tenant.tenant_id)") {
                        continue;
                    }
                    result.push(*line);
                }
                patched = result.join("\n");
                if !patched.ends_with('\n') {
                    patched.push('\n');
                }
                std::fs::write(&path, patched)?;
                println!(
                    "  {} backend/src/handlers/{}.rs (removed tenant_id)",
                    "patch".yellow(),
                    name
                );
            }
        }
    }

    // Clean up entity model files
    let entities_dir = project_root.join("backend/src/entities");
    if entities_dir.exists() {
        for entry in std::fs::read_dir(&entities_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                if matches!(name.as_str(), "user" | "tenant" | "mod" | "audit_entry") {
                    continue;
                }
                let content = std::fs::read_to_string(&path)?;
                if !content.contains("tenant_id") && !content.contains("super::tenant") {
                    continue;
                }

                let lines: Vec<&str> = content.lines().collect();
                let mut result: Vec<&str> = Vec::new();
                let mut skip_relation_attr = false;
                let mut skip_related_impl = false;
                let mut impl_depth: i32 = 0;

                for line in &lines {
                    let trimmed = line.trim();

                    // Remove tenant_id field
                    if trimmed == "pub tenant_id: Uuid," {
                        continue;
                    }

                    // Skip Tenant relation attributes block
                    if trimmed.contains("belongs_to = \"super::tenant::Entity\"") {
                        skip_relation_attr = true;
                        // Also remove the #[sea_orm( line we already pushed
                        if let Some(last) = result.last() {
                            if last.trim() == "#[sea_orm(" {
                                result.pop();
                            }
                        }
                        continue;
                    }
                    if skip_relation_attr {
                        if trimmed == "Tenant," {
                            skip_relation_attr = false;
                        }
                        continue;
                    }

                    // Skip Related<super::tenant::Entity> impl block (track brace depth)
                    if trimmed.starts_with("impl Related<super::tenant::Entity>") {
                        skip_related_impl = true;
                        impl_depth = 0;
                        for ch in trimmed.chars() {
                            if ch == '{' { impl_depth += 1; }
                            if ch == '}' { impl_depth -= 1; }
                        }
                        continue;
                    }
                    if skip_related_impl {
                        for ch in trimmed.chars() {
                            if ch == '{' { impl_depth += 1; }
                            if ch == '}' { impl_depth -= 1; }
                        }
                        if impl_depth <= 0 {
                            skip_related_impl = false;
                        }
                        continue;
                    }

                    result.push(*line);
                }

                let mut patched = result.join("\n");
                if !patched.ends_with('\n') {
                    patched.push('\n');
                }
                std::fs::write(&path, patched)?;
                println!(
                    "  {} backend/src/entities/{}.rs (removed tenant_id)",
                    "patch".yellow(),
                    name
                );
            }
        }
    }

    Ok(())
}
