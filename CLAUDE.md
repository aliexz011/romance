# Romance CLI — Developer & AI Assistant Guide

## Project Overview

**Romance** is a full-stack code generator CLI that scaffolds and extends projects using **Axum + SeaORM** (Rust backend) and **React + TypeScript + shadcn/ui** (frontend). One command creates a complete project; another generates full CRUD entities across the entire stack.

**Stack:** Rust (Axum, SeaORM, Tera) · React 19 · TypeScript · TanStack Query · shadcn/ui · Vite · PostgreSQL

**Binary:** `romance`

## Architecture

### Crate Dependency Graph

```
romance-cli (binary)
  └── romance-core (library: generators, scaffold, entity, template engine)
        └── romance-templates (rust-embed: .tera templates compiled into binary)
```

### Data Flow

```
CLI args/interactive prompt
  → EntityDefinition { name, fields: [FieldDefinition], relations: [RelationDefinition] }
    → TemplateEngine builds Tera Context (field types mapped to target representations)
      → Tera renders .tera templates
        → Files written to disk + markers updated via insert_at_marker()
          → Reverse relations injected into existing entity files
            → Pending relations stored in .romance/ for deferred application
```

### Key Design Decisions

- **rust-embed** — all `.tera` templates in `romance-templates/templates/` are compiled into the binary at build time. No runtime file access needed.
- **Marker system** — generated files use markers to enable re-generation without losing user code.
- **Idempotent inserts** — `insert_at_marker()` checks if a line already exists before inserting.
- **Convention over configuration** — opinionated file structure enables consistent code generation.
- **Manifest tracking** — `.romance/manifest.json` stores SHA-256 hashes of generated files for the `romance update` command.
- **Pending relations** — `.romance/pending_relations.json` stores M2M relations when target entity doesn't exist yet; applied when target is generated later.
- **Reverse relation injection** — generating entity A with FK→B automatically injects has-many code into entity B's files.

## Marker System

Markers are comment lines in generated files that serve as insertion points for code generation.

### Format

All markers follow: `// === ROMANCE:<TYPE> ===`

### Marker Reference

| Marker | Location(s) | Purpose | What gets inserted |
|--------|-------------|---------|-------------------|
| `ROMANCE:MODS` | `backend/src/entities/mod.rs`, `backend/src/handlers/mod.rs`, `backend/src/routes/mod.rs` | Module declarations | `pub mod {entity_snake};` |
| `ROMANCE:ROUTES` | `backend/src/routes/mod.rs` | Route registration | `.merge({entity_snake}::router())` |
| `ROMANCE:MIGRATION_MODS` | `backend/migration/src/lib.rs` | Migration module declarations | `mod m{timestamp}_create_{entity}_table;` |
| `ROMANCE:MIGRATIONS` | `backend/migration/src/lib.rs` | Migration registration in vec | `Box::new(m{timestamp}::Migration),` |
| `ROMANCE:RELATIONS` | `backend/src/entities/{entity}.rs` | Relation impls | `impl Related<T> for Entity` blocks |
| `ROMANCE:RELATION_HANDLERS` | `backend/src/handlers/{entity}.rs` | Relation handlers | has-many list, M2M list/add/remove handlers |
| `ROMANCE:RELATION_ROUTES` | `backend/src/routes/{entity}.rs` | Relation routes | `.route("/api/{entity}s/:id/{related}s", ...)` |
| `ROMANCE:MIDDLEWARE` | `backend/src/routes/mod.rs` | Middleware layers | `.layer(...)` before `.with_state()` |
| `ROMANCE:IMPORTS` | `frontend/src/App.tsx` | Import statements | `import ...` for new routes/components |
| `ROMANCE:APP_ROUTES` | `frontend/src/App.tsx` | Route elements | `<Route path="..." element={...} />` |
| `ROMANCE:CUSTOM` | All entity template outputs | Separates generated from user code | Nothing — content below is preserved on re-generation |

### How `insert_at_marker()` Works

Located in `romance-core/src/utils.rs`:

1. Reads file content
2. Checks if the line already exists (idempotency) — if yes, returns early
3. Replaces `marker` with `{new_line}\n{marker}` — inserts **before** the marker
4. Writes updated content back

### How `ROMANCE:CUSTOM` Works

