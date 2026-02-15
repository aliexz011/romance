pub mod api_keys;
pub mod audit_log;
pub mod cache;
pub mod dashboard;
pub mod email;
pub mod i18n;
pub mod multitenancy;
pub mod oauth;
pub mod observability;
pub mod search;
pub mod security;
pub mod soft_delete;
pub mod storage;
pub mod tasks;
pub mod validation;
pub mod websocket;

use anyhow::Result;
use std::path::Path;

/// Trait that all addons implement to provide a consistent installation interface.
pub trait Addon {
    fn name(&self) -> &str;
    fn check_prerequisites(&self, project_root: &Path) -> Result<()>;
    fn is_already_installed(&self, project_root: &Path) -> bool;
    fn install(&self, project_root: &Path) -> Result<()>;

    /// Uninstall the addon. Default implementation returns an error.
    fn uninstall(&self, project_root: &Path) -> Result<()> {
        let _ = project_root;
        anyhow::bail!("Uninstall not yet supported for '{}'", self.name())
    }

    /// Return the names of addons this addon depends on.
    fn dependencies(&self) -> Vec<&str> {
        vec![]
    }
}

/// Resolve an addon name to its concrete instance and run it.
/// Used for auto-installing dependencies.
fn resolve_and_install_dependency(name: &str, project_root: &Path) -> Result<()> {
    use colored::Colorize;

    match name {
        "auth" => {
            // Auth is not an addon, it's a generator. Just check it exists.
            if !project_root.join("backend/src/auth.rs").exists() {
                anyhow::bail!(
                    "Addon requires auth. Run {} first.",
                    "romance generate auth".bold()
                );
            }
            Ok(())
        }
        "validation" => run_addon(&validation::ValidationAddon, project_root),
        "soft-delete" => run_addon(&soft_delete::SoftDeleteAddon, project_root),
        "security" => run_addon(&security::SecurityAddon, project_root),
        "observability" => run_addon(&observability::ObservabilityAddon, project_root),
        "storage" => run_addon(&storage::StorageAddon, project_root),
        "search" => run_addon(&search::SearchAddon, project_root),
        "cache" => run_addon(&cache::CacheAddon, project_root),
        "email" => run_addon(&email::EmailAddon, project_root),
        "tasks" => run_addon(&tasks::TasksAddon, project_root),
        "websocket" => run_addon(&websocket::WebsocketAddon, project_root),
        "i18n" => run_addon(&i18n::I18nAddon, project_root),
        "dashboard" => run_addon(&dashboard::DashboardAddon, project_root),
        "audit-log" => run_addon(&audit_log::AuditLogAddon, project_root),
        "api-keys" => run_addon(&api_keys::ApiKeysAddon, project_root),
        "multitenancy" => run_addon(&multitenancy::MultitenancyAddon, project_root),
        _ => anyhow::bail!("Unknown addon dependency: '{}'", name),
    }
}

/// Run an addon: check prerequisites, skip if already installed, then install.
pub fn run_addon(addon: &dyn Addon, project_root: &Path) -> Result<()> {
    addon.check_prerequisites(project_root)?;

    if addon.is_already_installed(project_root) {
        println!("'{}' is already installed, skipping.", addon.name());
        return Ok(());
    }

    // Auto-install dependencies
    let deps = addon.dependencies();
    if !deps.is_empty() {
        use colored::Colorize;
        for dep in &deps {
            println!("{}", format!("Checking dependency: {}...", dep).dimmed());
            resolve_and_install_dependency(dep, project_root)?;
        }
        println!();
    }

    addon.install(project_root)?;

    // Regenerate AI context
    crate::ai_context::regenerate(project_root)?;

    Ok(())
}

/// Uninstall an addon: check if installed, then uninstall.
pub fn run_uninstall(addon: &dyn Addon, project_root: &Path) -> Result<()> {
    if !addon.is_already_installed(project_root) {
        println!("'{}' is not installed, nothing to remove.", addon.name());
        return Ok(());
    }

    addon.uninstall(project_root)?;

    // Regenerate AI context
    crate::ai_context::regenerate(project_root).ok();

    Ok(())
}

// =========================================================================
// Shared helper functions for addon implementations
// =========================================================================

/// Check that the project root contains a romance.toml file.
pub fn check_romance_project(project_root: &Path) -> Result<()> {
    if !project_root.join("romance.toml").exists() {
        anyhow::bail!("Not a Romance project (romance.toml not found)");
    }
    Ok(())
}

