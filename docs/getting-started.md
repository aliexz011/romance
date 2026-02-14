# Getting Started with Romance CLI

Romance is a full-stack code generator that scaffolds and extends projects using **Rust (Axum + SeaORM)** on the backend and **React (TypeScript + shadcn/ui)** on the frontend. A single command creates a complete project structure; another generates full CRUD entities across the entire stack.

## Prerequisites

Before using Romance, make sure you have the following installed:

| Dependency | Minimum Version | Purpose |
|------------|----------------|---------|
| **Rust** | 1.75+ | Backend language and toolchain (`rustup`, `cargo`) |
| **Node.js** | 18+ | Frontend build tooling and package management |
| **npm** | 9+ | Installing frontend dependencies |
| **PostgreSQL** | 14+ | Application database |
| **cargo-watch** | latest | Live-reloading during development (optional, used by `romance dev`) |

Verify your installations:

```bash
rustc --version
node --version
npm --version
psql --version
```

## Installation

### From crates.io (recommended)

```bash
cargo install romance
```

### Build from source

```bash
git clone https://github.com/romance-cli/romance.git
cd romance
cargo install --path crates/romance-cli
```

After installation, verify the CLI is available:

```bash
romance --version
```

## Creating Your First Project

Generate a new full-stack project with a single command:

```bash
romance new my-app
```

This creates a complete project with the following structure:

```
my-app/
  backend/
    src/
      main.rs            # Axum server entry point
      config.rs          # Configuration from environment variables
      db.rs              # Database connection pool
      errors.rs          # Error types and handlers
      api.rs             # API response helpers
      pagination.rs      # Pagination utilities
      entities/
        mod.rs           # Entity module declarations
      handlers/
        mod.rs           # Handler module declarations
      routes/
        mod.rs           # Route registration and middleware
    migration/
      src/
        main.rs          # Migration binary entry point
        lib.rs           # Migration registry
    Cargo.toml           # Rust dependencies
    .env                 # Environment variables (DATABASE_URL, etc.)
    .env.example         # Environment variable template
  frontend/
    src/
      App.tsx            # React app with routing
      main.tsx           # React entry point
      index.css          # Tailwind CSS import
      lib/
        utils.ts         # Utility functions (cn helper)
    index.html           # HTML shell
    package.json         # Node.js dependencies
    vite.config.ts       # Vite build configuration
    tsconfig.json        # TypeScript configuration
  romance.toml           # Romance project configuration
  .romance/
    manifest.json        # Tracks generated files (SHA-256 hashes)
  .gitignore
  README.md
```

### Key directories

- **`backend/`** -- Rust project using Axum for HTTP routing and SeaORM for database access. Runs on port 3001.
- **`frontend/`** -- React 19 project with TypeScript, TanStack Query for data fetching, shadcn/ui for components, and Vite for bundling. Runs on port 5173.
- **`romance.toml`** -- Project configuration file read by the Romance CLI.
- **`.romance/`** -- Internal state directory. Contains `manifest.json` for tracking generated file hashes (used by `romance update`) and `pending_relations.json` for deferred M2M relations.

## Setting Up the Database

1. Create a PostgreSQL database:

```bash
createdb my_app
```

2. Configure the database URL in `backend/.env`:

```env
DATABASE_URL=postgres://localhost/my_app
```

The `.env` file is generated automatically by `romance new` with a sensible default. Edit it if your PostgreSQL setup requires a username, password, or non-default port.

3. Install frontend dependencies:

```bash
cd my-app/frontend && npm install && cd ..
```

## Creating Your First Entity

Generate a CRUD entity with fields specified on the command line:

```bash
romance generate entity Product title:string price:decimal description:text? in_stock:bool
```

This single command generates files across the entire stack:

**Backend (Rust):**
- `backend/src/entities/product.rs` -- SeaORM model with all columns
- `backend/src/handlers/product.rs` -- CRUD handlers (list, get, create, update, delete, bulk_create, bulk_delete)
- `backend/src/routes/product.rs` -- Axum route definitions
- `backend/migration/src/m{timestamp}_create_product_table.rs` -- Database migration

**Frontend (React + TypeScript):**
- `frontend/src/features/product/types.ts` -- TypeScript interfaces
- `frontend/src/features/product/api.ts` -- API client functions
- `frontend/src/features/product/hooks.ts` -- TanStack Query hooks
- `frontend/src/features/product/ProductList.tsx` -- List page with pagination, filtering, and sorting
- `frontend/src/features/product/ProductForm.tsx` -- Create/edit form with shadcn/ui components
- `frontend/src/features/product/ProductDetail.tsx` -- Detail page

