# Romance Deployment Guide

Production deployment guide covering Docker, CI/CD, environment configuration, and security hardening.

---

## Table of Contents

- [Docker Setup](#docker-setup)
  - [Backend Dockerfile](#backend-dockerfile)
  - [Frontend Dockerfile](#frontend-dockerfile)
  - [Docker Compose](#docker-compose)
  - [Building Images](#building-images)
- [Nginx Configuration](#nginx-configuration)
- [Environment Variables for Production](#environment-variables-for-production)
- [Database Migrations in Production](#database-migrations-in-production)
- [Health Check Endpoints](#health-check-endpoints)
- [Graceful Shutdown](#graceful-shutdown)
- [CI/CD with GitHub Actions](#cicd-with-github-actions)
- [Building for Production Without Docker](#building-for-production-without-docker)
- [Security Checklist](#security-checklist)

---

## Docker Setup

Romance generates Docker and Docker Compose files during project scaffolding (`romance new`). These files are located in the project root directory.

### Backend Dockerfile

**File:** `Dockerfile`

The backend uses a multi-stage build to produce a minimal runtime image.

```dockerfile
# Build stage
FROM rust:1.83 AS builder
WORKDIR /app
COPY backend/ .
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/my_app-backend /usr/local/bin/app
EXPOSE 3001
CMD ["app"]
```

**Key points:**
- The builder stage compiles the Rust backend in release mode
- The runtime stage uses `debian:bookworm-slim` for a small image with glibc compatibility
- Only `ca-certificates` and `libssl3` are installed as runtime dependencies
- The compiled binary is copied as `/usr/local/bin/app`
- Port 3001 is exposed (configurable via the `PORT` environment variable)

### Frontend Dockerfile

**File:** `Dockerfile.frontend`

The frontend uses a multi-stage build with Node.js for building and nginx for serving.

```dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
COPY frontend/nginx.conf /etc/nginx/conf.d/default.conf
EXPOSE 80
```

**Key points:**
- `npm ci` is used instead of `npm install` for reproducible builds
- The Vite build output from `/app/dist` is copied into the nginx html directory
- A custom nginx configuration is included to handle SPA routing and API proxying

### Docker Compose

**File:** `docker-compose.yml`

The generated Docker Compose file defines four services: PostgreSQL, Redis, backend, and frontend.

```yaml
version: '3.8'

services:
  db:
    image: postgres:16
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: my_app
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data

  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"

  backend:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "3001:3001"
    environment:
      DATABASE_URL: postgres://postgres:postgres@db:5432/my_app
      REDIS_URL: redis://redis:6379
      JWT_SECRET: change-me-in-production
      RUST_LOG: info
    depends_on:
      - db
      - redis

  frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile.frontend
    ports:
      - "5173:80"
    depends_on:
      - backend

volumes:
  pgdata:
```

**Service details:**

| Service | Image | Ports | Purpose |
|---------|-------|-------|---------|
| `db` | `postgres:16` | 5432 | PostgreSQL database with persistent volume |
| `redis` | `redis:7-alpine` | 6379 | Redis for caching and task queues |
| `backend` | Custom (Dockerfile) | 3001 | Axum API server |
| `frontend` | Custom (Dockerfile.frontend) | 5173 -> 80 | nginx serving the React SPA |

### Building Images

```bash
# Build all services
docker compose build

# Build and start all services
docker compose up --build

# Start in detached mode
docker compose up -d

# View logs
docker compose logs -f backend
docker compose logs -f frontend

# Stop all services
docker compose down

# Stop and remove volumes (destroys database data)
docker compose down -v
```

**Production build recommendations:**

```bash
# Build with specific tags
docker build -t my-app-backend:v1.0.0 -f Dockerfile .
docker build -t my-app-frontend:v1.0.0 -f Dockerfile.frontend .

# Push to a container registry
docker tag my-app-backend:v1.0.0 registry.example.com/my-app-backend:v1.0.0
docker push registry.example.com/my-app-backend:v1.0.0
```

---

## Nginx Configuration

**File:** `frontend/nginx.conf`

The generated nginx configuration handles SPA routing and API reverse proxying.

```nginx
server {
    listen 80;
    root /usr/share/nginx/html;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api {
        proxy_pass http://backend:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

**Behavior:**

- All paths under `/` serve static files from the Vite build output. If a file is not found, `/index.html` is served (required for SPA client-side routing).
- All paths under `/api` are reverse-proxied to the backend service on port 3001. The `Host` and `X-Real-IP` headers are forwarded.

**Production hardening suggestions:**

For a production deployment behind a load balancer or reverse proxy, consider extending the nginx configuration:

```nginx
server {
    listen 80;
    root /usr/share/nginx/html;
    index index.html;

    # Security headers
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header Referrer-Policy "strict-origin-when-cross-origin" always;

    # Gzip compression
    gzip on;
    gzip_types text/plain text/css application/json application/javascript text/xml application/xml text/javascript;

    # Cache static assets
    location /assets {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api {
        proxy_pass http://backend:3001;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts
        proxy_connect_timeout 30s;
        proxy_read_timeout 60s;
        proxy_send_timeout 60s;
    }
}
```

---

## Environment Variables for Production

The following environment variables should be configured for production deployments. These are set either in `docker-compose.yml`, your hosting provider's environment configuration, or a production `.env` file.

### Required

| Variable | Example | Notes |
|----------|---------|-------|
| `DATABASE_URL` | `postgres://user:pass@host:5432/dbname` | Use strong credentials. Enable SSL: `?sslmode=require`. |
| `JWT_SECRET` | (64+ character random string) | Generate with `openssl rand -base64 64`. Never use the development default. |
| `PORT` | `8080` | Production port. Often `8080` behind a reverse proxy. |
| `RUST_LOG` | `info` | Use `info` or `warn` in production. Avoid `debug` for performance. |
| `CORS_ORIGIN` | `https://my-app.example.com` | Set to your production frontend domain. |

### Optional (addon-specific)

| Variable | Required When | Example |
|----------|---------------|---------|
| `REDIS_URL` | cache, tasks addons | `redis://redis-host:6379` |
| `S3_BUCKET` | storage addon (S3 backend) | `my-app-uploads` |
| `S3_REGION` | storage addon (S3 backend) | `us-east-1` |
| `S3_ACCESS_KEY` | storage addon (S3 backend) | `AKIA...` |
| `S3_SECRET_KEY` | storage addon (S3 backend) | (secret) |
| `OAUTH_*_CLIENT_ID` | OAuth addon | (from OAuth provider) |
| `OAUTH_*_CLIENT_SECRET` | OAuth addon | (from OAuth provider) |
| `SMTP_HOST` | email addon | `smtp.example.com` |
| `SMTP_PORT` | email addon | `587` |
| `SMTP_USER` | email addon | `noreply@example.com` |
| `SMTP_PASSWORD` | email addon | (secret) |
| `FROM_EMAIL` | email addon | `noreply@example.com` |

### Using romance.production.toml

The `romance.production.toml` file provides configuration overrides for the Romance CLI when running in production mode. Activate it by setting:

```bash
export ROMANCE_ENV=production
```

Default production overrides generated by `romance new`:

```toml
[backend]
port = 8080

[security]
rate_limit_anon_rpm = 30
rate_limit_auth_rpm = 120
cors_origins = []

[storage]
backend = "s3"
```

---

## Database Migrations in Production

### Running Migrations

Migrations should be run before starting the application. In a Docker environment, this is typically done as a startup step or an init container.

**Manual execution:**

```bash
# From the project root
romance db migrate

# Or directly with cargo
cd backend && cargo run --bin migration -- up
```

**In Docker Compose, add a migration service:**

```yaml
services:
  migrate:
    build:
      context: .
      dockerfile: Dockerfile
    command: ["sh", "-c", "cd /app && cargo run --bin migration -- up"]
    environment:
      DATABASE_URL: postgres://postgres:postgres@db:5432/my_app
    depends_on:
      db:
        condition: service_healthy

  backend:
    # ...
    depends_on:
      migrate:
        condition: service_completed_successfully
```

**Alternatively, run migrations in the application entrypoint:**

```bash
# Create a custom entrypoint script
#!/bin/sh
set -e
# Run migrations
cd /app && cargo run --release --bin migration -- up
# Start the application
exec app
```

### Rollback

```bash
romance db rollback

# Or directly
cd backend && cargo run --bin migration -- down
```

### Migration Status

```bash
romance db status

# Or directly
cd backend && cargo run --bin migration -- status
```

### Best Practices

- Always back up the database before running migrations in production
- Test migrations against a staging database that mirrors production
- Migration files are immutable once applied -- never modify a migration that has been run in production
- For complex schema changes, consider multi-step migrations with backward compatibility
- The `romance destroy entity` command does NOT remove migration files; manage those manually

---

## Health Check Endpoints

The generated backend includes two health check endpoints at the router level.

### GET /health

Returns the server's status and current timestamp. Does not check external dependencies.

**Response (200 OK):**

```json
{
  "status": "ok",
  "timestamp": "2025-01-15T10:30:00.000Z"
}
```

This endpoint is suitable for load balancer liveness probes. It responds immediately without any I/O operations.

### GET /ready

Checks that the server can connect to the database. Returns an error if the database is unreachable.

**Response (200 OK):**

```json
{
  "status": "ready",
  "database": "connected"
}
```

**Response (503 Service Unavailable):**

Returned when the database ping fails. No body is returned.

This endpoint is suitable for Kubernetes readiness probes or load balancer health checks that should only route traffic to healthy instances.

### Docker Compose Health Check

Add health checks to the backend service in `docker-compose.yml`:

```yaml
services:
  backend:
    # ...
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3001/health"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
```

### Kubernetes Probes

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 3001
  initialDelaySeconds: 5
  periodSeconds: 15

readinessProbe:
  httpGet:
    path: /ready
    port: 3001
  initialDelaySeconds: 10
  periodSeconds: 10
```

---

## Graceful Shutdown

The generated backend application includes a graceful shutdown handler that listens for termination signals.

**Signals handled:**

- `SIGINT` (Ctrl+C) -- Immediate graceful shutdown
- `SIGTERM` (Unix) -- Standard termination signal used by Docker, Kubernetes, and process managers

**Behavior:**

1. The signal is received
2. A log message is emitted: `"Shutdown signal received, starting graceful shutdown..."`
3. Axum's `with_graceful_shutdown` stops accepting new connections
4. In-flight requests are allowed to complete
5. The server exits cleanly

**Docker stop behavior:**

Docker sends `SIGTERM` first and waits for the `stop_grace_period` (default: 10 seconds). If the process does not exit within this period, Docker sends `SIGKILL`. For applications with long-running requests, consider increasing the grace period:

```yaml
services:
  backend:
    # ...
    stop_grace_period: 30s
```

---

## CI/CD with GitHub Actions

Romance generates a GitHub Actions workflow at `.github/workflows/ci.yml`.

### Generated Workflow

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  backend:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_USER: postgres
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: my_app_test
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workdir: backend

      - name: Check
        working-directory: backend
        run: cargo check

      - name: Test
        working-directory: backend
        env:
          DATABASE_URL: postgres://postgres:postgres@localhost:5432/my_app_test
          JWT_SECRET: test-secret
        run: cargo test

      - name: Clippy
        working-directory: backend
        run: cargo clippy -- -D warnings

  frontend:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: frontend/package-lock.json

      - name: Install
        working-directory: frontend
        run: npm ci

      - name: Type check
        working-directory: frontend
        run: npx tsc --noEmit

      - name: Build
        working-directory: frontend
        run: npm run build
```

### Workflow Details

**Backend job:**

| Step | What it does |
|------|-------------|
| PostgreSQL service | Spins up a PostgreSQL 16 container with health checks |
| `actions/checkout@v4` | Checks out the repository |
| `dtolnay/rust-toolchain@stable` | Installs the stable Rust toolchain |
| `Swatinem/rust-cache@v2` | Caches Cargo build artifacts for faster CI runs |
| `cargo check` | Verifies the backend compiles |
| `cargo test` | Runs all backend tests against the test database |
| `cargo clippy -- -D warnings` | Lint check; fails on any warnings |

**Frontend job:**

| Step | What it does |
|------|-------------|
| `actions/checkout@v4` | Checks out the repository |
| `actions/setup-node@v4` | Installs Node.js 20 with npm cache |
| `npm ci` | Installs dependencies from lockfile |
| `npx tsc --noEmit` | TypeScript type checking |
| `npm run build` | Verifies the production build succeeds |

### Adding a Deployment Step

To add automatic deployment after CI passes, extend the workflow:

```yaml
  deploy:
    needs: [backend, frontend]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main' && github.event_name == 'push'
    steps:
      - uses: actions/checkout@v4

      - name: Build and push Docker images
        run: |
          docker build -t my-registry/my-app-backend:${{ github.sha }} -f Dockerfile .
          docker build -t my-registry/my-app-frontend:${{ github.sha }} -f Dockerfile.frontend .
          docker push my-registry/my-app-backend:${{ github.sha }}
          docker push my-registry/my-app-frontend:${{ github.sha }}

      - name: Deploy
        run: |
          # Your deployment steps here (e.g., kubectl apply, docker compose pull, etc.)
```

---

## Building for Production Without Docker

If you prefer to deploy without Docker, build the backend binary and the frontend static assets separately.

### Backend

```bash
cd backend
cargo build --release

# The binary is at:
# backend/target/release/{project_name_snake}-backend
```

Run the binary with required environment variables:

```bash
DATABASE_URL="postgres://user:pass@host:5432/dbname" \
JWT_SECRET="your-secret" \
PORT=8080 \
RUST_LOG=info \
CORS_ORIGIN="https://your-domain.com" \
./target/release/my_app-backend
```

### Frontend

```bash
cd frontend
npm ci
npm run build

# Static files are in: frontend/dist/
```

Serve `frontend/dist/` with any static file server (nginx, caddy, S3 + CloudFront, etc.). Configure the server to fall back to `index.html` for SPA routing.

---

## Security Checklist

Before deploying to production, verify the following:

### Secrets and Credentials

- [ ] `JWT_SECRET` is a strong random value (64+ characters), not the development default
- [ ] `DATABASE_URL` uses a dedicated database user with minimal required privileges
- [ ] Database password is strong and unique
- [ ] All OAuth client secrets are properly configured (if using OAuth)
- [ ] `backend/.env` is not committed to version control (check `.gitignore`)

### Network and Transport

- [ ] HTTPS is enabled (via reverse proxy, load balancer, or TLS termination)
- [ ] `CORS_ORIGIN` is set to the specific production domain, not a wildcard
- [ ] The backend is not directly exposed to the internet (use a reverse proxy)
- [ ] Database port (5432) is not publicly accessible
- [ ] Redis port (6379) is not publicly accessible

### Application Security

- [ ] The generated security headers are active (X-Content-Type-Options, X-Frame-Options, X-XSS-Protection, Strict-Transport-Security are set by default in the generated `main.rs`)
- [ ] Rate limiting is configured appropriately (`romance add security`)
- [ ] Validation is enabled for all user inputs (`romance add validation`)
- [ ] CSRF protection is evaluated and enabled if needed (`[security] csrf = true`)

### Infrastructure

- [ ] Database backups are configured and tested
- [ ] Health check endpoints (`/health`, `/ready`) are monitored
- [ ] Log level is set to `info` or `warn` (not `debug` or `trace`)
- [ ] Graceful shutdown timeout is appropriate for your workload
- [ ] Resource limits (CPU, memory) are set for containers

### Ongoing Maintenance

- [ ] Dependencies are regularly updated (`cargo update`, `npm update`)
- [ ] Romance scaffold is kept current (`romance update`)
- [ ] Database migrations are tested in staging before production
- [ ] Monitoring and alerting are configured for error rates and latency