Located in `romance-core/src/utils.rs`:

- `read_with_custom_block(path)` — splits file at marker, returns `(generated_part, custom_block)`
- `write_generated(path, new_generated)` — if file exists with custom block, appends preserved custom block to new generated content

## Template Context Reference

### Scaffold Templates (`romance new`)

| Variable | Type | Description |
|----------|------|-------------|
| `project_name` | String | Original project name |
| `project_name_snake` | String | snake_case project name |

### Backend Entity Templates

| Variable | Type | Description |
|----------|------|-------------|
| `entity_name` | String | PascalCase entity name |
| `entity_name_snake` | String | snake_case entity name |
| `fields` | Array | Field objects (see below) |

**Backend field object:**

| Key | Source method | Example |
|-----|-------------|---------|
| `name` | — | `"title"` |
| `rust_type` | `to_rust()` | `"String"` |
| `postgres_type` | `to_postgres()` | `"VARCHAR(255)"` |
| `sea_orm_column` | `to_sea_orm_column()` | `"ColumnType::String(StringLen::N(255))"` |
| `optional` | — | `true` / `false` |
| `relation` | — | `"Category"` or `null` |

### Migration Templates

| Key | Source method | Example |
|-----|-------------|---------|
| `name` | — | `"title"` |
| `postgres_type` | `to_postgres()` | `"VARCHAR(255)"` |
| `sea_orm_column` | `to_sea_orm_column()` | `"ColumnType::String(StringLen::N(255))"` |
| `migration_method` | `to_sea_orm_migration()` | `"string_len(255)"` |
| `optional` | — | `true` / `false` |
| `relation` | — | `"Category"` or `null` |

Additional context: `timestamp` (format: `%Y%m%d%H%M%S`)

### Frontend Entity Templates

| Variable | Type | Description |
|----------|------|-------------|
| `entity_name` | String | PascalCase entity name |
| `entity_name_snake` | String | snake_case entity name |
| `entity_name_camel` | String | camelCase entity name |
| `fields` | Array | Field objects (see below) |
| `m2m_relations` | Array | M2M relation objects: `{target, target_snake, target_camel}` |
| `has_many_relations` | Array | Has-many relation objects: `{target, target_snake, target_camel}` |

**Frontend field object:**

| Key | Source method | Example |
|-----|-------------|---------|
| `name` | — | `"title"` |
| `ts_type` | `to_typescript()` | `"string"` |
| `shadcn_component` | `to_shadcn()` | `"Input"` |
| `input_type` | `input_type()` | `"text"` |
| `optional` | — | `true` / `false` |
| `relation` | — | `"Category"` or `null` |

### Tera Filters

| Filter | Implementation | Example |
|--------|---------------|---------|
| `snake_case` | `heck::ToSnakeCase` | `{{ entity_name \| snake_case }}` → `"product_category"` |
| `pascal_case` | `heck::ToPascalCase` | `{{ name \| pascal_case }}` → `"ProductCategory"` |
| `camel_case` | `heck::ToLowerCamelCase` | `{{ entity_name \| camel_case }}` → `"productCategory"` |
| `plural` | Custom (s/x/ch/sh→es, consonant+y→ies, else→s) | `{{ "Category" \| plural }}` → `"Categories"` |

## Type Mapping Reference

