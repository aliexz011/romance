use crate::relation;
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use heck::{ToPascalCase, ToSnakeCase};
use std::path::Path;
use tera::Context;

/// Generate a junction table for a many-to-many relation between two entities.
/// Creates:
/// 1. Junction entity model (backend/src/entities/{junction}.rs)
/// 2. Junction migration (backend/migration/src/m{ts}_create_{junction}_table.rs)
/// 3. Registers junction mod in entities/mod.rs
/// 4. Registers junction migration in migration/src/lib.rs
/// 5. Injects Related<T> via junction into both entities (if they exist)
/// 6. Injects M2M handlers + routes into the source entity
/// 7. If target entity exists, injects M2M handlers + routes into target too
pub fn generate(
    source_entity: &str,
    target_entity: &str,
) -> Result<()> {
    let engine = TemplateEngine::new()?;
    let junction = relation::junction_name(source_entity, target_entity);
    let junction_snake = junction.to_snake_case();

    let base = Path::new("backend/src");
    let project_root = Path::new(".");

    // If target entity doesn't exist, store as pending and return
    if !relation::entity_exists(project_root, target_entity) {
        println!(
            "  Target entity '{}' not found — storing pending M2M relation",
            target_entity
        );
        relation::store_pending(
            project_root,
            relation::PendingRelation {
                source_entity: source_entity.to_string(),
                target_entity: target_entity.to_string(),
                relation_type: "ManyToMany".to_string(),
            },
        )?;
        return Ok(());
    }

    // Check if junction already exists (idempotency / circular M2M protection)
    let junction_path = base.join(format!("entities/{}.rs", junction_snake));
    if junction_path.exists() {
        println!("  Junction entity '{}' already exists, skipping", junction);
        // Still inject relations if needed
        inject_m2m_into_entity(&engine, source_entity, target_entity, &junction)?;
        if relation::entity_exists(project_root, target_entity) {
            inject_m2m_into_entity(&engine, target_entity, source_entity, &junction)?;
        }
        return Ok(());
    }

    // Determine alphabetical ordering (junction naming convention)
    let (entity_a, entity_b) = if source_entity.to_snake_case() < target_entity.to_snake_case() {
        (source_entity, target_entity)
    } else {
        (target_entity, source_entity)
    };

    let entity_a_snake = entity_a.to_snake_case();
    let entity_b_snake = entity_b.to_snake_case();

    // Build context for junction templates
    let mut ctx = Context::new();
    ctx.insert("junction_table", &format!("{}_{}", entity_a_snake, entity_b_snake));
    ctx.insert("junction_snake", &junction_snake);
    ctx.insert("junction_iden", &junction.to_pascal_case());
    ctx.insert("entity_a", &entity_a.to_pascal_case());
    ctx.insert("entity_b", &entity_b.to_pascal_case());
    ctx.insert("entity_a_snake", &entity_a_snake);
    ctx.insert("entity_b_snake", &entity_b_snake);
    ctx.insert("entity_a_table", &utils::pluralize(&entity_a_snake));
    ctx.insert("entity_b_table", &utils::pluralize(&entity_b_snake));

    // 1. Generate junction model
    let model_content = engine.render("entity/backend/junction_model.rs.tera", &ctx)?;
    utils::write_file(&junction_path, &model_content)?;

    // 2. Register junction mod in entities/mod.rs
    let entities_mod = base.join("entities/mod.rs");
    utils::insert_at_marker(
        &entities_mod,
        "// === ROMANCE:MODS ===",
        &format!("pub mod {};", junction_snake),
    )?;

    // 3. Generate junction migration
    let timestamp = super::migration::next_timestamp();
    let migration_content = engine.render("entity/backend/junction_migration.rs.tera", &ctx)?;
    let migration_module = format!("m{}_create_{}_table", timestamp, junction_snake);
    let migration_path =
        Path::new("backend/migration/src").join(format!("{}.rs", migration_module));
    utils::write_file(&migration_path, &migration_content)?;

    // 4. Register migration in lib.rs
    let lib_path = Path::new("backend/migration/src/lib.rs");
    utils::insert_at_marker(
        lib_path,
        "// === ROMANCE:MIGRATION_MODS ===",
        &format!("mod {};", migration_module),
    )?;
    utils::insert_at_marker(
        lib_path,
        "// === ROMANCE:MIGRATIONS ===",
        &format!("            Box::new({}::Migration),", migration_module),
    )?;

    // 5. Inject Related<T> via junction + M2M handlers/routes into source entity
    inject_m2m_into_entity(&engine, source_entity, target_entity, &junction)?;

    // 6. If target entity exists, inject reverse M2M into it too
    if relation::entity_exists(project_root, target_entity) {
        inject_m2m_into_entity(&engine, target_entity, source_entity, &junction)?;
    }

    println!(
        "  Generated M2M junction: {} <-> {} (via {})",
        source_entity, target_entity, junction
    );
    Ok(())
}

