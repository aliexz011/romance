use crate::addon::Addon;
use anyhow::Result;
use std::path::Path;

pub struct TasksAddon;

impl Addon for TasksAddon {
    fn name(&self) -> &str {
        "tasks"
    }

    fn check_prerequisites(&self, project_root: &Path) -> Result<()> {
        super::check_romance_project(project_root)
    }

    fn is_already_installed(&self, project_root: &Path) -> bool {
        project_root.join("backend/src/tasks.rs").exists()
    }

    fn install(&self, project_root: &Path) -> Result<()> {
        install_tasks(project_root)
    }

    fn uninstall(&self, project_root: &Path) -> Result<()> {
        use colored::Colorize;

        println!("{}", "Uninstalling background tasks...".bold());

        // Delete files
        if super::remove_file_if_exists(&project_root.join("backend/src/tasks.rs"))? {
            println!("  {} backend/src/tasks.rs", "delete".red());
        }
        if super::remove_file_if_exists(&project_root.join("backend/src/scheduler.rs"))? {
            println!("  {} backend/src/scheduler.rs", "delete".red());
        }
        if super::remove_file_if_exists(
            &project_root.join("backend/src/entities/background_task.rs"),
        )? {
            println!(
                "  {} backend/src/entities/background_task.rs",
                "delete".red()
            );
        }

        // Remove mod declarations from main.rs
        super::remove_mod_from_main(project_root, "tasks")?;
        super::remove_mod_from_main(project_root, "scheduler")?;

        // Remove from entities/mod.rs
        super::remove_line_from_file(
            &project_root.join("backend/src/entities/mod.rs"),
            "pub mod background_task;",
        )?;

        // Remove feature flag
        super::remove_feature_flag(project_root, "background_tasks")?;

        // Regenerate AI context
        crate::ai_context::regenerate(project_root).ok();

        println!();
        println!(
            "{}",
            "Background tasks uninstalled successfully.".green().bold()
        );

        Ok(())
    }
}

fn install_tasks(project_root: &Path) -> Result<()> {
    use crate::template::TemplateEngine;
    use crate::utils;
    use colored::Colorize;
    use tera::Context;

    println!("{}", "Installing background tasks...".bold());

    let engine = TemplateEngine::new()?;
    let timestamp = crate::generator::migration::next_timestamp();

    let mut ctx = Context::new();
    ctx.insert("timestamp", &timestamp);

    // Generate tasks module
    let content = engine.render("addon/tasks/tasks.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/tasks.rs"), &content)?;
    println!("  {} backend/src/tasks.rs", "create".green());

    // Generate background_task entity model
    let content = engine.render("addon/tasks/model.rs.tera", &ctx)?;
    utils::write_file(
        &project_root.join("backend/src/entities/background_task.rs"),
        &content,
    )?;
    println!(
        "  {} backend/src/entities/background_task.rs",
        "create".green()
    );

    // Generate migration
    let content = engine.render("addon/tasks/migration.rs.tera", &ctx)?;
    let migration_module = format!("m{}_create_background_tasks_table", timestamp);
    utils::write_file(
        &project_root.join(format!("backend/migration/src/{}.rs", migration_module)),
        &content,
    )?;
    println!(
        "  {} backend/migration/src/{}.rs",
        "create".green(),
        migration_module
    );

    // Register entity module
    let mods_marker = "// === ROMANCE:MODS ===";
    utils::insert_at_marker(
        &project_root.join("backend/src/entities/mod.rs"),
        mods_marker,
        "pub mod background_task;",
    )?;

    // Register migration
    let lib_path = project_root.join("backend/migration/src/lib.rs");
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATION_MODS ===",
        &format!("mod {};", migration_module),
    )?;
    utils::insert_at_marker(
        &lib_path,
        "// === ROMANCE:MIGRATIONS ===",
        &format!("            Box::new({}::Migration),", migration_module),
    )?;

    // Generate scheduler module
    let content = engine.render("addon/tasks/scheduler.rs.tera", &ctx)?;
    utils::write_file(&project_root.join("backend/src/scheduler.rs"), &content)?;
    println!("  {} backend/src/scheduler.rs", "create".green());

    // Add mod tasks and mod scheduler to main.rs
    super::add_mod_to_main(project_root, "tasks")?;
    super::add_mod_to_main(project_root, "scheduler")?;

    // Update romance.toml
    super::update_feature_flag(project_root, "background_tasks", true)?;

    println!();
    println!(
        "{}",
        "Background tasks installed successfully!".green().bold()
    );
    println!("  Run migrations: romance db migrate");
    println!("  Enqueue tasks: TaskQueue::new(db).enqueue(\"send_email\", payload).await?");
    println!("  Start worker: TaskQueue::new(db).start_worker(4, handler).await");
    println!();
    println!("  Scheduler (recurring jobs):");
    println!("    let mut scheduler = scheduler::Scheduler::new();");
    println!("    scheduler.add_job(\"cleanup\", Duration::from_secs(3600), || {{");
    println!("        tokio::spawn(async {{ /* ... */ }})");
    println!("    }});");
    println!("    scheduler.start();");

    Ok(())
}
