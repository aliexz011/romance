use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct RomanceConfig {
    pub project: ProjectConfig,
    pub backend: BackendConfig,
    pub frontend: FrontendConfig,
    #[serde(default)]
    pub codegen: CodegenConfig,
    #[serde(default)]
    pub features: FeaturesConfig,
    #[serde(default)]
    pub security: Option<SecurityConfig>,
    #[serde(default)]
    pub storage: Option<StorageConfig>,
    #[serde(default)]
    pub environment: EnvironmentConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Active environment: "development", "staging", "production"
    #[serde(default = "default_environment")]
    pub active: String,
}

impl Default for EnvironmentConfig {
    fn default() -> Self {
        Self {
            active: default_environment(),
        }
    }
}

fn default_environment() -> String {
    std::env::var("ROMANCE_ENV").unwrap_or_else(|_| "development".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackendConfig {
    pub port: u16,
    pub database_url: String,
    /// API route prefix (default: "/api"). Use for API versioning, e.g. "/api/v1".
    pub api_prefix: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FrontendConfig {
    pub port: u16,
    pub api_base_url: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CodegenConfig {
    #[serde(default = "default_true")]
    pub generate_openapi: bool,
    #[serde(default = "default_true")]
    pub generate_ts_types: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct FeaturesConfig {
    #[serde(default)]
    pub validation: bool,
    #[serde(default)]
    pub soft_delete: bool,
    #[serde(default)]
    pub audit_log: bool,
    #[serde(default)]
    pub search: bool,
    #[serde(default)]
    pub multitenancy: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_rate_limit")]
    pub rate_limit_rpm: u32,
    #[serde(default)]
    pub cors_origins: Vec<String>,
    #[serde(default)]
    pub csrf: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            rate_limit_rpm: 60,
            cors_origins: vec!["http://localhost:5173".to_string()],
            csrf: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "default_storage_backend")]
    pub backend: String,
    #[serde(default = "default_upload_dir")]
    pub upload_dir: String,
    #[serde(default = "default_max_file_size")]
    pub max_file_size: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "local".to_string(),
            upload_dir: "./uploads".to_string(),
            max_file_size: "10MB".to_string(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_rate_limit() -> u32 {
    60
}

fn default_storage_backend() -> String {
    "local".to_string()
}

fn default_upload_dir() -> String {
    "./uploads".to_string()
}

fn default_max_file_size() -> String {
    "10MB".to_string()
}

/// Deep-merge two TOML values. The `override_val` takes precedence over `base`.
/// Tables are merged recursively; all other types are replaced.
fn deep_merge(base: toml::Value, override_val: toml::Value) -> toml::Value {
    match (base, override_val) {
        (toml::Value::Table(mut base_table), toml::Value::Table(override_table)) => {
            for (key, override_v) in override_table {
                let merged = if let Some(base_v) = base_table.remove(&key) {
                    deep_merge(base_v, override_v)
                } else {
                    override_v
                };
                base_table.insert(key, merged);
            }
            toml::Value::Table(base_table)
        }
        // For non-table types, the override wins
        (_base, override_val) => override_val,
    }
}

impl RomanceConfig {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join("romance.toml");
        let content = std::fs::read_to_string(&path)?;
        let config: RomanceConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load config with environment-specific overrides.
    ///
    /// 1. Loads base `romance.toml`
    /// 2. Determines the active environment from `ROMANCE_ENV` env var
    ///    (or from `[environment] active` in the base config), defaulting to "development"
    /// 3. If `romance.{env}.toml` exists (e.g. `romance.production.toml`),
    ///    deep-merges those overrides on top of the base config
    ///
    /// This is fully backward-compatible: projects without env-specific files
    /// behave exactly as before.
    pub fn load_with_env(dir: &Path) -> Result<Self> {
        let base_path = dir.join("romance.toml");
        let base_content = std::fs::read_to_string(&base_path)?;
        let base_value: toml::Value = toml::from_str(&base_content)?;

        // Determine active environment: ROMANCE_ENV takes priority, then config field
        let env_name = std::env::var("ROMANCE_ENV").unwrap_or_else(|_| {
            base_value
                .get("environment")
                .and_then(|e| e.get("active"))
                .and_then(|a| a.as_str())
                .unwrap_or("development")
                .to_string()
        });

        // Check for environment-specific override file
        let env_path = dir.join(format!("romance.{}.toml", env_name));
        let merged_value = if env_path.exists() {
            let env_content = std::fs::read_to_string(&env_path)?;
            let env_value: toml::Value = toml::from_str(&env_content)?;
            deep_merge(base_value, env_value)
        } else {
            base_value
        };

        // Deserialize the merged TOML value into RomanceConfig
        let config: RomanceConfig = merged_value.try_into()?;
        Ok(config)
    }

    /// Check if a feature is enabled.
    pub fn has_feature(&self, feature: &str) -> bool {
        match feature {
            "validation" => self.features.validation,
            "soft_delete" => self.features.soft_delete,
            "audit_log" => self.features.audit_log,
            "search" => self.features.search,
            "multitenancy" => self.features.multitenancy,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: write a romance.toml and return the tempdir.
    fn write_config(toml_content: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("romance.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();
        dir
    }

    // ── Loading a valid config ────────────────────────────────────────

    #[test]
    fn load_valid_config() {
        let dir = write_config(
            r#"
[project]
name = "my-app"

[backend]
port = 3000
database_url = "postgres://localhost/mydb"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert_eq!(config.project.name, "my-app");
        assert_eq!(config.backend.port, 3000);
        assert_eq!(config.frontend.port, 5173);
    }

    // ── Default values for optional sections ──────────────────────────

    #[test]
    fn default_codegen_when_section_omitted() {
        // When [codegen] section is omitted entirely, CodegenConfig::default() is used
        // which gives false for both fields (Rust Default for bool).
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert!(!config.codegen.generate_openapi);
        assert!(!config.codegen.generate_ts_types);
    }

    #[test]
    fn codegen_fields_default_to_true_when_section_present() {
        // When [codegen] section is present but fields are omitted,
        // serde(default = "default_true") kicks in.
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"

[codegen]
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert!(config.codegen.generate_openapi);
        assert!(config.codegen.generate_ts_types);
    }

    #[test]
    fn default_features_all_false() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert!(!config.features.validation);
        assert!(!config.features.soft_delete);
        assert!(!config.features.audit_log);
        assert!(!config.features.search);
    }

    // ── has_feature ───────────────────────────────────────────────────

    #[test]
    fn has_feature_enabled() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"

[features]
validation = true
search = true
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert!(config.has_feature("validation"));
        assert!(config.has_feature("search"));
        assert!(!config.has_feature("soft_delete"));
        assert!(!config.has_feature("audit_log"));
    }

    #[test]
    fn has_feature_unknown_returns_false() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert!(!config.has_feature("nonexistent_feature"));
    }

    // ── api_prefix ────────────────────────────────────────────────────

    #[test]
    fn api_prefix_none_by_default() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert!(config.backend.api_prefix.is_none());
    }

    #[test]
    fn api_prefix_custom_value() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"
api_prefix = "/api/v1"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert_eq!(config.backend.api_prefix.as_deref(), Some("/api/v1"));
    }

    // ── Security and storage configs ──────────────────────────────────

    #[test]
    fn security_config_defaults() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"

[security]
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        let sec = config.security.unwrap();
        assert_eq!(sec.rate_limit_rpm, 60);
        assert!(!sec.csrf);
    }

    #[test]
    fn storage_config_defaults() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"

[storage]
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        let store = config.storage.unwrap();
        assert_eq!(store.backend, "local");
        assert_eq!(store.upload_dir, "./uploads");
        assert_eq!(store.max_file_size, "10MB");
    }

    // ── Missing file returns error ────────────────────────────────────

    #[test]
    fn load_missing_file_errors() {
        let dir = tempfile::tempdir().unwrap();
        assert!(RomanceConfig::load(dir.path()).is_err());
    }

    // ── Project description optional ──────────────────────────────────

    #[test]
    fn project_description_optional() {
        let dir = write_config(
            r#"
[project]
name = "test"
description = "A test project"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert_eq!(config.project.description.as_deref(), Some("A test project"));
    }

    // ── Environment config ───────────────────────────────────────────

    #[test]
    fn default_environment_is_development() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load(dir.path()).unwrap();
        assert_eq!(config.environment.active, "development");
    }

    #[test]
    fn load_with_env_no_override_file() {
        // Without an env-specific file, load_with_env behaves like load
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"
"#,
        );

        let config = RomanceConfig::load_with_env(dir.path()).unwrap();
        assert_eq!(config.project.name, "test");
        assert_eq!(config.backend.port, 3000);
    }

    #[test]
    fn load_with_env_merges_override() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"

[environment]
active = "production"
"#,
        );

        // Write the production override file
        let prod_path = dir.path().join("romance.production.toml");
        let mut f = std::fs::File::create(&prod_path).unwrap();
        f.write_all(
            br#"
[backend]
port = 8080
"#,
        )
        .unwrap();

        let config = RomanceConfig::load_with_env(dir.path()).unwrap();
        // port should be overridden
        assert_eq!(config.backend.port, 8080);
        // database_url should remain from base
        assert_eq!(config.backend.database_url, "postgres://localhost/test");
        // project name should remain from base
        assert_eq!(config.project.name, "test");
    }

    #[test]
    fn load_with_env_deep_merge_preserves_unrelated_sections() {
        let dir = write_config(
            r#"
[project]
name = "test"

[backend]
port = 3000
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3000"

[environment]
active = "staging"

[features]
validation = true
"#,
        );

        // Write staging override that only touches backend
        let staging_path = dir.path().join("romance.staging.toml");
        let mut f = std::fs::File::create(&staging_path).unwrap();
        f.write_all(
            br#"
[backend]
port = 4000
"#,
        )
        .unwrap();

        let config = RomanceConfig::load_with_env(dir.path()).unwrap();
        assert_eq!(config.backend.port, 4000);
        // features should be preserved from base
        assert!(config.features.validation);
        assert_eq!(config.frontend.port, 5173);
    }

    // ── deep_merge unit tests ────────────────────────────────────────

    #[test]
    fn deep_merge_tables() {
        let base: toml::Value = toml::from_str(
            r#"
[a]
x = 1
y = 2
[b]
z = 3
"#,
        )
        .unwrap();

        let over: toml::Value = toml::from_str(
            r#"
[a]
x = 10
"#,
        )
        .unwrap();

        let merged = deep_merge(base, over);
        let tbl = merged.as_table().unwrap();
        let a = tbl["a"].as_table().unwrap();
        assert_eq!(a["x"].as_integer().unwrap(), 10);
        assert_eq!(a["y"].as_integer().unwrap(), 2);
        assert_eq!(tbl["b"].as_table().unwrap()["z"].as_integer().unwrap(), 3);
    }

    #[test]
    fn deep_merge_override_scalar() {
        let base: toml::Value = toml::from_str("val = 1").unwrap();
        let over: toml::Value = toml::from_str("val = 99").unwrap();
        let merged = deep_merge(base, over);
        assert_eq!(merged.as_table().unwrap()["val"].as_integer().unwrap(), 99);
    }
}