The generator also automatically:
- Registers the entity module in `entities/mod.rs`, `handlers/mod.rs`, and `routes/mod.rs`
- Adds routes to the Axum router
- Adds imports and `<Route>` elements to `frontend/src/App.tsx`
- Adds a navigation link

### Field syntax

Fields follow the format `name:type`:

```bash
# Required string field
title:string

# Optional text field (nullable, indicated by ? suffix)
description:text?

# Decimal field for prices
price:decimal

# Boolean field
in_stock:bool

# Foreign key to another entity
category_id:uuid->Category

# Enum field with defined variants
status:enum(draft,published,archived)
```

See the [Entities Guide](./entities.md) for the full list of field types, validation rules, and advanced options.

## Running the Development Servers

Run both the backend and frontend simultaneously with hot-reloading:

```bash
romance dev
```

This spawns two processes:
- **Backend:** `cargo watch -x run` -- recompiles and restarts the Rust server on file changes (port 3001)
- **Frontend:** `npm run dev` -- Vite dev server with HMR (port 5173)

The frontend Vite config includes a proxy that forwards `/api` requests to the backend, so you can access everything from `http://localhost:5173`.

## Running Database Migrations

After generating entities, apply the database migrations:

```bash
romance db migrate
```

Other database commands:

```bash
# Check which migrations have been applied
romance db status

# Rollback the last migration
romance db rollback

# Run seed data (if configured)
romance db seed
```

## Checking Everything Works

Run all project checks in one command:

```bash
romance check
```

This runs three checks sequentially:
1. `cargo check` -- verifies the Rust backend compiles
2. `cargo test` -- runs backend unit tests
3. `npx tsc --noEmit` -- verifies TypeScript types in the frontend

## Next Steps

With your project set up and your first entity created, here are some things to explore:

- **Add more entities with relations** -- See the [Entities Guide](./entities.md) and [Relations Guide](./relations.md) for foreign keys, has-many, and many-to-many relationships.

- **Add authentication:**
  ```bash
  romance generate auth
  ```
  Generates JWT authentication with a User entity, login/register handlers, and frontend auth context with protected routes.

- **Add an admin panel:**
  ```bash
  romance generate admin
  ```
  Generates an admin dashboard with entity management (requires auth to be generated first).

- **Add project features with addons:**
  ```bash
  romance add validation      # Backend (validator) + frontend (Zod) validation
  romance add storage         # File/image upload with pluggable storage
  romance add search          # Full-text search (PostgreSQL tsvector)
  romance add security        # Rate limiting + security headers
  romance add observability   # Structured logging + request ID tracing
  romance add soft-delete     # Soft-delete with restore/force-delete
  romance add audit-log       # Audit logging (requires auth)
  romance add oauth google    # OAuth social authentication
  romance add dashboard       # Developer dashboard
  ```

- **Generate TypeScript types from Rust structs:**
  ```bash
  romance generate types
  ```

- **Generate an OpenAPI spec:**
  ```bash
  romance generate openapi
  ```

- **Run tests with a temporary database:**
  ```bash
  romance test
  ```

- **Update scaffold files when Romance releases new templates:**
  ```bash
  romance update
  ```
  This command compares your generated files against the latest templates using SHA-256 hashes and offers interactive conflict resolution when you have local modifications.

## CLI Command Reference

| Command | Description |
|---------|-------------|
| `romance new <name>` | Create a new full-stack project |
| `romance generate entity <name> [fields...]` | Generate CRUD entity (interactive if no fields given) |
| `romance generate auth` | Generate JWT authentication |
| `romance generate admin` | Generate admin panel (requires auth) |
| `romance generate types` | Generate TypeScript types via ts-rs |
| `romance generate openapi` | Generate OpenAPI spec via utoipa |
| `romance add <feature>` | Add a feature addon to the project |
| `romance dev` | Run backend + frontend dev servers |
| `romance check` | Run cargo check + cargo test + tsc |
| `romance test` | Run tests with temporary database |
| `romance db migrate` | Run pending migrations |
| `romance db rollback` | Rollback last migration |
| `romance db status` | Show migration status |
| `romance db seed` | Run seed data |
| `romance update` | Update scaffold files to latest templates |
| `romance update --init` | Create baseline manifest for existing project |
| `romance destroy entity <name>` | Remove a generated entity and its files |
| `romance doctor` | Check project health and dependencies |
| `romance completions <shell>` | Generate shell completions (bash, zsh, fish) |
| `romance run <command> [args]` | Run a custom management command |
