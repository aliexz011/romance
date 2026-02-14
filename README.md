# Romance

**Full-stack code generator for Rust + React.** One command creates a complete project. Another generates full CRUD entities across the entire stack.

[![Rust](https://img.shields.io/badge/Rust-1.75+-orange?logo=rust)](https://www.rust-lang.org)
[![License: Commercial](https://img.shields.io/badge/License-Commercial-orange.svg)](LICENSE)

Think Django's `startproject` + `startapp`, but for **Axum + SeaORM** backends and **React + TypeScript + shadcn/ui** frontends.

```
romance new my-app
cd my-app
romance generate entity Post title:string body:text published:bool author_id:uuid->User
romance dev
```

That's it. You now have a working full-stack CRUD app with a Rust API, database migrations, TypeScript types, React forms, and auto-wired routes.

---

## Features

**Project Scaffolding**
- Complete Axum + SeaORM backend with config, error handling, pagination, and CORS
- React 19 + TypeScript frontend with Vite, TanStack Query, and shadcn/ui
- PostgreSQL migrations via SeaORM, ready to run

**Entity Generation**
- Full CRUD across the entire stack from a single command
- Backend: SeaORM model, Axum handlers (list/get/create/update/delete), routes, migration
- Frontend: TypeScript types, API client, TanStack Query hooks, List/Form/Detail components
- 14 field types with automatic mapping across Rust, TypeScript, PostgreSQL, and UI components
- Inline validation rules and searchable field annotations

**Relations**
- BelongsTo with automatic reverse has-many injection into the target entity
- ManyToMany with auto-generated junction tables, handlers, and routes on both sides
- Deferred relations: M2M targets that don't exist yet are stored and applied when generated later

**Authentication & Admin**
- JWT auth generation: user entity, login/register handlers, frontend auth context and protected routes
- Admin panel generation: dashboard, layout, entity management for all existing entities

**15 Addons**
- Validation, soft-delete, audit logging, file/image storage, full-text search, OAuth, security headers, rate limiting, observability, email, i18n, caching, background tasks, WebSockets, API keys

**Developer Experience**
- `romance dev` runs backend + frontend concurrently
- `romance check` runs cargo check, cargo test, and tsc in one command
- `romance test` with automatic temporary database creation
- `romance doctor` to verify project health and dependencies
- `romance destroy entity` to cleanly remove generated code
- Shell completions for bash, zsh, fish, and PowerShell
- Idempotent code generation with custom code preservation via marker system

---

## Quick Start

### Install

```bash
cargo install romance-cli
```

### Create a Project

```bash
romance new my-app
cd my-app
```

This generates:

```
my-app/
  backend/
    src/
      main.rs          # Axum server with CORS, config, error handling
      config.rs        # Environment-aware configuration
      db.rs            # SeaORM database connection pool
      errors.rs        # Unified error responses
      pagination.rs    # Cursor/offset pagination
      entities/mod.rs  # Entity models
      handlers/mod.rs  # Request handlers
      routes/mod.rs    # Route registration
    migration/
      src/lib.rs       # Migration runner
    Cargo.toml
    .env
  frontend/
    src/
      App.tsx          # React Router setup
      main.tsx         # Entry point with QueryClientProvider
      lib/utils.ts     # Tailwind merge utility
    package.json       # React 19, TanStack Query, shadcn/ui, Vite
    vite.config.ts
    tsconfig.json
  romance.toml         # Project configuration
```

### Generate an Entity

```bash
romance generate entity Product title:string price:decimal description:text? in_stock:bool
```

This generates across the full stack:

| Layer | Files Generated |
|-------|----------------|
| **Model** | `backend/src/entities/product.rs` -- SeaORM entity with all columns |
| **Handlers** | `backend/src/handlers/product.rs` -- list, get, create, update, delete |
| **Routes** | `backend/src/routes/product.rs` -- all REST endpoints wired up |
| **Migration** | `backend/migration/src/m{timestamp}_create_product_table.rs` |
| **Types** | `frontend/src/features/product/types.ts` -- TypeScript interfaces |
| **API** | `frontend/src/features/product/api.ts` -- fetch client |
| **Hooks** | `frontend/src/features/product/hooks.ts` -- TanStack Query hooks |
| **List** | `frontend/src/features/product/ProductList.tsx` -- data table |
| **Form** | `frontend/src/features/product/ProductForm.tsx` -- create/edit form |
| **Detail** | `frontend/src/features/product/ProductDetail.tsx` -- detail view |

Plus automatic registration in route files and App.tsx.

### Add Relations

```bash
romance generate entity Category name:string
romance generate entity Product title:string category_id:uuid->Category tags:m2m->Tag
romance generate entity Tag name:string
```

The `->Category` foreign key automatically injects a has-many reverse relation into Category. The `m2m->Tag` creates a junction table with handlers and routes on both sides.

### Run the Dev Servers

```bash
romance dev
```

Starts `cargo watch` (backend) and `npm run dev` (frontend) concurrently.

---

## Generated Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **HTTP Server** | Axum | Async Rust web framework |
| **ORM** | SeaORM | Database models, queries, migrations |
| **Database** | PostgreSQL | Primary data store |
| **Frontend** | React 19 + TypeScript | UI framework |
| **Data Fetching** | TanStack Query | Server state management, caching |
| **UI Components** | shadcn/ui | Accessible, composable components |
| **Bundler** | Vite | Fast dev server and builds |
| **Styling** | Tailwind CSS | Utility-first CSS |

---

## CLI Commands

### Core Commands

| Command | Description |
|---------|-------------|
| `romance new <name>` | Create a new full-stack project |
| `romance dev` | Run backend + frontend dev servers concurrently |
| `romance check` | Run `cargo check` + `cargo test` + `npx tsc --noEmit` |
| `romance test` | Run tests with a temporary database |
| `romance doctor` | Check project health and dependencies |

### Code Generation

| Command | Description |
|---------|-------------|
| `romance generate entity <name> [fields...]` | Generate full CRUD entity (interactive if no fields given) |
| `romance generate auth` | Generate JWT authentication (user entity, login/register, auth context) |
| `romance generate admin` | Generate admin panel (dashboard, layout, entity management) |
| `romance generate types` | Generate TypeScript types via ts-rs |
| `romance generate openapi` | Generate OpenAPI spec via utoipa |

### Addons

| Command | Description |
|---------|-------------|
| `romance add validation` | Backend validation (validator) + frontend validation (Zod) |
| `romance add soft-delete` | Soft-delete with `deleted_at`, restore, and force-delete endpoints |
| `romance add audit-log` | Audit logging with user attribution (requires auth) |
| `romance add storage` | File/image upload with pluggable storage (local or S3) |
| `romance add search` | Full-text search using PostgreSQL tsvector + GIN index |
| `romance add oauth <provider>` | OAuth social auth (google, github, discord) |
| `romance add security` | Security middleware: rate limiting + security headers |
| `romance add observability` | Structured logging + request ID tracing |
| `romance add dashboard` | Developer dashboard at `/dev` route |
| `romance add email` | SMTP email via lettre with password reset handler |
| `romance add i18n` | Internationalization with locale detection |
| `romance add cache` | Redis-backed caching layer |
| `romance add tasks` | PostgreSQL-backed background task queue |
| `romance add websocket` | WebSocket support with EventBus bridge |
| `romance add api-keys` | API key authentication for machine-to-machine auth |

### Database

| Command | Description |
|---------|-------------|
| `romance db migrate` | Run pending migrations |
| `romance db rollback` | Rollback the last migration |
| `romance db status` | Show migration status |
| `romance db seed` | Run seed data |

### Maintenance

| Command | Description |
|---------|-------------|
| `romance update` | Update scaffold files to latest templates (interactive conflict resolution) |
| `romance update --init` | Create baseline manifest for pre-existing projects |
| `romance destroy entity <name>` | Remove a generated entity and all its files |
| `romance run <command> [args...]` | Run a custom management command |
| `romance completions <shell>` | Generate shell completions (bash, zsh, fish, powershell) |

---

## Field Types

Romance supports 14 field types. Each type is automatically mapped across every layer of the stack.

| CLI Type | Aliases | Rust | TypeScript | PostgreSQL | UI Component |
|----------|---------|------|------------|------------|-------------|
| `string` | `str` | `String` | `string` | `VARCHAR(255)` | Input |
| `text` | | `String` | `string` | `TEXT` | Textarea |
| `bool` | `boolean` | `bool` | `boolean` | `BOOLEAN` | Switch |
| `int` | `i32`, `int32`, `integer` | `i32` | `number` | `INTEGER` | Input (number) |
| `bigint` | `i64`, `int64` | `i64` | `number` | `BIGINT` | Input (number) |
| `float` | `f64`, `float64`, `double` | `f64` | `number` | `DOUBLE PRECISION` | Input (number) |
| `decimal` | `money` | `Decimal` | `number` | `DECIMAL` | Input (number) |
| `uuid` | | `Uuid` | `string` | `UUID` | Input |
| `datetime` | `timestamp` | `DateTimeWithTimeZone` | `string` | `TIMESTAMPTZ` | Input (datetime-local) |
| `date` | | `Date` | `string` | `DATE` | Input (date) |
| `json` | `jsonb` | `Json` | `unknown` | `JSONB` | Textarea |
| `file` | | `String` | `string` | `VARCHAR(512)` | FileInput |
| `image` | | `String` | `string` | `VARCHAR(512)` | ImageInput |
| `enum(...)` | | `String` | `string` | `VARCHAR(255)` | Select |

### Field Modifiers

```bash
# Optional (nullable) field -- append ?
description:text?

# Foreign key -- append ->EntityName
category_id:uuid->Category

# Validation rules -- use brackets
title:string[min=3,max=100]
email:string[email]
slug:string[unique]
website:string[url]
code:string[regex=^[A-Z]{3}$]

# Searchable field (for full-text search addon)
title:string[searchable]
body:text[searchable]

# Combine modifiers
author_id:uuid[required]->User
title:string[min=3,max=200,searchable]
```

---

## Relations

### BelongsTo (Foreign Key)

```bash
romance generate entity Post title:string author_id:uuid->User
```

Creates a `author_id` UUID column on the `posts` table with a foreign key to `users`. If the User entity already exists, a has-many reverse relation is automatically injected into User's handlers and routes.

### ManyToMany

```bash
romance generate entity Post title:string tags:m2m->Tag
```

Creates a `post_tag` junction table (alphabetical naming convention) with a model and migration. Injects list/add/remove handlers and routes into both Post and Tag. If Tag doesn't exist yet, the relation is stored in `.romance/pending_relations.json` and applied automatically when Tag is generated.

### HasMany (Explicit)

```bash
romance generate entity User name:string posts:has_many->Post
```

Usually unnecessary since BelongsTo automatically injects the reverse. Use only when you need to explicitly declare the reverse side.

---

## Addons

Addons extend your project with production-ready features via `romance add`.

| Addon | Command | What It Adds |
|-------|---------|-------------|
| **Validation** | `romance add validation` | Backend: `validator` crate with middleware; Frontend: Zod schemas |
| **Soft Delete** | `romance add soft-delete` | `deleted_at` column, restore endpoint, force-delete, query filtering |
| **Audit Log** | `romance add audit-log` | Tracks all create/update/delete operations with user attribution |
| **Storage** | `romance add storage` | File/image upload handlers, pluggable backends (local filesystem, S3) |
| **Search** | `romance add search` | PostgreSQL full-text search with tsvector, GIN index, SearchBar component |
| **OAuth** | `romance add oauth google` | Social login (Google, GitHub, Discord) with OAuthButton component |
| **Security** | `romance add security` | Rate limiting middleware + security headers |
| **Observability** | `romance add observability` | Structured tracing + request ID propagation |
| **Dashboard** | `romance add dashboard` | Developer dashboard with entity stats at `/dev` |
| **Email** | `romance add email` | SMTP via lettre, password reset handler |
| **i18n** | `romance add i18n` | Internationalization with locale detection |
| **Cache** | `romance add cache` | Redis-backed caching layer |
| **Tasks** | `romance add tasks` | PostgreSQL-backed background task queue |
| **WebSocket** | `romance add websocket` | Real-time communication with EventBus bridge |
| **API Keys** | `romance add api-keys` | Machine-to-machine authentication via API keys |

---

## Configuration

Romance projects are configured via `romance.toml` at the project root.

```toml
[project]
name = "my-app"
description = "My full-stack application"

[backend]
port = 3000
database_url = "postgres://localhost/my_app"
api_prefix = "/api"                          # Optional, for API versioning

[frontend]
port = 5173
api_base_url = "http://localhost:3000"

[codegen]
generate_openapi = true
generate_ts_types = true

[features]
validation = true
soft_delete = false
audit_log = false
search = true

[security]
rate_limit_rpm = 60
cors_origins = ["http://localhost:5173"]
csrf = false

[storage]
backend = "local"          # "local" or "s3"
upload_dir = "./uploads"
max_file_size = "10MB"

[environment]
active = "development"     # or set ROMANCE_ENV env var
```

### Environment Overrides

Create `romance.{env}.toml` files for environment-specific settings:

```toml
# romance.production.toml
[backend]
port = 8080
```

The override file is deep-merged on top of the base config. Set the active environment via the `ROMANCE_ENV` environment variable or the `[environment] active` field.

---

## How It Works

Romance uses a template-based code generation architecture:

1. **CLI** parses commands and field definitions into an `EntityDefinition`
2. **Template Engine** (Tera) builds context with type mappings for every target language
3. **Templates** (compiled into the binary via `rust-embed`) render to source files
4. **Marker System** (`// === ROMANCE:TYPE ===` comments) enables incremental code insertion without overwriting user code
5. **Custom Blocks** (`// === ROMANCE:CUSTOM ===`) preserve hand-written code below the marker across re-generations
6. **Manifest Tracking** (`.romance/manifest.json`) stores SHA-256 hashes of generated files for the `romance update` command

The generator is fully idempotent: running the same command twice produces the same result without duplicating code.

---

## Documentation

Detailed guides are available in the [`docs/`](docs/) directory:

- [Architecture](docs/architecture.md) -- crate structure, data flow, design decisions
- [Templates](docs/templates.md) -- template context variables, Tera filters, creating new templates
- [Relations](docs/relations.md) -- deep dive into BelongsTo, HasMany, ManyToMany internals
- [Addons](docs/addons.md) -- how addons work, creating custom addons
- [Contributing](CONTRIBUTING.md) -- development setup, checklists for new features

---

## License

This software is proprietary and requires a valid subscription license. See [LICENSE](LICENSE) for details. Generated code output belongs to you and can be used freely in your projects.
