use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct I18nAddon;

impl Addon for I18nAddon {
    fn name(&self) -> &str {
        "i18n"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/i18n.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_i18n(project_root)
    }
}

fn install_i18n(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing i18n (internationalization)...".bold());

    let engine = TemplateEngine::new()?;
    let ctx = Context::new();

    // Generate backend i18n module
    let content = engine.render("addon/i18n/i18n.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/i18n.rs"), &content)?;
    println!("  {} backend/src/i18n.rs", "create".green());

    // Generate English locale file
    let content = engine.render("addon/i18n/en.json.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/locales/en.json"), &content)?;
    println!("  {} backend/locales/en.json", "create".green());

    // Generate Russian locale file
    let content = engine.render("addon/i18n/ru.json.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/locales/ru.json"), &content)?;
    println!("  {} backend/locales/ru.json", "create".green());

    // Generate frontend i18n module
    let content = engine.render("addon/i18n/i18n_frontend.ts.tera", &ctx)?;
    utils::write_file(&project_root.join("frontend/src/lib/i18n.ts"), &content)?;
    println!("  {} frontend/src/lib/i18n.ts", "create".green());

    // Add mod i18n to main.rs
    super::add_mod_to_main(project_root, "i18n")?;

    // Inject Accept-Language middleware via ROMANCE:MIDDLEWARE marker
    utils::insert_at_marker(
        &project_root.join("backend/src/routes/mod.rs"),
        "// === ROMANCE:MIDDLEWARE ===",
        "        .layer(axum::middleware::from_fn(crate::i18n::locale_middleware))",
    )?;

    // Add serde_json dependency if not present (should already be there)
    crate::generator::auth::insert_cargo_dependency(
        &project_root.join("backend/Cargo.toml"),
        &[("serde_json", r#""1""#)],
    )?;

    // Update romance.toml with i18n feature
    super::update_feature_flag(project_root, "i18n", true)?;

    println!();
    println!(
        "{}",
        "i18n (internationalization) installed successfully!"
            .green()
            .bold()
    );
    println!("  Locale files: backend/locales/en.json, backend/locales/ru.json");
    println!("  Backend usage: i18n::t(\"en\", \"common.success\")");
    println!("  Frontend usage: import {{ t }} from '@/lib/i18n'");
    println!("  The Accept-Language middleware extracts locale from request headers.");
    println!("  Access locale in handlers via: request.extensions().get::<i18n::Locale>()");

    Ok(())
}
