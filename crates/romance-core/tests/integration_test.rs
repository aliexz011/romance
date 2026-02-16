//! Integration tests for the Romance code generation pipeline.
//!
//! These tests exercise the full generation pipeline without requiring
//! a database or network. They create temp directories, scaffold projects,
//! generate entities, and verify the expected files exist with expected content.
//!
//! NOTE: `scaffold::create_project()` unconditionally runs `npm install` and
//! `npx shadcn`. The scaffold tests that call `create_project` are marked
//! `#[ignore]` because they depend on npm being available. Run them explicitly
//! with `cargo test -- --ignored` when npm is present.
//!
//! For entity generation tests, we manually set up the minimal project
//! structure (marker files) so that entity generators can work without
//! needing a full scaffold (and thus without npm).
//!
//! IMPORTANT: The generators use relative paths from the current working
//! directory (e.g., `Path::new("backend/src")`). Since tests share a single
//! process and `set_current_dir` affects all threads, tests that change the
//! working directory MUST be serialized via the CWD_LOCK mutex.

use std::fs;
use std::path::Path;
use std::sync::Mutex;

use romance_core::addon::Addon;

/// Global mutex to serialize tests that change the working directory.
/// The generators use relative paths, so concurrent cwd changes would
/// cause tests to interfere with each other.
static CWD_LOCK: Mutex<()> = Mutex::new(());

/// Helper: create a minimal project structure that entity generators need.
/// This avoids calling `create_project()` (which runs npm), while still
/// providing the marker files and romance.toml required by generators.
fn setup_minimal_project(project_dir: &Path) {
    // Create directory structure
    fs::create_dir_all(project_dir.join("backend/src/entities")).unwrap();
    fs::create_dir_all(project_dir.join("backend/src/handlers")).unwrap();
    fs::create_dir_all(project_dir.join("backend/src/routes")).unwrap();
    fs::create_dir_all(project_dir.join("backend/migration/src")).unwrap();
    fs::create_dir_all(project_dir.join("frontend/src/features")).unwrap();
    fs::create_dir_all(project_dir.join(".romance")).unwrap();

    // entities/mod.rs with marker
    fs::write(
        project_dir.join("backend/src/entities/mod.rs"),
        "// === ROMANCE:MODS ===\n",
    )
    .unwrap();

    // handlers/mod.rs with marker
    fs::write(
        project_dir.join("backend/src/handlers/mod.rs"),
        "// === ROMANCE:MODS ===\n",
    )
    .unwrap();

    // routes/mod.rs with markers
    fs::write(
        project_dir.join("backend/src/routes/mod.rs"),
        r#"// === ROMANCE:MODS ===

use axum::Router;
use crate::db::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // === ROMANCE:ROUTES ===
        // === ROMANCE:MIDDLEWARE ===
}
"#,
    )
    .unwrap();

    // migration/src/lib.rs with markers
    fs::write(
        project_dir.join("backend/migration/src/lib.rs"),
        r#"pub use sea_orm_migration::prelude::*;

// === ROMANCE:MIGRATION_MODS ===

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // === ROMANCE:MIGRATIONS ===
        ]
    }
}
"#,
    )
    .unwrap();

    // Frontend App.tsx with markers
    fs::write(
        project_dir.join("frontend/src/App.tsx"),
        r#"import { BrowserRouter, Routes, Route } from "react-router-dom";
// === ROMANCE:IMPORTS ===

function App() {
  return (
    <BrowserRouter>
      <Routes>
        {/* === ROMANCE:APP_ROUTES === */}
      </Routes>
      <nav>
        {/* === ROMANCE:NAV_LINKS === */}
      </nav>
    </BrowserRouter>
  );
}

export default App;
"#,
    )
    .unwrap();

    // romance.toml (required by generators that load config)
    fs::write(
        project_dir.join("romance.toml"),
        r#"[project]
name = "test-app"

[backend]
port = 3001
database_url = "postgres://localhost/test"

[frontend]
port = 5173
api_base_url = "http://localhost:3001"
"#,
    )
    .unwrap();
}

/// Run a closure with the cwd set to the given project directory.
/// Acquires the CWD_LOCK to prevent concurrent cwd changes.
/// Always restores the original cwd, even on panic.
fn with_cwd<F, R>(project_dir: &Path, f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = CWD_LOCK.lock().unwrap();
    let original_dir = std::env::current_dir().unwrap();
    std::env::set_current_dir(project_dir).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    std::env::set_current_dir(&original_dir).unwrap();
    match result {
        Ok(val) => val,
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

// ==========================================================================
// Scaffold tests (require npm -- marked #[ignore])
// ==========================================================================

#[test]
#[ignore]
fn test_scaffold_creates_expected_files() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_name = "test-scaffold-app";
    let project_dir = dir.path().join(project_name);

    with_cwd(dir.path(), || {
        romance_core::scaffold::create_project(project_name).unwrap();
    });

    // Backend files
    assert!(project_dir.join("backend/Cargo.toml").exists());
    assert!(project_dir.join("backend/src/main.rs").exists());
    assert!(project_dir.join("backend/src/config.rs").exists());
    assert!(project_dir.join("backend/src/db.rs").exists());
    assert!(project_dir.join("backend/src/errors.rs").exists());
    assert!(project_dir.join("backend/src/api.rs").exists());
    assert!(project_dir.join("backend/src/pagination.rs").exists());
    assert!(project_dir.join("backend/src/routes/mod.rs").exists());
    assert!(project_dir.join("backend/src/entities/mod.rs").exists());
    assert!(project_dir.join("backend/src/handlers/mod.rs").exists());

    // Migration crate
    assert!(project_dir.join("backend/migration/Cargo.toml").exists());
    assert!(project_dir.join("backend/migration/src/lib.rs").exists());
    assert!(project_dir.join("backend/migration/src/main.rs").exists());

    // Frontend files
    assert!(project_dir.join("frontend/package.json").exists());
    assert!(project_dir.join("frontend/vite.config.ts").exists());
    assert!(project_dir.join("frontend/tsconfig.json").exists());
    assert!(project_dir.join("frontend/src/App.tsx").exists());
    assert!(project_dir.join("frontend/src/main.tsx").exists());
    assert!(project_dir.join("frontend/index.html").exists());

    // Config and metadata
    assert!(project_dir.join("romance.toml").exists());
    assert!(project_dir.join("README.md").exists());
    assert!(project_dir.join(".gitignore").exists());
    assert!(project_dir.join(".romance/manifest.json").exists());

    // Docker and CI
    assert!(project_dir.join("Dockerfile").exists());
    assert!(project_dir.join("docker-compose.yml").exists());
    assert!(project_dir.join(".github/workflows/ci.yml").exists());
}

#[test]
#[ignore]
fn test_scaffold_marker_files_contain_markers() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_name = "test-markers-app";
    let project_dir = dir.path().join(project_name);

    with_cwd(dir.path(), || {
        romance_core::scaffold::create_project(project_name).unwrap();
    });

    // Verify markers exist in key files
    let entities_mod = fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert!(entities_mod.contains("// === ROMANCE:MODS ==="));

    let handlers_mod = fs::read_to_string(project_dir.join("backend/src/handlers/mod.rs")).unwrap();
    assert!(handlers_mod.contains("// === ROMANCE:MODS ==="));

    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert!(routes_mod.contains("// === ROMANCE:MODS ==="));
    assert!(routes_mod.contains("// === ROMANCE:ROUTES ==="));

    let migration_lib =
        fs::read_to_string(project_dir.join("backend/migration/src/lib.rs")).unwrap();
    assert!(migration_lib.contains("// === ROMANCE:MIGRATION_MODS ==="));
    assert!(migration_lib.contains("// === ROMANCE:MIGRATIONS ==="));
}

// ==========================================================================
// Entity generation tests (no npm required)
// ==========================================================================

#[test]
fn test_entity_generation_creates_backend_files() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("entity-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Product",
        &[
            "title:string".to_string(),
            "price:decimal".to_string(),
            "description:text?".to_string(),
        ],
    )
    .unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Backend entity files should exist
    assert!(
        project_dir
            .join("backend/src/entities/product.rs")
            .exists(),
        "Entity model file should exist"
    );
    assert!(
        project_dir
            .join("backend/src/handlers/product.rs")
            .exists(),
        "Handler file should exist"
    );
    assert!(
        project_dir.join("backend/src/routes/product.rs").exists(),
        "Routes file should exist"
    );

    // Migration file should exist (pattern: m{timestamp}_create_product_table.rs)
    let migration_dir = project_dir.join("backend/migration/src");
    let migration_files: Vec<_> = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .contains("create_product_table")
        })
        .collect();
    assert!(
        !migration_files.is_empty(),
        "Migration file for Product should exist"
    );
}

