use anyhow::Result;
use colored::Colorize;
use heck::{ToLowerCamelCase, ToSnakeCase};
use std::fs;
use std::path::Path;

/// Remove all lines in a file that contain the given pattern.
fn remove_line_containing(file: &Path, pattern: &str) -> Result<bool> {
    let content = fs::read_to_string(file)?;
    let new_content: String = content
        .lines()
        .filter(|line| !line.contains(pattern))
        .collect::<Vec<_>>()
        .join("\n");
    if content != new_content {
        // Preserve trailing newline
        fs::write(file, new_content + "\n")?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Remove a file and print a message. Returns true if the file existed.
fn remove_file(path: &Path) -> bool {
    if path.exists() {
        if let Err(e) = fs::remove_file(path) {
            eprintln!(
                "  {} Failed to remove {}: {}",
                "!".yellow(),
                path.display(),
                e
            );
            return false;
        }
        println!("  {} Removed {}", "x".red(), path.display());
        true
    } else {
        println!("  {} Already absent: {}", "-".dimmed(), path.display());
        false
    }
}

/// Remove a directory recursively and print a message. Returns true if the directory existed.
fn remove_dir(path: &Path) -> bool {
    if path.exists() {
        if let Err(e) = fs::remove_dir_all(path) {
            eprintln!(
                "  {} Failed to remove {}: {}",
                "!".yellow(),
                path.display(),
                e
            );
            return false;
        }
        println!("  {} Removed {}/", "x".red(), path.display());
        true
    } else {
        println!("  {} Already absent: {}/", "-".dimmed(), path.display());
        false
    }
}

pub fn run_entity(name: &str) -> Result<()> {
    let snake = name.to_snake_case();
    let camel = name.to_lower_camel_case();

    println!(
        "{}",
        format!("Destroying entity '{}'...", name).bold()
    );
    println!();

    let mut removed = 0u32;
    let mut total = 0u32;

    // ── Backend files ────────────────────────────────────────────────
    println!("{}", "Backend files:".bold());

    let backend_files = [
        format!("backend/src/entities/{}.rs", snake),
        format!("backend/src/handlers/{}.rs", snake),
        format!("backend/src/routes/{}.rs", snake),
    ];

    for file in &backend_files {
        total += 1;
        if remove_file(Path::new(file)) {
            removed += 1;
        }
    }

    println!();

    // ── Frontend files ───────────────────────────────────────────────
    println!("{}", "Frontend files:".bold());

    let feature_dir = format!("frontend/src/features/{}", camel);
    total += 1;
    if remove_dir(Path::new(&feature_dir)) {
        removed += 1;
    }

    println!();

    // ── Clean markers ────────────────────────────────────────────────
    println!("{}", "Cleaning markers:".bold());

    // Remove `pub mod {snake};` from entities/mod.rs, handlers/mod.rs, routes/mod.rs
    let mod_pattern = format!("pub mod {};", snake);
    let mod_files = [
        "backend/src/entities/mod.rs",
        "backend/src/handlers/mod.rs",
        "backend/src/routes/mod.rs",
    ];

    for file in &mod_files {
        let path = Path::new(file);
        if path.exists() {
            match remove_line_containing(path, &mod_pattern) {
                Ok(true) => {
                    println!("  {} Cleaned {} from {}", "~".cyan(), mod_pattern, file);
                }
                Ok(false) => {
                    println!("  {} No match in {}", "-".dimmed(), file);
                }
                Err(e) => {
                    eprintln!("  {} Error cleaning {}: {}", "!".yellow(), file, e);
                }
            }
        }
    }

    // Remove `.merge({snake}::router())` from routes/mod.rs
    let route_pattern = format!("{}::router()", snake);
    let routes_mod = Path::new("backend/src/routes/mod.rs");
    if routes_mod.exists() {
        match remove_line_containing(routes_mod, &route_pattern) {
            Ok(true) => {
                println!(
                    "  {} Cleaned route merge for {} from routes/mod.rs",
                    "~".cyan(),
                    snake
                );
            }
            Ok(false) => {
                println!(
                    "  {} No route merge found in routes/mod.rs",
                    "-".dimmed()
                );
            }
            Err(e) => {
                eprintln!("  {} Error cleaning routes/mod.rs: {}", "!".yellow(), e);
            }
        }
    }

    // ── Clean frontend App.tsx ────────────────────────────────────────
    let app_tsx = Path::new("frontend/src/App.tsx");
    if app_tsx.exists() {
        // Remove import lines containing the entity feature path
        let import_pattern = format!("features/{}", camel);
        match remove_line_containing(app_tsx, &import_pattern) {
            Ok(true) => {
                println!(
                    "  {} Cleaned imports for {} from App.tsx",
                    "~".cyan(),
                    camel
                );
            }
            Ok(false) => {
                println!("  {} No imports found in App.tsx", "-".dimmed());
            }
            Err(e) => {
                eprintln!("  {} Error cleaning App.tsx imports: {}", "!".yellow(), e);
            }
        }

        // Remove Route elements for this entity (match on the snake_case path pattern)
        let route_path_pattern = format!("\"/{}", snake);
        match remove_line_containing(app_tsx, &route_path_pattern) {
            Ok(true) => {
                println!(
                    "  {} Cleaned routes for {} from App.tsx",
                    "~".cyan(),
                    snake
                );
            }
            Ok(false) => {
                println!("  {} No routes found in App.tsx", "-".dimmed());
            }
            Err(e) => {
                eprintln!("  {} Error cleaning App.tsx routes: {}", "!".yellow(), e);
            }
        }
    }

    println!();
    println!(
        "{}",
        format!("Done. Removed {}/{} targets.", removed, total)
            .bold()
    );

    Ok(())
}
