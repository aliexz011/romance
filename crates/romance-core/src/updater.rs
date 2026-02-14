use crate::manifest::{content_hash, FileCategory, Manifest};
use crate::template::TemplateEngine;
use crate::utils;
use anyhow::Result;
use heck::ToSnakeCase;
use std::path::Path;
use tera::Context;

/// Describes one file's update status.
#[derive(Debug)]
pub struct UpdateItem {
    pub output_path: String,
    pub template_path: String,
    pub new_content: String,
    pub old_generated_hash: String,
    pub user_modified: bool,
    pub template_changed: bool,
    pub current_content: Option<String>,
}

/// Result of scanning the project for updatable files.
#[derive(Debug)]
pub struct UpdatePlan {
    pub auto_update: Vec<UpdateItem>,
    pub conflicts: Vec<UpdateItem>,
    pub unchanged: Vec<UpdateItem>,
    pub new_files: Vec<UpdateItem>,
    pub deleted: Vec<String>,
}

/// Files safe for automatic update (not user-edited config, not marker-managed).
pub fn get_updatable_scaffold_mappings() -> Vec<(&'static str, &'static str)> {
    vec![
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
            "scaffold/backend/env.example.tera",
            "backend/.env.example",
        ),
        (
            "scaffold/backend/migration/main.rs.tera",
            "backend/migration/src/main.rs",
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
            "scaffold/frontend/vite-env.d.ts.tera",
            "frontend/src/vite-env.d.ts",
        ),
        ("scaffold/README.md.tera", "README.md"),
        // Docker files
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
        // CI files
        (
            "scaffold/ci/github-actions.yml.tera",
            ".github/workflows/ci.yml",
        ),
    ]
}

/// All scaffold mappings (used for manifest recording).
pub fn get_scaffold_mappings() -> Vec<(&'static str, &'static str)> {
    vec![
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
        (
            "scaffold/backend/env.example.tera",
            "backend/.env.example",
        ),
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
            "scaffold/frontend/vite-env.d.ts.tera",
            "frontend/src/vite-env.d.ts",
        ),
        ("scaffold/romance.toml.tera", "romance.toml"),
        ("scaffold/README.md.tera", "README.md"),
        // Docker files
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
        // CI files
        (
            "scaffold/ci/github-actions.yml.tera",
            ".github/workflows/ci.yml",
        ),
    ]
}

/// Scan the project and build an update plan for scaffold files.
pub fn plan_update(project_dir: &Path) -> Result<UpdatePlan> {
    let manifest = Manifest::load(project_dir)?;
    let config = crate::config::RomanceConfig::load(project_dir)?;
    let engine = TemplateEngine::new()?;

    let mut ctx = Context::new();
    ctx.insert("project_name", &config.project.name);
    ctx.insert("project_name_snake", &config.project.name.to_snake_case());

    let mut plan = UpdatePlan {
        auto_update: vec![],
        conflicts: vec![],
        unchanged: vec![],
        new_files: vec![],
        deleted: vec![],
    };

    let mappings = get_updatable_scaffold_mappings();

    for (template_path, output_path) in &mappings {
        let new_content = engine.render(template_path, &ctx)?;
        let new_hash = content_hash(&new_content);

        let full_path = project_dir.join(output_path);
        let current_content = std::fs::read_to_string(&full_path).ok();

        if let Some(record) = manifest.files.get(*output_path) {
            let template_changed = new_hash != record.generated_hash;
            let user_modified = match &current_content {
                Some(content) => content_hash(content) != record.generated_hash,
                None => true,
            };

            if current_content.is_none() {
                plan.deleted.push(output_path.to_string());
                continue;
            }

            let item = UpdateItem {
                output_path: output_path.to_string(),
                template_path: template_path.to_string(),
                new_content,
                old_generated_hash: record.generated_hash.clone(),
                user_modified,
                template_changed,
                current_content,
            };

            if !template_changed {
                plan.unchanged.push(item);
            } else if !user_modified {
                plan.auto_update.push(item);
            } else {
                plan.conflicts.push(item);
            }
        } else {
            plan.new_files.push(UpdateItem {
                output_path: output_path.to_string(),
                template_path: template_path.to_string(),
                new_content,
                old_generated_hash: String::new(),
                user_modified: false,
                template_changed: true,
                current_content,
            });
        }
    }

    Ok(plan)
}

/// Apply an update: write new content and update manifest record.
pub fn apply_update(
    project_dir: &Path,
    manifest: &mut Manifest,
    item: &UpdateItem,
) -> Result<()> {
    let full_path = project_dir.join(&item.output_path);
    utils::write_file(&full_path, &item.new_content)?;
    manifest.record_file(
        &item.output_path,
        Some(&item.template_path),
        FileCategory::Scaffold,
        &item.new_content,
        None,
    );
    Ok(())
}

/// Generate a unified diff between two strings.
pub fn generate_diff(old: &str, new: &str, path: &str) -> String {
    use similar::TextDiff;
    let diff = TextDiff::from_lines(old, new);
    let mut output = format!("--- a/{}\n+++ b/{}\n", path, path);
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{}", hunk));
    }
    output
}
