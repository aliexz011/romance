use anyhow::Result;
use heck::ToSnakeCase;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Discover existing entity module names by scanning backend/src/entities/ directory.
pub fn discover_entities(project_root: &Path) -> Result<Vec<String>> {
    let entities_dir = project_root.join("backend/src/entities");
    let mut entities = Vec::new();

    if !entities_dir.exists() {
        return Ok(entities);
    }

    for entry in fs::read_dir(&entities_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "rs" {
                if let Some(stem) = path.file_stem() {
                    let name = stem.to_string_lossy().to_string();
                    if name != "mod" {
                        entities.push(name);
                    }
                }
            }
        }
    }

    Ok(entities)
}

/// Check if an entity exists on disk.
pub fn entity_exists(project_root: &Path, entity_name: &str) -> bool {
    let snake = entity_name.to_snake_case();
    project_root
        .join(format!("backend/src/entities/{}.rs", snake))
        .exists()
}

/// Get the junction table name for two entities (alphabetical order).
pub fn junction_name(entity_a: &str, entity_b: &str) -> String {
    let a = entity_a.to_snake_case();
    let b = entity_b.to_snake_case();
    if a < b {
        format!("{}_{}", a, b)
    } else {
        format!("{}_{}", b, a)
    }
}

/// A pending relation that couldn't be fully applied because the target entity didn't exist yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRelation {
    pub source_entity: String,
    pub target_entity: String,
    pub relation_type: String,
}

const PENDING_FILE: &str = ".romance/pending_relations.json";

/// Store a pending relation for later application.
pub fn store_pending(project_root: &Path, pending: PendingRelation) -> Result<()> {
    let path = project_root.join(PENDING_FILE);
    let mut pendings = load_pending(project_root)?;

    // Avoid duplicates
    let already = pendings.iter().any(|p| {
        p.source_entity == pending.source_entity
            && p.target_entity == pending.target_entity
            && p.relation_type == pending.relation_type
    });
    if already {
        return Ok(());
    }

    pendings.push(pending);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(&pendings)?)?;
    Ok(())
}

/// Load all pending relations.
pub fn load_pending(project_root: &Path) -> Result<Vec<PendingRelation>> {
    let path = project_root.join(PENDING_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)?;
    let pendings: Vec<PendingRelation> = serde_json::from_str(&content)?;
    Ok(pendings)
}

/// Remove resolved pending relations for a given target entity and return them.
pub fn take_pending_for(project_root: &Path, target_entity: &str) -> Result<Vec<PendingRelation>> {
    let mut all = load_pending(project_root)?;
    let target_snake = target_entity.to_snake_case();

    let (matched, remaining): (Vec<_>, Vec<_>) = all.drain(..).partition(|p| {
        p.target_entity.to_snake_case() == target_snake
    });

    if !matched.is_empty() {
        let path = project_root.join(PENDING_FILE);
        if remaining.is_empty() {
            let _ = fs::remove_file(&path);
        } else {
            fs::write(&path, serde_json::to_string_pretty(&remaining)?)?;
        }
    }

    Ok(matched)
}
