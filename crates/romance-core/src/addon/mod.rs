pub mod api_keys;
pub mod audit_log;
pub mod cache;
pub mod dashboard;
pub mod email;
pub mod i18n;
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
}

/// Run an addon: check prerequisites, skip if already installed, then install.
pub fn run_addon(addon: &dyn Addon, project_root: &Path) -> Result<()> {
    addon.check_prerequisites(project_root)?;

    if addon.is_already_installed(project_root) {
        println!("'{}' is already installed, skipping.", addon.name());
        return Ok(());
    }

    addon.install(project_root)?;

    // Regenerate AI context
    crate::ai_context::regenerate(project_root)?;

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
}
