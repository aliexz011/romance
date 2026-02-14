use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub romance_version: String,
    pub created_at: String,
    pub updated_at: String,
    pub project_name: String,
    pub files: BTreeMap<String, FileRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    pub category: FileCategory,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_name: Option<String>,
    pub generated_hash: String,
    pub generated_at: String,
    pub generated_by_version: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileCategory {
    Scaffold,
    Entity,
    Marker,
    Static,
}

impl Manifest {
    pub fn new(project_name: &str, romance_version: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            romance_version: romance_version.to_string(),
            created_at: now.clone(),
            updated_at: now,
            project_name: project_name.to_string(),
            files: BTreeMap::new(),
        }
    }

    pub fn load(project_dir: &Path) -> Result<Self> {
        let path = project_dir.join(".romance/manifest.json");
        let content = std::fs::read_to_string(&path)?;
        let manifest: Manifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    pub fn save(&self, project_dir: &Path) -> Result<()> {
        let dir = project_dir.join(".romance");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("manifest.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn exists(project_dir: &Path) -> bool {
        project_dir.join(".romance/manifest.json").exists()
    }

    pub fn record_file(
        &mut self,
        output_path: &str,
        template: Option<&str>,
        category: FileCategory,
        content: &str,
        entity_name: Option<&str>,
    ) {
        self.files.insert(
            output_path.to_string(),
            FileRecord {
                template: template.map(|s| s.to_string()),
                category,
                entity_name: entity_name.map(|s| s.to_string()),
                generated_hash: content_hash(content),
                generated_at: chrono::Utc::now().to_rfc3339(),
                generated_by_version: self.romance_version.clone(),
            },
        );
    }
}

/// Compute SHA-256 hex digest of content.
pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}