#[test]
fn test_entity_generation_registers_modules() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("register-test");
    setup_minimal_project(&project_dir);

    let entity =
        romance_core::entity::parse_entity("Category", &["name:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Check that module was registered in entities/mod.rs
    let entities_mod =
        fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert!(
        entities_mod.contains("pub mod category;"),
        "Entity mod should be registered in entities/mod.rs"
    );

    // Check that module was registered in handlers/mod.rs
    let handlers_mod =
        fs::read_to_string(project_dir.join("backend/src/handlers/mod.rs")).unwrap();
    assert!(
        handlers_mod.contains("pub mod category;"),
        "Handler mod should be registered in handlers/mod.rs"
    );

    // Check that module and route was registered in routes/mod.rs
    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert!(
        routes_mod.contains("pub mod category;"),
        "Route mod should be registered in routes/mod.rs"
    );
    assert!(
        routes_mod.contains(".merge(category::router())"),
        "Route should be registered in routes/mod.rs"
    );

    // Check that migration was registered in lib.rs
    let migration_lib =
        fs::read_to_string(project_dir.join("backend/migration/src/lib.rs")).unwrap();
    assert!(
        migration_lib.contains("mod m"),
        "Migration module should be declared in lib.rs"
    );
    assert!(
        migration_lib.contains("create_category_table"),
        "Migration for category table should be declared in lib.rs"
    );
}

#[test]
fn test_entity_model_contains_fields() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("fields-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Article",
        &[
            "title:string".to_string(),
            "body:text".to_string(),
            "published:bool".to_string(),
            "view_count:int".to_string(),
        ],
    )
    .unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let model_content =
        fs::read_to_string(project_dir.join("backend/src/entities/article.rs")).unwrap();

    // Check that field names appear in the generated model
    assert!(
        model_content.contains("title"),
        "Model should contain 'title' field"
    );
    assert!(
        model_content.contains("body"),
        "Model should contain 'body' field"
    );
    assert!(
        model_content.contains("published"),
        "Model should contain 'published' field"
    );
    assert!(
        model_content.contains("view_count"),
        "Model should contain 'view_count' field"
    );
}

#[test]
fn test_entity_with_optional_field() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("optional-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Post",
        &[
            "title:string".to_string(),
            "subtitle:string?".to_string(),
        ],
    )
    .unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let model_content =
        fs::read_to_string(project_dir.join("backend/src/entities/post.rs")).unwrap();

    // The optional field should result in Option<T> in the model
    assert!(
        model_content.contains("Option<"),
        "Optional field should generate Option<T> type"
    );
}

