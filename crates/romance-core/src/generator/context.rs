use crate::config::RomanceConfig;
use crate::entity::{FieldType, ValidationRule};
use crate::utils;
use anyhow::Result;
use std::path::Path;

/// All marker strings used across generators.
pub mod markers {
    pub const MODS: &str = "// === ROMANCE:MODS ===";
    pub const ROUTES: &str = "// === ROMANCE:ROUTES ===";
    pub const MIGRATION_MODS: &str = "// === ROMANCE:MIGRATION_MODS ===";
    pub const MIGRATIONS: &str = "// === ROMANCE:MIGRATIONS ===";
    pub const RELATIONS: &str = "// === ROMANCE:RELATIONS ===";
    pub const RELATION_HANDLERS: &str = "// === ROMANCE:RELATION_HANDLERS ===";
    pub const RELATION_ROUTES: &str = "// === ROMANCE:RELATION_ROUTES ===";
    pub const MIDDLEWARE: &str = "// === ROMANCE:MIDDLEWARE ===";
    pub const IMPORTS: &str = "// === ROMANCE:IMPORTS ===";
    pub const APP_ROUTES: &str = "{/* === ROMANCE:APP_ROUTES === */}";
    pub const NAV_LINKS: &str = "{/* === ROMANCE:NAV_LINKS === */}";
    pub const OPENAPI_PATHS: &str = "// === ROMANCE:OPENAPI_PATHS ===";
    pub const OPENAPI_SCHEMAS: &str = "// === ROMANCE:OPENAPI_SCHEMAS ===";
    pub const OPENAPI_TAGS: &str = "// === ROMANCE:OPENAPI_TAGS ===";
    pub const SEEDS: &str = "// === ROMANCE:SEEDS ===";
    pub const CUSTOM: &str = "// === ROMANCE:CUSTOM ===";
}

/// Project-level feature flags loaded once from `romance.toml`.
pub struct ProjectFeatures {
    pub soft_delete: bool,
    pub has_validation: bool,
    pub has_search: bool,
    pub has_audit: bool,
    pub has_multitenancy: bool,
    pub has_auth: bool,
    pub api_prefix: String,
}

impl ProjectFeatures {
    /// Load feature flags from `romance.toml` at `project_root`.
    /// Falls back to defaults if the config file is missing.
    pub fn load(project_root: &Path) -> Self {
        let config = RomanceConfig::load(project_root).ok();
        let soft_delete = config.as_ref().map(|c| c.has_feature("soft_delete")).unwrap_or(false);
        let has_validation = config.as_ref().map(|c| c.has_feature("validation")).unwrap_or(false);
        let has_search = config.as_ref().map(|c| c.has_feature("search")).unwrap_or(false);
        let has_audit = config.as_ref().map(|c| c.has_feature("audit_log")).unwrap_or(false);
        let has_multitenancy = config.as_ref().map(|c| c.has_feature("multitenancy")).unwrap_or(false);
        let has_auth = project_root.join("backend/src/auth.rs").exists();
        let api_prefix = config.as_ref()
            .and_then(|c| c.backend.api_prefix.clone())
            .unwrap_or_else(|| "/api".to_string());

        Self {
            soft_delete,
            has_validation,
            has_search,
            has_audit,
            has_multitenancy,
            has_auth,
            api_prefix,
        }
    }
}

/// Convert validation rules to JSON values for template context.
pub fn validation_rules_to_json(rules: &[ValidationRule]) -> Vec<serde_json::Value> {
    rules
        .iter()
        .map(|v| match v {
            ValidationRule::Min(n) => serde_json::json!({"type": "min", "value": n}),
            ValidationRule::Max(n) => serde_json::json!({"type": "max", "value": n}),
            ValidationRule::Email => serde_json::json!({"type": "email"}),
            ValidationRule::Url => serde_json::json!({"type": "url"}),
            ValidationRule::Regex(r) => serde_json::json!({"type": "regex", "value": r}),
            ValidationRule::Required => serde_json::json!({"type": "required"}),
            ValidationRule::Unique => serde_json::json!({"type": "unique"}),
        })
        .collect()
}

