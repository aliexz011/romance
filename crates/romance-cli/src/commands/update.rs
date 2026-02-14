use anyhow::Result;
use colored::Colorize;
use dialoguer::Select;
use romance_core::manifest::{FileCategory, Manifest};
use romance_core::updater;
use std::path::Path;

pub fn run(init: bool) -> Result<()> {
    if init {
        return run_init();
    }

    let project_dir = Path::new(".");

    if !project_dir.join("romance.toml").exists() {
        anyhow::bail!("Not a Romance project (romance.toml not found)");
    }

    if !Manifest::exists(project_dir) {
        println!("{}", "No .romance/manifest.json found.".yellow());
        println!("This project was created before version tracking was available.");
        println!(
            "Run {} to create a baseline manifest.",
            "romance update --init".bold()
        );
        return Ok(());
    }

    println!("{}", "Checking for template updates...".bold());

    let plan = updater::plan_update(project_dir)?;
    let mut manifest = Manifest::load(project_dir)?;

    let total_changes = plan.auto_update.len() + plan.conflicts.len() + plan.new_files.len();
    if total_changes == 0 {
        println!("{}", "All scaffold files are up to date.".green());
        manifest.romance_version = env!("CARGO_PKG_VERSION").to_string();
        manifest.updated_at = chrono::Utc::now().to_rfc3339();
        manifest.save(project_dir)?;
        return Ok(());
    }

    println!(
        "  {} file(s) can be auto-updated",
        plan.auto_update.len().to_string().green()
    );
    println!(
        "  {} file(s) have conflicts",
        plan.conflicts.len().to_string().yellow()
    );
    println!(
        "  {} file(s) are new in this version",
        plan.new_files.len().to_string().cyan()
    );
    if !plan.deleted.is_empty() {
        println!(
            "  {} file(s) were deleted by user (skipping)",
            plan.deleted.len().to_string().red()
        );
    }
    println!("  {} file(s) are unchanged", plan.unchanged.len());
    println!();

    // Auto-update unmodified files
    if !plan.auto_update.is_empty() {
        println!("{}", "Auto-updating unmodified files:".bold());
        for item in &plan.auto_update {
            updater::apply_update(project_dir, &mut manifest, item)?;
            println!("  {} {}", "update".green(), item.output_path);
        }
        println!();
    }

    // New files
    if !plan.new_files.is_empty() {
        println!("{}", "Creating new files:".bold());
        for item in &plan.new_files {
            if item.current_content.is_some() {
                println!(
                    "  {} {} (already exists, skipping)",
                    "skip".yellow(),
                    item.output_path
                );
            } else {
                updater::apply_update(project_dir, &mut manifest, item)?;
                println!("  {} {}", "create".green(), item.output_path);
            }
        }
        println!();
    }

    // Handle conflicts interactively
    if !plan.conflicts.is_empty() {
        println!("{}", "Resolving conflicts:".bold());
        for item in &plan.conflicts {
            loop {
                let choices = &[
                    "Overwrite (use new template)",
                    "Skip (keep your version)",
                    "Show diff",
                ];
                let selection = Select::new()
                    .with_prompt(format!(
                        "  {} was modified by you AND template changed",
                        item.output_path.yellow()
                    ))
                    .items(choices)
                    .default(1)
                    .interact()?;

                match selection {
                    0 => {
                        updater::apply_update(project_dir, &mut manifest, item)?;
                        println!("    {} {}", "overwrite".green(), item.output_path);
                        break;
                    }
                    1 => {
                        println!("    {} {}", "skip".yellow(), item.output_path);
                        break;
                    }
                    2 => {
                        let current = item.current_content.as_deref().unwrap_or("");
                        let diff =
                            updater::generate_diff(current, &item.new_content, &item.output_path);
                        println!("{}", diff);
                    }
                    _ => unreachable!(),
                }
            }
        }
        println!();
    }

    manifest.romance_version = env!("CARGO_PKG_VERSION").to_string();
    manifest.updated_at = chrono::Utc::now().to_rfc3339();
    manifest.save(project_dir)?;

    println!("{}", "Update complete!".green().bold());
    Ok(())
}

fn run_init() -> Result<()> {
    let project_dir = Path::new(".");

    if !project_dir.join("romance.toml").exists() {
        anyhow::bail!("Not a Romance project (romance.toml not found)");
    }

    if Manifest::exists(project_dir) {
        println!(
            "{}",
            "Manifest already exists. Use `romance update` instead.".yellow()
        );
        return Ok(());
    }

    let config = romance_core::config::RomanceConfig::load(project_dir)?;
    let mut manifest = Manifest::new(&config.project.name, env!("CARGO_PKG_VERSION"));

    let all_mappings = updater::get_scaffold_mappings();
    for (template_path, output_path) in &all_mappings {
        let full_path = project_dir.join(output_path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let category = if output_path.contains("mod.rs") || output_path.contains("lib.rs") {
                FileCategory::Marker
            } else {
                FileCategory::Scaffold
            };
            manifest.record_file(output_path, Some(template_path), category, &content, None);
        }
    }

    manifest.save(project_dir)?;
    println!(
        "{}",
        "Manifest created at .romance/manifest.json".green().bold()
    );
    println!("Future `romance update` runs will track changes from this baseline.");

    Ok(())
}