#[test]
fn test_entity_with_belongs_to_relation() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("belongs-to-test");
    setup_minimal_project(&project_dir);

    let category =
        romance_core::entity::parse_entity("Category", &["name:string".to_string()]).unwrap();

    let product = romance_core::entity::parse_entity(
        "Product",
        &[
            "title:string".to_string(),
            "category_id:uuid->Category".to_string(),
        ],
    )
    .unwrap();

    with_cwd(&project_dir, || {
        // First generate the target entity (Category)
        romance_core::generator::backend::generate(&category, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&category, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        // Then generate the entity with FK relation
        romance_core::generator::backend::generate(&product, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&product, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Product model should reference Category
    let product_model =
        fs::read_to_string(project_dir.join("backend/src/entities/product.rs")).unwrap();
    assert!(
        product_model.contains("category_id"),
        "Product model should contain category_id field"
    );
    assert!(
        product_model.contains("Category"),
        "Product model should reference Category relation"
    );

    // Category model should have reverse relation injected
    let category_model =
        fs::read_to_string(project_dir.join("backend/src/entities/category.rs")).unwrap();
    assert!(
        category_model.contains("product"),
        "Category model should have reverse has-many relation to Product injected"
    );

    // Category handlers should have list_products handler injected
    let category_handlers =
        fs::read_to_string(project_dir.join("backend/src/handlers/category.rs")).unwrap();
    assert!(
        category_handlers.contains("list_products"),
        "Category handlers should have list_products handler injected"
    );

    // Category routes should have products route injected
    let category_routes =
        fs::read_to_string(project_dir.join("backend/src/routes/category.rs")).unwrap();
    assert!(
        category_routes.contains("products"),
        "Category routes should have products route injected"
    );
}

#[test]
fn test_entity_frontend_generation() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("frontend-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Task",
        &[
            "title:string".to_string(),
            "completed:bool".to_string(),
        ],
    )
    .unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let feature_dir = project_dir.join("frontend/src/features/task");

    // Frontend feature files should exist
    assert!(feature_dir.join("types.ts").exists(), "types.ts should exist");
    assert!(feature_dir.join("api.ts").exists(), "api.ts should exist");
    assert!(feature_dir.join("hooks.ts").exists(), "hooks.ts should exist");
    assert!(
        feature_dir.join("TaskList.tsx").exists(),
        "TaskList.tsx should exist"
    );
    assert!(
        feature_dir.join("TaskForm.tsx").exists(),
        "TaskForm.tsx should exist"
    );
    assert!(
        feature_dir.join("TaskDetail.tsx").exists(),
        "TaskDetail.tsx should exist"
    );
}

#[test]
fn test_entity_frontend_types_contain_fields() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("frontend-fields-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Invoice",
        &[
            "amount:decimal".to_string(),
            "due_date:date".to_string(),
            "notes:text?".to_string(),
        ],
    )
    .unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let types_content = fs::read_to_string(
        project_dir.join("frontend/src/features/invoice/types.ts"),
    )
    .unwrap();

    assert!(
        types_content.contains("amount"),
        "Types should contain 'amount' field"
    );
    assert!(
        types_content.contains("due_date"),
        "Types should contain 'due_date' field"
    );
    assert!(
        types_content.contains("notes"),
        "Types should contain 'notes' field"
    );
}

#[test]
fn test_multiple_entity_generation() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("multi-entity-test");
    setup_minimal_project(&project_dir);

    let user =
        romance_core::entity::parse_entity("User", &["email:string".to_string()]).unwrap();

    let post = romance_core::entity::parse_entity(
        "Post",
        &[
            "title:string".to_string(),
            "body:text".to_string(),
        ],
    )
    .unwrap();

    with_cwd(&project_dir, || {
        // Generate two entities
        romance_core::generator::backend::generate(&user, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&user, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        romance_core::generator::backend::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Both entities should be registered
    let entities_mod =
        fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert!(entities_mod.contains("pub mod user;"));
    assert!(entities_mod.contains("pub mod post;"));

    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert!(routes_mod.contains("pub mod user;"));
    assert!(routes_mod.contains("pub mod post;"));
    assert!(routes_mod.contains(".merge(user::router())"));
    assert!(routes_mod.contains(".merge(post::router())"));

    // Both entity files should exist
    assert!(project_dir.join("backend/src/entities/user.rs").exists());
    assert!(project_dir.join("backend/src/entities/post.rs").exists());
}

#[test]
fn test_entity_generation_idempotent_registration() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("idempotent-test");
    setup_minimal_project(&project_dir);

    let entity =
        romance_core::entity::parse_entity("Tag", &["name:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        // Generate twice
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Module should appear exactly once in mod.rs files (idempotent)
    let entities_mod =
        fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert_eq!(
        entities_mod.matches("pub mod tag;").count(),
        1,
        "Tag module should be registered exactly once in entities/mod.rs"
    );

    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert_eq!(
        routes_mod.matches("pub mod tag;").count(),
        1,
        "Tag module should be registered exactly once in routes/mod.rs"
    );
    assert_eq!(
        routes_mod.matches(".merge(tag::router())").count(),
        1,
        "Tag route should be merged exactly once"
    );
}

// ==========================================================================
// Orchestrator / prerequisites tests
// ==========================================================================

#[test]
fn test_check_entity_prerequisites_no_warnings_when_no_relations() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("prereq-test");
    setup_minimal_project(&project_dir);

    let entity =
        romance_core::entity::parse_entity("Simple", &["name:string".to_string()]).unwrap();

    let warnings =
        romance_core::generator::check_entity_prerequisites(&entity, &project_dir);
    assert!(
        warnings.is_empty(),
        "No warnings expected for entity without relations"
    );
}

#[test]
fn test_check_entity_prerequisites_warns_missing_target() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("prereq-warn-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Post",
        &["author_id:uuid->User".to_string()],
    )
    .unwrap();

    let warnings =
        romance_core::generator::check_entity_prerequisites(&entity, &project_dir);
    assert!(
        !warnings.is_empty(),
        "Should warn when FK target entity does not exist"
    );
    assert!(
        warnings[0].contains("User"),
        "Warning should mention the missing target entity 'User'"
    );
}

#[test]
fn test_check_entity_prerequisites_no_warning_when_target_exists() {
    // no license gate

    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("prereq-exists-test");
    setup_minimal_project(&project_dir);

    let user =
        romance_core::entity::parse_entity("User", &["email:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        // Generate the target entity first
        romance_core::generator::backend::generate(&user, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Now check prerequisites for an entity that references User
    let post = romance_core::entity::parse_entity(
        "Post",
        &["author_id:uuid->User".to_string()],
    )
    .unwrap();

    let warnings =
        romance_core::generator::check_entity_prerequisites(&post, &project_dir);
    assert!(
        warnings.is_empty(),
        "No warnings expected when FK target entity exists"
    );
}

#[test]
fn test_check_entity_prerequisites_multiple_missing_targets() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("prereq-multi-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Comment",
        &[
            "text:text".to_string(),
            "author_id:uuid->User".to_string(),
            "post_id:uuid->Post".to_string(),
        ],
    )
    .unwrap();

    let warnings =
        romance_core::generator::check_entity_prerequisites(&entity, &project_dir);
    assert_eq!(
        warnings.len(),
        2,
        "Should have two warnings for two missing FK targets"
    );
}

#[test]
fn test_check_entity_prerequisites_ignores_m2m_and_has_many() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("prereq-m2m-test");
    setup_minimal_project(&project_dir);

    // M2M and HasMany relations are handled by the pending relations system,
    // not by the prerequisites check
    let entity = romance_core::entity::parse_entity(
        "Post",
        &[
            "title:string".to_string(),
            "tags:m2m->Tag".to_string(),
            "comments:has_many->Comment".to_string(),
        ],
    )
    .unwrap();

    let warnings =
        romance_core::generator::check_entity_prerequisites(&entity, &project_dir);
    assert!(
        warnings.is_empty(),
        "M2M and HasMany relations should not trigger warnings"
    );
}

// ==========================================================================
// Pre-validation tests
// ==========================================================================

#[test]
fn test_backend_validate_fails_on_missing_marker() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("validate-fail-test");

    // Create directory structure WITHOUT markers
    fs::create_dir_all(project_dir.join("backend/src/entities")).unwrap();
    fs::create_dir_all(project_dir.join("backend/src/handlers")).unwrap();
    fs::create_dir_all(project_dir.join("backend/src/routes")).unwrap();

    // Write files without markers
    fs::write(
        project_dir.join("backend/src/routes/mod.rs"),
        "// no markers here\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("backend/src/entities/mod.rs"),
        "// no markers here\n",
    )
    .unwrap();
    fs::write(
        project_dir.join("backend/src/handlers/mod.rs"),
        "// no markers here\n",
    )
    .unwrap();

    let entity =
        romance_core::entity::parse_entity("Product", &["title:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        let result = romance_core::generator::backend::validate(&entity);
        assert!(result.is_err(), "Validation should fail when markers are missing");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Pre-validation failed"), "Error should mention pre-validation");
    });
}

#[test]
fn test_prevalidation_prevents_file_creation() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("no-write-test");

    // Set up a project where backend markers exist but migration markers are MISSING
    setup_minimal_project(&project_dir);

    // Corrupt migration lib.rs by removing markers
    fs::write(
        project_dir.join("backend/migration/src/lib.rs"),
        "// markers removed\n",
    )
    .unwrap();

    let entity =
        romance_core::entity::parse_entity("Widget", &["name:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        // Backend validation should pass
        assert!(
            romance_core::generator::backend::validate(&entity).is_ok(),
            "Backend validation should pass"
        );

        // Migration validation should fail
        let result = romance_core::generator::migration::validate(&entity);
        assert!(
            result.is_err(),
            "Migration validation should fail without markers"
        );

        // Since pre-validation would catch this, no entity files should be written
        // (simulating what the CLI does: validate all, then generate)
        assert!(
            !project_dir
                .join("backend/src/entities/widget.rs")
                .exists(),
            "Entity file should NOT exist since we never called generate"
        );
    });
}

#[test]
fn test_frontend_validate_fails_on_missing_nav_links() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("fe-validate-test");

    fs::create_dir_all(project_dir.join("frontend/src")).unwrap();

    // App.tsx with IMPORTS and APP_ROUTES but missing NAV_LINKS
    fs::write(
        project_dir.join("frontend/src/App.tsx"),
        r#"// === ROMANCE:IMPORTS ===
{/* === ROMANCE:APP_ROUTES === */}
"#,
    )
    .unwrap();

    let entity =
        romance_core::entity::parse_entity("Task", &["title:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        let result = romance_core::generator::frontend::validate(&entity);
        assert!(result.is_err(), "Should fail when NAV_LINKS marker is missing");
        assert!(result.unwrap_err().to_string().contains("NAV_LINKS"));
    });
}

// ==========================================================================
// Auth generation tests
// ==========================================================================

/// Helper: extend minimal project with files auth generator needs.
fn setup_project_for_auth(project_dir: &Path) {
    setup_minimal_project(project_dir);

    // main.rs (auth replaces "mod errors;" to inject "mod auth;")
    fs::write(
        project_dir.join("backend/src/main.rs"),
        "mod errors;\nmod handlers;\nmod entities;\nmod routes;\n",
    )
    .unwrap();

    // Cargo.toml (for adding argon2/jsonwebtoken deps)
    fs::write(
        project_dir.join("backend/Cargo.toml"),
        "[package]\nname = \"test-backend\"\nversion = \"0.1.0\"\n\n[dependencies]\naxum = \"0.8\"\n",
    )
    .unwrap();

    // .env and .env.example (for JWT_SECRET)
    fs::write(project_dir.join("backend/.env"), "DATABASE_URL=postgres://localhost/test\n").unwrap();
    fs::write(project_dir.join("backend/.env.example"), "DATABASE_URL=postgres://localhost/test\n").unwrap();
}

#[test]
fn test_auth_generation_creates_backend_files() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("auth-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();
    });

    // Backend auth files
    assert!(project_dir.join("backend/src/auth.rs").exists(), "auth.rs should exist");
    assert!(project_dir.join("backend/src/entities/user.rs").exists(), "user entity should exist");
    assert!(project_dir.join("backend/src/handlers/auth.rs").exists(), "auth handlers should exist");
    assert!(project_dir.join("backend/src/routes/auth.rs").exists(), "auth routes should exist");

    // Migration file
    let migration_dir = project_dir.join("backend/migration/src");
    let migration_files: Vec<_> = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("create_users_table"))
        .collect();
    assert!(!migration_files.is_empty(), "User migration should exist");
}

#[test]
fn test_auth_generation_creates_frontend_files() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("auth-fe-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();
    });

    let auth_dir = project_dir.join("frontend/src/features/auth");
    assert!(auth_dir.join("types.ts").exists(), "auth types.ts should exist");
    assert!(auth_dir.join("api.ts").exists(), "auth api.ts should exist");
    assert!(auth_dir.join("hooks.ts").exists(), "auth hooks.ts should exist");
    assert!(auth_dir.join("AuthContext.tsx").exists(), "AuthContext.tsx should exist");
    assert!(auth_dir.join("LoginPage.tsx").exists(), "LoginPage.tsx should exist");
    assert!(auth_dir.join("RegisterPage.tsx").exists(), "RegisterPage.tsx should exist");
    assert!(auth_dir.join("ProtectedRoute.tsx").exists(), "ProtectedRoute.tsx should exist");
}

