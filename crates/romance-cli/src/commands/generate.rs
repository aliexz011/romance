use anyhow::Result;
use colored::Colorize;

pub fn run_entity(name: &str, fields: &[String]) -> Result<()> {
    let entity = if fields.is_empty() {
        let (prompted_fields, prompted_relations) = romance_core::entity::prompt_entity_fields(name)?;
        romance_core::entity::EntityDefinition {
            name: name.to_string(),
            fields: prompted_fields,
            relations: prompted_relations,
        }
    } else {
        romance_core::entity::parse_entity(name, fields)?
    };

    // Check prerequisites and print warnings before generation
    let project_root = std::path::Path::new(".");
    let warnings = romance_core::generator::check_entity_prerequisites(&entity, project_root);
    for warning in &warnings {
        eprintln!("  {} {}", "warn".yellow(), warning);
    }

    romance_core::generator::backend::generate(&entity)?;
    romance_core::generator::migration::generate(&entity)?;
    romance_core::generator::backend::generate_relations(&entity)?;
    romance_core::generator::frontend::generate(&entity)?;

    // Regenerate AI context with updated schema
    romance_core::ai_context::regenerate(project_root)?;

    println!("Entity '{}' generated successfully!", name);
    Ok(())
}

pub fn run_types() -> Result<()> {
    romance_core::generator::types::generate()
}

pub fn run_openapi() -> Result<()> {
    romance_core::generator::openapi::generate()
}

pub fn run_auth() -> Result<()> {
    romance_core::generator::auth::generate()?;
    let project_root = std::path::Path::new(".");
    romance_core::ai_context::regenerate(project_root)?;
    Ok(())
}

pub fn run_admin() -> Result<()> {
    romance_core::generator::admin::generate()?;
    let project_root = std::path::Path::new(".");
    romance_core::ai_context::regenerate(project_root)?;
    Ok(())
}