/// Check that auth has been generated (backend/src/auth.rs exists).
pub fn check_auth_exists(project_root: &Path) -> Result<()> {
    if !project_root.join("backend/src/auth.rs").exists() {
        anyhow::bail!("Auth must be generated first. Run: romance generate auth");
    }
    Ok(())
}

/// Add a `mod <mod_name>;` declaration to `backend/src/main.rs`.
///
/// Uses `insert_at_marker()` with the `// === ROMANCE:MAIN_MODS ===` marker
/// if present, otherwise falls back to `str::replace("mod errors;", ...)`.
pub fn add_mod_to_main(project_root: &Path, mod_name: &str) -> Result<()> {
    let main_path = project_root.join("backend/src/main.rs");
    let main_content = std::fs::read_to_string(&main_path)?;
    let mod_line = format!("mod {};", mod_name);

    if main_content.contains(&mod_line) {
        return Ok(());
    }

    let marker = "// === ROMANCE:MAIN_MODS ===";
    if main_content.contains(marker) {
        crate::utils::insert_at_marker(&main_path, marker, &mod_line)?;
    } else {
        // Fallback for projects scaffolded before the marker existed
        let new_content = main_content.replace("mod errors;", &format!("mod errors;\n{}", mod_line));
        std::fs::write(&main_path, new_content)?;
    }

    Ok(())
}

/// Add a dependency line to `backend/Cargo.toml`.
///
/// Uses `insert_at_marker()` with the `# === ROMANCE:DEPENDENCIES ===` marker
/// if present, otherwise falls back to appending at end of file.
pub fn add_cargo_dependency(project_root: &Path, dep_line: &str) -> Result<()> {
    let cargo_path = project_root.join("backend/Cargo.toml");
    let content = std::fs::read_to_string(&cargo_path)?;

    // Extract dependency name (everything before ' =')
    let dep_name = dep_line.split('=').next().unwrap_or("").trim();
    if content.contains(&format!("{} =", dep_name)) {
        return Ok(());
    }

    let marker = "# === ROMANCE:DEPENDENCIES ===";
    if content.contains(marker) {
        crate::utils::insert_at_marker(&cargo_path, marker, dep_line)?;
    } else {
        // Fallback: append to end of file
        let new_content = format!("{}\n{}\n", content.trim_end(), dep_line);
        std::fs::write(&cargo_path, new_content)?;
    }

    Ok(())
}

/// Update `romance.toml` to set a feature flag under the `[features]` section.
///
/// If the `[features]` section doesn't exist, it creates one.
pub fn update_feature_flag(project_root: &Path, feature: &str, value: bool) -> Result<()> {
    let config_path = project_root.join("romance.toml");
    let content = std::fs::read_to_string(&config_path)?;
    let line = format!("{} = {}", feature, value);

    if content.contains(&line) {
        return Ok(());
    }

    if content.contains("[features]") {
        if !content.contains(feature) {
            let new_content = content.replace("[features]", &format!("[features]\n{}", line));
            std::fs::write(&config_path, new_content)?;
        }
    } else {
        let new_content = format!("{}\n[features]\n{}\n", content.trim_end(), line);
        std::fs::write(&config_path, new_content)?;
    }

    Ok(())
}

/// Append an environment variable line to a `.env` file if not already present.
pub fn append_env_var(path: &Path, line: &str) -> Result<()> {
    crate::generator::auth::append_env_var(path, line)
}