#[test]
fn test_auth_generation_registers_modules() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("auth-reg-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();
    });

    // Modules registered in mod.rs files
    let entities_mod = fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert!(entities_mod.contains("pub mod user;"), "user entity should be registered");

    let handlers_mod = fs::read_to_string(project_dir.join("backend/src/handlers/mod.rs")).unwrap();
    assert!(handlers_mod.contains("pub mod auth;"), "auth handler should be registered");

    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert!(routes_mod.contains("pub mod auth;"), "auth routes should be registered");
    assert!(routes_mod.contains(".merge(auth::router())"), "auth router should be merged");

    // Migration registered
    let migration_lib = fs::read_to_string(project_dir.join("backend/migration/src/lib.rs")).unwrap();
    assert!(migration_lib.contains("create_users_table"), "user migration should be registered");

    // mod auth; added to main.rs
    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod auth;"), "mod auth should be in main.rs");

    // JWT_SECRET added to .env
    let env = fs::read_to_string(project_dir.join("backend/.env")).unwrap();
    assert!(env.contains("JWT_SECRET="), "JWT_SECRET should be in .env");
}

#[test]
fn test_auth_generation_idempotency() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("auth-idem-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();
        // Second call should fail because auth.rs already exists
        let result = romance_core::generator::auth::generate();
        assert!(result.is_err(), "Auth generation should be idempotent");
        assert!(result.unwrap_err().to_string().contains("already generated"));
    });
}

#[test]
fn test_auth_generation_adds_cargo_dependencies() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("auth-deps-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();
    });

    let cargo = fs::read_to_string(project_dir.join("backend/Cargo.toml")).unwrap();
    assert!(cargo.contains("argon2"), "argon2 dependency should be added");
    assert!(cargo.contains("jsonwebtoken"), "jsonwebtoken dependency should be added");
}

// ==========================================================================
// Admin generation tests
// ==========================================================================

#[test]
fn test_admin_generation_requires_auth() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("admin-no-auth-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        let result = romance_core::generator::admin::generate();
        assert!(result.is_err(), "Admin should fail without auth");
        assert!(result.unwrap_err().to_string().contains("Auth must be generated first"));
    });
}

#[test]
fn test_admin_generation_creates_files() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("admin-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        // First generate auth (admin requires it)
        romance_core::generator::auth::generate().unwrap();
        // Then generate admin
        romance_core::generator::admin::generate().unwrap();
    });

    // Frontend admin files
    assert!(project_dir.join("frontend/src/admin/AdminLayout.tsx").exists(), "AdminLayout.tsx should exist");
    assert!(project_dir.join("frontend/src/admin/Dashboard.tsx").exists(), "Dashboard.tsx should exist");
    assert!(project_dir.join("frontend/src/admin/routes.tsx").exists(), "routes.tsx should exist");

    // Backend admin files
    assert!(project_dir.join("backend/src/routes/admin.rs").exists(), "admin routes should exist");
    assert!(project_dir.join("backend/src/handlers/admin.rs").exists(), "admin handlers should exist");
}

#[test]
fn test_admin_generation_registers_modules() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("admin-reg-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();
        romance_core::generator::admin::generate().unwrap();
    });

    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert!(routes_mod.contains("pub mod admin;"), "admin routes should be registered");
    assert!(routes_mod.contains(".merge(admin::router())"), "admin router should be merged");

    let handlers_mod = fs::read_to_string(project_dir.join("backend/src/handlers/mod.rs")).unwrap();
    assert!(handlers_mod.contains("pub mod admin;"), "admin handler should be registered");
}

#[test]
fn test_admin_generation_discovers_entities() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("admin-discover-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();

        // Generate a Product entity before admin
        let product = romance_core::entity::parse_entity(
            "Product",
            &["title:string".to_string(), "price:decimal".to_string()],
        ).unwrap();
        romance_core::generator::backend::generate(&product, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&product, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        romance_core::generator::admin::generate().unwrap();
    });

    // Admin dashboard should reference the Product entity
    let dashboard = fs::read_to_string(project_dir.join("frontend/src/admin/Dashboard.tsx")).unwrap();
    assert!(dashboard.contains("Product") || dashboard.contains("product"),
        "Dashboard should reference discovered Product entity");
}

#[test]
fn test_admin_generation_idempotency() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("admin-idem-test");
    setup_project_for_auth(&project_dir);

    with_cwd(&project_dir, || {
        romance_core::generator::auth::generate().unwrap();
        romance_core::generator::admin::generate().unwrap();
        let result = romance_core::generator::admin::generate();
        assert!(result.is_err(), "Admin generation should be idempotent");
        assert!(result.unwrap_err().to_string().contains("already generated"));
    });
}

// ==========================================================================
// Junction / M2M generation tests
// ==========================================================================

#[test]
fn test_m2m_junction_creates_entity_and_migration() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("m2m-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        // Generate both entities first
        let post = romance_core::entity::parse_entity("Post", &["title:string".to_string()]).unwrap();
        let tag = romance_core::entity::parse_entity("Tag", &["name:string".to_string()]).unwrap();

        romance_core::generator::backend::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        romance_core::generator::backend::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        // Now generate the junction
        romance_core::generator::junction::generate("Post", "Tag").unwrap();
    });

    // Junction entity should exist (alphabetical order: post_tag)
    assert!(
        project_dir.join("backend/src/entities/post_tag.rs").exists(),
        "Junction entity post_tag.rs should exist"
    );

    // Junction migration should exist
    let migration_dir = project_dir.join("backend/migration/src");
    let junction_migrations: Vec<_> = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("create_post_tag_table"))
        .collect();
    assert!(!junction_migrations.is_empty(), "Junction migration should exist");

    // Junction entity registered in entities/mod.rs
    let entities_mod = fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert!(entities_mod.contains("pub mod post_tag;"), "Junction mod should be registered");
}

#[test]
fn test_m2m_injects_related_impls() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("m2m-inject-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        let post = romance_core::entity::parse_entity("Post", &["title:string".to_string()]).unwrap();
        let tag = romance_core::entity::parse_entity("Tag", &["name:string".to_string()]).unwrap();

        romance_core::generator::backend::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::backend::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        romance_core::generator::junction::generate("Post", "Tag").unwrap();
    });

    // Post model should have Related<tag::Entity> via junction
    let post_model = fs::read_to_string(project_dir.join("backend/src/entities/post.rs")).unwrap();
    assert!(post_model.contains("Related<super::tag::Entity>"),
        "Post should have Related<tag::Entity> impl");

    // Tag model should have Related<post::Entity> via junction
    let tag_model = fs::read_to_string(project_dir.join("backend/src/entities/tag.rs")).unwrap();
    assert!(tag_model.contains("Related<super::post::Entity>"),
        "Tag should have Related<post::Entity> impl");
}

#[test]
fn test_m2m_injects_handlers_and_routes() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("m2m-routes-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        let post = romance_core::entity::parse_entity("Post", &["title:string".to_string()]).unwrap();
        let tag = romance_core::entity::parse_entity("Tag", &["name:string".to_string()]).unwrap();

        romance_core::generator::backend::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::backend::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        romance_core::generator::junction::generate("Post", "Tag").unwrap();
    });

    // Post handlers should have M2M handlers injected
    let post_handlers = fs::read_to_string(project_dir.join("backend/src/handlers/post.rs")).unwrap();
    assert!(post_handlers.contains("tags") || post_handlers.contains("tag"),
        "Post handlers should have tag-related M2M handlers");

    // Post routes should have M2M routes injected
    let post_routes = fs::read_to_string(project_dir.join("backend/src/routes/post.rs")).unwrap();
    assert!(post_routes.contains("tags") || post_routes.contains("tag"),
        "Post routes should have tag-related M2M routes");
}

#[test]
fn test_m2m_pending_relations_stored_when_target_missing() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("m2m-pending-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        // Generate Post but NOT Tag
        let post = romance_core::entity::parse_entity("Post", &["title:string".to_string()]).unwrap();
        romance_core::generator::backend::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        // Try to create junction — Tag doesn't exist, should store pending
        romance_core::generator::junction::generate("Post", "Tag").unwrap();
    });

    // Junction entity should NOT exist (target missing)
    assert!(
        !project_dir.join("backend/src/entities/post_tag.rs").exists(),
        "Junction should NOT be created when target entity is missing"
    );

    // Pending relation should be stored
    let pending = romance_core::relation::load_pending(&project_dir).unwrap();
    assert!(!pending.is_empty(), "Pending relation should be stored");
    assert_eq!(pending[0].source_entity, "Post");
    assert_eq!(pending[0].target_entity, "Tag");
    assert_eq!(pending[0].relation_type, "ManyToMany");
}

