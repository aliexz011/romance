use crate::entity::{EntityDefinition, RelationType};
use crate::generator::context::{self, markers, ProjectFeatures};
use crate::generator::plan::{self, GenerationTracker};
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::path::Path;
use tera::Context;

/// Pre-validate that frontend markers exist in App.tsx.
pub fn validate(_entity: &EntityDefinition) -> Result<()> {
    let app_path = Path::new("frontend/src/App.tsx");
    let checks = vec![
        plan::check(app_path, markers::IMPORTS),
        plan::check(app_path, markers::APP_ROUTES),
        plan::check(app_path, markers::NAV_LINKS),
    ];
    plan::validate_markers(&checks)
}

pub fn generate(entity: &EntityDefinition, tracker: &mut GenerationTracker) -> Result<()> {
    let engine = TemplateEngine::new()?;
    let ctx = build_context(entity);
    let snake_name = entity.name.to_snake_case();
    let camel_name = entity.name.to_lower_camel_case();

    let base = Path::new("frontend/src");
    let feature_dir = base.join(format!("features/{}", camel_name));

    // Types
    let content = engine.render("entity/frontend/types.ts.tera", &ctx)?;
    let types_path = feature_dir.join("types.ts");
    utils::write_generated(&types_path, &content)?;
    tracker.track(types_path);

    // API client
    let content = engine.render("entity/frontend/api.ts.tera", &ctx)?;
    let api_path = feature_dir.join("api.ts");
    utils::write_generated(&api_path, &content)?;
    tracker.track(api_path);

    // Hooks
    let content = engine.render("entity/frontend/hooks.ts.tera", &ctx)?;
    let hooks_path = feature_dir.join("hooks.ts");
    utils::write_generated(&hooks_path, &content)?;
    tracker.track(hooks_path);

    // List component
    let content = engine.render("entity/frontend/List.tsx.tera", &ctx)?;
    let list_path = feature_dir.join(format!("{}List.tsx", entity.name));
    utils::write_generated(&list_path, &content)?;
    tracker.track(list_path);

    // Form component
    let content = engine.render("entity/frontend/Form.tsx.tera", &ctx)?;
    let form_path = feature_dir.join(format!("{}Form.tsx", entity.name));
    utils::write_generated(&form_path, &content)?;
    tracker.track(form_path);

    // Detail component
    let content = engine.render("entity/frontend/Detail.tsx.tera", &ctx)?;
    let detail_path = feature_dir.join(format!("{}Detail.tsx", entity.name));
    utils::write_generated(&detail_path, &content)?;
    tracker.track(detail_path);

    // Inject imports and routes into App.tsx
    let app_path = base.join("App.tsx");
    let entity_pascal = &entity.name;
    let plural = utils::pluralize(&snake_name);

    // Imports
    utils::insert_at_marker(
        &app_path,
        markers::IMPORTS,
        &format!(
            "import {entity_pascal}List from '@/features/{camel_name}/{entity_pascal}List'\nimport {entity_pascal}Form from '@/features/{camel_name}/{entity_pascal}Form'\nimport {entity_pascal}Detail from '@/features/{camel_name}/{entity_pascal}Detail'",
        ),
    )?;

    // Routes
    utils::insert_at_marker(
        &app_path,
        markers::APP_ROUTES,
        &format!(
            "              <Route path=\"/{plural}\" element={{<{entity_pascal}List />}} />\n              <Route path=\"/{plural}/new\" element={{<{entity_pascal}Form />}} />\n              <Route path=\"/{plural}/:id\" element={{<{entity_pascal}Detail />}} />\n              <Route path=\"/{plural}/:id/edit\" element={{<{entity_pascal}Form />}} />",
        ),
    )?;

    // Nav link
    utils::insert_at_marker(
        &app_path,
        markers::NAV_LINKS,
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
            let rel_hooks_path = feature_dir.join(format!("{}_hooks.ts", related_snake));
            utils::write_file(&rel_hooks_path, &content)?;
            tracker.track(rel_hooks_path);
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

    let features = ProjectFeatures::load(Path::new("."));
    ctx.insert("has_validation", &features.has_validation);
    ctx.insert("api_prefix", &features.api_prefix);

    let fields: Vec<serde_json::Value> = entity
        .fields
        .iter()
        .map(|f| {
            let validations = context::validation_rules_to_json(&f.validations);
            let has_validations = !f.validations.is_empty();

            let relation_snake = f.relation.as_ref().map(|r| r.to_snake_case());
            let relation_camel = f.relation.as_ref().map(|r| r.to_lower_camel_case());
            let relation_plural = f.relation.as_ref().map(|r| crate::utils::pluralize(&r.to_snake_case()));

            // Unique variable name for FK options queries (disambiguates multiple FKs to same entity)
            // e.g., sender_id -> senderOptions, receiver_id -> receiverOptions
            let fk_options_var = if f.relation.is_some() {
                let base = f.name.strip_suffix("_id").unwrap_or(&f.name);
                format!("{}Options", base.to_lower_camel_case())
            } else {
                String::new()
            };

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
                "fk_options_var": fk_options_var,
                "validations": validations,
                "has_validations": has_validations,
                "is_numeric": context::is_numeric(&f.field_type),
                "searchable": f.searchable,
                "is_file": matches!(f.field_type, crate::entity::FieldType::File),
                "is_image": matches!(f.field_type, crate::entity::FieldType::Image),
                "filter_method": context::filter_method(&f.field_type),
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