| FieldType | CLI aliases | Rust | TypeScript | PostgreSQL | SeaORM Column | Migration method | shadcn | HTML input |
|-----------|-------------|------|------------|------------|---------------|-----------------|--------|------------|
| `String` | `string`, `str` | `String` | `string` | `VARCHAR(255)` | `ColumnType::String(StringLen::N(255))` | `string_len(255)` | `Input` | `text` |
| `Text` | `text` | `String` | `string` | `TEXT` | `ColumnType::Text` | `text()` | `Textarea` | `text` |
| `Bool` | `bool`, `boolean` | `bool` | `boolean` | `BOOLEAN` | `ColumnType::Boolean` | `boolean()` | `Switch` | `text` |
| `Int32` | `i32`, `int`, `int32`, `integer` | `i32` | `number` | `INTEGER` | `ColumnType::Integer` | `integer()` | `Input` | `number` |
| `Int64` | `i64`, `int64`, `bigint` | `i64` | `number` | `BIGINT` | `ColumnType::BigInteger` | `big_integer()` | `Input` | `number` |
| `Float64` | `f64`, `float`, `float64`, `double` | `f64` | `number` | `DOUBLE PRECISION` | `ColumnType::Double` | `double()` | `Input` | `number` |
| `Decimal` | `decimal`, `money` | `Decimal` | `number` | `DECIMAL` | `ColumnType::Decimal(None)` | `decimal()` | `Input` | `number` |
| `Uuid` | `uuid` | `Uuid` | `string` | `UUID` | `ColumnType::Uuid` | `uuid()` | `Input` | `text` |
| `DateTime` | `datetime`, `timestamp` | `DateTimeWithTimeZone` | `string` | `TIMESTAMPTZ` | `ColumnType::TimestampWithTimeZone` | `timestamp_with_time_zone()` | `Input` | `datetime-local` |
| `Date` | `date` | `Date` | `string` | `DATE` | `ColumnType::Date` | `date()` | `Input` | `date` |
| `Json` | `json`, `jsonb` | `Json` | `unknown` | `JSONB` | `ColumnType::JsonBinary` | `json_binary()` | `Textarea` | `text` |
| `Enum(variants)` | — (not in CLI parse) | `String` | `string` | `VARCHAR(255)` | `ColumnType::String(StringLen::N(255))` | `string_len(255)` | `Select` | `text` |
| `File` | `file` | `String` | `string` | `VARCHAR(512)` | `ColumnType::String(StringLen::N(512))` | `string_len(512)` | `FileInput` | `file` |
| `Image` | `image` | `String` | `string` | `VARCHAR(512)` | `ColumnType::String(StringLen::N(512))` | `string_len(512)` | `ImageInput` | `file` |

## Checklists

### Checklist: New Scaffold Template

1. Create `.tera` file in `crates/romance-templates/templates/scaffold/` (backend or frontend subdirectory)
2. Add rendering + file write entry in `crates/romance-core/src/scaffold.rs` — add `(template_path, output_path)` tuple to the appropriate `_files` vec
3. rust-embed picks it up automatically at compile time — no registration needed
4. `cargo check`

### Checklist: New Entity Template

1. Create `.tera` file in `crates/romance-templates/templates/entity/{backend|frontend}/`
2. Add rendering + write call in the corresponding generator:
   - Backend: `crates/romance-core/src/generator/backend.rs` — `engine.render()` + `utils::write_generated()`
   - Frontend: `crates/romance-core/src/generator/frontend.rs` — same pattern
   - Migration: `crates/romance-core/src/generator/migration.rs` — uses `utils::write_file()` (no CUSTOM block)
3. If the new file needs registration via marker — add `utils::insert_at_marker()` call
4. Ensure context has all needed variables (extend `build_context()` if necessary)
5. `cargo check`

### Checklist: New Field Type (FieldType)

1. Add variant to `enum FieldType` in `crates/romance-core/src/entity.rs`
2. Implement **all 7 methods** in the `impl FieldType` block:
   - `to_rust()` — Rust type string
   - `to_typescript()` — TypeScript type string
   - `to_postgres()` — PostgreSQL column type
   - `to_sea_orm_column()` — SeaORM `ColumnType::*`
   - `to_sea_orm_migration()` — SeaORM migration builder method
   - `to_shadcn()` — shadcn/ui component name
   - `input_type()` — HTML input type attribute
3. Add parsing alias(es) in `parse_field_type()` match arms
4. Add entry to `FIELD_TYPE_OPTIONS` const for interactive dialoguer prompt
5. `cargo check`

### Checklist: New CLI Command

1. Add variant to `enum Commands` (or `GenerateCommands` / `DbCommands`) in `crates/romance-cli/src/commands/mod.rs`
2. Create file `crates/romance-cli/src/commands/{name}.rs` with `pub fn run(...) -> Result<()>`
3. Add `pub mod {name};` at top of `crates/romance-cli/src/commands/mod.rs`
4. Add match arm in `pub fn run(cli: Cli)` in `crates/romance-cli/src/commands/mod.rs`
5. If the command needs core logic — add module in `crates/romance-core/src/` and export via `crates/romance-core/src/lib.rs`
6. `cargo check`