#[test]
fn test_m2m_pending_relations_applied_when_target_generated() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("m2m-apply-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        // Generate Post with M2M relation to Tag (Tag doesn't exist yet)
        let post = romance_core::entity::parse_entity(
            "Post",
            &["title:string".to_string(), "tags:m2m->Tag".to_string()],
        ).unwrap();
        romance_core::generator::backend::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::backend::generate_relations(&post).unwrap();

        // Pending should exist
        let pending = romance_core::relation::load_pending(Path::new(".")).unwrap();
        assert!(!pending.is_empty(), "Pending M2M should be stored since Tag doesn't exist");

        // Now generate Tag — pending relations should be applied
        let tag = romance_core::entity::parse_entity("Tag", &["name:string".to_string()]).unwrap();
        romance_core::generator::backend::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::backend::generate_relations(&tag).unwrap();

        // Pending should now be empty
        let pending = romance_core::relation::load_pending(Path::new(".")).unwrap();
        assert!(pending.is_empty(), "Pending relations should be consumed after Tag is generated");
    });

    // Junction entity should now exist
    assert!(
        project_dir.join("backend/src/entities/post_tag.rs").exists(),
        "Junction post_tag.rs should be created when pending relation is applied"
    );
}

#[test]
fn test_m2m_junction_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("m2m-idem-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        let post = romance_core::entity::parse_entity("Post", &["title:string".to_string()]).unwrap();
        let tag = romance_core::entity::parse_entity("Tag", &["name:string".to_string()]).unwrap();

        romance_core::generator::backend::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&post, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::backend::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&tag, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        // Generate junction twice — should not fail or duplicate
        romance_core::generator::junction::generate("Post", "Tag").unwrap();
        romance_core::generator::junction::generate("Post", "Tag").unwrap();
    });

    // Junction entity should still exist and mod should be registered once
    let entities_mod = fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert_eq!(
        entities_mod.matches("pub mod post_tag;").count(),
        1,
        "Junction mod should be registered exactly once"
    );
}

#[test]
fn test_m2m_junction_alphabetical_ordering() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("m2m-alpha-test");
    setup_minimal_project(&project_dir);

    with_cwd(&project_dir, || {
        let article = romance_core::entity::parse_entity("Article", &["title:string".to_string()]).unwrap();
        let zebra = romance_core::entity::parse_entity("Zebra", &["name:string".to_string()]).unwrap();

        romance_core::generator::backend::generate(&article, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&article, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::backend::generate(&zebra, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&zebra, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        // Generate with Zebra as source, Article as target
        // Junction should still be article_zebra (alphabetical)
        romance_core::generator::junction::generate("Zebra", "Article").unwrap();
    });

    assert!(
        project_dir.join("backend/src/entities/article_zebra.rs").exists(),
        "Junction should use alphabetical ordering: article_zebra, not zebra_article"
    );
    assert!(
        !project_dir.join("backend/src/entities/zebra_article.rs").exists(),
        "zebra_article should not exist"
    );
}

// ==========================================================================
// Frontend content verification tests
// ==========================================================================

#[test]
fn test_frontend_injects_imports_into_app_tsx() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("fe-imports-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Task",
        &["title:string".to_string(), "completed:bool".to_string()],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let app_tsx = fs::read_to_string(project_dir.join("frontend/src/App.tsx")).unwrap();

    // Should have imports
    assert!(app_tsx.contains("import TaskList"), "App.tsx should import TaskList");
    assert!(app_tsx.contains("import TaskForm"), "App.tsx should import TaskForm");
    assert!(app_tsx.contains("import TaskDetail"), "App.tsx should import TaskDetail");
}

#[test]
fn test_frontend_injects_routes_into_app_tsx() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("fe-routes-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity("Product", &["name:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let app_tsx = fs::read_to_string(project_dir.join("frontend/src/App.tsx")).unwrap();

    // Should have routes
    assert!(app_tsx.contains("/products"), "App.tsx should have /products route");
    assert!(app_tsx.contains("/products/new"), "App.tsx should have /products/new route");
    assert!(app_tsx.contains("/products/:id"), "App.tsx should have /products/:id route");
}

#[test]
fn test_frontend_injects_nav_links() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("fe-nav-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity("Invoice", &["amount:decimal".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let app_tsx = fs::read_to_string(project_dir.join("frontend/src/App.tsx")).unwrap();

    // Should have nav link
    assert!(app_tsx.contains("Invoice"), "App.tsx should have nav link for Invoice");
    assert!(app_tsx.contains("/invoices"), "App.tsx should link to /invoices");
}

#[test]
fn test_frontend_types_ts_contains_correct_typescript_types() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("fe-types-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Event",
        &[
            "name:string".to_string(),
            "capacity:int".to_string(),
            "active:bool".to_string(),
            "start_date:datetime".to_string(),
            "metadata:json?".to_string(),
        ],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let types = fs::read_to_string(project_dir.join("frontend/src/features/event/types.ts")).unwrap();
    assert!(types.contains("name"), "Should contain name field");
    assert!(types.contains("capacity"), "Should contain capacity field");
    assert!(types.contains("active"), "Should contain active field");
    assert!(types.contains("start_date"), "Should contain start_date field");
    assert!(types.contains("metadata"), "Should contain metadata field");
    assert!(types.contains("string"), "Should have string type");
    assert!(types.contains("number"), "Should have number type");
    assert!(types.contains("boolean"), "Should have boolean type");
}

// ==========================================================================
// ROMANCE:CUSTOM block preservation tests
// ==========================================================================

#[test]
fn test_custom_block_preserved_on_regeneration() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("custom-block-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity("Widget", &["name:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        // First generation
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Simulate user adding custom code below the CUSTOM marker
    let model_path = project_dir.join("backend/src/entities/widget.rs");
    let content = fs::read_to_string(&model_path).unwrap();
    let custom_code = "\n// My custom implementation\nfn custom_method() { }\n";
    let new_content = content.replace(
        "// === ROMANCE:CUSTOM ===",
        &format!("// === ROMANCE:CUSTOM ==={}", custom_code),
    );
    fs::write(&model_path, new_content).unwrap();

    // Regenerate
    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Custom code should be preserved
    let final_content = fs::read_to_string(&model_path).unwrap();
    assert!(final_content.contains("custom_method"), "Custom code should be preserved after regeneration");
    assert!(final_content.contains("// === ROMANCE:CUSTOM ==="), "CUSTOM marker should still exist");
}

// ==========================================================================
// All field types tests
// ==========================================================================

#[test]
fn test_entity_with_all_field_types() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("all-types-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "AllTypes",
        &[
            "name:string".to_string(),
            "bio:text".to_string(),
            "active:bool".to_string(),
            "count:int".to_string(),
            "big_count:i64".to_string(),
            "score:float".to_string(),
            "price:decimal".to_string(),
            "ref_id:uuid".to_string(),
            "event_time:datetime".to_string(),
            "birth_date:date".to_string(),
            "metadata:json".to_string(),
        ],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // All backend files should exist
    assert!(project_dir.join("backend/src/entities/all_types.rs").exists());
    assert!(project_dir.join("backend/src/handlers/all_types.rs").exists());
    assert!(project_dir.join("backend/src/routes/all_types.rs").exists());

    // Migration should exist
    let migration_dir = project_dir.join("backend/migration/src");
    let migration_files: Vec<_> = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("create_all_types_table"))
        .collect();
    assert!(!migration_files.is_empty(), "Migration should exist for AllTypes");

    // Frontend files
    let fe_dir = project_dir.join("frontend/src/features/allTypes");
    assert!(fe_dir.join("types.ts").exists());
    assert!(fe_dir.join("api.ts").exists());
    assert!(fe_dir.join("hooks.ts").exists());
    assert!(fe_dir.join("AllTypesList.tsx").exists());
    assert!(fe_dir.join("AllTypesForm.tsx").exists());
    assert!(fe_dir.join("AllTypesDetail.tsx").exists());

    // Model should contain appropriate Rust types
    let model = fs::read_to_string(project_dir.join("backend/src/entities/all_types.rs")).unwrap();
    assert!(model.contains("String"), "Should have String type");
    assert!(model.contains("bool"), "Should have bool type");
    assert!(model.contains("i32"), "Should have i32 type");
    assert!(model.contains("i64"), "Should have i64 type");
    assert!(model.contains("f64"), "Should have f64 type");
    assert!(model.contains("Decimal"), "Should have Decimal type");
    assert!(model.contains("Uuid"), "Should have Uuid type");
    assert!(model.contains("Json"), "Should have Json type");
}

