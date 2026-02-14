# Romance CLI -- Addons Reference

Addons extend a Romance project with opt-in features. Each addon is installed with `romance add <name>`, is idempotent (skipped if already installed), and automatically regenerates the project's AI context after installation.

All addons require a valid Romance project (`romance.toml` must exist). Some addons have additional prerequisites noted below.

---

## Table of Contents

- **Security & Auth**
  - [Validation](#validation)
  - [Security](#security)
  - [OAuth](#oauth)
  - [API Keys](#api-keys)
- **Data Management**
  - [Soft Delete](#soft-delete)
  - [Audit Log](#audit-log)
  - [Search](#search)
  - [Storage](#storage)
- **Infrastructure**
  - [Observability](#observability)
  - [Email](#email)
  - [Cache](#cache)
  - [Tasks](#tasks)
  - [WebSocket](#websocket)
- **Developer Experience**
  - [Dashboard](#dashboard)
  - [i18n](#i18n)

---

# Security & Auth

## Validation

Adds field-level validation to entities using the `validator` crate. Once installed, entity generation supports validation rule annotations on fields.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add validation
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/validation.rs` | Validation middleware and helpers |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod validation;` declaration |
| `backend/Cargo.toml` | Adds `validator = { version = "0.19", features = ["derive"] }` |
| `romance.toml` | Adds `validation = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
validation = true
```

### Usage

After installation, entity fields support validation rules in bracket syntax:

```bash
romance generate entity Product name:string[min=3,max=100] price:decimal
```

---

## Security

Adds security headers middleware and per-user rate limiting to the Axum backend. Configures both anonymous (IP-based) and authenticated (user-based) rate limit tiers.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add security
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/middleware/security_headers.rs` | Security headers middleware (CSP, HSTS, etc.) |
| `backend/src/middleware/rate_limit.rs` | Rate limiter middleware |
| `backend/src/middleware/mod.rs` | Module declarations for middleware directory |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod middleware;` declaration |
| `backend/src/routes/mod.rs` | Injects security headers and rate limit middleware layers via `ROMANCE:MIDDLEWARE` marker |
| `backend/Cargo.toml` | Adds `tower = { version = "0.5", features = ["limit", "timeout"] }`, `governor = "0.7"`, `tower-governor = "0.5"`, `base64 = "0.22"` |
| `backend/.env` | Adds `RATE_LIMIT_ANON_RPM=30`, `RATE_LIMIT_AUTH_RPM=120` |
| `backend/.env.example` | Adds `RATE_LIMIT_ANON_RPM=30`, `RATE_LIMIT_AUTH_RPM=120` |

### Configuration

```toml
# romance.toml
[security]
rate_limit_anon_rpm = 30
rate_limit_auth_rpm = 120
cors_origins = ["http://localhost:5173"]
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RATE_LIMIT_ANON_RPM` | `30` | Requests per minute for anonymous users (IP-based) |
| `RATE_LIMIT_AUTH_RPM` | `120` | Requests per minute for authenticated users (user-based) |

---

## OAuth

Adds OAuth2 social login for a specified provider. Generates backend OAuth flow, migration to add OAuth columns to the users table, and a frontend OAuth button component.

### Prerequisites

- Auth must be generated first: `romance generate auth`

### Installation

```bash
romance add oauth --provider google
romance add oauth --provider github
romance add oauth --provider discord
```

Supported providers: `google`, `github`, `discord`.

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/oauth.rs` | OAuth2 client configuration and token exchange |
| `backend/src/handlers/oauth.rs` | OAuth callback and login initiation handlers |
| `backend/src/routes/oauth.rs` | OAuth route registration |
| `backend/migration/src/m{timestamp}_add_oauth_to_users.rs` | Migration adding `oauth_provider` and `oauth_id` columns to users table |
| `frontend/src/features/auth/OAuthButton.tsx` | Frontend OAuth login button component |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod oauth;` declaration |
| `backend/src/handlers/mod.rs` | Adds `pub mod oauth;` via `ROMANCE:MODS` marker |
| `backend/src/routes/mod.rs` | Adds `pub mod oauth;` and `.merge(oauth::router())` via markers |
| `backend/migration/src/lib.rs` | Registers migration module and migration instance via markers |
| `backend/src/entities/user.rs` | Injects `oauth_provider: Option<String>` and `oauth_id: Option<String>` fields |
| `backend/Cargo.toml` | Adds `oauth2 = "4"`, `reqwest = { version = "0.12", features = ["json"] }` |
| `backend/.env` | Adds `{PROVIDER}_CLIENT_ID`, `{PROVIDER}_CLIENT_SECRET` |
| `backend/.env.example` | Adds `{PROVIDER}_CLIENT_ID`, `{PROVIDER}_CLIENT_SECRET` |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `{PROVIDER}_CLIENT_ID` | `your-client-id` | OAuth2 client ID from the provider |
| `{PROVIDER}_CLIENT_SECRET` | `your-client-secret` | OAuth2 client secret from the provider |

Where `{PROVIDER}` is the uppercase provider name (e.g., `GOOGLE`, `GITHUB`, `DISCORD`).

---

## API Keys

Adds API key authentication for machine-to-machine access. Keys are hashed with SHA-256 before storage. Authenticated via the `X-API-Key` request header.

### Prerequisites

- Auth must be generated first: `romance generate auth`

### Installation

```bash
romance add api-keys
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/api_keys.rs` | API key generation, hashing, and validation logic |
| `backend/migration/src/m{timestamp}_create_api_keys_table.rs` | Migration creating the `api_keys` table |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod api_keys;` declaration |
| `backend/migration/src/lib.rs` | Registers migration module and migration instance via markers |
| `backend/Cargo.toml` | Adds `sha2 = "0.10"` |

### Configuration

No additional configuration. Run migrations after installation:

```bash
romance db migrate
```

### Usage

Clients authenticate by sending the `X-API-Key` header with their key value. Keys are hashed before comparison against stored values.

---

# Data Management

## Soft Delete

Adds soft-delete support to entity generation. Instead of permanently removing records, a `deleted_at` timestamp is set. Provides restore and permanent delete endpoints.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add soft-delete
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/soft_delete.rs` | Soft-delete helper functions (filter, soft-delete, restore) |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod soft_delete;` declaration |
| `romance.toml` | Adds `soft_delete = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
soft_delete = true
```

### Usage

Once installed, all future entities generated with `romance generate entity` will use soft-delete by default. Each entity gets three delete-related endpoints:

- `DELETE /api/{entity}s/:id` -- Soft delete (sets `deleted_at`)
- `POST /api/{entity}s/:id/restore` -- Restore a soft-deleted record
- `DELETE /api/{entity}s/:id/permanent` -- Permanently delete a record

List queries automatically filter out soft-deleted records.

---

## Audit Log

Adds automatic audit logging for all create, update, and delete operations. Tracks which user performed each action and stores a JSON snapshot of the change. Includes a frontend viewer for the admin panel.

### Prerequisites

- Auth must be generated first: `romance generate auth`

### Installation

```bash
romance add audit-log
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/audit.rs` | Audit logging service and helpers |
| `backend/src/entities/audit_entry.rs` | SeaORM entity model for audit entries |
| `backend/migration/src/m{timestamp}_create_audit_entries_table.rs` | Migration creating the `audit_entries` table |
| `backend/src/handlers/audit_log.rs` | Audit log query handlers (for admin) |
| `frontend/src/features/admin/AuditLog.tsx` | Frontend audit log viewer component |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod audit;` declaration |
| `backend/src/entities/mod.rs` | Adds `pub mod audit_entry;` via `ROMANCE:MODS` marker |
| `backend/src/handlers/mod.rs` | Adds `pub mod audit_log;` via `ROMANCE:MODS` marker |
| `backend/migration/src/lib.rs` | Registers migration module and migration instance via markers |
| `romance.toml` | Adds `audit_log = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
audit_log = true
```

### Usage

After installation, all create/update/delete operations on entities are automatically logged. The audit log is viewable at `/admin/audit-log` in the admin panel. Run migrations after installation:

```bash
romance db migrate
```

---

## Search

Adds PostgreSQL full-text search support. Generates a search module, search handler, and a frontend search bar component. Entity fields can be marked as searchable.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add search
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/search.rs` | Full-text search query builder and helpers |
| `backend/src/handlers/search.rs` | Search endpoint handler |
| `frontend/src/components/SearchBar.tsx` | Reusable search bar component |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod search;` declaration |
| `backend/src/handlers/mod.rs` | Adds `pub mod search;` via `ROMANCE:MODS` marker |
| `romance.toml` | Adds `search = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
search = true
```

### Usage

Mark fields as searchable during entity generation using the `[searchable]` annotation:

```bash
romance generate entity Article title:string[searchable] body:text[searchable] author:string
```

Search endpoint for each entity: `GET /api/{entity}s/search?q=term`

---

## Storage

Adds file upload and storage support. Generates a storage backend abstraction, upload handlers and routes, and a frontend file upload component. Supports local filesystem storage by default.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add storage
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/storage.rs` | Storage backend trait and local filesystem implementation |
| `backend/src/handlers/upload.rs` | Multipart upload handler |
| `backend/src/routes/upload.rs` | Upload route registration |
| `frontend/src/components/FileUpload.tsx` | Drag-and-drop file upload component |
| `backend/uploads/` | Directory for uploaded files (created automatically) |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod storage;` declaration |
| `backend/src/handlers/mod.rs` | Adds `pub mod upload;` via `ROMANCE:MODS` marker |
| `backend/src/routes/mod.rs` | Adds `pub mod upload;` and `.merge(upload::router())` via markers |
| `backend/Cargo.toml` | Adds multipart feature to axum: `axum = { version = "0.8", features = ["json", "multipart"] }`, adds `mime = "0.3"` |
| `backend/.env` | Adds `UPLOAD_DIR`, `UPLOAD_URL`, `MAX_FILE_SIZE` |
| `backend/.env.example` | Adds `UPLOAD_DIR`, `UPLOAD_URL`, `MAX_FILE_SIZE` |

### Configuration

```toml
# romance.toml
[storage]
backend = "local"
upload_dir = "./uploads"
max_file_size = "10MB"
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `UPLOAD_DIR` | `./uploads` | Directory for storing uploaded files |
| `UPLOAD_URL` | `/uploads` | URL prefix for serving uploaded files |
| `MAX_FILE_SIZE` | `10MB` | Maximum allowed file size |

### Usage

Use file-related field types during entity generation:

```bash
romance generate entity Profile name:string avatar:image document:file
```

---

# Infrastructure

## Observability

Adds structured logging with request ID propagation. Replaces the default tracing setup with a full observability module that attaches unique request IDs to every log line and response header.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add observability
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/middleware/tracing_setup.rs` | Structured tracing initialization with formatted output |
| `backend/src/middleware/request_id.rs` | Request ID generation and propagation middleware |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod middleware;` declaration; replaces scaffold's `tracing_subscriber::registry()` block with `crate::middleware::tracing_setup::init_tracing()`; removes unused `tracing_subscriber` import |
| `backend/src/middleware/mod.rs` | Created (if absent) or updated with `pub mod tracing_setup;` and `pub mod request_id;` |
| `backend/src/routes/mod.rs` | Injects request ID layer via `ROMANCE:MIDDLEWARE` marker |
| `backend/Cargo.toml` | Adds `tower-http = { version = "0.6", features = ["cors", "trace", "request-id", "propagate-header"] }` |
| `backend/.env` | Adds `RUST_LOG=info` |
| `backend/.env.example` | Adds `RUST_LOG=info` |

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RUST_LOG` | `info` | Log level filter. Set to `debug` for verbose logging. |

---

## Email

Adds an SMTP email service with the `lettre` crate. Generates a reusable email service module and a password reset handler out of the box.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add email
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/email.rs` | Email service with SMTP transport |
| `backend/src/handlers/password_reset.rs` | Password reset request and confirmation handlers |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod email;` declaration |
| `backend/src/handlers/mod.rs` | Adds `pub mod password_reset;` via `ROMANCE:MODS` marker |
| `backend/Cargo.toml` | Adds `lettre = { version = "0.11", features = ["tokio1-native-tls"] }` |
| `backend/.env` | Adds `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `FROM_EMAIL` |
| `backend/.env.example` | Adds `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `FROM_EMAIL` |
| `romance.toml` | Adds `email = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
email = true
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SMTP_HOST` | `smtp.example.com` | SMTP server hostname |
| `SMTP_PORT` | `587` | SMTP server port |
| `SMTP_USER` | `your_smtp_user` | SMTP authentication username |
| `SMTP_PASS` | `your_smtp_password` | SMTP authentication password |
| `FROM_EMAIL` | `noreply@example.com` | Sender email address |

### Usage

```rust
let email_service = EmailService::new();
// Password reset handler available at /api/auth/password-reset
```

---

## Cache

Adds a Redis-backed caching layer. Generates a cache service module that provides get/set/delete operations with TTL support.

### Prerequisites

None (beyond a Romance project). Requires a running Redis instance.

### Installation

```bash
romance add cache
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/cache.rs` | Cache service with Redis connection manager |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod cache;` declaration |
| `backend/Cargo.toml` | Adds `redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }` |
| `backend/.env` | Adds `REDIS_URL=redis://127.0.0.1:6379` |
| `backend/.env.example` | Adds `REDIS_URL=redis://127.0.0.1:6379` |
| `romance.toml` | Adds `cache = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
cache = true
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection URL |

### Usage

```rust
let cache = CacheService::new()?;
cache.set("key", &value, 300).await?;  // TTL in seconds
let val: Option<T> = cache.get("key").await?;
cache.del("key").await?;
```

---

## Tasks

Adds a database-backed background task queue with a configurable worker pool, plus a recurring job scheduler. Creates a `background_task` entity with its own migration.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add tasks
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/tasks.rs` | Task queue: enqueue, dequeue, worker pool |
| `backend/src/scheduler.rs` | Recurring job scheduler (cron-like intervals) |
| `backend/src/entities/background_task.rs` | SeaORM entity model for background tasks |
| `backend/migration/src/m{timestamp}_create_background_tasks_table.rs` | Migration creating the `background_tasks` table |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod tasks;` and `mod scheduler;` declarations |
| `backend/src/entities/mod.rs` | Adds `pub mod background_task;` via `ROMANCE:MODS` marker |
| `backend/migration/src/lib.rs` | Registers migration module and migration instance via markers |
| `romance.toml` | Adds `background_tasks = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
background_tasks = true
```

### Usage

Run migrations after installation:

```bash
romance db migrate
```

Enqueue and process tasks:

```rust
// Enqueue a task
TaskQueue::new(db).enqueue("send_email", payload).await?;

// Start a worker pool (4 concurrent workers)
TaskQueue::new(db).start_worker(4, handler).await;
```

Schedule recurring jobs:

```rust
let mut scheduler = scheduler::Scheduler::new();
scheduler.add_job("cleanup", Duration::from_secs(3600), || {
    tokio::spawn(async { /* ... */ })
});
scheduler.start();
```

---

## WebSocket

Adds real-time WebSocket support. Generates a backend WebSocket handler with connection management and a frontend `useWebSocket` React hook. Entity events are automatically bridged to connected WebSocket clients.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add websocket
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/ws.rs` | WebSocket handler, `WebSocketState` (connection registry), event bridge |
| `frontend/src/lib/useWebSocket.ts` | React hook for WebSocket connections |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod ws;` declaration |
| `backend/src/routes/mod.rs` | Adds `/ws` route via `ROMANCE:MIDDLEWARE` marker; adds `WebSocketState` import, `ws` field to `AppState`, and event bridge spawn in `create_router` |
| `backend/Cargo.toml` | Adds `"ws"` feature to axum: `features = ["json", "ws"]` |
| `romance.toml` | Adds `websocket = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
websocket = true
```

### Usage

Backend WebSocket endpoint: `/ws`

Frontend usage:

```tsx
import { useWebSocket } from '@/lib/useWebSocket';

function MyComponent() {
  const { messages, sendMessage, isConnected } = useWebSocket('ws://localhost:3000/ws');
  // ...
}
```

Entity create/update/delete events are automatically broadcast to all connected WebSocket clients via the event bridge.

---

# Developer Experience

## Dashboard

Adds a developer dashboard that provides an overview of the project: discovered entities, auth status, and audit log status. Generates both backend API endpoints and a frontend React page.

### Prerequisites

None (beyond a Romance project). The dashboard auto-detects whether auth and audit log addons are installed.

### Installation

```bash
romance add dashboard
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/handlers/dev_dashboard.rs` | Dashboard data handlers (entity list, stats) |
| `backend/src/routes/dev_dashboard.rs` | Dashboard route registration |
| `frontend/src/features/dev/DevDashboard.tsx` | Developer dashboard page component |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/handlers/mod.rs` | Adds `pub mod dev_dashboard;` via `ROMANCE:MODS` marker |
| `backend/src/routes/mod.rs` | Adds `pub mod dev_dashboard;` and `.merge(dev_dashboard::router())` via markers |
| `frontend/src/App.tsx` | Adds import for `DevDashboard` via `ROMANCE:IMPORTS` marker; adds `<Route path="/dev" element={<DevDashboard />} />` via `ROMANCE:APP_ROUTES` marker |

### Configuration

No additional configuration required.

### Usage

Visit `/dev` in the browser to access the developer dashboard. The dashboard automatically discovers all entities in the project (excluding `user` and `audit_entry` system entities) and reflects the current state of installed addons.

---

## i18n

Adds internationalization support with locale-aware request handling. Generates backend locale loading with JSON locale files, an Accept-Language header middleware, and a frontend translation utility.

### Prerequisites

None (beyond a Romance project).

### Installation

```bash
romance add i18n
```

### Generated Files

| File | Purpose |
|------|---------|
| `backend/src/i18n.rs` | Locale loading, translation function `t()`, and `locale_middleware` |
| `backend/locales/en.json` | English locale file (starter translations) |
| `backend/locales/ru.json` | Russian locale file (starter translations) |
| `frontend/src/lib/i18n.ts` | Frontend translation utility |

### Modified Files

| File | Change |
|------|--------|
| `backend/src/main.rs` | Adds `mod i18n;` declaration |
| `backend/src/routes/mod.rs` | Injects Accept-Language locale middleware via `ROMANCE:MIDDLEWARE` marker |
| `backend/Cargo.toml` | Adds `serde_json = "1"` (if not already present) |
| `romance.toml` | Adds `i18n = true` under `[features]` |

### Configuration

```toml
# romance.toml
[features]
i18n = true
```

### Usage

Backend -- translate strings using the `t()` function:

```rust
let msg = i18n::t("en", "common.success");
```

Access the current locale in handlers (extracted from the `Accept-Language` header):

```rust
let locale = request.extensions().get::<i18n::Locale>();
```

Frontend -- import and use the translation function:

```tsx
import { t } from '@/lib/i18n';

<p>{t("common.welcome")}</p>
```

Add new locales by creating JSON files in `backend/locales/` following the same key structure as `en.json`.

---

# Quick Reference

| Addon | Command | Requires Auth | Creates Migration | Key Dependency |
|-------|---------|:---:|:---:|----------------|
| validation | `romance add validation` | No | No | `validator 0.19` |
| security | `romance add security` | No | No | `governor 0.7`, `tower-governor 0.5` |
| oauth | `romance add oauth --provider <name>` | Yes | Yes | `oauth2 4`, `reqwest 0.12` |
| api-keys | `romance add api-keys` | Yes | Yes | `sha2 0.10` |
| soft-delete | `romance add soft-delete` | No | No | -- |
| audit-log | `romance add audit-log` | Yes | Yes | -- |
| search | `romance add search` | No | No | -- |
| storage | `romance add storage` | No | No | `mime 0.3` |
| observability | `romance add observability` | No | No | `tower-http 0.6` |
| email | `romance add email` | No | No | `lettre 0.11` |
| cache | `romance add cache` | No | No | `redis 0.27` |
| tasks | `romance add tasks` | No | Yes | -- |
| websocket | `romance add websocket` | No | No | axum `ws` feature |
| dashboard | `romance add dashboard` | No | No | -- |
| i18n | `romance add i18n` | No | No | `serde_json 1` |

For addons that create migrations, run `romance db migrate` after installation.