### Checklist: New Generator

1. Create `crates/romance-core/src/generator/{name}.rs` with `pub fn generate(...) -> Result<()>`
2. Add `pub mod {name};` in `crates/romance-core/src/generator/mod.rs`
3. Create corresponding `.tera` templates in `crates/romance-templates/templates/entity/`
4. Add call in `crates/romance-cli/src/commands/generate.rs`
5. If needed — add new marker(s) in scaffold templates and handle them in the generator
6. `cargo check`

### Checklist: Adding a Dependency to Generated Projects

| Target | File to edit |
|--------|-------------|
| Backend Cargo.toml | `crates/romance-templates/templates/scaffold/backend/Cargo.toml.tera` |
| Frontend package.json | `crates/romance-templates/templates/scaffold/frontend/package.json.tera` |
| Migration Cargo.toml | `crates/romance-templates/templates/scaffold/backend/migration/Cargo.toml.tera` |

### Checklist: New Addon

1. Create `crates/romance-core/src/addon/{name}.rs` implementing the `Addon` trait
2. Add `pub mod {name};` in `crates/romance-core/src/addon/mod.rs`
3. Create `.tera` template files in `crates/romance-templates/templates/addon/{name}/`
4. Add variant to `AddCommands` enum in `crates/romance-cli/src/commands/mod.rs`
5. Add handler function in `crates/romance-cli/src/commands/add.rs`
6. Add match arm in `run()` dispatcher in `commands/mod.rs`
7. Update `scan_addons()` in `crates/romance-core/src/ai_context.rs`
8. `cargo check`

## File Inventory

### romance-cli

| File | Purpose |
|------|---------|
| `crates/romance-cli/src/main.rs` | Entry point — parses CLI args and calls `run()` |
| `crates/romance-cli/src/commands/mod.rs` | `Cli`, `Commands`, `GenerateCommands`, `DbCommands` enums + `run()` dispatcher |
| `crates/romance-cli/src/commands/new.rs` | `romance new` — delegates to `scaffold::create_project()` |
| `crates/romance-cli/src/commands/generate.rs` | `romance generate entity/auth/admin/types/openapi` — orchestrates generators |
| `crates/romance-cli/src/commands/dev.rs` | `romance dev` — spawns `cargo watch` + `npm run dev` |
| `crates/romance-cli/src/commands/check.rs` | `romance check` — runs `cargo check`, `cargo test`, `npx tsc --noEmit` |
| `crates/romance-cli/src/commands/db.rs` | `romance db migrate/rollback/status/seed` — delegates to migration binary |
| `crates/romance-cli/src/commands/update.rs` | `romance update` — interactive scaffold update with conflict resolution |
| `crates/romance-cli/src/commands/add.rs` | `romance add <feature>` — dispatches to addon installers |
| `crates/romance-cli/src/commands/test.rs` | `romance test` — runs tests with temporary database |

### romance-core