/// Determine the filter strategy based on field type.
///
/// - `"contains"` for String/Text types (partial match with ILIKE)
/// - `"eq"` for exact-match types (bool, int, uuid, etc.)
/// - `"skip"` for types that shouldn't be filtered (Json, File, Image)
pub fn filter_method(field_type: &FieldType) -> &'static str {
    match field_type {
        FieldType::String | FieldType::Text | FieldType::Enum(_) => "contains",
        FieldType::Bool
        | FieldType::Int32
        | FieldType::Int64
        | FieldType::Float64
        | FieldType::Decimal
        | FieldType::Uuid
        | FieldType::DateTime
        | FieldType::Date => "eq",
        FieldType::Json | FieldType::File | FieldType::Image => "skip",
    }
}

/// Check if a field type is numeric.
pub fn is_numeric(field_type: &FieldType) -> bool {
    matches!(
        field_type,
        FieldType::Int32 | FieldType::Int64 | FieldType::Float64 | FieldType::Decimal
    )
}

/// Register a backend module: adds `pub mod` to entities/handlers/routes mod.rs
/// and merges the router in routes/mod.rs.
pub fn register_backend_module(backend_src: &Path, module_name: &str) -> Result<()> {
    let routes_mod = backend_src.join("routes/mod.rs");
    utils::insert_at_marker(
        &routes_mod,
        markers::ROUTES,
        &format!("        .merge({module_name}::router())"),
    )?;
    utils::insert_at_marker(
        &routes_mod,
        markers::MODS,
        &format!("pub mod {};", module_name),
    )?;

    let entities_mod = backend_src.join("entities/mod.rs");
    utils::insert_at_marker(
        &entities_mod,
        markers::MODS,
        &format!("pub mod {};", module_name),
    )?;

    let handlers_mod = backend_src.join("handlers/mod.rs");
    utils::insert_at_marker(
        &handlers_mod,
        markers::MODS,
        &format!("pub mod {};", module_name),
    )?;

    Ok(())
}

