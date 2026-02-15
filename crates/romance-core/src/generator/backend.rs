use crate::entity::{EntityDefinition, FieldType, FieldVisibility, RelationType, ValidationRule};
use crate::generator::junction;
use crate::relation;
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use heck::{ToPascalCase, ToSnakeCase};
use std::path::Path;
use tera::Context;

pub fn generate(entity: &EntityDefinition) -> Result<()> {
    let engine = TemplateEngine::new()?;
    let ctx = build_context(entity);
    let snake_name = entity.name.to_snake_case();

    let base = Path::new("backend/src");

    // Generate model
    let model_content = engine.render("entity/backend/model.rs.tera", &ctx)?;
    let model_path = base.join(format!("entities/{}.rs", snake_name));
    utils::write_generated(&model_path, &model_content)?;

    // Generate handlers
    let handlers_content = engine.render("entity/backend/handlers.rs.tera", &ctx)?;
    let handlers_path = base.join(format!("handlers/{}.rs", snake_name));
    utils::write_generated(&handlers_path, &handlers_content)?;

    // Generate routes
    let routes_content = engine.render("entity/backend/routes.rs.tera", &ctx)?;
    let routes_path = base.join(format!("routes/{}.rs", snake_name));
    utils::write_generated(&routes_path, &routes_content)?;

    // Register route in routes/mod.rs
    let routes_mod = base.join("routes/mod.rs");
    let routes_marker = "// === ROMANCE:ROUTES ===";
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &routes_mod,
        routes_marker,
        &format!("        .merge({snake_name}::router())"),
    )?;

    // Add mod declaration to routes/mod.rs
    utils::insert_at_marker(
        &routes_mod,
        mods_marker,
        &format!("pub mod {};", snake_name),
    )?;

    // Add mod declaration to entities/mod.rs
    let entities_mod = base.join("entities/mod.rs");
    utils::insert_at_marker(&entities_mod, mods_marker, &format!("pub mod {};", snake_name))?;

    // Add mod declaration to handlers/mod.rs
    let handlers_mod = base.join("handlers/mod.rs");
    utils::insert_at_marker(&handlers_mod, mods_marker, &format!("pub mod {};", snake_name))?;

    // Register entity in OpenAPI spec
    let main_rs = base.join("main.rs");
    if main_rs.exists() {
        // Add handler paths
        let paths = vec![
            format!("crate::handlers::{}::list", snake_name),
            format!("crate::handlers::{}::get", snake_name),
            format!("crate::handlers::{}::create", snake_name),
            format!("crate::handlers::{}::update", snake_name),
            format!("crate::handlers::{}::delete", snake_name),
            format!("crate::handlers::{}::bulk_create", snake_name),
            format!("crate::handlers::{}::bulk_delete", snake_name),
        ];
        for path in &paths {
            utils::insert_at_marker(
                &main_rs,
                "// === ROMANCE:OPENAPI_PATHS ===",
                &format!("        {},", path),
            )?;
        }

        // Add schema types
        let schemas = vec![
            format!("crate::entities::{}::Model", snake_name),
            format!("crate::entities::{}::Create{}", snake_name, entity.name),
            format!("crate::entities::{}::Update{}", snake_name, entity.name),
            format!(
                "crate::entities::{}::{}Response",
                snake_name, entity.name
            ),
        ];
        for schema in &schemas {
            utils::insert_at_marker(
                &main_rs,
                "// === ROMANCE:OPENAPI_SCHEMAS ===",
                &format!("            {},", schema),
            )?;
        }

        // Add tag
        utils::insert_at_marker(
            &main_rs,
            "// === ROMANCE:OPENAPI_TAGS ===",
            &format!(
                "        (name = \"{}\", description = \"{} management\"),",
                entity.name, entity.name
            ),
        )?;
    }

    // Handle reverse relations: inject has-many into target entities
    // Note: junction (M2M) generation is deferred to generate_relations()
    // to ensure the entity migration runs first.
    let project_root = Path::new(".");
    let config = crate::config::RomanceConfig::load(project_root).ok();
    let api_prefix = config.as_ref()
        .and_then(|c| c.backend.api_prefix.clone())
        .unwrap_or_else(|| "/api".to_string());
    for rel in &entity.relations {
        if rel.relation_type == RelationType::BelongsTo {
            if relation::entity_exists(project_root, &rel.target_entity) {
                inject_has_many(
                    base,
                    &rel.target_entity,
                    &entity.name,
                    &rel.fk_column.clone().unwrap_or_else(|| format!("{}_id", rel.target_entity.to_snake_case())),
                    &api_prefix,
                )?;
            }
        }
    }

    utils::ui::success(&format!("Generated backend files for '{}'", entity.name));

    // Insert seed function if seed.rs exists
    let seed_path = Path::new("backend/src/seed.rs");
    if seed_path.exists() {
        let seed_fn = build_seed_function(entity);
        utils::insert_at_marker(seed_path, "// === ROMANCE:SEEDS ===", &seed_fn)?;
    }

    Ok(())
}

