use crate::entity::{EntityDefinition, RelationType, ValidationRule};
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::path::Path;
use tera::Context;

pub fn generate(entity: &EntityDefinition) -> Result<()> {
    let engine = TemplateEngine::new()?;
    let ctx = build_context(entity);
    let snake_name = entity.name.to_snake_case();
    let camel_name = entity.name.to_lower_camel_case();

    let base = Path::new("frontend/src");
    let feature_dir = base.join(format!("features/{}", camel_name));

    // Types
    let content = engine.render("entity/frontend/types.ts.tera", &ctx)?;
    utils::write_generated(&feature_dir.join("types.ts"), &content)?;

    // API client
    let content = engine.render("entity/frontend/api.ts.tera", &ctx)?;
    utils::write_generated(&feature_dir.join("api.ts"), &content)?;

    // Hooks
    let content = engine.render("entity/frontend/hooks.ts.tera", &ctx)?;
    utils::write_generated(&feature_dir.join("hooks.ts"), &content)?;

    // List component
    let content = engine.render("entity/frontend/List.tsx.tera", &ctx)?;
    utils::write_generated(&feature_dir.join(format!("{}List.tsx", entity.name)), &content)?;

    // Form component
    let content = engine.render("entity/frontend/Form.tsx.tera", &ctx)?;
    utils::write_generated(&feature_dir.join(format!("{}Form.tsx", entity.name)), &content)?;

    // Detail component
    let content = engine.render("entity/frontend/Detail.tsx.tera", &ctx)?;
    utils::write_generated(&feature_dir.join(format!("{}Detail.tsx", entity.name)), &content)?;

    // Inject imports and routes into App.tsx
    let app_path = base.join("App.tsx");
    let entity_pascal = &entity.name;
    let plural = utils::pluralize(&snake_name);

    // Imports
    utils::insert_at_marker(
        &app_path,
        "// === ROMANCE:IMPORTS ===",
        &format!(
            "import {entity_pascal}List from '@/features/{camel_name}/{entity_pascal}List'\nimport {entity_pascal}Form from '@/features/{camel_name}/{entity_pascal}Form'\nimport {entity_pascal}Detail from '@/features/{camel_name}/{entity_pascal}Detail'",
        ),
    )?;

    // Routes
    utils::insert_at_marker(
        &app_path,
        "{/* === ROMANCE:APP_ROUTES === */}",
        &format!(
            "              <Route path=\"/{plural}\" element={{<{entity_pascal}List />}} />\n              <Route path=\"/{plural}/new\" element={{<{entity_pascal}Form />}} />\n              <Route path=\"/{plural}/:id\" element={{<{entity_pascal}Detail />}} />\n              <Route path=\"/{plural}/:id/edit\" element={{<{entity_pascal}Form />}} />",
        ),
    )?;

    // Nav link
    utils::insert_at_marker(
        &app_path,
        "{/* === ROMANCE:NAV_LINKS === */}",
        &format!(
            "                <Link to=\"/{plural}\" className=\"text-muted-foreground hover:text-foreground transition-colors\">{entity_pascal}</Link>",
            plural = plural,
            entity_pascal = entity_pascal,
        ),
    )?;

    // Generate M2M relation hooks for each ManyToMany relation
    for rel in &entity.relations {
        if rel.relation_type == RelationType::ManyToMany {
            let rel_ctx = build_relation_context(&entity.name, &rel.target_entity);
            let content = engine.render("entity/frontend/relation_hooks.ts.tera", &rel_ctx)?;
            let related_snake = rel.target_entity.to_snake_case();
            utils::write_file(
                &feature_dir.join(format!("{}_hooks.ts", related_snake)),
                &content,
            )?;
        }
    }

    utils::ui::success(&format!(
        "Generated frontend files for '{}' in features/{}",
        entity.name, snake_name
    ));
    Ok(())
}

fn build_relation_context(entity_name: &str, related_name: &str) -> Context {
    let mut ctx = Context::new();
    ctx.insert("entity_name", &entity_name.to_pascal_case());
    ctx.insert("entity_name_snake", &entity_name.to_snake_case());
    ctx.insert("entity_name_camel", &entity_name.to_lower_camel_case());
    ctx.insert("related_name", &related_name.to_pascal_case());
    ctx.insert("related_snake", &related_name.to_snake_case());
    ctx.insert("related_camel", &related_name.to_lower_camel_case());
    ctx
}