| File | Purpose |
|------|---------|
| `crates/romance-core/src/lib.rs` | Re-exports: `addon`, `ai_context`, `config`, `entity`, `generator`, `manifest`, `relation`, `scaffold`, `seed`, `template`, `test_runner`, `updater`, `utils` |
| `crates/romance-core/src/entity.rs` | `EntityDefinition`, `FieldDefinition`, `FieldType`, `RelationType`, `RelationDefinition`, CLI parsing, interactive prompts |
| `crates/romance-core/src/template.rs` | `TemplateEngine` — loads embedded templates, registers Tera filters |
| `crates/romance-core/src/scaffold.rs` | `create_project()` — renders scaffold templates, creates directory structure, writes manifest |
| `crates/romance-core/src/utils.rs` | `write_file()`, `read_with_custom_block()`, `write_generated()`, `insert_at_marker()` |
| `crates/romance-core/src/config.rs` | Project configuration (reads `romance.toml`) |
| `crates/romance-core/src/relation.rs` | Entity discovery, `entity_exists()`, `junction_name()`, pending relation storage |
| `crates/romance-core/src/manifest.rs` | `Manifest` struct, `FileRecord` (with per-file `generated_by_version`), `content_hash()` — tracks generated file state |
| `crates/romance-core/src/ai_context.rs` | Generates project-level `CLAUDE.md` by scanning entities, relations, auth, admin — auto-updated on every generation |
| `crates/romance-core/src/updater.rs` | `UpdatePlan`, `plan_update()`, `apply_update()`, `generate_diff()` — scaffold update logic |
| `crates/romance-core/src/generator/mod.rs` | Re-exports: `admin`, `auth`, `backend`, `frontend`, `junction`, `migration`, `openapi`, `types` |
| `crates/romance-core/src/generator/backend.rs` | Generates entity model, handlers, routes; injects reverse relations (has-many, M2M) |
| `crates/romance-core/src/generator/frontend.rs` | Generates types, API client, hooks, List/Form/Detail; generates M2M relation hooks |
| `crates/romance-core/src/generator/migration.rs` | Generates timestamped migration file; registers in `lib.rs` via markers |
| `crates/romance-core/src/generator/junction.rs` | Generates M2M junction table entity + migration; injects Related impls + handlers + routes |
| `crates/romance-core/src/generator/auth.rs` | Generates JWT auth module, user entity, auth handlers/routes, frontend auth components |
| `crates/romance-core/src/generator/admin.rs` | Generates admin panel: layout, dashboard, routes; discovers existing entities |
| `crates/romance-core/src/generator/types.rs` | Runs `cargo test -- export_bindings` (ts-rs) |
| `crates/romance-core/src/generator/openapi.rs` | Runs `cargo run --bin openapi-export` (utoipa) |
| `crates/romance-core/src/addon/mod.rs` | Addon trait + registry + `run_addon()` helper |
| `crates/romance-core/src/addon/validation.rs` | Validation addon installer (validator + Zod) |
| `crates/romance-core/src/addon/security.rs` | Security addon installer (rate limit, headers) |
| `crates/romance-core/src/addon/observability.rs` | Observability addon installer (tracing, request ID) |
| `crates/romance-core/src/addon/storage.rs` | Storage addon installer (file/image upload) |
| `crates/romance-core/src/addon/soft_delete.rs` | Soft-delete addon installer |
| `crates/romance-core/src/addon/audit_log.rs` | Audit log addon installer (requires auth) |
| `crates/romance-core/src/addon/oauth.rs` | OAuth addon installer (google/github/discord) |
| `crates/romance-core/src/addon/search.rs` | Full-text search addon installer |
| `crates/romance-core/src/addon/dashboard.rs` | Dev dashboard addon installer |
| `crates/romance-core/src/seed.rs` | Seed file generator for `romance db seed` |
| `crates/romance-core/src/test_runner.rs` | Test runner with temporary database for `romance test` |

### romance-templates

| File | Purpose |
|------|---------|
| `crates/romance-templates/src/lib.rs` | `Templates` struct with `#[derive(RustEmbed)]` pointing to `templates/` |

### Entity Templates (used by `romance generate entity`)

| Template | Output | Generator |
|----------|--------|-----------|
| `entity/backend/model.rs.tera` | `backend/src/entities/{entity}.rs` | `backend.rs` |
| `entity/backend/handlers.rs.tera` | `backend/src/handlers/{entity}.rs` | `backend.rs` |
| `entity/backend/routes.rs.tera` | `backend/src/routes/{entity}.rs` | `backend.rs` |
| `entity/backend/migration.rs.tera` | `backend/migration/src/m{ts}_create_{entity}_table.rs` | `migration.rs` |
| `entity/backend/junction_model.rs.tera` | `backend/src/entities/{a}_{b}.rs` | `junction.rs` |
| `entity/backend/junction_migration.rs.tera` | `backend/migration/src/m{ts}_create_{a}_{b}_table.rs` | `junction.rs` |
| `entity/frontend/types.ts.tera` | `frontend/src/features/{entityCamel}/types.ts` | `frontend.rs` |
| `entity/frontend/api.ts.tera` | `frontend/src/features/{entityCamel}/api.ts` | `frontend.rs` |
| `entity/frontend/hooks.ts.tera` | `frontend/src/features/{entityCamel}/hooks.ts` | `frontend.rs` |
| `entity/frontend/relation_hooks.ts.tera` | `frontend/src/features/{entityCamel}/{related}_hooks.ts` | `frontend.rs` |
| `entity/frontend/List.tsx.tera` | `frontend/src/features/{entityCamel}/{Entity}List.tsx` | `frontend.rs` |
| `entity/frontend/Form.tsx.tera` | `frontend/src/features/{entityCamel}/{Entity}Form.tsx` | `frontend.rs` |
| `entity/frontend/Detail.tsx.tera` | `frontend/src/features/{entityCamel}/{Entity}Detail.tsx` | `frontend.rs` |