#[test]
fn test_entity_with_file_and_image_types() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("file-image-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "Document",
        &[
            "title:string".to_string(),
            "attachment:file".to_string(),
            "preview:image?".to_string(),
        ],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    assert!(project_dir.join("backend/src/entities/document.rs").exists());

    let fe_form = fs::read_to_string(
        project_dir.join("frontend/src/features/document/DocumentForm.tsx"),
    ).unwrap();
    // File and image fields should have appropriate handling
    assert!(fe_form.contains("attachment") || fe_form.contains("file"),
        "Form should reference file field");
}

// ==========================================================================
// Addon install integration tests
// ==========================================================================

/// Helper: create a minimal project for addon installation.
fn setup_project_for_addon(project_dir: &Path) {
    setup_minimal_project(project_dir);
    fs::write(
        project_dir.join("backend/src/main.rs"),
        "mod errors;\n// === ROMANCE:MAIN_MODS ===\nmod handlers;\nmod entities;\nmod routes;\n",
    ).unwrap();
    fs::write(
        project_dir.join("backend/Cargo.toml"),
        "[package]\nname = \"test-backend\"\nversion = \"0.1.0\"\n\n[dependencies]\naxum = \"0.8\"\n# === ROMANCE:DEPENDENCIES ===\n",
    ).unwrap();
    fs::write(project_dir.join("backend/.env"), "DATABASE_URL=postgres://localhost/test\n").unwrap();
    fs::write(project_dir.join("backend/.env.example"), "DATABASE_URL=postgres://localhost/test\n").unwrap();
    // Create frontend dirs for addons that write to them
    fs::create_dir_all(project_dir.join("frontend/src/components")).unwrap();
    fs::create_dir_all(project_dir.join("frontend/src/features/admin")).unwrap();
    fs::create_dir_all(project_dir.join("frontend/src/features/dev")).unwrap();
    fs::create_dir_all(project_dir.join("frontend/src/features/auth")).unwrap();
}

/// Helper: add auth files for addons that depend on auth.
fn add_auth_stub(project_dir: &Path) {
    fs::write(project_dir.join("backend/src/auth.rs"), "// auth stub\n").unwrap();
}

#[test]
fn test_addon_validation_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-validation-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::validation::ValidationAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/validation.rs").exists(), "validation.rs should be created");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod validation;"), "validation mod should be in main.rs");

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(romance_toml.contains("validation = true"), "validation feature flag should be set");
}

#[test]
fn test_addon_soft_delete_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-soft-delete-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::soft_delete::SoftDeleteAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/soft_delete.rs").exists(), "soft_delete.rs should be created");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod soft_delete;"), "soft_delete mod should be in main.rs");

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(romance_toml.contains("soft_delete = true"), "soft_delete feature flag should be set");
}

#[test]
fn test_addon_security_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-security-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::security::SecurityAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/middleware/security_headers.rs").exists(), "security_headers.rs should exist");
    assert!(project_dir.join("backend/src/middleware/rate_limit.rs").exists(), "rate_limit.rs should exist");
    assert!(project_dir.join("backend/src/middleware/mod.rs").exists(), "middleware/mod.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod middleware;"), "middleware mod should be in main.rs");

    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert!(routes_mod.contains("security_headers"), "security_headers middleware should be injected");
    assert!(routes_mod.contains("rate_limit"), "rate_limit middleware should be injected");

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(romance_toml.contains("[security]"), "security config section should be added");
}

#[test]
fn test_addon_storage_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-storage-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::storage::StorageAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/storage.rs").exists(), "storage.rs should exist");
    assert!(project_dir.join("backend/src/handlers/upload.rs").exists(), "upload handler should exist");
    assert!(project_dir.join("backend/src/routes/upload.rs").exists(), "upload routes should exist");
    assert!(project_dir.join("frontend/src/components/FileUpload.tsx").exists(), "FileUpload component should exist");
    assert!(project_dir.join("backend/uploads").exists(), "uploads directory should exist");

    let routes_mod = fs::read_to_string(project_dir.join("backend/src/routes/mod.rs")).unwrap();
    assert!(routes_mod.contains("pub mod upload;"), "upload routes should be registered");
    assert!(routes_mod.contains(".merge(upload::router())"), "upload router should be merged");

    let env = fs::read_to_string(project_dir.join("backend/.env")).unwrap();
    assert!(env.contains("UPLOAD_DIR"), "UPLOAD_DIR should be in .env");
}

#[test]
fn test_addon_observability_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-obs-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::observability::ObservabilityAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/middleware/request_id.rs").exists(),
        "request_id.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod middleware;"), "middleware mod should be in main.rs");
}

#[test]
fn test_addon_search_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-search-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::search::SearchAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/search.rs").exists(), "search.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod search;"), "search mod should be in main.rs");

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(romance_toml.contains("search = true"), "search feature flag should be set");
}

#[test]
fn test_addon_audit_log_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-audit-test");
    setup_project_for_addon(&project_dir);
    add_auth_stub(&project_dir);

    romance_core::addon::audit_log::AuditLogAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/audit.rs").exists(), "audit.rs should exist");
    assert!(project_dir.join("backend/src/entities/audit_entry.rs").exists(), "audit_entry entity should exist");
    assert!(project_dir.join("backend/src/handlers/audit_log.rs").exists(), "audit_log handler should exist");
    assert!(project_dir.join("frontend/src/features/admin/AuditLog.tsx").exists(), "AuditLog.tsx should exist");

    // Migration should exist
    let migration_dir = project_dir.join("backend/migration/src");
    let migration_files: Vec<_> = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("create_audit_entries_table"))
        .collect();
    assert!(!migration_files.is_empty(), "Audit migration should exist");

    let entities_mod = fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert!(entities_mod.contains("pub mod audit_entry;"), "audit_entry should be registered");

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(romance_toml.contains("audit_log = true"), "audit_log feature flag should be set");
}

#[test]
fn test_addon_email_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-email-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::email::EmailAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/email.rs").exists(), "email.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod email;"), "email mod should be in main.rs");
}

#[test]
fn test_addon_cache_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-cache-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::cache::CacheAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/cache.rs").exists(), "cache.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod cache;"), "cache mod should be in main.rs");
}

#[test]
fn test_addon_tasks_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-tasks-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::tasks::TasksAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/tasks.rs").exists(), "tasks.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod tasks;"), "tasks mod should be in main.rs");
}

#[test]
fn test_addon_websocket_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-ws-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::websocket::WebsocketAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/ws.rs").exists(), "ws.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod ws;"), "ws mod should be in main.rs");
}

#[test]
fn test_addon_i18n_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-i18n-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::i18n::I18nAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/i18n.rs").exists(), "i18n.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod i18n;"), "i18n mod should be in main.rs");
}

#[test]
fn test_addon_dashboard_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-dashboard-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::dashboard::DashboardAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/handlers/dev_dashboard.rs").exists(), "dev_dashboard handler should exist");
    assert!(project_dir.join("backend/src/routes/dev_dashboard.rs").exists(), "dev_dashboard routes should exist");
    assert!(project_dir.join("frontend/src/features/dev/DevDashboard.tsx").exists(), "DevDashboard component should exist");
}

#[test]
fn test_addon_api_keys_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-api-keys-test");
    setup_project_for_addon(&project_dir);
    add_auth_stub(&project_dir);

    romance_core::addon::api_keys::ApiKeysAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/api_keys.rs").exists(), "api_keys.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod api_keys;"), "api_keys mod should be in main.rs");
}

#[test]
fn test_addon_multitenancy_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-mt-test");
    setup_project_for_addon(&project_dir);
    add_auth_stub(&project_dir);

    romance_core::addon::multitenancy::MultitenancyAddon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/tenant.rs").exists(), "tenant.rs should exist");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(main_rs.contains("mod tenant;"), "tenant mod should be in main.rs");

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(romance_toml.contains("multitenancy = true"), "multitenancy feature flag should be set");
}

