use crate::generator::auth::generate_jwt_secret;
use crate::manifest::{FileCategory, Manifest};
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use colored::Colorize;
use heck::ToSnakeCase;
use std::path::Path;
use std::process::Command;
use tera::Context;

/// A rendered file entry: (output_path, template_name_or_none, category, content).
struct RenderedFile {
    output: String,
    template: Option<String>,
    category: FileCategory,
    content: String,
}

/// Render backend source templates (Cargo.toml, main.rs, config.rs, etc.).
fn render_backend_files(
    engine: &TemplateEngine,
    ctx: &Context,
    project_dir: &Path,
) -> Result<Vec<RenderedFile>> {
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

    let mut rendered = Vec::new();
    for (template, output) in &backend_files {
        let content = engine.render(template, ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        let category = if output.contains("mod.rs") {
            FileCategory::Marker
        } else {
            FileCategory::Scaffold
        };
        rendered.push(RenderedFile {
            output: output.to_string(),
            template: Some(template.to_string()),
            category,
            content,
        });
        println!("  {} {}", "create".green(), output);
    }

    // Backend .env files
    let env_content = engine.render("scaffold/backend/env.example.tera", ctx)?;
    utils::write_file(&project_dir.join("backend/.env.example"), &env_content)?;
    rendered.push(RenderedFile {
        output: "backend/.env.example".to_string(),
        template: Some("scaffold/backend/env.example.tera".to_string()),
        category: FileCategory::Scaffold,
        content: env_content.clone(),
    });
    println!("  {} backend/.env.example", "create".green());
    utils::write_file(&project_dir.join("backend/.env"), &env_content)?;
    println!("  {} backend/.env", "create".green());

    Ok(rendered)
}

/// Render migration crate templates (Cargo.toml, lib.rs, main.rs).
fn render_migration_files(
    engine: &TemplateEngine,
    ctx: &Context,
    project_dir: &Path,
) -> Result<Vec<RenderedFile>> {
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

    let mut rendered = Vec::new();
    for (template, output) in &migration_files {
        let content = engine.render(template, ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        let category = if output.contains("lib.rs") {
            FileCategory::Marker
        } else {
            FileCategory::Scaffold
        };
        rendered.push(RenderedFile {
            output: output.to_string(),
            template: Some(template.to_string()),
            category,
            content,
        });
        println!("  {} {}", "create".green(), output);
    }

    Ok(rendered)
}

/// Render frontend templates (package.json, vite config, App.tsx, etc.).
fn render_frontend_files(
    engine: &TemplateEngine,
    ctx: &Context,
    project_dir: &Path,
    name: &str,
) -> Result<Vec<RenderedFile>> {
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
        (
            "scaffold/frontend/components/AppSidebar.tsx.tera",
            "frontend/src/components/AppSidebar.tsx",
        ),
    ];

    let mut rendered = Vec::new();
    for (template, output) in &frontend_files {
        let content = engine.render(template, ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        rendered.push(RenderedFile {
            output: output.to_string(),
            template: Some(template.to_string()),
            category: FileCategory::Scaffold,
            content,
        });
        println!("  {} {}", "create".green(), output);
    }

    // Frontend vite-env.d.ts
    let vite_env_content = engine.render("scaffold/frontend/vite-env.d.ts.tera", ctx)?;
    utils::write_file(
        &project_dir.join("frontend/src/vite-env.d.ts"),
        &vite_env_content,
    )?;
    rendered.push(RenderedFile {
        output: "frontend/src/vite-env.d.ts".to_string(),
        template: Some("scaffold/frontend/vite-env.d.ts.tera".to_string()),
        category: FileCategory::Scaffold,
        content: vite_env_content,
    });
    println!("  {} frontend/src/vite-env.d.ts", "create".green());

    // Frontend index.css (shadcn/ui theme with Tailwind v4)
    let index_css = engine.render("scaffold/frontend/index.css.tera", ctx)?;
    utils::write_file(&project_dir.join("frontend/src/index.css"), &index_css)?;
    rendered.push(RenderedFile {
        output: "frontend/src/index.css".to_string(),
        template: Some("scaffold/frontend/index.css.tera".to_string()),
        category: FileCategory::Scaffold,
        content: index_css,
    });
    println!("  {} frontend/src/index.css", "create".green());

    // Frontend components.json (shadcn/ui config)
    let components_json = engine.render("scaffold/frontend/components.json.tera", ctx)?;
    utils::write_file(
        &project_dir.join("frontend/components.json"),
        &components_json,
    )?;
    rendered.push(RenderedFile {
        output: "frontend/components.json".to_string(),
        template: Some("scaffold/frontend/components.json.tera".to_string()),
        category: FileCategory::Scaffold,
        content: components_json,
    });
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
    rendered.push(RenderedFile {
        output: "frontend/index.html".to_string(),
        template: None,
        category: FileCategory::Static,
        content: index_html,
    });
    println!("  {} frontend/index.html", "create".green());

    // Static component files (not templated)
    let static_components = vec![
        (
            "scaffold/frontend/components/ThemeProvider.tsx",
            "frontend/src/components/ThemeProvider.tsx",
        ),
        (
            "scaffold/frontend/components/ThemeToggle.tsx",
            "frontend/src/components/ThemeToggle.tsx",
        ),
    ];

    for (embedded_path, output) in &static_components {
        let content = engine.get_raw(embedded_path)?;
        utils::write_file(&project_dir.join(output), &content)?;
        rendered.push(RenderedFile {
            output: output.to_string(),
            template: None,
            category: FileCategory::Scaffold,
            content,
        });
        println!("  {} {}", "create".green(), output);
    }

    Ok(rendered)
}

/// Render Docker, CI, and project root config templates.
fn render_docker_files(
    engine: &TemplateEngine,
    ctx: &Context,
    project_dir: &Path,
) -> Result<Vec<RenderedFile>> {
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

    let mut rendered = Vec::new();
    for (template, output) in &docker_files {
        let content = engine.render(template, ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        rendered.push(RenderedFile {
            output: output.to_string(),
            template: Some(template.to_string()),
            category: FileCategory::Scaffold,
            content,
        });
        println!("  {} {}", "create".green(), output);
    }

    // CI files
    let ci_files = vec![(
        "scaffold/ci/github-actions.yml.tera",
        ".github/workflows/ci.yml",
    )];

    for (template, output) in &ci_files {
        let content = engine.render(template, ctx)?;
        let path = project_dir.join(output);
        utils::write_file(&path, &content)?;
        rendered.push(RenderedFile {
            output: output.to_string(),
            template: Some(template.to_string()),
            category: FileCategory::Scaffold,
            content,
        });
        println!("  {} {}", "create".green(), output);
    }

    // romance.toml
    let content = engine.render("scaffold/romance.toml.tera", ctx)?;
    utils::write_file(&project_dir.join("romance.toml"), &content)?;
    rendered.push(RenderedFile {
        output: "romance.toml".to_string(),
        template: Some("scaffold/romance.toml.tera".to_string()),
        category: FileCategory::Scaffold,
        content,
    });
    println!("  {} romance.toml", "create".green());

    // romance.production.toml (environment override example)
    let content = engine.render("scaffold/romance.production.toml.tera", ctx)?;
    utils::write_file(&project_dir.join("romance.production.toml"), &content)?;
    rendered.push(RenderedFile {
        output: "romance.production.toml".to_string(),
        template: Some("scaffold/romance.production.toml.tera".to_string()),
        category: FileCategory::Scaffold,
        content,
    });
    println!("  {} romance.production.toml", "create".green());

    // README
    let content = engine.render("scaffold/README.md.tera", ctx)?;
    utils::write_file(&project_dir.join("README.md"), &content)?;
    rendered.push(RenderedFile {
        output: "README.md".to_string(),
        template: Some("scaffold/README.md.tera".to_string()),
        category: FileCategory::Scaffold,
        content,
    });
    println!("  {} README.md", "create".green());

    Ok(rendered)
}

/// Create non-template stub files (mod.rs markers, .gitignore).
fn create_stub_files(project_dir: &Path) -> Result<Vec<RenderedFile>> {
    let mut rendered = Vec::new();

    // Backend stub files
    let entities_mod = "// === ROMANCE:MODS ===\n";
    utils::write_file(
        &project_dir.join("backend/src/entities/mod.rs"),
        entities_mod,
    )?;
    rendered.push(RenderedFile {
        output: "backend/src/entities/mod.rs".to_string(),
        template: None,
        category: FileCategory::Marker,
        content: entities_mod.to_string(),
    });
    println!("  {} backend/src/entities/mod.rs", "create".green());

    let handlers_mod = "// === ROMANCE:MODS ===\n";
    utils::write_file(
        &project_dir.join("backend/src/handlers/mod.rs"),
        handlers_mod,
    )?;
    rendered.push(RenderedFile {
        output: "backend/src/handlers/mod.rs".to_string(),
        template: None,
        category: FileCategory::Marker,
        content: handlers_mod.to_string(),
    });
    println!("  {} backend/src/handlers/mod.rs", "create".green());

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
    rendered.push(RenderedFile {
        output: ".gitignore".to_string(),
        template: None,
        category: FileCategory::Static,
        content: gitignore.to_string(),
    });
    println!("  {} .gitignore", "create".green());

    Ok(rendered)
}

/// Install frontend npm dependencies and shadcn/ui components.
fn install_frontend_deps(project_dir: &Path, name: &str) -> Result<()> {
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

    Ok(())
}

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
    ctx.insert("jwt_secret", &generate_jwt_secret());

    let mut manifest = Manifest::new(name, env!("CARGO_PKG_VERSION"));

    // Render all template groups
    let backend = render_backend_files(&engine, &ctx, project_dir)?;
    let migration = render_migration_files(&engine, &ctx, project_dir)?;
    let frontend = render_frontend_files(&engine, &ctx, project_dir, name)?;
    let docker = render_docker_files(&engine, &ctx, project_dir)?;
    let stubs = create_stub_files(project_dir)?;

    // Record all rendered files in the manifest
    for file in backend
        .iter()
        .chain(migration.iter())
        .chain(frontend.iter())
        .chain(docker.iter())
        .chain(stubs.iter())
    {
        manifest.record_file(
            &file.output,
            file.template.as_deref(),
            file.category.clone(),
            &file.content,
            None,
        );
    }

    // Save manifest
    manifest.save(project_dir)?;
    println!("  {} .romance/manifest.json", "create".green());

    // Generate project-level CLAUDE.md for AI assistants
    crate::ai_context::regenerate(project_dir)?;

    // Install frontend dependencies and shadcn/ui components
    install_frontend_deps(project_dir, name)?;

    println!();
    println!("{}", "Project created successfully!".green().bold());
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  cd backend && cargo build");
    println!("  romance dev");

    Ok(())
}