### Auth Templates (used by `romance generate auth`)

| Template | Output | Generator |
|----------|--------|-----------|
| `auth/backend/auth.rs.tera` | `backend/src/auth.rs` | `auth.rs` |
| `auth/backend/user_model.rs.tera` | `backend/src/entities/user.rs` | `auth.rs` |
| `auth/backend/auth_handlers.rs.tera` | `backend/src/handlers/auth.rs` | `auth.rs` |
| `auth/backend/auth_routes.rs.tera` | `backend/src/routes/auth.rs` | `auth.rs` |
| `auth/backend/user_migration.rs.tera` | `backend/migration/src/m{ts}_create_users_table.rs` | `auth.rs` |
| `auth/frontend/types.ts.tera` | `frontend/src/features/auth/types.ts` | `auth.rs` |
| `auth/frontend/api.ts.tera` | `frontend/src/features/auth/api.ts` | `auth.rs` |
| `auth/frontend/hooks.ts.tera` | `frontend/src/features/auth/hooks.ts` | `auth.rs` |
| `auth/frontend/AuthContext.tsx.tera` | `frontend/src/features/auth/AuthContext.tsx` | `auth.rs` |
| `auth/frontend/LoginPage.tsx.tera` | `frontend/src/features/auth/LoginPage.tsx` | `auth.rs` |
| `auth/frontend/RegisterPage.tsx.tera` | `frontend/src/features/auth/RegisterPage.tsx` | `auth.rs` |
| `auth/frontend/ProtectedRoute.tsx.tera` | `frontend/src/features/auth/ProtectedRoute.tsx` | `auth.rs` |

### Admin Templates (used by `romance generate admin`)

| Template | Output | Generator |
|----------|--------|-----------|
| `admin/backend/admin_routes.rs.tera` | `backend/src/routes/admin.rs` | `admin.rs` |
| `admin/backend/admin_handlers.rs.tera` | `backend/src/handlers/admin.rs` | `admin.rs` |
| `admin/frontend/AdminLayout.tsx.tera` | `frontend/src/features/admin/AdminLayout.tsx` | `admin.rs` |
| `admin/frontend/Dashboard.tsx.tera` | `frontend/src/features/admin/Dashboard.tsx` | `admin.rs` |
| `admin/frontend/adminRoutes.tsx.tera` | `frontend/src/features/admin/adminRoutes.tsx` | `admin.rs` |

### Scaffold Templates (used by `romance new`)

| Template | Output |
|----------|--------|
| `scaffold/backend/Cargo.toml.tera` | `backend/Cargo.toml` |
| `scaffold/backend/main.rs.tera` | `backend/src/main.rs` |
| `scaffold/backend/config.rs.tera` | `backend/src/config.rs` |
| `scaffold/backend/db.rs.tera` | `backend/src/db.rs` |
| `scaffold/backend/errors.rs.tera` | `backend/src/errors.rs` |
| `scaffold/backend/api.rs.tera` | `backend/src/api.rs` |
| `scaffold/backend/pagination.rs.tera` | `backend/src/pagination.rs` |
| `scaffold/backend/routes.rs.tera` | `backend/src/routes/mod.rs` |
| `scaffold/backend/env.example.tera` | `backend/.env.example` + `backend/.env` |
| `scaffold/backend/migration/Cargo.toml.tera` | `backend/migration/Cargo.toml` |
| `scaffold/backend/migration/lib.rs.tera` | `backend/migration/src/lib.rs` |
| `scaffold/backend/migration/main.rs.tera` | `backend/migration/src/main.rs` |
| `scaffold/frontend/package.json.tera` | `frontend/package.json` |
| `scaffold/frontend/vite.config.ts.tera` | `frontend/vite.config.ts` |
| `scaffold/frontend/tsconfig.json.tera` | `frontend/tsconfig.json` |
| `scaffold/frontend/App.tsx.tera` | `frontend/src/App.tsx` |
| `scaffold/frontend/main.tsx.tera` | `frontend/src/main.tsx` |
| `scaffold/frontend/lib/utils.ts.tera` | `frontend/src/lib/utils.ts` |
| `scaffold/frontend/vite-env.d.ts.tera` | `frontend/src/vite-env.d.ts` |
| `scaffold/romance.toml.tera` | `romance.toml` |
| `scaffold/README.md.tera` | `README.md` |

