use anyhow::Result;
use romance_core::addon;
use std::path::Path;

pub fn run_validation() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::validation::ValidationAddon, project_root)
}

pub fn run_soft_delete() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::soft_delete::SoftDeleteAddon, project_root)
}

pub fn run_audit_log() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::audit_log::AuditLogAddon, project_root)
}

pub fn run_storage() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::storage::StorageAddon, project_root)
}

pub fn run_search() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::search::SearchAddon, project_root)
}

pub fn run_oauth(provider: &str) -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(
        &addon::oauth::OauthAddon {
            provider: provider.to_string(),
        },
        project_root,
    )
}

pub fn run_security() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::security::SecurityAddon, project_root)
}

pub fn run_observability() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::observability::ObservabilityAddon, project_root)
}

pub fn run_dashboard() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::dashboard::DashboardAddon, project_root)
}

pub fn run_i18n() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::i18n::I18nAddon, project_root)
}

pub fn run_email() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::email::EmailAddon, project_root)
}

pub fn run_cache() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::cache::CacheAddon, project_root)
}

pub fn run_tasks() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::tasks::TasksAddon, project_root)
}

pub fn run_websocket() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::websocket::WebsocketAddon, project_root)
}

pub fn run_api_keys() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::api_keys::ApiKeysAddon, project_root)
}

pub fn run_multitenancy() -> Result<()> {
    let project_root = Path::new(".");
    addon::run_addon(&addon::multitenancy::MultitenancyAddon, project_root)
}