fn build_context(entity: &EntityDefinition) -> Context {
    let mut ctx = Context::new();
    ctx.insert("entity_name", &entity.name);
    ctx.insert("entity_name_snake", &entity.name.to_snake_case());
    ctx.insert("entity_name_camel", &entity.name.to_lower_camel_case());

    // Check project-level features
    let project_root = Path::new(".");
    let config = crate::config::RomanceConfig::load(project_root).ok();
    let has_validation = config.as_ref().map(|c| c.has_feature("validation")).unwrap_or(false);

    ctx.insert("has_validation", &has_validation);

    let api_prefix = config.as_ref()
        .and_then(|c| c.backend.api_prefix.clone())
        .unwrap_or_else(|| "/api".to_string());
    ctx.insert("api_prefix", &api_prefix);

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

            let relation_snake = f.relation.as_ref().map(|r| r.to_snake_case());
            let relation_camel = f.relation.as_ref().map(|r| r.to_lower_camel_case());
            let relation_plural = f.relation.as_ref().map(|r| crate::utils::pluralize(&r.to_snake_case()));

            // Smart input type: use field name hints for better HTML input types
            let input_type = if f.name.contains("email") {
                "email"
            } else if f.name.contains("url") || f.name.contains("website") || f.name.contains("link") {
                "url"
            } else if f.name.contains("phone") || f.name.contains("tel") {
                "tel"
            } else if f.name.contains("password") || f.name.contains("secret") {
                "password"
            } else {
                f.field_type.input_type()
            };

            // Determine filter method for List component filter inputs
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

            serde_json::json!({
                "name": f.name,
                "ts_type": f.field_type.to_typescript(),
                "shadcn_component": f.field_type.to_shadcn(),
                "input_type": input_type,
                "optional": f.optional,
                "relation": f.relation,
                "relation_snake": relation_snake,
                "relation_camel": relation_camel,
                "relation_plural": relation_plural,
                "validations": validations,
                "has_validations": has_validations,
                "is_numeric": is_numeric,
                "searchable": f.searchable,
                "is_file": matches!(f.field_type, crate::entity::FieldType::File),
                "is_image": matches!(f.field_type, crate::entity::FieldType::Image),
                "filter_method": filter_method,
            })
        })
        .collect();

    ctx.insert("fields", &fields);

    // Check if any fields have FK relations (for conditional imports in forms)
    let has_fk_fields = entity.fields.iter().any(|f| f.relation.is_some());
    ctx.insert("has_fk_fields", &has_fk_fields);

    // Check if entity has a "status" field (for conditional Badge import)
    let has_status_field = entity.fields.iter().any(|f| f.name == "status");
    ctx.insert("has_status_field", &has_status_field);

    // Check if entity has textarea fields
    let has_textarea_field = entity
        .fields
        .iter()
        .any(|f| f.field_type.to_shadcn() == "Textarea");
    ctx.insert("has_textarea_field", &has_textarea_field);

    // Build relation arrays for templates
    let m2m_relations: Vec<serde_json::Value> = entity
        .relations
        .iter()
        .filter(|r| r.relation_type == RelationType::ManyToMany)
        .map(|r| {
            serde_json::json!({
                "target": r.target_entity,
                "target_snake": r.target_entity.to_snake_case(),
                "target_camel": r.target_entity.to_lower_camel_case(),
            })
        })
        .collect();
    ctx.insert("m2m_relations", &m2m_relations);

    let has_many_relations: Vec<serde_json::Value> = entity
        .relations
        .iter()
        .filter(|r| r.relation_type == RelationType::HasMany)
        .map(|r| {
            serde_json::json!({
                "target": r.target_entity,
                "target_snake": r.target_entity.to_snake_case(),
                "target_camel": r.target_entity.to_lower_camel_case(),
            })
        })
        .collect();
    ctx.insert("has_many_relations", &has_many_relations);

    // Track if any field has validation rules (for conditional Zod schema generation)
    let has_any_validations = entity.fields.iter().any(|f| !f.validations.is_empty());
    ctx.insert("has_any_validations", &has_any_validations);

    ctx
}