Additionally, `scaffold.rs` creates these non-template files directly:
- `backend/src/entities/mod.rs` — stub with `// === ROMANCE:MODS ===`
- `backend/src/handlers/mod.rs` — stub with `// === ROMANCE:MODS ===`
- `frontend/src/index.css` — `@import "tailwindcss";`
- `frontend/index.html` — HTML shell
- `.gitignore`

### Addon Templates (used by `romance add <feature>`)

| Template | Output | Addon |
|----------|--------|-------|
| `addon/validation/validate_middleware.rs.tera` | `backend/src/validation.rs` | validation |
| `addon/security/security_headers.rs.tera` | `backend/src/middleware/security_headers.rs` | security |
| `addon/security/rate_limit.rs.tera` | `backend/src/middleware/rate_limit.rs` | security |
| `addon/security/middleware_mod.rs.tera` | `backend/src/middleware/mod.rs` | security |
| `addon/observability/tracing_setup.rs.tera` | `backend/src/middleware/tracing_setup.rs` | observability |
| `addon/observability/request_id.rs.tera` | `backend/src/middleware/request_id.rs` | observability |
| `addon/storage/storage.rs.tera` | `backend/src/storage.rs` | storage |
| `addon/storage/upload_handler.rs.tera` | `backend/src/handlers/upload.rs` | storage |
| `addon/storage/upload_routes.rs.tera` | `backend/src/routes/upload.rs` | storage |
| `addon/storage/FileUpload.tsx.tera` | `frontend/src/components/FileUpload.tsx` | storage |
| `addon/soft_delete/soft_delete.rs.tera` | `backend/src/soft_delete.rs` | soft-delete |
| `addon/audit_log/audit.rs.tera` | `backend/src/audit.rs` | audit-log |
| `addon/audit_log/model.rs.tera` | `backend/src/entities/audit_entry.rs` | audit-log |
| `addon/audit_log/migration.rs.tera` | `backend/migration/src/m{ts}_create_audit_entries_table.rs` | audit-log |
| `addon/audit_log/handlers.rs.tera` | `backend/src/handlers/audit_log.rs` | audit-log |
| `addon/audit_log/AuditLog.tsx.tera` | `frontend/src/features/admin/AuditLog.tsx` | audit-log |
| `addon/oauth/oauth.rs.tera` | `backend/src/oauth.rs` | oauth |
| `addon/oauth/oauth_handlers.rs.tera` | `backend/src/handlers/oauth.rs` | oauth |
| `addon/oauth/oauth_routes.rs.tera` | `backend/src/routes/oauth.rs` | oauth |
| `addon/oauth/oauth_migration.rs.tera` | `backend/migration/src/m{ts}_add_oauth_to_users.rs` | oauth |
| `addon/oauth/OAuthButton.tsx.tera` | `frontend/src/features/auth/OAuthButton.tsx` | oauth |
| `addon/search/search.rs.tera` | `backend/src/search.rs` | search |
| `addon/search/search_handler.rs.tera` | `backend/src/handlers/search.rs` | search |
| `addon/search/SearchBar.tsx.tera` | `frontend/src/components/SearchBar.tsx` | search |
| `addon/seed/seed.rs.tera` | `backend/src/seed.rs` | seed |
| `addon/dashboard/dev_handlers.rs.tera` | `backend/src/handlers/dev_dashboard.rs` | dashboard |
| `addon/dashboard/dev_routes.rs.tera` | `backend/src/routes/dev_dashboard.rs` | dashboard |
| `addon/dashboard/DevDashboard.tsx.tera` | `frontend/src/features/dev/DevDashboard.tsx` | dashboard |
| `addon/test/test_helpers.rs.tera` | `backend/src/test_helpers.rs` | test |

## CLI Commands Reference

