use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use tera::Context;

/// Generate the initial seed.rs file in the backend project.
pub fn generate_seed_file(project_root: &Path) -> Result<()> {
    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    let seed_path = project_root.join("backend/src/seed.rs");
    if seed_path.exists() {
        return Ok(());
    }

    let content = engine.render("addon/seed/seed.rs.tera", &ctx)?;
    utils::write_file(&seed_path, &content)?;
    println!("  {} backend/src/seed.rs", "create".green());

    // Add mod seed to main.rs
    crate::addon::add_mod_to_main(project_root, "seed")?;

    // Add fake crate dependency
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[("fake", r#"{ version = "3", features = ["derive", "uuid", "chrono"] }"#),
          ("rand", r#""0.8""#)],
    )?;

    Ok(())
}