/// Generate M2M junction tables and apply pending relations.
/// Must be called AFTER migration::generate() to ensure correct migration order.
pub fn generate_relations(entity: &EntityDefinition) -> Result<()> {
    let project_root = Path::new(".");

    for rel in &entity.relations {
        if rel.relation_type == RelationType::ManyToMany {
            junction::generate(&entity.name, &rel.target_entity)?;
        }
    }

    // Apply any pending relations that target this newly generated entity
    let pending = relation::take_pending_for(project_root, &entity.name)?;
    for p in &pending {
        if p.relation_type == "ManyToMany" {
            junction::generate(&p.source_entity, &p.target_entity)?;
            println!("  Applied pending M2M: {} <-> {}", p.source_entity, p.target_entity);
        }
    }

    Ok(())
}

/// Inject has-many relation into an existing target entity.
/// When we generate Post with author_id:uuid->User, we inject into User:
/// 1. Related<post::Entity> impl in user model
/// 2. list_posts handler in user handlers
/// 3. /users/:id/posts route in user routes
fn inject_has_many(
    base: &Path,
    parent_entity: &str,
    child_entity: &str,
    fk_column: &str,
    api_prefix: &str,
) -> Result<()> {
    let parent_snake = parent_entity.to_snake_case();
    let child_snake = child_entity.to_snake_case();
    let fk_pascal = fk_column.to_pascal_case();

    // 1. Inject Related impl into parent model
    let model_path = base.join(format!("entities/{}.rs", parent_snake));
    let related_impl = format!(
        r#"impl Related<super::{}::Entity> for Entity {{
    fn to() -> RelationDef {{
        super::{}::Relation::{}.def().rev()
    }}
}}"#,
        child_snake, child_snake, parent_entity
    );
    utils::insert_at_marker(&model_path, "// === ROMANCE:RELATIONS ===", &related_impl)?;

    // 2. Inject list handler into parent handlers
    let handlers_path = base.join(format!("handlers/{}.rs", parent_snake));
    let child_plural = utils::pluralize(&child_snake);
    let handler_code = format!(
        r#"pub async fn list_{child_plural}(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<crate::pagination::PageRequest>,
) -> crate::errors::AppResult<crate::api::ApiResponse<Vec<crate::entities::{child_snake}::Model>>> {{
    use crate::api::ok_page;
    use crate::pagination::PageMeta;

    let page = params.page();
    let per_page = params.per_page();
    let paginator = crate::entities::{child_snake}::Entity::find()
        .filter(crate::entities::{child_snake}::Column::{fk_pascal}.eq(id))
        .paginate(&state.db, per_page);
    let total = paginator.num_items().await?;
    let data = paginator.fetch_page(page - 1).await?;
    let meta = PageMeta::from_request(&params, total);
    Ok(ok_page(data, meta))
}}"#,
        child_plural = child_plural,
        child_snake = child_snake,
        fk_pascal = fk_pascal,
    );
    utils::insert_at_marker(
        &handlers_path,
        "// === ROMANCE:RELATION_HANDLERS ===",
        &handler_code,
    )?;

    // 3. Inject route into parent routes
    let routes_path = base.join(format!("routes/{}.rs", parent_snake));
    let parent_plural = utils::pluralize(&parent_snake);
    let route_line = format!(
        "        .route(\"{}/{}/{{id}}/{}\", get({}::list_{}))",
        api_prefix, parent_plural, child_plural, parent_snake, child_plural
    );
    utils::insert_at_marker(
        &routes_path,
        "// === ROMANCE:RELATION_ROUTES ===",
        &route_line,
    )?;

    println!(
        "  Injected has-many: {} -> {}",
        parent_entity, utils::pluralize(child_entity)
    );
    Ok(())
}