| Command | Description |
|---------|-------------|
| `romance new <name>` | Create a new full-stack project |
| `romance generate entity <name> [field:type...]` | Generate CRUD entity (interactive if no fields) |
| `romance generate auth` | Generate JWT authentication (user entity, login/register, auth context) |
| `romance generate admin` | Generate admin panel (dashboard, layout, entity management) |
| `romance generate types` | Generate TypeScript types via ts-rs |
| `romance generate openapi` | Generate OpenAPI spec via utoipa |
| `romance add validation` | Add backend (validator) + frontend (Zod) validation |
| `romance add soft-delete` | Add soft-delete (deleted_at, restore, force-delete) |
| `romance add audit-log` | Add audit logging (requires auth) |
| `romance add storage` | Add file/image upload with pluggable storage |
| `romance add search` | Add full-text search (PostgreSQL) |
| `romance add oauth <provider>` | Add OAuth social auth (google, github, discord) |
| `romance add security` | Add security middleware (rate limit, headers) |
| `romance add observability` | Add structured logging + request ID tracing |
| `romance add dashboard` | Add developer dashboard (/dev route) |
| `romance test` | Run tests with temporary database |
| `romance update` | Update scaffold files to latest templates (interactive conflict resolution) |
| `romance update --init` | Create baseline manifest for pre-existing projects |
| `romance dev` | Run backend (`cargo watch`) + frontend (`npm run dev`) |
| `romance check` | Run `cargo check` + `cargo test` + `npx tsc --noEmit` |
| `romance db migrate` | Run pending migrations |
| `romance db rollback` | Rollback last migration |
| `romance db status` | Show migration status |
| `romance db seed` | Run seed data |

**Field format:** `name:type`, `name:type?` (optional), `name:type->Entity` (foreign key)

**Validation:** `name:type[min=3,max=100]`, `email:string[email]`, `name:type[searchable]`

**Relation format:** `name:has_many->Entity`, `name:m2m->Entity`

## Relation System

### Relation Types

| Type | CLI Syntax | Effect |
|------|-----------|--------|
| BelongsTo | `author_id:uuid->User` | FK column on this entity; injects has-many into target |
| HasMany | `posts:has_many->Post` | Explicit reverse side (usually auto-inferred from BelongsTo) |
| ManyToMany | `tags:m2m->Tag` | Creates junction table, injects Related via junction into both entities |

### How Relations Work

1. **BelongsTo** (`author_id:uuid->User`): Creates FK column, adds `Relation::User` to model, generates `Related<user::Entity>`. If User entity exists, injects reverse has-many into User's files (Related impl, list handler, route).

2. **ManyToMany** (`tags:m2m->Tag`): If Tag exists, generates junction table `post_tag` (alphabetical order) with model + migration, injects `Related<tag::Entity>` via junction into both Post and Tag, generates list/add/remove handlers and routes on both sides. If Tag doesn't exist, stores pending relation in `.romance/pending_relations.json` — applied automatically when Tag is generated later.

3. **HasMany** (`posts:has_many->Post`): Explicit declaration of reverse side. Usually unnecessary since BelongsTo auto-injects the reverse.

### Junction Table Convention

For M2M between entities A and B: junction name is `{a_snake}_{b_snake}` in alphabetical order. Table name is the same. Example: Post + Tag → `post_tag` entity, `post_tag` table.

## Verification

```bash
# Workspace compiles
cargo check

# Scaffold works
romance new test-app && cd test-app/backend && cargo check

# Entity generation works
cd test-app && romance generate entity Product title:string price:decimal description:text?

# Entity with validation + searchable
romance generate entity Article title:string[min=3,max=100,searchable] body:text[searchable] published:bool

# Relations work
romance generate entity Category name:string
romance generate entity Product title:string category_id:uuid->Category tags:m2m->Tag
romance generate entity Tag name:string

# File/image field types
romance generate entity Document title:string file:file preview:image?

# Auth generation
romance generate auth

# Admin generation (requires auth)
romance generate admin

# Addon installation
romance add validation
romance add security
romance add observability
romance add storage
romance add soft-delete
romance add audit-log
romance add search
romance add oauth google
romance add dashboard

# Verify backend compiles
cd backend && cargo check

# Test runner
romance test

# Seed data
romance db seed
```
