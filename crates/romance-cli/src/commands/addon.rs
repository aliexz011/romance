use anyhow::Result;
use colored::Colorize;
use romance_core::addon::Addon;
use std::path::Path;

/// List all available addons with descriptions.
pub fn run_list() -> Result<()> {
    println!("{}", "Available addons:".bold());
    println!();
    println!(
        "  {:<16} {}",
        "validation".cyan(),
        "Backend validation (validator) + frontend (Zod)"
    );
    println!(
        "  {:<16} {}",
        "soft-delete".cyan(),
        "Soft delete with deleted_at timestamp"
    );
    println!(
        "  {:<16} {}",
        "audit-log".cyan(),
        "Track create/update/delete operations"
    );
    println!(
        "  {:<16} {}",
        "storage".cyan(),
        "File and image uploads (local/S3)"
    );
    println!(
        "  {:<16} {}",
        "search".cyan(),
        "Full-text search (PostgreSQL tsvector)"
    );
    println!(
        "  {:<16} {}",
        "security".cyan(),
        "Rate limiting, CORS, security headers"
    );
    println!(
        "  {:<16} {}",
        "observability".cyan(),
        "Structured logging and tracing"
    );
    println!(
        "  {:<16} {}",
        "cache".cyan(),
        "Redis caching layer"
    );
    println!(
        "  {:<16} {}",
        "email".cyan(),
        "Email sending (SMTP/templates)"
    );
    println!(
        "  {:<16} {}",
        "oauth".cyan(),
        "Social authentication (Google/GitHub/Discord)"
    );
    println!(
        "  {:<16} {}",
        "tasks".cyan(),
        "Background task processing"
    );
    println!(
        "  {:<16} {}",
        "websocket".cyan(),
        "WebSocket support"
    );
    println!(
        "  {:<16} {}",
        "api-keys".cyan(),
        "API key authentication"
    );
    println!(
        "  {:<16} {}",
        "i18n".cyan(),
        "Internationalization"
    );
    println!(
        "  {:<16} {}",
        "dashboard".cyan(),
        "Developer dashboard"
    );
    println!();
    println!(
        "Install with: {}",
        "romance add <addon-name>".green()
    );
    Ok(())
}

/// Check which addons are installed in the current project.
pub fn run_status() -> Result<()> {
    let project_root = Path::new(".");

    // Verify we are in a Romance project
    if !project_root.join("romance.toml").exists() {
        anyhow::bail!(
            "No romance.toml found. Please run this command from a Romance project root."
        );
    }

    println!("{}", "Addon status:".bold());
    println!();

    let addons: Vec<(&str, Box<dyn Addon>)> = vec![
        ("validation", Box::new(romance_core::addon::validation::ValidationAddon)),
        ("soft-delete", Box::new(romance_core::addon::soft_delete::SoftDeleteAddon)),
        ("audit-log", Box::new(romance_core::addon::audit_log::AuditLogAddon)),
        ("storage", Box::new(romance_core::addon::storage::StorageAddon)),
        ("search", Box::new(romance_core::addon::search::SearchAddon)),
        ("security", Box::new(romance_core::addon::security::SecurityAddon)),
        ("observability", Box::new(romance_core::addon::observability::ObservabilityAddon)),
        ("cache", Box::new(romance_core::addon::cache::CacheAddon)),
        ("email", Box::new(romance_core::addon::email::EmailAddon)),
        ("tasks", Box::new(romance_core::addon::tasks::TasksAddon)),
        ("websocket", Box::new(romance_core::addon::websocket::WebsocketAddon)),
        ("api-keys", Box::new(romance_core::addon::api_keys::ApiKeysAddon)),
        ("i18n", Box::new(romance_core::addon::i18n::I18nAddon)),
        ("dashboard", Box::new(romance_core::addon::dashboard::DashboardAddon)),
    ];

    for (label, addon) in &addons {
        let installed = addon.is_already_installed(project_root);
        let status = if installed {
            "installed".green().to_string()
        } else {
            "not installed".dimmed().to_string()
        };
        println!("  {:<16} {}", label, status);
    }

    // OAuth is special -- check with a default provider since is_already_installed
    // checks for backend/src/oauth.rs which is provider-independent
    let oauth_addon = romance_core::addon::oauth::OauthAddon {
        provider: "google".to_string(),
    };
    let oauth_installed = oauth_addon.is_already_installed(project_root);
    let oauth_status = if oauth_installed {
        "installed".green().to_string()
    } else {
        "not installed".dimmed().to_string()
    };
    println!("  {:<16} {}", "oauth", oauth_status);

    println!();
    Ok(())
}