/// Register a migration module in `backend/migration/src/lib.rs`.
pub fn register_migration(project_root: &Path, migration_module: &str) -> Result<()> {
    let lib_path = project_root.join("backend/migration/src/lib.rs");
    utils::insert_at_marker(
        &lib_path,
        markers::MIGRATION_MODS,
        &format!("mod {};", migration_module),
    )?;
    utils::insert_at_marker(
        &lib_path,
        markers::MIGRATIONS,
        &format!("            Box::new({}::Migration),", migration_module),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn validation_rules_to_json_empty() {
        let json = validation_rules_to_json(&[]);
        assert!(json.is_empty());
    }

    #[test]
    fn validation_rules_to_json_all_types() {
        let rules = vec![
            ValidationRule::Min(3),
            ValidationRule::Max(100),
            ValidationRule::Email,
            ValidationRule::Url,
            ValidationRule::Regex("^[a-z]+$".to_string()),
            ValidationRule::Required,
            ValidationRule::Unique,
        ];
        let json = validation_rules_to_json(&rules);
        assert_eq!(json.len(), 7);
        assert_eq!(json[0]["type"], "min");
        assert_eq!(json[0]["value"], 3);
        assert_eq!(json[1]["type"], "max");
        assert_eq!(json[1]["value"], 100);
        assert_eq!(json[2]["type"], "email");
        assert_eq!(json[3]["type"], "url");
        assert_eq!(json[4]["type"], "regex");
        assert_eq!(json[4]["value"], "^[a-z]+$");
        assert_eq!(json[5]["type"], "required");
        assert_eq!(json[6]["type"], "unique");
    }

    #[test]
    fn filter_method_contains_for_strings() {
        assert_eq!(filter_method(&FieldType::String), "contains");
        assert_eq!(filter_method(&FieldType::Text), "contains");
        assert_eq!(filter_method(&FieldType::Enum(vec!["A".into()])), "contains");
    }

    #[test]
    fn filter_method_eq_for_exact_types() {
        assert_eq!(filter_method(&FieldType::Bool), "eq");
        assert_eq!(filter_method(&FieldType::Int32), "eq");
        assert_eq!(filter_method(&FieldType::Uuid), "eq");
        assert_eq!(filter_method(&FieldType::DateTime), "eq");
    }

    #[test]
    fn filter_method_skip_for_complex_types() {
        assert_eq!(filter_method(&FieldType::Json), "skip");
        assert_eq!(filter_method(&FieldType::File), "skip");
        assert_eq!(filter_method(&FieldType::Image), "skip");
    }

    #[test]
    fn is_numeric_correct() {
        assert!(is_numeric(&FieldType::Int32));
        assert!(is_numeric(&FieldType::Int64));
        assert!(is_numeric(&FieldType::Float64));
        assert!(is_numeric(&FieldType::Decimal));
        assert!(!is_numeric(&FieldType::String));
        assert!(!is_numeric(&FieldType::Bool));
        assert!(!is_numeric(&FieldType::Uuid));
    }

    #[test]
    fn register_backend_module_inserts_mods_and_routes() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        // Create directories
        std::fs::create_dir_all(base.join("routes")).unwrap();
        std::fs::create_dir_all(base.join("entities")).unwrap();
        std::fs::create_dir_all(base.join("handlers")).unwrap();

        // Write marker files
        let mut f = std::fs::File::create(base.join("routes/mod.rs")).unwrap();
        writeln!(f, "// === ROMANCE:MODS ===").unwrap();
        writeln!(f, "// === ROMANCE:ROUTES ===").unwrap();

        let mut f = std::fs::File::create(base.join("entities/mod.rs")).unwrap();
        writeln!(f, "// === ROMANCE:MODS ===").unwrap();

        let mut f = std::fs::File::create(base.join("handlers/mod.rs")).unwrap();
        writeln!(f, "// === ROMANCE:MODS ===").unwrap();

        register_backend_module(base, "product").unwrap();

        let routes = std::fs::read_to_string(base.join("routes/mod.rs")).unwrap();
        assert!(routes.contains("pub mod product;"));
        assert!(routes.contains(".merge(product::router())"));

        let entities = std::fs::read_to_string(base.join("entities/mod.rs")).unwrap();
        assert!(entities.contains("pub mod product;"));

        let handlers = std::fs::read_to_string(base.join("handlers/mod.rs")).unwrap();
        assert!(handlers.contains("pub mod product;"));
    }

    #[test]
    fn register_backend_module_errors_on_missing_marker() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        std::fs::create_dir_all(base.join("routes")).unwrap();
        std::fs::create_dir_all(base.join("entities")).unwrap();
        std::fs::create_dir_all(base.join("handlers")).unwrap();

        // routes/mod.rs without ROUTES marker
        std::fs::write(base.join("routes/mod.rs"), "// no markers here\n").unwrap();
        std::fs::write(base.join("entities/mod.rs"), "// === ROMANCE:MODS ===\n").unwrap();
        std::fs::write(base.join("handlers/mod.rs"), "// === ROMANCE:MODS ===\n").unwrap();

        let result = register_backend_module(base, "product");
        assert!(result.is_err());
    }

    #[test]
    fn register_migration_inserts_mod_and_box() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("backend/migration/src")).unwrap();

        let lib_content = r#"pub use sea_orm_migration::prelude::*;

// === ROMANCE:MIGRATION_MODS ===

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // === ROMANCE:MIGRATIONS ===
        ]
    }
}
"#;
        std::fs::write(
            dir.path().join("backend/migration/src/lib.rs"),
            lib_content,
        )
        .unwrap();

        register_migration(dir.path(), "m20260216_create_product_table").unwrap();

        let content =
            std::fs::read_to_string(dir.path().join("backend/migration/src/lib.rs")).unwrap();
        assert!(content.contains("mod m20260216_create_product_table;"));
        assert!(content.contains("Box::new(m20260216_create_product_table::Migration),"));
    }
}