/// Remove a file if it exists. Returns true if file was removed.
pub fn remove_file_if_exists(path: &Path) -> Result<bool> {
    if path.exists() {
        std::fs::remove_file(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Remove a line containing `needle` from a file.
pub fn remove_line_from_file(path: &Path, needle: &str) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(path)?;
    let new_content: String = content
        .lines()
        .filter(|line| !line.contains(needle))
        .collect::<Vec<_>>()
        .join("\n");
    // Preserve trailing newline
    let new_content = if content.ends_with('\n') {
        format!("{}\n", new_content)
    } else {
        new_content
    };
    std::fs::write(path, new_content)?;
    Ok(())
}

/// Remove a `mod <name>;` declaration from `backend/src/main.rs`.
pub fn remove_mod_from_main(project_root: &Path, mod_name: &str) -> Result<()> {
    let main_path = project_root.join("backend/src/main.rs");
    remove_line_from_file(&main_path, &format!("mod {};", mod_name))
}

/// Remove a feature flag from `romance.toml`'s `[features]` section.
pub fn remove_feature_flag(project_root: &Path, feature: &str) -> Result<()> {
    let config_path = project_root.join("romance.toml");
    let line = format!("{} = true", feature);
    remove_line_from_file(&config_path, &line)?;
    // Also remove "feature = false" in case
    let line_false = format!("{} = false", feature);
    remove_line_from_file(&config_path, &line_false)
}

/// Remove a TOML section (e.g., `[security]`) and all its contents until the next section.
pub fn remove_toml_section(project_root: &Path, section_name: &str) -> Result<()> {
    let config_path = project_root.join("romance.toml");
    if !config_path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&config_path)?;
    let section_header = format!("[{}]", section_name);
    if !content.contains(&section_header) {
        return Ok(());
    }
    let mut result_lines: Vec<&str> = Vec::new();
    let mut skipping = false;
    for line in content.lines() {
        if line.trim() == section_header {
            skipping = true;
            continue;
        }
        if skipping && line.trim().starts_with('[') {
            skipping = false;
        }
        if !skipping {
            result_lines.push(line);
        }
    }
    let new_content = format!("{}\n", result_lines.join("\n").trim_end());
    std::fs::write(&config_path, new_content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_romance_toml(dir: &std::path::Path) {
        std::fs::write(
            dir.join("romance.toml"),
            "[project]\nname = \"test\"\n[backend]\nport = 3001\ndatabase_url = \"postgres://localhost/test\"",
        )
        .unwrap();
    }

    // =========================================================================
    // A) Addon name / identity tests
    // =========================================================================

    #[test]
    fn security_addon_name() {
        let addon = security::SecurityAddon;
        assert_eq!(addon.name(), "security");
    }

    #[test]
    fn validation_addon_name() {
        let addon = validation::ValidationAddon;
        assert_eq!(addon.name(), "validation");
    }

    #[test]
    fn soft_delete_addon_name() {
        let addon = soft_delete::SoftDeleteAddon;
        assert_eq!(addon.name(), "soft-delete");
    }

    #[test]
    fn observability_addon_name() {
        let addon = observability::ObservabilityAddon;
        assert_eq!(addon.name(), "observability");
    }

    #[test]
    fn search_addon_name() {
        let addon = search::SearchAddon;
        assert_eq!(addon.name(), "search");
    }

    #[test]
    fn email_addon_name() {
        let addon = email::EmailAddon;
        assert_eq!(addon.name(), "email");
    }

    #[test]
    fn cache_addon_name() {
        let addon = cache::CacheAddon;
        assert_eq!(addon.name(), "cache");
    }

    #[test]
    fn dashboard_addon_name() {
        let addon = dashboard::DashboardAddon;
        assert_eq!(addon.name(), "dashboard");
    }

    #[test]
    fn storage_addon_name() {
        let addon = storage::StorageAddon;
        assert_eq!(addon.name(), "storage");
    }

    #[test]
    fn websocket_addon_name() {
        let addon = websocket::WebsocketAddon;
        assert_eq!(addon.name(), "websocket");
    }

    #[test]
    fn i18n_addon_name() {
        let addon = i18n::I18nAddon;
        assert_eq!(addon.name(), "i18n");
    }

    #[test]
    fn tasks_addon_name() {
        let addon = tasks::TasksAddon;
        assert_eq!(addon.name(), "tasks");
    }

    #[test]
    fn api_keys_addon_name() {
        let addon = api_keys::ApiKeysAddon;
        assert_eq!(addon.name(), "api-keys");
    }

    #[test]
    fn audit_log_addon_name() {
        let addon = audit_log::AuditLogAddon;
        assert_eq!(addon.name(), "audit-log");
    }

    #[test]
    fn multitenancy_addon_name() {
        let addon = multitenancy::MultitenancyAddon;
        assert_eq!(addon.name(), "multitenancy");
    }

    #[test]
    fn oauth_addon_name() {
        let addon = oauth::OauthAddon {
            provider: "google".to_string(),
        };
        assert_eq!(addon.name(), "oauth");
    }

    // =========================================================================
    // B) Prerequisites check tests
    // =========================================================================

    #[test]
    fn security_prerequisites_fail_without_romance_toml() {
        let dir = tempfile::tempdir().unwrap();
        let result = security::SecurityAddon.check_prerequisites(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn security_prerequisites_pass_with_romance_toml() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        let result = security::SecurityAddon.check_prerequisites(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn validation_prerequisites_fail_without_romance_toml() {
        let dir = tempfile::tempdir().unwrap();
        let result = validation::ValidationAddon.check_prerequisites(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn validation_prerequisites_pass_with_romance_toml() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        let result = validation::ValidationAddon.check_prerequisites(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn soft_delete_prerequisites_fail_without_romance_toml() {
        let dir = tempfile::tempdir().unwrap();
        let result = soft_delete::SoftDeleteAddon.check_prerequisites(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn soft_delete_prerequisites_pass_with_romance_toml() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        let result = soft_delete::SoftDeleteAddon.check_prerequisites(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn api_keys_prerequisites_fail_without_romance_toml() {
        let dir = tempfile::tempdir().unwrap();
        let result = api_keys::ApiKeysAddon.check_prerequisites(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn api_keys_prerequisites_fail_without_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        let result = api_keys::ApiKeysAddon.check_prerequisites(dir.path());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Auth must be generated first"));
    }

    #[test]
    fn api_keys_prerequisites_pass_with_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(dir.path().join("backend/src/auth.rs"), "").unwrap();
        let result = api_keys::ApiKeysAddon.check_prerequisites(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn audit_log_prerequisites_fail_without_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        let result = audit_log::AuditLogAddon.check_prerequisites(dir.path());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Auth must be generated first"));
    }

    #[test]
    fn audit_log_prerequisites_pass_with_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(dir.path().join("backend/src/auth.rs"), "").unwrap();
        let result = audit_log::AuditLogAddon.check_prerequisites(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn multitenancy_prerequisites_fail_without_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        let result = multitenancy::MultitenancyAddon.check_prerequisites(dir.path());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Auth must be generated first"));
    }

    #[test]
    fn multitenancy_prerequisites_pass_with_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(dir.path().join("backend/src/auth.rs"), "").unwrap();
        let result = multitenancy::MultitenancyAddon.check_prerequisites(dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn oauth_prerequisites_fail_without_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        let addon = oauth::OauthAddon {
            provider: "google".to_string(),
        };
        let result = addon.check_prerequisites(dir.path());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Auth must be generated first"));
    }

    #[test]
    fn oauth_prerequisites_pass_with_auth() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(dir.path().join("backend/src/auth.rs"), "").unwrap();
        let addon = oauth::OauthAddon {
            provider: "github".to_string(),
        };
        let result = addon.check_prerequisites(dir.path());
        assert!(result.is_ok());
    }

    // =========================================================================
    // C) is_already_installed tests
    // =========================================================================

    #[test]
    fn security_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!security::SecurityAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn security_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let middleware_dir = dir.path().join("backend/src/middleware");
        std::fs::create_dir_all(&middleware_dir).unwrap();
        std::fs::write(middleware_dir.join("security_headers.rs"), "").unwrap();
        assert!(security::SecurityAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn validation_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!validation::ValidationAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn validation_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("validation.rs"), "").unwrap();
        assert!(validation::ValidationAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn soft_delete_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!soft_delete::SoftDeleteAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn soft_delete_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("soft_delete.rs"), "").unwrap();
        assert!(soft_delete::SoftDeleteAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn observability_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!observability::ObservabilityAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn observability_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let middleware_dir = dir.path().join("backend/src/middleware");
        std::fs::create_dir_all(&middleware_dir).unwrap();
        std::fs::write(middleware_dir.join("request_id.rs"), "").unwrap();
        assert!(observability::ObservabilityAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn search_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!search::SearchAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn search_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("search.rs"), "").unwrap();
        assert!(search::SearchAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn email_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!email::EmailAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn email_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("email.rs"), "").unwrap();
        assert!(email::EmailAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn cache_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!cache::CacheAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn cache_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("cache.rs"), "").unwrap();
        assert!(cache::CacheAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn dashboard_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!dashboard::DashboardAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn dashboard_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let dev_dir = dir.path().join("frontend/src/features/dev");
        std::fs::create_dir_all(&dev_dir).unwrap();
        std::fs::write(dev_dir.join("DevDashboard.tsx"), "").unwrap();
        assert!(dashboard::DashboardAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn storage_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!storage::StorageAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn storage_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("storage.rs"), "").unwrap();
        assert!(storage::StorageAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn websocket_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!websocket::WebsocketAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn websocket_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("ws.rs"), "").unwrap();
        assert!(websocket::WebsocketAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn i18n_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!i18n::I18nAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn i18n_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("i18n.rs"), "").unwrap();
        assert!(i18n::I18nAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn tasks_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!tasks::TasksAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn tasks_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("tasks.rs"), "").unwrap();
        assert!(tasks::TasksAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn api_keys_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!api_keys::ApiKeysAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn api_keys_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("api_keys.rs"), "").unwrap();
        assert!(api_keys::ApiKeysAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn audit_log_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!audit_log::AuditLogAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn audit_log_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("audit.rs"), "").unwrap();
        assert!(audit_log::AuditLogAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn multitenancy_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!multitenancy::MultitenancyAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn multitenancy_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("tenant.rs"), "").unwrap();
        assert!(multitenancy::MultitenancyAddon.is_already_installed(dir.path()));
    }

    #[test]
    fn oauth_not_installed_in_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let addon = oauth::OauthAddon {
            provider: "google".to_string(),
        };
        assert!(!addon.is_already_installed(dir.path()));
    }

    #[test]
    fn oauth_installed_when_marker_exists() {
        let dir = tempfile::tempdir().unwrap();
        let backend_src = dir.path().join("backend/src");
        std::fs::create_dir_all(&backend_src).unwrap();
        std::fs::write(backend_src.join("oauth.rs"), "").unwrap();
        let addon = oauth::OauthAddon {
            provider: "google".to_string(),
        };
        assert!(addon.is_already_installed(dir.path()));
    }

    // =========================================================================
    // D) Uninstall helper tests
    // =========================================================================

    #[test]
    fn remove_file_if_exists_returns_true_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        std::fs::write(&path, "content").unwrap();
        assert!(remove_file_if_exists(&path).unwrap());
        assert!(!path.exists());
    }

    #[test]
    fn remove_file_if_exists_returns_false_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.rs");
        assert!(!remove_file_if_exists(&path).unwrap());
    }

    #[test]
    fn remove_line_from_file_removes_matching_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        std::fs::write(&path, "mod a;\nmod b;\nmod c;\n").unwrap();
        remove_line_from_file(&path, "mod b;").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(!content.contains("mod b;"));
        assert!(content.contains("mod a;"));
        assert!(content.contains("mod c;"));
    }

    #[test]
    fn remove_line_from_file_noop_when_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.rs");
        std::fs::write(&path, "mod a;\nmod c;\n").unwrap();
        remove_line_from_file(&path, "mod b;").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("mod a;"));
        assert!(content.contains("mod c;"));
    }

    #[test]
    fn remove_mod_from_main_works() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(
            dir.path().join("backend/src/main.rs"),
            "mod errors;\nmod validation;\n// === ROMANCE:MAIN_MODS ===\nmod handlers;\n",
        )
        .unwrap();
        remove_mod_from_main(dir.path(), "validation").unwrap();
        let content =
            std::fs::read_to_string(dir.path().join("backend/src/main.rs")).unwrap();
        assert!(!content.contains("mod validation;"));
        assert!(content.contains("mod errors;"));
    }

    #[test]
    fn remove_feature_flag_works() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("romance.toml"),
            "[project]\nname = \"test\"\n[features]\nvalidation = true\ncache = true\n",
        )
        .unwrap();
        remove_feature_flag(dir.path(), "validation").unwrap();
        let content = std::fs::read_to_string(dir.path().join("romance.toml")).unwrap();
        assert!(!content.contains("validation"));
        assert!(content.contains("cache = true"));
    }

    #[test]
    fn remove_toml_section_works() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("romance.toml"),
            "[project]\nname = \"test\"\n\n[security]\nrate_limit = 60\ncors = true\n\n[features]\nauth = true\n",
        )
        .unwrap();
        remove_toml_section(dir.path(), "security").unwrap();
        let content = std::fs::read_to_string(dir.path().join("romance.toml")).unwrap();
        assert!(!content.contains("[security]"));
        assert!(!content.contains("rate_limit"));
        assert!(content.contains("[features]"));
        assert!(content.contains("[project]"));
    }

    // =========================================================================
    // E) Dependencies tests
    // =========================================================================

    #[test]
    fn audit_log_depends_on_auth() {
        let addon = audit_log::AuditLogAddon;
        assert_eq!(addon.dependencies(), vec!["auth"]);
    }

    #[test]
    fn oauth_depends_on_auth() {
        let addon = oauth::OauthAddon {
            provider: "google".to_string(),
        };
        assert_eq!(addon.dependencies(), vec!["auth"]);
    }

    #[test]
    fn api_keys_depends_on_auth() {
        let addon = api_keys::ApiKeysAddon;
        assert_eq!(addon.dependencies(), vec!["auth"]);
    }

    #[test]
    fn multitenancy_depends_on_auth() {
        let addon = multitenancy::MultitenancyAddon;
        assert_eq!(addon.dependencies(), vec!["auth"]);
    }

    #[test]
    fn security_has_no_dependencies() {
        let addon = security::SecurityAddon;
        assert!(addon.dependencies().is_empty());
    }

    #[test]
    fn validation_has_no_dependencies() {
        let addon = validation::ValidationAddon;
        assert!(addon.dependencies().is_empty());
    }

    // =========================================================================
    // F) Shared helper tests
    // =========================================================================

    #[test]
    fn check_romance_project_fails_without_toml() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_romance_project(dir.path()).is_err());
    }

    #[test]
    fn check_romance_project_passes_with_toml() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        assert!(check_romance_project(dir.path()).is_ok());
    }

    #[test]
    fn check_auth_exists_fails_without_auth() {
        let dir = tempfile::tempdir().unwrap();
        assert!(check_auth_exists(dir.path()).is_err());
    }

    #[test]
    fn check_auth_exists_passes_with_auth() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(dir.path().join("backend/src/auth.rs"), "").unwrap();
        assert!(check_auth_exists(dir.path()).is_ok());
    }

    #[test]
    fn add_mod_to_main_with_marker() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(
            dir.path().join("backend/src/main.rs"),
            "mod errors;\n// === ROMANCE:MAIN_MODS ===\nmod handlers;\n",
        )
        .unwrap();
        add_mod_to_main(dir.path(), "storage").unwrap();
        let content = std::fs::read_to_string(dir.path().join("backend/src/main.rs")).unwrap();
        assert!(content.contains("mod storage;"));
        assert!(content.contains("// === ROMANCE:MAIN_MODS ==="));
    }

    #[test]
    fn add_mod_to_main_without_marker_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(
            dir.path().join("backend/src/main.rs"),
            "mod errors;\nmod handlers;\n",
        )
        .unwrap();
        add_mod_to_main(dir.path(), "storage").unwrap();
        let content = std::fs::read_to_string(dir.path().join("backend/src/main.rs")).unwrap();
        assert!(content.contains("mod storage;"));
    }

    #[test]
    fn add_mod_to_main_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("backend/src")).unwrap();
        std::fs::write(
            dir.path().join("backend/src/main.rs"),
            "mod errors;\n// === ROMANCE:MAIN_MODS ===\nmod handlers;\n",
        )
        .unwrap();
        add_mod_to_main(dir.path(), "storage").unwrap();
        add_mod_to_main(dir.path(), "storage").unwrap();
        let content = std::fs::read_to_string(dir.path().join("backend/src/main.rs")).unwrap();
        assert_eq!(content.matches("mod storage;").count(), 1);
    }

    #[test]
    fn update_feature_flag_creates_section() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        update_feature_flag(dir.path(), "cache", true).unwrap();
        let content = std::fs::read_to_string(dir.path().join("romance.toml")).unwrap();
        assert!(content.contains("[features]"));
        assert!(content.contains("cache = true"));
    }

    #[test]
    fn update_feature_flag_appends_to_existing_section() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("romance.toml"),
            "[project]\nname = \"test\"\n[features]\nauth = true\n",
        )
        .unwrap();
        update_feature_flag(dir.path(), "cache", true).unwrap();
        let content = std::fs::read_to_string(dir.path().join("romance.toml")).unwrap();
        assert!(content.contains("cache = true"));
        assert!(content.contains("auth = true"));
    }

    #[test]
    fn update_feature_flag_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        write_romance_toml(dir.path());
        update_feature_flag(dir.path(), "cache", true).unwrap();
        update_feature_flag(dir.path(), "cache", true).unwrap();
        let content = std::fs::read_to_string(dir.path().join("romance.toml")).unwrap();
        assert_eq!(content.matches("cache = true").count(), 1);
    }
}
