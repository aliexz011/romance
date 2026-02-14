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
        romance_core::generator::backend::generate(&entity).unwrap();
        romance_core::generator::migration::generate(&entity).unwrap();
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
        romance_core::generator::backend::generate(&entity).unwrap();
        romance_core::generator::migration::generate(&entity).unwrap();
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
        romance_core::generator::backend::generate(&entity).unwrap();
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
        romance_core::generator::backend::generate(&entity).unwrap();
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
        romance_core::generator::backend::generate(&category).unwrap();
        romance_core::generator::migration::generate(&category).unwrap();

        // Then generate the entity with FK relation
        romance_core::generator::backend::generate(&product).unwrap();
        romance_core::generator::migration::generate(&product).unwrap();
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
        romance_core::generator::frontend::generate(&entity).unwrap();
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
        romance_core::generator::frontend::generate(&entity).unwrap();
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
        romance_core::generator::backend::generate(&user).unwrap();
        romance_core::generator::migration::generate(&user).unwrap();

        romance_core::generator::backend::generate(&post).unwrap();
        romance_core::generator::migration::generate(&post).unwrap();
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
        romance_core::generator::backend::generate(&entity).unwrap();
        romance_core::generator::backend::generate(&entity).unwrap();
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
        romance_core::generator::backend::generate(&user).unwrap();
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