#[test]
fn test_addon_oauth_install() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-oauth-test");
    setup_project_for_addon(&project_dir);
    add_auth_stub(&project_dir);

    let addon = romance_core::addon::oauth::OauthAddon {
        provider: "google".to_string(),
    };
    addon.install(&project_dir).unwrap();

    assert!(project_dir.join("backend/src/oauth.rs").exists(), "oauth.rs should exist");
    assert!(project_dir.join("backend/src/handlers/oauth.rs").exists(), "oauth handler should exist");
    assert!(project_dir.join("backend/src/routes/oauth.rs").exists(), "oauth routes should exist");
}

// ==========================================================================
// Addon uninstall tests
// ==========================================================================

#[test]
fn test_addon_validation_uninstall() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-val-uninstall-test");
    setup_project_for_addon(&project_dir);

    // Install first
    romance_core::addon::validation::ValidationAddon.install(&project_dir).unwrap();
    assert!(project_dir.join("backend/src/validation.rs").exists());

    // Then uninstall
    romance_core::addon::validation::ValidationAddon.uninstall(&project_dir).unwrap();
    assert!(!project_dir.join("backend/src/validation.rs").exists(), "validation.rs should be deleted");

    let main_rs = fs::read_to_string(project_dir.join("backend/src/main.rs")).unwrap();
    assert!(!main_rs.contains("mod validation;"), "validation mod should be removed from main.rs");
}

#[test]
fn test_addon_security_uninstall() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-sec-uninstall-test");
    setup_project_for_addon(&project_dir);

    // Install first
    romance_core::addon::security::SecurityAddon.install(&project_dir).unwrap();
    assert!(project_dir.join("backend/src/middleware/security_headers.rs").exists());

    // Then uninstall
    romance_core::addon::security::SecurityAddon.uninstall(&project_dir).unwrap();
    assert!(!project_dir.join("backend/src/middleware/security_headers.rs").exists());
    assert!(!project_dir.join("backend/src/middleware/rate_limit.rs").exists());

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(!romance_toml.contains("[security]"), "security section should be removed from romance.toml");
}

#[test]
fn test_addon_soft_delete_uninstall() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("addon-sd-uninstall-test");
    setup_project_for_addon(&project_dir);

    romance_core::addon::soft_delete::SoftDeleteAddon.install(&project_dir).unwrap();
    assert!(project_dir.join("backend/src/soft_delete.rs").exists());

    romance_core::addon::soft_delete::SoftDeleteAddon.uninstall(&project_dir).unwrap();
    assert!(!project_dir.join("backend/src/soft_delete.rs").exists());

    let romance_toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    assert!(!romance_toml.contains("soft_delete = true"), "soft_delete feature flag should be removed");
}

// ==========================================================================
// run_addon helper tests
// ==========================================================================

#[test]
fn test_run_addon_skips_if_already_installed() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("run-addon-skip-test");
    setup_project_for_addon(&project_dir);

    // Install validation first
    romance_core::addon::validation::ValidationAddon.install(&project_dir).unwrap();

    // run_addon should skip since it's already installed
    // (it should not error, just print a skip message)
    romance_core::addon::run_addon(
        &romance_core::addon::validation::ValidationAddon,
        &project_dir,
    ).unwrap();

    // Should still have exactly one validation.rs
    assert!(project_dir.join("backend/src/validation.rs").exists());
}

// ==========================================================================
// Edge cases
// ==========================================================================

#[test]
fn test_entity_with_fk_generates_select_dropdown() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("fk-dropdown-test");
    setup_minimal_project(&project_dir);

    let category = romance_core::entity::parse_entity("Category", &["name:string".to_string()]).unwrap();
    let product = romance_core::entity::parse_entity(
        "Product",
        &["title:string".to_string(), "category_id:uuid->Category".to_string()],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&category, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::frontend::generate(&category, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        romance_core::generator::backend::generate(&product, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::frontend::generate(&product, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Product form should fetch categories for select dropdown
    let form = fs::read_to_string(project_dir.join("frontend/src/features/product/ProductForm.tsx")).unwrap();
    assert!(form.contains("category") || form.contains("Category"),
        "Product form should reference Category for FK select");
    assert!(form.contains("select") || form.contains("Select"),
        "Product form should use select component for FK field");
}

#[test]
fn test_entity_with_validation_rules() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("validation-rules-test");
    setup_minimal_project(&project_dir);

    // Enable validation feature in romance.toml
    let toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    let new_toml = format!("{}\n[features]\nvalidation = true\n", toml.trim_end());
    fs::write(project_dir.join("romance.toml"), new_toml).unwrap();

    let entity = romance_core::entity::parse_entity(
        "Article",
        &[
            "title:string[min=3,max=100]".to_string(),
            "email:string[email]".to_string(),
        ],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::frontend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Backend model should exist
    assert!(project_dir.join("backend/src/entities/article.rs").exists());
    // Frontend form should exist
    assert!(project_dir.join("frontend/src/features/article/ArticleForm.tsx").exists());
}

#[test]
fn test_entity_with_searchable_fields() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("searchable-test");
    setup_minimal_project(&project_dir);

    // Enable search feature
    let toml = fs::read_to_string(project_dir.join("romance.toml")).unwrap();
    let new_toml = format!("{}\n[features]\nsearch = true\n", toml.trim_end());
    fs::write(project_dir.join("romance.toml"), new_toml).unwrap();

    let entity = romance_core::entity::parse_entity(
        "Article",
        &[
            "title:string[searchable]".to_string(),
            "body:text[searchable]".to_string(),
        ],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    assert!(project_dir.join("backend/src/entities/article.rs").exists());

    // Migration should contain search-related setup (GIN index or tsvector)
    let migration_dir = project_dir.join("backend/migration/src");
    let migration_files: Vec<_> = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("create_article_table"))
        .collect();
    assert!(!migration_files.is_empty());
}

#[test]
fn test_multiple_fks_to_same_entity() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("multi-fk-test");
    setup_minimal_project(&project_dir);

    let user = romance_core::entity::parse_entity("User", &["name:string".to_string()]).unwrap();
    let transfer = romance_core::entity::parse_entity(
        "Transfer",
        &[
            "amount:decimal".to_string(),
            "sender_id:uuid->User".to_string(),
            "receiver_id:uuid->User".to_string(),
        ],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&user, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&user, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();

        romance_core::generator::backend::generate(&transfer, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&transfer, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let model = fs::read_to_string(project_dir.join("backend/src/entities/transfer.rs")).unwrap();
    assert!(model.contains("sender_id"), "Model should have sender_id field");
    assert!(model.contains("receiver_id"), "Model should have receiver_id field");
}

#[test]
fn test_migration_timestamp_collision_avoidance() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("timestamp-test");
    setup_minimal_project(&project_dir);

    let entity_a = romance_core::entity::parse_entity("EntityA", &["name:string".to_string()]).unwrap();
    let entity_b = romance_core::entity::parse_entity("EntityB", &["name:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::migration::generate(&entity_a, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
        romance_core::generator::migration::generate(&entity_b, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    // Both migrations should exist with different timestamps
    let migration_dir = project_dir.join("backend/migration/src");
    let migration_files: Vec<_> = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with('m') && name.ends_with(".rs") && name != "main.rs"
        })
        .collect();
    // Should have lib.rs + 2 migration files in the dir (lib.rs is not a migration)
    let migration_count = migration_files.len();
    assert!(
        migration_count >= 2,
        "Should have at least 2 migrations (entity_a and entity_b), got {}",
        migration_count
    );
}

#[test]
fn test_relation_discovery() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("discover-test");
    setup_minimal_project(&project_dir);

    // Create some entity files
    fs::write(project_dir.join("backend/src/entities/user.rs"), "// user entity\n").unwrap();
    fs::write(project_dir.join("backend/src/entities/post.rs"), "// post entity\n").unwrap();
    fs::write(project_dir.join("backend/src/entities/tag.rs"), "// tag entity\n").unwrap();

    let entities = romance_core::relation::discover_entities(&project_dir).unwrap();
    assert!(entities.contains(&"user".to_string()));
    assert!(entities.contains(&"post".to_string()));
    assert!(entities.contains(&"tag".to_string()));
    assert!(!entities.contains(&"mod".to_string()), "mod.rs should be excluded");
}

#[test]
fn test_junction_name_alphabetical() {
    assert_eq!(romance_core::relation::junction_name("Post", "Tag"), "post_tag");
    assert_eq!(romance_core::relation::junction_name("Tag", "Post"), "post_tag");
    assert_eq!(romance_core::relation::junction_name("Article", "Zebra"), "article_zebra");
    assert_eq!(romance_core::relation::junction_name("Zebra", "Article"), "article_zebra");
}

#[test]
fn test_pending_relations_store_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("pending-test");
    fs::create_dir_all(project_dir.join(".romance")).unwrap();

    // Store pending
    romance_core::relation::store_pending(
        &project_dir,
        romance_core::relation::PendingRelation {
            source_entity: "Post".to_string(),
            target_entity: "Tag".to_string(),
            relation_type: "ManyToMany".to_string(),
        },
    ).unwrap();

    // Load and verify
    let pending = romance_core::relation::load_pending(&project_dir).unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].source_entity, "Post");
    assert_eq!(pending[0].target_entity, "Tag");
}

