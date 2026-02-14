use crate::manifest::{FileCategory, Manifest};
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use heck::ToSnakeCase;
use std::path::Path;
use std::process::Command;
use tera::Context;

pub fn create_project(name: &str) -> Result<()> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    println!(
        "{}",
        format!("Creating new Romance project: {}", name).bold()
    );

    let engine = TemplateEngine::new()?;
    let mut ctx = Context::new();
    ctx.insert("project_name", name);
    ctx.insert("project_name_snake", &name.to_snake_case());

    let mut manifest = Manifest::new(name, env!("CARGO_PKG_VERSION"));

    // Backend files
    let backend_files = vec![
        ("scaffold/backend/Cargo.toml.tera", "backend/Cargo.toml"),
        ("scaffold/backend/main.rs.tera", "backend/src/main.rs"),
        ("scaffold/backend/config.rs.tera", "backend/src/config.rs"),
        ("scaffold/backend/db.rs.tera", "backend/src/db.rs"),
        ("scaffold/backend/errors.rs.tera", "backend/src/errors.rs"),
        ("scaffold/backend/api.rs.tera", "backend/src/api.rs"),
        (
            "scaffold/backend/pagination.rs.tera",
            "backend/src/pagination.rs",
        ),
        (
            "scaffold/backend/events.rs.tera",
            "backend/src/events.rs",
        ),
        (
            "scaffold/backend/commands.rs.tera",
            "backend/src/commands.rs",
        ),
        (
            "scaffold/backend/routes.rs.tera",
            "backend/src/routes/mod.rs",
        ),
    ];

    for (template, output) in &backend_files {
        let content = engine.render(template, &ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        let category = if output.contains("mod.rs") {
            FileCategory::Marker
        } else {
            FileCategory::Scaffold
        };
        manifest.record_file(output, Some(template), category, &content, None);
        println!("  {} {}", "create".green(), output);
    }

    // Migration crate
    let migration_files = vec![
        (
            "scaffold/backend/migration/Cargo.toml.tera",
            "backend/migration/Cargo.toml",
        ),
        (
            "scaffold/backend/migration/lib.rs.tera",
            "backend/migration/src/lib.rs",
        ),
        (
            "scaffold/backend/migration/main.rs.tera",
            "backend/migration/src/main.rs",
        ),
    ];

    for (template, output) in &migration_files {
        let content = engine.render(template, &ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        let category = if output.contains("lib.rs") {
            FileCategory::Marker
        } else {
            FileCategory::Scaffold
        };
        manifest.record_file(output, Some(template), category, &content, None);
        println!("  {} {}", "create".green(), output);
    }

    // Backend .env files
    let env_content = engine.render("scaffold/backend/env.example.tera", &ctx)?;
    utils::write_file(&project_dir.join("backend/.env.example"), &env_content)?;
    manifest.record_file(
        "backend/.env.example",
        Some("scaffold/backend/env.example.tera"),
        FileCategory::Scaffold,
        &env_content,
        None,
    );
    println!("  {} backend/.env.example", "create".green());
    utils::write_file(&project_dir.join("backend/.env"), &env_content)?;
    println!("  {} backend/.env", "create".green());

    // Backend stub files
    let entities_mod = "// === ROMANCE:MODS ===\n";
    utils::write_file(
        &project_dir.join("backend/src/entities/mod.rs"),
        entities_mod,
    )?;
    manifest.record_file(
        "backend/src/entities/mod.rs",
        None,
        FileCategory::Marker,
        entities_mod,
        None,
    );
    println!("  {} backend/src/entities/mod.rs", "create".green());

    let handlers_mod = "// === ROMANCE:MODS ===\n";
    utils::write_file(
        &project_dir.join("backend/src/handlers/mod.rs"),
        handlers_mod,
    )?;
    manifest.record_file(
        "backend/src/handlers/mod.rs",
        None,
        FileCategory::Marker,
        handlers_mod,
        None,
    );
    println!("  {} backend/src/handlers/mod.rs", "create".green());

    // Frontend files
    let frontend_files = vec![
        (
            "scaffold/frontend/package.json.tera",
            "frontend/package.json",
        ),
        (
            "scaffold/frontend/vite.config.ts.tera",
            "frontend/vite.config.ts",
        ),
        (
            "scaffold/frontend/tsconfig.json.tera",
            "frontend/tsconfig.json",
        ),
        ("scaffold/frontend/App.tsx.tera", "frontend/src/App.tsx"),
        ("scaffold/frontend/main.tsx.tera", "frontend/src/main.tsx"),
        (
            "scaffold/frontend/lib/utils.ts.tera",
            "frontend/src/lib/utils.ts",
        ),
    ];

    for (template, output) in &frontend_files {
        let content = engine.render(template, &ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        manifest.record_file(output, Some(template), FileCategory::Scaffold, &content, None);
        println!("  {} {}", "create".green(), output);
    }

    // Frontend vite-env.d.ts
    let vite_env_content = engine.render("scaffold/frontend/vite-env.d.ts.tera", &ctx)?;
    utils::write_file(
        &project_dir.join("frontend/src/vite-env.d.ts"),
        &vite_env_content,
    )?;
    manifest.record_file(
        "frontend/src/vite-env.d.ts",
        Some("scaffold/frontend/vite-env.d.ts.tera"),
        FileCategory::Scaffold,
        &vite_env_content,
        None,
    );
    println!("  {} frontend/src/vite-env.d.ts", "create".green());

    // Frontend index.css (shadcn/ui theme with Tailwind v4)
    let index_css = engine.render("scaffold/frontend/index.css.tera", &ctx)?;
    utils::write_file(&project_dir.join("frontend/src/index.css"), &index_css)?;
    manifest.record_file(
        "frontend/src/index.css",
        Some("scaffold/frontend/index.css.tera"),
        FileCategory::Scaffold,
        &index_css,
        None,
    );
    println!("  {} frontend/src/index.css", "create".green());

    // Frontend components.json (shadcn/ui config)
    let components_json = engine.render("scaffold/frontend/components.json.tera", &ctx)?;
    utils::write_file(
        &project_dir.join("frontend/components.json"),
        &components_json,
    )?;
    manifest.record_file(
        "frontend/components.json",
        Some("scaffold/frontend/components.json.tera"),
        FileCategory::Scaffold,
        &components_json,
        None,
    );
    println!("  {} frontend/components.json", "create".green());

    // Frontend index.html
    let index_html = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>{}</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
"#,
        name
    );
    utils::write_file(&project_dir.join("frontend/index.html"), &index_html)?;
    manifest.record_file(
        "frontend/index.html",
        None,
        FileCategory::Static,
        &index_html,
        None,
    );
    println!("  {} frontend/index.html", "create".green());

    // romance.toml
    let content = engine.render("scaffold/romance.toml.tera", &ctx)?;
    utils::write_file(&project_dir.join("romance.toml"), &content)?;
    manifest.record_file(
        "romance.toml",
        Some("scaffold/romance.toml.tera"),
        FileCategory::Scaffold,
        &content,
        None,
    );
    println!("  {} romance.toml", "create".green());

    // romance.production.toml (environment override example)
    let content = engine.render("scaffold/romance.production.toml.tera", &ctx)?;
    utils::write_file(&project_dir.join("romance.production.toml"), &content)?;
    manifest.record_file(
        "romance.production.toml",
        Some("scaffold/romance.production.toml.tera"),
        FileCategory::Scaffold,
        &content,
        None,
    );
    println!("  {} romance.production.toml", "create".green());

    // README
    let content = engine.render("scaffold/README.md.tera", &ctx)?;
    utils::write_file(&project_dir.join("README.md"), &content)?;
    manifest.record_file(
        "README.md",
        Some("scaffold/README.md.tera"),
        FileCategory::Scaffold,
        &content,
        None,
    );
    println!("  {} README.md", "create".green());

    // Docker files (project root)
    let docker_files = vec![
        ("scaffold/docker/Dockerfile.tera", "Dockerfile"),
        (
            "scaffold/docker/docker-compose.yml.tera",
            "docker-compose.yml",
        ),
        (
            "scaffold/docker/Dockerfile.frontend.tera",
            "Dockerfile.frontend",
        ),
        (
            "scaffold/docker/nginx.conf.tera",
            "frontend/nginx.conf",
        ),
        ("scaffold/docker/dockerignore.tera", ".dockerignore"),
    ];

    for (template, output) in &docker_files {
        let content = engine.render(template, &ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        manifest.record_file(output, Some(template), FileCategory::Scaffold, &content, None);
        println!("  {} {}", "create".green(), output);
    }

    // CI files
    let ci_files = vec![
        (
            "scaffold/ci/github-actions.yml.tera",
            ".github/workflows/ci.yml",
        ),
    ];

    for (template, output) in &ci_files {
        let content = engine.render(template, &ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        manifest.record_file(output, Some(template), FileCategory::Scaffold, &content, None);
        println!("  {} {}", "create".green(), output);
    }

    // .gitignore
    let gitignore = "\
/target/
/backend/target/
/backend/migration/target/
/frontend/node_modules/
/frontend/dist/
*.env
!*.env.example
";
    utils::write_file(&project_dir.join(".gitignore"), gitignore)?;
    manifest.record_file(".gitignore", None, FileCategory::Static, gitignore, None);
    println!("  {} .gitignore", "create".green());

    // Save manifest
    manifest.save(project_dir)?;
    println!("  {} .romance/manifest.json", "create".green());

    // Generate project-level CLAUDE.md for AI assistants
    crate::ai_context::regenerate(project_dir)?;

    // Install frontend dependencies and shadcn/ui components
    let frontend_dir = project_dir.join("frontend");
    println!();
    println!(
        "{}",
        "Installing frontend dependencies...".cyan().bold()
    );
    let npm_status = Command::new("npm")
        .args(["install"])
        .current_dir(&frontend_dir)
        .status();

    match npm_status {
        Ok(status) if status.success() => {
            println!("  {} npm install", "done".green());

            // Install ALL shadcn/ui components
            println!(
                "{}",
                "Installing shadcn/ui components...".cyan().bold()
            );
            let shadcn_status = Command::new("npx")
                .args(["shadcn@latest", "add", "--all", "--yes", "--overwrite"])
                .current_dir(&frontend_dir)
                .status();

            match shadcn_status {
                Ok(status) if status.success() => {
                    println!("  {} shadcn/ui components", "done".green());
                }
                _ => {
                    println!(
                        "  {} Failed to install shadcn/ui components. Run manually:",
                        "warn".yellow()
                    );
                    println!("    cd {}/frontend && npx shadcn@latest add --all --yes", name);
                }
            }
        }
        _ => {
            println!(
                "  {} Failed to install npm dependencies. Run manually:",
                "warn".yellow()
            );
            println!("    cd {}/frontend && npm install", name);
            println!("    npx shadcn@latest add --all --yes");
        }
    }

    println!();
    println!("{}", "Project created successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  cd backend && cargo build");
    println!("  romance dev");

    Ok(())
}