/// Build a seed function string for the given entity.
///
/// Generates a `seed_{entity}s()` async function that uses the `fake` crate
/// to insert randomised rows, plus a call line to invoke it from `run()`.
/// Both are inserted before the `ROMANCE:SEEDS` marker in `seed.rs`.
fn build_seed_function(entity: &EntityDefinition) -> String {
    let snake = entity.name.to_snake_case();

    // Build field assignment lines. We skip:
    //  - fields with a relation (FK fields) — handled by ..Default::default()
    //  - optional fields — default to None via ..Default::default()
    let mut field_lines: Vec<String> = Vec::new();
    for f in &entity.fields {
        if f.relation.is_some() || f.optional {
            continue;
        }
        let faker_expr = field_type_to_faker(&f.field_type);
        field_lines.push(format!("            {}: Set({}),", f.name, faker_expr));
    }

    let fields_block = field_lines.join("\n");

    // The function itself + the call that goes into run().
    // We put the call first so that when inserted before the marker the
    // function definition sits above the call (insert_at_marker prepends).
    format!(
        r#"pub async fn seed_{snake}s(db: &DatabaseConnection, count: usize) -> Result<()> {{
    use fake::Fake;
    use fake::faker::lorem::en::*;
    use fake::faker::name::en::*;
    use fake::faker::internet::en::*;
    use crate::entities::{snake}::ActiveModel;
    use sea_orm::Set;

    for _ in 0..count {{
        let model = ActiveModel {{
            id: Set(uuid::Uuid::new_v4()),
{fields_block}
            created_at: Set(chrono::Utc::now().fixed_offset()),
            updated_at: Set(chrono::Utc::now().fixed_offset()),
            ..Default::default()
        }};
        model.insert(db).await?;
    }}
    tracing::info!("Seeded {{}} {snake}s", count);
    Ok(())
}}

seed_{snake}s(db, 10).await?;"#,
        snake = snake,
        fields_block = fields_block,
    )
}

/// Map a `FieldType` to a faker/random expression used inside seed functions.
fn field_type_to_faker(ft: &FieldType) -> String {
    match ft {
        FieldType::String | FieldType::Enum(_) | FieldType::File | FieldType::Image => {
            "Sentence(3..8).fake::<String>()".to_string()
        }
        FieldType::Text => "Paragraph(1..3).fake::<String>()".to_string(),
        FieldType::Bool => "rand::random::<bool>()".to_string(),
        FieldType::Int32 => "(1..1000i32).fake::<i32>()".to_string(),
        FieldType::Int64 => "(1..10000i64).fake::<i64>()".to_string(),
        FieldType::Float64 => "rand::random::<f64>() * 100.0".to_string(),
        FieldType::Decimal => "rust_decimal::Decimal::new((1..10000).fake::<i64>(), 2)".to_string(),
        FieldType::Uuid => "uuid::Uuid::new_v4()".to_string(),
        FieldType::DateTime => "chrono::Utc::now().fixed_offset()".to_string(),
        FieldType::Date => "chrono::Utc::now().date_naive()".to_string(),
        FieldType::Json => "serde_json::json!({{}})".to_string(),
    }
}

