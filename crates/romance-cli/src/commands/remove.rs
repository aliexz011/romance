use anyhow::Result;
use romance_core::addon;
use std::path::Path;

pub fn run_validation() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::validation::ValidationAddon, project_root)
}

pub fn run_soft_delete() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::soft_delete::SoftDeleteAddon, project_root)
}

pub fn run_audit_log() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::audit_log::AuditLogAddon, project_root)
}

pub fn run_storage() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::storage::StorageAddon, project_root)
}

pub fn run_search() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::search::SearchAddon, project_root)
}

pub fn run_oauth() -> Result<()> {
    let project_root = Path::new(".");
    // Provider doesn't matter for uninstall — we just need an OauthAddon instance
    addon::run_uninstall(
        &addon::oauth::OauthAddon {
            provider: String::new(),
        },
        project_root,
    )
}

pub fn run_security() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::security::SecurityAddon, project_root)
}

pub fn run_observability() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::observability::ObservabilityAddon, project_root)
}

pub fn run_dashboard() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::dashboard::DashboardAddon, project_root)
}

pub fn run_email() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::email::EmailAddon, project_root)
}

pub fn run_i18n() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::i18n::I18nAddon, project_root)
}

pub fn run_cache() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::cache::CacheAddon, project_root)
}

pub fn run_tasks() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::tasks::TasksAddon, project_root)
}

pub fn run_websocket() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::websocket::WebsocketAddon, project_root)
}

pub fn run_api_keys() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_uninstall(&addon::api_keys::ApiKeysAddon, project_root)
}
