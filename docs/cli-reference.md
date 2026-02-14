# Romance CLI Reference

Complete reference for every command, subcommand, flag, and argument in the Romance CLI.

**Binary:** `romance`

---

## Table of Contents

- [romance new](#romance-new)
- [romance generate](#romance-generate)
  - [romance generate entity](#romance-generate-entity)
  - [romance generate auth](#romance-generate-auth)
  - [romance generate admin](#romance-generate-admin)
  - [romance generate types](#romance-generate-types)
  - [romance generate openapi](#romance-generate-openapi)
- [romance add](#romance-add)
  - [romance add validation](#romance-add-validation)
  - [romance add soft-delete](#romance-add-soft-delete)
  - [romance add audit-log](#romance-add-audit-log)
  - [romance add storage](#romance-add-storage)
  - [romance add search](#romance-add-search)
  - [romance add oauth](#romance-add-oauth)
  - [romance add security](#romance-add-security)
  - [romance add observability](#romance-add-observability)
  - [romance add dashboard](#romance-add-dashboard)
  - [romance add email](#romance-add-email)
  - [romance add i18n](#romance-add-i18n)
  - [romance add cache](#romance-add-cache)
  - [romance add tasks](#romance-add-tasks)
  - [romance add websocket](#romance-add-websocket)
  - [romance add api-keys](#romance-add-api-keys)
- [romance dev](#romance-dev)
- [romance check](#romance-check)
- [romance test](#romance-test)
- [romance db](#romance-db)
  - [romance db migrate](#romance-db-migrate)
  - [romance db rollback](#romance-db-rollback)
  - [romance db status](#romance-db-status)
  - [romance db seed](#romance-db-seed)
- [romance update](#romance-update)
- [romance run](#romance-run)
- [romance destroy](#romance-destroy)
  - [romance destroy entity](#romance-destroy-entity)
- [romance doctor](#romance-doctor)
- [romance completions](#romance-completions)
- [Field Syntax Reference](#field-syntax-reference)
- [Relation Syntax Reference](#relation-syntax-reference)
- [Supported Field Types](#supported-field-types)

---

## romance new

Create a new full-stack project with the complete Axum + React scaffold.

**Syntax:**

```
romance new <name>
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Project name. Used as directory name and in generated config. |

**What it generates:**

- `backend/` -- Rust (Axum + SeaORM) application with entities, handlers, routes, config, error handling, pagination, events, and commands modules
- `backend/migration/` -- SeaORM migration crate
- `frontend/` -- React 19 + TypeScript + Vite application with shadcn/ui
- `romance.toml` -- Project configuration
- `romance.production.toml` -- Production environment overrides
- `Dockerfile` -- Multi-stage Docker build for the backend
- `Dockerfile.frontend` -- Multi-stage Docker build for the frontend with nginx
- `docker-compose.yml` -- Docker Compose with PostgreSQL, Redis, backend, and frontend
- `.github/workflows/ci.yml` -- GitHub Actions CI pipeline
- `.gitignore`, `README.md`
- `.romance/manifest.json` -- File tracking manifest

After creating files, `romance new` automatically runs `npm install` and installs all shadcn/ui components in the frontend directory.

**Example:**

```bash
romance new my-blog

# Output:
# Creating new Romance project: my-blog
#   create backend/Cargo.toml
#   create backend/src/main.rs
#   ...
#   create .github/workflows/ci.yml
# Installing frontend dependencies...
# Project created successfully!
```

---

## romance generate

Generate code for entities, authentication, admin panels, and more.

### romance generate entity

Generate a full CRUD entity across the entire stack: backend model, handlers, routes, migration, and frontend types, API client, hooks, list/form/detail components.

**Syntax:**

```
romance generate entity <name> [field:type...]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Entity name in PascalCase (e.g., `Product`, `BlogPost`) |
| `fields` | No | Field definitions. If omitted, an interactive prompt is launched. |

**Field format:** See [Field Syntax Reference](#field-syntax-reference) below.

**Generated files:**

| File | Description |
|------|-------------|
| `backend/src/entities/{entity}.rs` | SeaORM entity model |
| `backend/src/handlers/{entity}.rs` | CRUD handlers (list, get, create, update, delete) |
| `backend/src/routes/{entity}.rs` | Axum route definitions |
| `backend/migration/src/m{timestamp}_create_{entity}_table.rs` | Database migration |
| `frontend/src/features/{entityCamel}/types.ts` | TypeScript interfaces |
| `frontend/src/features/{entityCamel}/api.ts` | API client functions |
| `frontend/src/features/{entityCamel}/hooks.ts` | TanStack Query hooks |
| `frontend/src/features/{entityCamel}/{Entity}List.tsx` | List view component |
| `frontend/src/features/{entityCamel}/{Entity}Form.tsx` | Create/edit form component |
| `frontend/src/features/{entityCamel}/{Entity}Detail.tsx` | Detail view component |

**Examples:**

```bash
# With inline fields
romance generate entity Product title:string price:decimal description:text?

# With a foreign key relation
romance generate entity Product title:string category_id:uuid->Category

# With many-to-many relation
romance generate entity Post title:string body:text tags:m2m->Tag

# With validation constraints and searchable fields
romance generate entity Article title:string[min=3,max=100,searchable] body:text[searchable] published:bool

# With file/image fields
romance generate entity Document title:string file:file preview:image?

# Interactive mode (no fields specified)
romance generate entity Product
```

### romance generate auth

Generate JWT authentication: user entity, auth middleware, login/register handlers, and frontend auth components.

**Syntax:**

```
romance generate auth
```

**No arguments or options.**

**Generated files:**

| File | Description |
|------|-------------|
| `backend/src/auth.rs` | JWT middleware and token utilities |
| `backend/src/entities/user.rs` | User entity model |
| `backend/src/handlers/auth.rs` | Login, register, me handlers |
| `backend/src/routes/auth.rs` | Auth route definitions |
| `backend/migration/src/m{timestamp}_create_users_table.rs` | Users table migration |
| `frontend/src/features/auth/types.ts` | Auth TypeScript types |
| `frontend/src/features/auth/api.ts` | Auth API client |
| `frontend/src/features/auth/hooks.ts` | Auth hooks |
| `frontend/src/features/auth/AuthContext.tsx` | React auth context provider |
| `frontend/src/features/auth/LoginPage.tsx` | Login page component |
| `frontend/src/features/auth/RegisterPage.tsx` | Registration page component |
| `frontend/src/features/auth/ProtectedRoute.tsx` | Route guard component |

**Example:**

```bash
romance generate auth
```

### romance generate admin

Generate an admin panel with dashboard, layout, and entity management. Requires auth to be generated first.

**Syntax:**

```
romance generate admin
```

**No arguments or options.**

**Generated files:**

| File | Description |
|------|-------------|
| `backend/src/routes/admin.rs` | Admin route definitions |
| `backend/src/handlers/admin.rs` | Admin handlers |
| `frontend/src/features/admin/AdminLayout.tsx` | Admin layout with sidebar |
| `frontend/src/features/admin/Dashboard.tsx` | Admin dashboard |
| `frontend/src/features/admin/adminRoutes.tsx` | Admin route configuration |

The admin generator automatically discovers existing entities and includes them in the admin panel.

**Example:**

```bash
# Generate auth first
romance generate auth

# Then generate admin
romance generate admin
```

### romance generate types

Generate TypeScript type definitions from Rust structs using ts-rs.

**Syntax:**

```
romance generate types
```

**No arguments or options.**

Runs `cargo test -- export_bindings` in the backend directory to export TypeScript types from Rust structs annotated with `#[derive(TS)]`.

**Example:**

```bash
romance generate types
```

### romance generate openapi

Generate an OpenAPI specification using utoipa.

**Syntax:**

```
romance generate openapi
```

**No arguments or options.**

Runs `cargo run --bin openapi-export` in the backend directory to produce the OpenAPI JSON specification.

**Example:**

```bash
romance generate openapi
```

---

## romance add

Add features and addons to an existing project. Each addon generates the necessary backend and/or frontend code, installs dependencies, and updates configuration.

### romance add validation

Add backend validation using the `validator` crate and frontend validation using Zod schemas.

**Syntax:**

```
romance add validation
```

**What it adds:**

- `backend/src/validation.rs` -- Validation middleware
- Sets `features.validation = true` in `romance.toml`
- Future entity generation will include validation constraints

### romance add soft-delete

Add soft-delete support with `deleted_at` column, restore endpoint, and force-delete endpoint.

**Syntax:**

```
romance add soft-delete
```

**What it adds:**

- `backend/src/soft_delete.rs` -- Soft-delete trait and helpers
- Sets `features.soft_delete = true` in `romance.toml`

### romance add audit-log

Add audit logging that tracks all create, update, and delete operations with user attribution. Requires auth to be generated first.

**Syntax:**

```
romance add audit-log
```

**What it adds:**

- `backend/src/audit.rs` -- Audit logging middleware
- `backend/src/entities/audit_entry.rs` -- Audit entry entity
- `backend/migration/src/m{timestamp}_create_audit_entries_table.rs` -- Migration
- `backend/src/handlers/audit_log.rs` -- Audit log handlers
- `frontend/src/features/admin/AuditLog.tsx` -- Audit log viewer component
- Sets `features.audit_log = true` in `romance.toml`

### romance add storage

Add file and image upload with pluggable storage backends (local filesystem or S3).

**Syntax:**

```
romance add storage
```

**What it adds:**

- `backend/src/storage.rs` -- Storage service abstraction
- `backend/src/handlers/upload.rs` -- Upload handlers
- `backend/src/routes/upload.rs` -- Upload routes
- `frontend/src/components/FileUpload.tsx` -- File upload component
- `[storage]` section in `romance.toml`

### romance add search

Add full-text search using PostgreSQL `tsvector` and GIN indexes.

**Syntax:**

```
romance add search
```

**What it adds:**

- `backend/src/search.rs` -- Search service
- `backend/src/handlers/search.rs` -- Search handlers
- `frontend/src/components/SearchBar.tsx` -- Search bar component
- Sets `features.search = true` in `romance.toml`

### romance add oauth

Add OAuth social authentication for a specific provider.

**Syntax:**

```
romance add oauth <provider>
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `provider` | Yes | OAuth provider: `google`, `github`, or `discord` |

**What it adds:**

- `backend/src/oauth.rs` -- OAuth client and token exchange
- `backend/src/handlers/oauth.rs` -- OAuth callback handlers
- `backend/src/routes/oauth.rs` -- OAuth routes
- `backend/migration/src/m{timestamp}_add_oauth_to_users.rs` -- Migration to add OAuth columns
- `frontend/src/features/auth/OAuthButton.tsx` -- OAuth login button component

**Example:**

```bash
romance add oauth google
romance add oauth github
romance add oauth discord
```

### romance add security

Add security middleware including rate limiting and security headers.

**Syntax:**

```
romance add security
```

**What it adds:**

- `backend/src/middleware/security_headers.rs` -- Security headers middleware
- `backend/src/middleware/rate_limit.rs` -- Rate limiting middleware
- `backend/src/middleware/mod.rs` -- Middleware module
- `[security]` section in `romance.toml`

### romance add observability

Add structured logging with tracing and request ID propagation.

**Syntax:**

```
romance add observability
```

**What it adds:**

- `backend/src/middleware/tracing_setup.rs` -- Tracing configuration
- `backend/src/middleware/request_id.rs` -- Request ID middleware

### romance add dashboard

Add a developer dashboard accessible at the `/dev` route.

**Syntax:**

```
romance add dashboard
```

**What it adds:**

- `backend/src/handlers/dev_dashboard.rs` -- Dashboard handlers
- `backend/src/routes/dev_dashboard.rs` -- Dashboard routes
- `frontend/src/features/dev/DevDashboard.tsx` -- Dashboard component

### romance add email

Add an email system using SMTP via the lettre crate, including password reset handler.

**Syntax:**

```
romance add email
```

### romance add i18n

Add internationalization (i18n) support with locale detection.

**Syntax:**

```
romance add i18n
```

### romance add cache

Add a caching layer backed by Redis.

**Syntax:**

```
romance add cache
```

### romance add tasks

Add background task processing using a PostgreSQL-backed task queue.

**Syntax:**

```
romance add tasks
```

### romance add websocket

Add WebSocket support for real-time communication with EventBus bridge.

**Syntax:**

```
romance add websocket
```

### romance add api-keys

Add API key authentication for machine-to-machine auth.

**Syntax:**

```
romance add api-keys
```

---

## romance dev

Start both development servers concurrently: the backend with `cargo watch` (auto-restart on changes) and the frontend with `npm run dev` (Vite dev server with HMR).

**Syntax:**

```
romance dev
```

**No arguments or options.**

The command spawns two child processes:
- `cargo watch -x run` in the `backend/` directory
- `npm run dev` in the `frontend/` directory

When either process exits, the other is terminated. Press Ctrl+C to stop both.

**Prerequisites:**
- `cargo-watch` must be installed: `cargo install cargo-watch`

**Example:**

```bash
romance dev
# Starting development servers...
# Backend: http://localhost:3001
# Frontend: http://localhost:5173
```

---

## romance check

Run all project verification checks: Rust compilation, Rust tests, and TypeScript type checking.

**Syntax:**

```
romance check
```

**No arguments or options.**

**Checks performed (in order):**

1. `cargo check` -- Verifies the backend compiles
2. `cargo test` -- Runs all backend tests
3. `npx tsc --noEmit` -- Verifies the frontend type-checks

If any check fails, the command exits immediately with a non-zero status.

**Example:**

```bash
romance check
# Running checks...
#   cargo check... OK
#   cargo test... OK
#   tsc --noEmit... OK
# All checks passed!
```

---

## romance test

Run the project test suite using a temporary database. Creates an isolated test database, runs migrations, executes tests, and tears down the database.

**Syntax:**

```
romance test
```

**No arguments or options.**

Must be run from the project root (where `romance.toml` exists).

**Example:**

```bash
romance test
```

---

## romance db

Database operations. All commands delegate to the SeaORM migration binary in `backend/migration/`.

### romance db migrate

Run all pending database migrations.

**Syntax:**

```
romance db migrate
```

Executes `cargo run --bin migration -- up` in the backend directory.

**Example:**

```bash
romance db migrate
# Running migrations...
# Migrations applied successfully!
```

### romance db rollback

Rollback the last applied migration.

**Syntax:**

```
romance db rollback
```

Executes `cargo run --bin migration -- down` in the backend directory.

**Example:**

```bash
romance db rollback
# Rolling back last migration...
# Rollback completed!
```

### romance db status

Show the status of all migrations (applied and pending).

**Syntax:**

```
romance db status
```

Executes `cargo run --bin migration -- status` in the backend directory.

**Example:**

```bash
romance db status
# Migration status:
# [Applied] m20250101000000_create_users_table
# [Applied] m20250101000001_create_products_table
# [Pending] m20250102000000_create_orders_table
```

### romance db seed

Run seed data. If `backend/src/seed.rs` does not exist, it is generated automatically.

**Syntax:**

```
romance db seed
```

Attempts to run `cargo run --bin seed` first. If no seed binary exists, falls back to `cargo test seed -- --ignored`.

**Example:**

```bash
romance db seed
# Running seed data...
# Seed data applied successfully!
```

---

## romance update

Update scaffold files to the latest Romance template versions. Compares the current state of generated files against new templates and applies updates intelligently.

**Syntax:**

```
romance update [--init]
```

**Options:**

| Flag | Description |
|------|-------------|
| `--init` | Create a baseline manifest for a project that was created before version tracking. Required before the first `romance update` on older projects. |

**Update behavior:**

- **Unmodified files** -- Automatically updated to the latest template version
- **New files** -- Created if they do not already exist
- **Conflicting files** (modified by user AND changed in template) -- Interactive resolution with three choices:
  - Overwrite: Replace with the new template version
  - Skip: Keep the user's version
  - Show diff: Display the differences before deciding
- **Deleted files** -- Skipped (user deletions are respected)

**Examples:**

```bash
# First-time setup for an existing project
romance update --init
# Manifest created at .romance/manifest.json

# Regular update
romance update
# Checking for template updates...
#   2 file(s) can be auto-updated
#   1 file(s) have conflicts
#   0 file(s) are new in this version
```

---

## romance run

Run a custom management command defined in the backend application.

**Syntax:**

```
romance run <command> [args...]
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `command` | Yes | Command name to execute |
| `args` | No | Additional arguments passed to the command |

Executes `cargo run --quiet -- run-command <command> [args...]` in the `backend/` directory. The backend application must have a command handler registered for the given command name in `backend/src/commands.rs`.

**Example:**

```bash
romance run cleanup --older-than 30d
# Running management command: cleanup
# Command 'cleanup' completed.
```

---

## romance destroy

Remove generated code from the project.

### romance destroy entity

Remove all generated files for an entity and clean up marker references.

**Syntax:**

```
romance destroy entity <name>
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `name` | Yes | Entity name in PascalCase (e.g., `Product`) |

**What it removes:**

- `backend/src/entities/{entity}.rs`
- `backend/src/handlers/{entity}.rs`
- `backend/src/routes/{entity}.rs`
- `frontend/src/features/{entityCamel}/` (entire directory)
- `pub mod {entity};` lines from `entities/mod.rs`, `handlers/mod.rs`, `routes/mod.rs`
- Route merge line from `routes/mod.rs`
- Import and Route lines from `frontend/src/App.tsx`

Note: Migration files are NOT removed. You must manage those manually or use `romance db rollback`.

**Example:**

```bash
romance destroy entity Product
# Destroying entity 'Product'...
#
# Backend files:
#   x Removed backend/src/entities/product.rs
#   x Removed backend/src/handlers/product.rs
#   x Removed backend/src/routes/product.rs
#
# Frontend files:
#   x Removed frontend/src/features/product/
#
# Cleaning markers:
#   ~ Cleaned pub mod product; from backend/src/entities/mod.rs
#   ~ Cleaned pub mod product; from backend/src/handlers/mod.rs
#   ~ Cleaned pub mod product; from backend/src/routes/mod.rs
#   ~ Cleaned route merge for product from routes/mod.rs
#   ~ Cleaned imports for product from App.tsx
#   ~ Cleaned routes for product from App.tsx
#
# Done. Removed 4/4 targets.
```

---

## romance doctor

Run health checks on the project to verify that the development environment and project structure are correct.

**Syntax:**

```
romance doctor
```

**No arguments or options.**

**Checks performed:**

| Check | What it verifies |
|-------|-----------------|
| romance.toml | File exists and contains valid TOML |
| Backend structure | `backend/src/main.rs`, `entities/mod.rs`, `handlers/mod.rs`, `routes/mod.rs` exist |
| Frontend structure | `frontend/package.json`, `frontend/src/App.tsx` exist |
| Markers | `// === ROMANCE:MODS ===` marker present in entities, handlers, and routes mod files |
| Cargo | Rust toolchain is installed (`cargo --version`) |
| Node.js | Node.js is installed (`node --version`) |
| DATABASE_URL | `backend/.env` exists and contains a `DATABASE_URL` entry |
| Manifest | `.romance/manifest.json` exists and contains valid JSON |

**Example:**

```bash
romance doctor
# Romance Doctor
#   V romance.toml found and valid
#   V Backend structure OK
#   V Frontend structure OK
#   V Markers intact (entities, handlers, routes)
#   V Cargo installed (1.83.0)
#   V Node.js installed (v20.11.0)
#   V DATABASE_URL configured
#   V .romance/manifest.json valid
#
# 8/8 checks passed
```

---

## romance completions

Generate shell completion scripts for the Romance CLI.

**Syntax:**

```
romance completions <shell>
```

**Arguments:**

| Argument | Required | Description |
|----------|----------|-------------|
| `shell` | Yes | Shell to generate completions for: `bash`, `zsh`, `fish`, `elvish`, or `powershell` |

**Examples:**

```bash
# Bash
romance completions bash > ~/.local/share/bash-completion/completions/romance

# Zsh
romance completions zsh > ~/.zfunc/_romance

# Fish
romance completions fish > ~/.config/fish/completions/romance.fish
```

---

## Field Syntax Reference

Fields are specified in the format `name:type[constraints]` with optional modifiers.

**Basic format:**

```
name:type
```

**Optional fields** (nullable in the database, `Option<T>` in Rust):

```
name:type?
```

**Foreign key** (creates a BelongsTo relation):

```
name:type->TargetEntity
```

**With validation constraints:**

```
name:type[min=3,max=100]
name:type[email]
name:type[searchable]
```

**Combined:**

```
name:type?->TargetEntity
title:string[min=3,max=100,searchable]
```

**Has-many relation** (explicit reverse side):

```
name:has_many->TargetEntity
```

**Many-to-many relation:**

```
name:m2m->TargetEntity
```

**Complete example with multiple field types:**

```bash
romance generate entity Product \
  title:string[min=1,max=200] \
  price:decimal \
  description:text? \
  published:bool \
  category_id:uuid->Category \
  tags:m2m->Tag
```

---

## Relation Syntax Reference

| Relation Type | Syntax | Effect |
|---------------|--------|--------|
| BelongsTo | `field_id:uuid->Target` | Creates FK column; auto-injects has-many into target entity |
| HasMany | `field:has_many->Target` | Explicit reverse side (usually auto-inferred from BelongsTo) |
| ManyToMany | `field:m2m->Target` | Creates junction table; injects Related impls into both entities |

**Junction table naming:** For M2M between entities A and B, the junction table is named `{a}_{b}` in alphabetical order. Example: `Post` + `Tag` produces a `post_tag` junction table.

**Deferred relations:** If the target entity does not exist yet when generating a M2M relation, the relation is stored in `.romance/pending_relations.json` and automatically applied when the target entity is generated later.

---

## Supported Field Types

| Type | CLI Aliases | Rust Type | TypeScript Type | PostgreSQL Type |
|------|-------------|-----------|-----------------|-----------------|
| String | `string`, `str` | `String` | `string` | `VARCHAR(255)` |
| Text | `text` | `String` | `string` | `TEXT` |
| Bool | `bool`, `boolean` | `bool` | `boolean` | `BOOLEAN` |
| Int32 | `i32`, `int`, `int32`, `integer` | `i32` | `number` | `INTEGER` |
| Int64 | `i64`, `int64`, `bigint` | `i64` | `number` | `BIGINT` |
| Float64 | `f64`, `float`, `float64`, `double` | `f64` | `number` | `DOUBLE PRECISION` |
| Decimal | `decimal`, `money` | `Decimal` | `number` | `DECIMAL` |
| Uuid | `uuid` | `Uuid` | `string` | `UUID` |
| DateTime | `datetime`, `timestamp` | `DateTimeWithTimeZone` | `string` | `TIMESTAMPTZ` |
| Date | `date` | `Date` | `string` | `DATE` |
| Json | `json`, `jsonb` | `Json` | `unknown` | `JSONB` |
| File | `file` | `String` | `string` | `VARCHAR(512)` |
| Image | `image` | `String` | `string` | `VARCHAR(512)` |