#[test]
fn test_pending_relations_no_duplicates() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("pending-dup-test");
    fs::create_dir_all(project_dir.join(".romance")).unwrap();

    let pending = romance_core::relation::PendingRelation {
        source_entity: "Post".to_string(),
        target_entity: "Tag".to_string(),
        relation_type: "ManyToMany".to_string(),
    };

    // Store twice
    romance_core::relation::store_pending(&project_dir, pending.clone()).unwrap();
    romance_core::relation::store_pending(&project_dir, pending).unwrap();

    // Should only have one
    let all = romance_core::relation::load_pending(&project_dir).unwrap();
    assert_eq!(all.len(), 1, "Duplicate pending relations should be prevented");
}

#[test]
fn test_take_pending_for_removes_matched() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("pending-take-test");
    fs::create_dir_all(project_dir.join(".romance")).unwrap();

    // Store two pending relations for different targets
    romance_core::relation::store_pending(
        &project_dir,
        romance_core::relation::PendingRelation {
            source_entity: "Post".to_string(),
            target_entity: "Tag".to_string(),
            relation_type: "ManyToMany".to_string(),
        },
    ).unwrap();
    romance_core::relation::store_pending(
        &project_dir,
        romance_core::relation::PendingRelation {
            source_entity: "Article".to_string(),
            target_entity: "Category".to_string(),
            relation_type: "ManyToMany".to_string(),
        },
    ).unwrap();

    // Take pending for "Tag"
    let taken = romance_core::relation::take_pending_for(&project_dir, "Tag").unwrap();
    assert_eq!(taken.len(), 1);
    assert_eq!(taken[0].target_entity, "Tag");

    // Remaining should only have Category relation
    let remaining = romance_core::relation::load_pending(&project_dir).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].target_entity, "Category");
}

#[test]
fn test_entity_exists_check() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("exists-test");
    setup_minimal_project(&project_dir);

    // No entity file exists yet
    assert!(!romance_core::relation::entity_exists(&project_dir, "User"));

    // Create user entity
    fs::write(project_dir.join("backend/src/entities/user.rs"), "// user\n").unwrap();
    assert!(romance_core::relation::entity_exists(&project_dir, "User"));

    // PascalCase to snake_case conversion
    assert!(romance_core::relation::entity_exists(&project_dir, "user"));
}

// ==========================================================================
// Tracker rollback integration test
// ==========================================================================

#[test]
fn test_tracker_rollback_cleans_up_generated_files() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("tracker-rollback-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity("Widget", &["name:string".to_string()]).unwrap();

    with_cwd(&project_dir, || {
        let mut tracker = romance_core::generator::plan::GenerationTracker::new();
        romance_core::generator::backend::generate(&entity, &mut tracker).unwrap();

        // Files should exist
        assert!(Path::new("backend/src/entities/widget.rs").exists());
        assert!(Path::new("backend/src/handlers/widget.rs").exists());
        assert!(Path::new("backend/src/routes/widget.rs").exists());

        // Rollback (must be done while cwd is still the project dir, since tracker stores relative paths)
        tracker.rollback();

        // Files should be cleaned up
        assert!(!Path::new("backend/src/entities/widget.rs").exists(),
            "Entity file should be deleted on rollback");
        assert!(!Path::new("backend/src/handlers/widget.rs").exists(),
            "Handler file should be deleted on rollback");
        assert!(!Path::new("backend/src/routes/widget.rs").exists(),
            "Route file should be deleted on rollback");
    });
}

// ==========================================================================
// Seed function generation test
// ==========================================================================

#[test]
fn test_entity_generation_with_seed_file() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("seed-test");
    setup_minimal_project(&project_dir);

    // Create a seed.rs with SEEDS marker
    fs::write(
        project_dir.join("backend/src/seed.rs"),
        "use sea_orm::DatabaseConnection;\nuse anyhow::Result;\n\npub async fn run(db: &DatabaseConnection) -> Result<()> {\n    // === ROMANCE:SEEDS ===\n    Ok(())\n}\n",
    ).unwrap();

    let entity = romance_core::entity::parse_entity(
        "Product",
        &["title:string".to_string(), "price:decimal".to_string()],
    ).unwrap();

    with_cwd(&project_dir, || {
        romance_core::generator::backend::generate(&entity, &mut romance_core::generator::plan::GenerationTracker::new()).unwrap();
    });

    let seed = fs::read_to_string(project_dir.join("backend/src/seed.rs")).unwrap();
    assert!(seed.contains("seed_products"), "Seed function should be injected for Product");
    assert!(seed.contains("// === ROMANCE:SEEDS ==="), "SEEDS marker should be preserved");
}

// ==========================================================================
// Full pipeline orchestration tests
// ==========================================================================

#[test]
fn test_full_entity_pipeline_backend_migration_frontend() {
    let dir = tempfile::tempdir().unwrap();
    let project_dir = dir.path().join("full-pipeline-test");
    setup_minimal_project(&project_dir);

    let entity = romance_core::entity::parse_entity(
        "BlogPost",
        &[
            "title:string".to_string(),
            "body:text".to_string(),
            "published:bool".to_string(),
            "author_email:string".to_string(),
        ],
    ).unwrap();

    with_cwd(&project_dir, || {
        // Simulate the full pipeline as done by the CLI
        romance_core::generator::backend::validate(&entity).unwrap();
        romance_core::generator::migration::validate(&entity).unwrap();
        romance_core::generator::frontend::validate(&entity).unwrap();

        let mut tracker = romance_core::generator::plan::GenerationTracker::new();
        romance_core::generator::backend::generate(&entity, &mut tracker).unwrap();
        romance_core::generator::migration::generate(&entity, &mut tracker).unwrap();
        romance_core::generator::backend::generate_relations(&entity).unwrap();
        romance_core::generator::frontend::generate(&entity, &mut tracker).unwrap();
    });

    // Backend files
    assert!(project_dir.join("backend/src/entities/blog_post.rs").exists());
    assert!(project_dir.join("backend/src/handlers/blog_post.rs").exists());
    assert!(project_dir.join("backend/src/routes/blog_post.rs").exists());

    // Migration
    let migration_dir = project_dir.join("backend/migration/src");
    let has_migration = fs::read_dir(&migration_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .any(|e| e.file_name().to_string_lossy().contains("create_blog_post_table"));
    assert!(has_migration, "Migration should exist");

    // Frontend
    let fe_dir = project_dir.join("frontend/src/features/blogPost");
    assert!(fe_dir.join("types.ts").exists());
    assert!(fe_dir.join("api.ts").exists());
    assert!(fe_dir.join("hooks.ts").exists());
    assert!(fe_dir.join("BlogPostList.tsx").exists());
    assert!(fe_dir.join("BlogPostForm.tsx").exists());
    assert!(fe_dir.join("BlogPostDetail.tsx").exists());

    // Module registration
    let entities_mod = fs::read_to_string(project_dir.join("backend/src/entities/mod.rs")).unwrap();
    assert!(entities_mod.contains("pub mod blog_post;"));

    // App.tsx injection
    let app_tsx = fs::read_to_string(project_dir.join("frontend/src/App.tsx")).unwrap();
    assert!(app_tsx.contains("BlogPost"));
    assert!(app_tsx.contains("/blog-posts") || app_tsx.contains("/blog_posts"));

    // Smart input type: author_email should get "email" input type
    let form = fs::read_to_string(fe_dir.join("BlogPostForm.tsx")).unwrap();
    assert!(form.contains("email"), "author_email field should have email input type");
}