fn build_context(entity: &EntityDefinition) -> Context {
    let mut ctx = Context::new();
    ctx.insert("entity_name", &entity.name);
    ctx.insert("entity_name_snake", &entity.name.to_snake_case());

    // Check project-level features
    let project_root = Path::new(".");
    let config = crate::config::RomanceConfig::load(project_root).ok();
    let soft_delete = config.as_ref().map(|c| c.has_feature("soft_delete")).unwrap_or(false);
    let has_validation = config.as_ref().map(|c| c.has_feature("validation")).unwrap_or(false);
    let has_search = config.as_ref().map(|c| c.has_feature("search")).unwrap_or(false);
    let has_audit = config.as_ref().map(|c| c.has_feature("audit_log")).unwrap_or(false);
    let has_multitenancy = config.as_ref().map(|c| c.has_feature("multitenancy")).unwrap_or(false);

    // Detect if auth has been generated (backend/src/auth.rs exists)
    let has_auth = project_root.join("backend/src/auth.rs").exists();

    let api_prefix = config.as_ref()
        .and_then(|c| c.backend.api_prefix.clone())
        .unwrap_or_else(|| "/api".to_string());
    ctx.insert("api_prefix", &api_prefix);

    ctx.insert("soft_delete", &soft_delete);
    ctx.insert("has_validation", &has_validation);
    ctx.insert("has_search", &has_search);
    ctx.insert("has_audit", &has_audit);
    ctx.insert("has_auth", &has_auth);
    ctx.insert("has_multitenancy", &has_multitenancy);

    let has_searchable_fields = entity.fields.iter().any(|f| f.searchable);
    ctx.insert("has_searchable_fields", &has_searchable_fields);

    let fields: Vec<serde_json::Value> = entity
        .fields
        .iter()
        .map(|f| {
            let validations: Vec<serde_json::Value> = f
                .validations
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
                .collect();

            let has_validations = !f.validations.is_empty();
            let is_numeric = matches!(
                f.field_type,
                crate::entity::FieldType::Int32
                    | crate::entity::FieldType::Int64
                    | crate::entity::FieldType::Float64
                    | crate::entity::FieldType::Decimal
            );

            // Determine the filter strategy based on field type:
            // - "contains" for String/Text types (partial match with ILIKE)
            // - "eq" for exact-match types (bool, int, uuid, etc.)
            // - "skip" for types that shouldn't be filtered (Json, File, Image)
            let filter_method = match f.field_type {
                crate::entity::FieldType::String
                | crate::entity::FieldType::Text
                | crate::entity::FieldType::Enum(_) => "contains",
                crate::entity::FieldType::Bool
                | crate::entity::FieldType::Int32
                | crate::entity::FieldType::Int64
                | crate::entity::FieldType::Float64
                | crate::entity::FieldType::Decimal
                | crate::entity::FieldType::Uuid
                | crate::entity::FieldType::DateTime
                | crate::entity::FieldType::Date => "eq",
                crate::entity::FieldType::Json
                | crate::entity::FieldType::File
                | crate::entity::FieldType::Image => "skip",
            };

            let visibility_str = match &f.visibility {
                FieldVisibility::Public => "public",
                FieldVisibility::Authenticated => "authenticated",
                FieldVisibility::AdminOnly => "admin_only",
                FieldVisibility::Roles(_) => "roles",
            };
            let visibility_roles: Vec<String> = match &f.visibility {
                FieldVisibility::Roles(r) => r.clone(),
                _ => vec![],
            };

            serde_json::json!({
                "name": f.name,
                "rust_type": f.field_type.to_rust(),
                "postgres_type": f.field_type.to_postgres(),
                "sea_orm_column": f.field_type.to_sea_orm_column(),
                "optional": f.optional,
                "relation": f.relation,
                "validations": validations,
                "has_validations": has_validations,
                "is_numeric": is_numeric,
                "searchable": f.searchable,
                "is_file": matches!(f.field_type, crate::entity::FieldType::File),
                "is_image": matches!(f.field_type, crate::entity::FieldType::Image),
                "filter_method": filter_method,
                "visibility": visibility_str,
                "visibility_roles": visibility_roles,
            })
        })
        .collect();

    ctx.insert("fields", &fields);

    // Track if any field has restricted visibility (for conditional filter_for_role method)
    let has_restricted_fields = entity.fields.iter().any(|f| f.visibility != FieldVisibility::Public);
    ctx.insert("has_restricted_fields", &has_restricted_fields);

    // Track if any field has validation rules (for conditional imports/derives)
    let has_any_validations = entity.fields.iter().any(|f| !f.validations.is_empty());
    ctx.insert("has_any_validations", &has_any_validations);

    // Track if any field has a regex validation (for lazy_static / once_cell statics)
    let has_regex_validations = entity.fields.iter().any(|f| {
        f.validations
            .iter()
            .any(|v| matches!(v, ValidationRule::Regex(_)))
    });
    ctx.insert("has_regex_validations", &has_regex_validations);

    // Build belongs_to_relations array for nested/related serialization (?include= support).
    // Each entry provides the target entity name, snake_case name, and the FK field name
    // so that templates can generate DetailResponse structs and eager-fetch handlers.
    let belongs_to_relations: Vec<serde_json::Value> = entity
        .fields
        .iter()
        .filter(|f| f.relation.is_some())
        .map(|f| {
            let target = f.relation.as_ref().unwrap();
            serde_json::json!({
                "target": target,
                "target_snake": target.to_snake_case(),
                "fk_field": f.name,
                "optional": f.optional,
            })
        })
        .collect();
    let has_belongs_to = !belongs_to_relations.is_empty();
    ctx.insert("belongs_to_relations", &belongs_to_relations);
    ctx.insert("has_belongs_to", &has_belongs_to);

    ctx
}
