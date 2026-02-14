use crate::entity::{EntityDefinition, RelationType};
use heck::ToSnakeCase;
use std::path::Path;

/// Check if prerequisites for entity generation are met.
///
/// Returns a list of warning messages for any issues found. Currently checks:
/// - Whether FK target entities exist on disk (for BelongsTo relations).
///   If a target entity does not exist, reverse has-many injection will be skipped.
///
/// M2M and HasMany relations are not checked here because the pending relations
/// system handles deferred application automatically.
pub fn check_entity_prerequisites(entity: &EntityDefinition, project_root: &Path) -> Vec<String> {
    let mut warnings = vec![];

    for rel in &entity.relations {
        if rel.relation_type == RelationType::BelongsTo {
            let target_snake = rel.target_entity.to_snake_case();
            let target_path = project_root.join(format!("backend/src/entities/{}.rs", target_snake));
            if !target_path.exists() {
                warnings.push(format!(
                    "Warning: Target entity '{}' does not exist yet. \
                     Has-many reverse relation will not be injected.",
                    rel.target_entity
                ));
            }
        }
    }

    warnings
}