/// Inject M2M relation code into an entity:
/// - Related<target::Entity> via junction into model
/// - M2M handlers (list/add/remove) into handlers
/// - M2M routes into routes
fn inject_m2m_into_entity(
    _engine: &TemplateEngine,
    entity: &str,
    related: &str,
    junction: &str,
) -> Result<()> {
    let base = Path::new("backend/src");
    let entity_snake = entity.to_snake_case();
    let related_snake = related.to_snake_case();
    let junction_snake = junction.to_snake_case();

    // 1. Inject Related<related::Entity> via junction into entity model
    let model_path = base.join(format!("entities/{}.rs", entity_snake));
    if model_path.exists() {
        let related_impl = format!(
            r#"impl Related<super::{}::Entity> for Entity {{
    fn to() -> RelationDef {{
        super::{}::Relation::{}.def()
    }}
    fn via() -> Option<RelationDef> {{
        Some(super::{}::Relation::{}.def().rev())
    }}
}}"#,
            related_snake,
            junction_snake,
            related.to_pascal_case(),
            junction_snake,
            entity.to_pascal_case(),
        );
        utils::insert_at_marker(
            &model_path,
            "// === ROMANCE:RELATIONS ===",
            &related_impl,
        )?;
    }

    // 2. Inject M2M handlers into entity handlers
    let handlers_path = base.join(format!("handlers/{}.rs", entity_snake));
    if handlers_path.exists() {
        // list handler
        let related_plural = utils::pluralize(&related_snake);
        let list_handler = format!(
            r#"pub async fn list_{related_plural}(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> crate::errors::AppResult<crate::api::ApiResponse<Vec<crate::entities::{related_snake}::Model>>> {{
    use crate::api::ok;

    let entity = crate::entities::{entity_snake}::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or_else(|| crate::errors::AppError::NotFound(format!("{entity} {{}} not found", id)))?;

    let items = entity
        .find_related(crate::entities::{related_snake}::Entity)
        .all(&state.db)
        .await?;
    Ok(ok(items))
}}"#,
            related_plural = related_plural,
            related_snake = related_snake,
            entity_snake = entity_snake,
            entity = entity,
        );
        utils::insert_at_marker(
            &handlers_path,
            "// === ROMANCE:RELATION_HANDLERS ===",
            &list_handler,
        )?;

        // add handler
        let add_handler = format!(
            r#"pub async fn add_{related_snake}(
    State(state): State<AppState>,
    Path((id, {related_snake}_id)): Path<(Uuid, Uuid)>,
) -> crate::errors::AppResult<crate::api::ApiResponse<()>> {{
    use crate::api::ok;

    let junction = crate::entities::{junction_snake}::ActiveModel {{
        id: sea_orm::Set(Uuid::new_v4()),
        {entity_snake}_id: sea_orm::Set(id),
        {related_snake}_id: sea_orm::Set({related_snake}_id),
        created_at: sea_orm::Set(chrono::Utc::now().fixed_offset()),
    }};
    junction.insert(&state.db).await?;
    Ok(ok(()))
}}"#,
            related_snake = related_snake,
            entity_snake = entity_snake,
            junction_snake = junction_snake,
        );
        utils::insert_at_marker(
            &handlers_path,
            "// === ROMANCE:RELATION_HANDLERS ===",
            &add_handler,
        )?;

        // remove handler
        let remove_handler = format!(
            r#"pub async fn remove_{related_snake}(
    State(state): State<AppState>,
    Path((id, {related_snake}_id)): Path<(Uuid, Uuid)>,
) -> crate::errors::AppResult<crate::api::ApiResponse<()>> {{
    use crate::api::ok;

    crate::entities::{junction_snake}::Entity::delete_many()
        .filter(crate::entities::{junction_snake}::Column::{entity_pascal}Id.eq(id))
        .filter(crate::entities::{junction_snake}::Column::{related_pascal}Id.eq({related_snake}_id))
        .exec(&state.db)
        .await?;
    Ok(ok(()))
}}"#,
            related_snake = related_snake,
            junction_snake = junction_snake,
            entity_pascal = entity.to_pascal_case(),
            related_pascal = related.to_pascal_case(),
        );
        utils::insert_at_marker(
            &handlers_path,
            "// === ROMANCE:RELATION_HANDLERS ===",
            &remove_handler,
        )?;
    }

    // 3. Inject M2M routes into entity routes
    let routes_path = base.join(format!("routes/{}.rs", entity_snake));
    if routes_path.exists() {
        let project_root = Path::new(".");
        let config = crate::config::RomanceConfig::load(project_root).ok();
        let api_prefix = config.as_ref()
            .and_then(|c| c.backend.api_prefix.clone())
            .unwrap_or_else(|| "/api".to_string());

        let entity_plural = utils::pluralize(&entity_snake);
        let related_plural = utils::pluralize(&related_snake);

        let list_route = format!(
            "        .route(\"{}/{}/{{id}}/{}\", get({}::list_{}))",
            api_prefix, entity_plural, related_plural, entity_snake, related_plural
        );
        utils::insert_at_marker(
            &routes_path,
            "// === ROMANCE:RELATION_ROUTES ===",
            &list_route,
        )?;

        let add_route = format!(
            "        .route(\"{}/{}/{{id}}/{}/{{{}_id}}\", post({}::add_{}))",
            api_prefix, entity_plural, related_plural, related_snake, entity_snake, related_snake
        );
        utils::insert_at_marker(
            &routes_path,
            "// === ROMANCE:RELATION_ROUTES ===",
            &add_route,
        )?;

        let remove_route = format!(
            "        .route(\"{}/{}/{{id}}/{}/{{{}_id}}\", delete({}::remove_{}))",
            api_prefix, entity_plural, related_plural, related_snake, entity_snake, related_snake
        );
        utils::insert_at_marker(
            &routes_path,
            "// === ROMANCE:RELATION_ROUTES ===",
            &remove_route,
        )?;
    }

    Ok(())
}
