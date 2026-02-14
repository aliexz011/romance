use crate::entity::EntityDefinition;
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use heck::ToSnakeCase;
use std::path::Path;
use tera::Context;

/// Generate a unique migration timestamp by scanning existing migration files.
/// If the current second already has migrations, increment until unique.
pub fn next_timestamp() -> String {
    let base = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let migration_dir = Path::new("backend/migration/src");

    if !migration_dir.exists() {
        return base;
    }

    // Collect all existing timestamps from migration filenames (m{timestamp}_...)
    let existing: Vec<String> = std::fs::read_dir(migration_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('m') && name.ends_with(".rs") {
                // Extract timestamp: m20260214173404_create_...
                name.get(1..15).map(|s| s.to_string())
            } else {
                None
            }
        })
        .collect();

    // If base timestamp is not taken, use it
    if !existing.contains(&base) {
        return base;
    }

    // Otherwise increment until we find a free slot
    if let Ok(num) = base.parse::<u64>() {
        let mut candidate = num + 1;
        loop {
            let candidate_str = candidate.to_string();
            if !existing.contains(&candidate_str) {
                return candidate_str;
            }
            candidate += 1;
        }
    }

    base
}

pub fn generate(entity: &EntityDefinition) -> Result<()> {
    let engine = TemplateEngine::new()?;

    let timestamp = next_timestamp();
    let snake_name = entity.name.to_snake_case();

    let mut ctx = Context::new();
    ctx.insert("entity_name", &entity.name);
    ctx.insert("entity_name_snake", &snake_name);
    ctx.insert("timestamp", &timestamp);

    // Check project-level features
    let project_root = Path::new(".");
    let config = crate::config::RomanceConfig::load(project_root).ok();
    let soft_delete = config.as_ref().map(|c| c.has_feature("soft_delete")).unwrap_or(false);
    let has_search = config.as_ref().map(|c| c.has_feature("search")).unwrap_or(false);
    let has_searchable_fields = entity.fields.iter().any(|f| f.searchable);

    ctx.insert("soft_delete", &soft_delete);
    ctx.insert("has_search", &has_search);
    ctx.insert("has_searchable_fields", &has_searchable_fields);

    let fields: Vec<serde_json::Value> = entity
        .fields
        .iter()
        .map(|f| {
            serde_json::json!({
                "name": f.name,
                "postgres_type": f.field_type.to_postgres(),
                "sea_orm_column": f.field_type.to_sea_orm_column(),
                "migration_method": f.field_type.to_sea_orm_migration(),
                "optional": f.optional,
                "relation": f.relation,
                "searchable": f.searchable,
            })
        })
        .collect();
    ctx.insert("fields", &fields);

    let content = engine.render("entity/backend/migration.rs.tera", &ctx)?;
    let migration_module = format!("m{}_create_{}_table", timestamp, snake_name);
    let migration_path =
        Path::new("backend/migration/src").join(format!("{}.rs", migration_module));
    utils::write_file(&migration_path, &content)?;

    // Register migration in lib.rs
    let lib_path = Path::new("backend/migration/src/lib.rs");
    utils::insert_at_marker(
        lib_path,
        "// === ROMANCE:MIGRATION_MODS ===",
        &format!("mod {};", migration_module),
    )?;
    utils::insert_at_marker(
        lib_path,
        "// === ROMANCE:MIGRATIONS ===",
        &format!(
            "            Box::new({}::Migration),",
            migration_module
        ),
    )?;

    println!("  Generated migration for '{}'", entity.name);
    Ok(())
}
