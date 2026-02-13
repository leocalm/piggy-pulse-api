# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project Overview

**Project Name:** Budget API
**Description:** A high-performance Rust backend for a personal budgeting tool.
**Date:** February 13, 2026

The application provides a RESTful API for managing personal finances, including budgets, transactions, accounts, categories, and vendors.

### Tech Stack

*   **Language:** Rust (2024 Edition)
*   **Web Framework:** Rocket v0.5.1
*   **Database:** PostgreSQL (via SQLx)
*   **Async Runtime:** Tokio
*   **Serialization:** Serde
*   **Authentication:** Argon2 (hashing), secure sessions
*   **Documentation:** Rocket Okapi (OpenAPI/Swagger)
*   **Infrastructure:** Docker, Terraform (Hetzner), Ansible

## Build and Development Commands

| Task | Command |
| :--- | :--- |
| **Run Server** | `cargo run` |
| **Run Cron** | `cargo run --bin cron -- generate-periods` |
| **Build** | `cargo build` |
| **Build (Release)** | `cargo build --release` |
| **Test** | `cargo test` |
| **Test Specific** | `cargo test <test_name>` |
| **Format** | `cargo fmt` |
| **Lint** | `cargo clippy --workspace --all-targets -- -D warnings` |
| **Audit Deps** | `cargo audit` |

## Project Structure

*   `src/` - Application source code.
    *   `main.rs` - Application entry point.
    *   `lib.rs` - Library root.
    *   `bin/` - Auxiliary binaries (e.g., `cron.rs`).
    *   `routes/` - API endpoint definitions.
    *   `models/` - Data structures and domain logic.
    *   `database/` - Database access layer (Repositories).
*   `migrations/` - SQL migration files (`up.sql` / `down.sql`).
*   `infra/` - Terraform configuration for Hetzner.
*   `ansible/` - Ansible playbooks for server hardening and deployment.
*   `deploy/` - Docker Compose setups for production.
*   `docs/` - Additional documentation (e.g., 2FA).

## Configuration

Configuration is loaded via `figment` in priority order (highest wins):

1. Environment variables prefixed with `BUDGET_` — use `__` to separate nested keys (e.g. `BUDGET_DATABASE__URL`)
2. `Budget.toml` in the project root
3. Compiled-in defaults

### Setup

1.  **Environment Variables:**
    *   `DATABASE_URL`: `postgres://user:password@localhost:5432/budget_db`
    *   `ROCKET_SECRET_KEY`: Required for non-debug profiles. Generate with `openssl rand -base64 32`.
2.  **Config File:**
    *   Copy `Budget.toml.example` to `Budget.toml` for local overrides.

### Key Config Sections

| Section | Key | Default |
|---|---|---|
| `[database]` | `url` | `postgres://localhost/budget_db` |
| | `max_connections` | 16 |
| | `min_connections` | 4 |
| `[server]` | `port` | 8000 |
| `[logging]` | `level` | `info` |
| `[cors]` | `allowed_origins` | `["*"]` |
| | `allow_credentials` | `false` |
| `[rate_limit]` | `read_limit` | 300 |
| `[api]` | `base_path` | `/api/v1` |
| | `expose_docs` | `false` |

> **Note:** Wildcard origins (`*`) combined with `allow_credentials = true` is an invalid combination.

## Database Setup

Migrations are managed by `sqlx-cli`.

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run       # apply all pending migrations
sqlx migrate revert    # roll back the last migration
sqlx migrate info      # show migration status
sqlx migrate add <description>   # create new migration
```

## Architecture

### Layered Architecture Pattern

1. **Routes Layer** (`src/routes/`): Rocket handlers for HTTP I/O.
2. **Service Layer** (`src/service/`): Light business logic helpers.
3. **Database Layer** (`src/database/`): Concrete data access methods implemented directly on `PostgresRepository`.

### API Versioning & Documentation

*   **Versioning:**
    *   Current stable version: `v1`.
    *   Base path: `/api/v1`.
    *   Follows semantic versioning. Breaking changes trigger a new major version (v2).
*   **Documentation:**
    *   OpenAPI docs are disabled by default. Enable via `api.expose_docs = true` (or `BUDGET_API__EXPOSE_DOCS=true`).
    *   Docs URL: `/api/v1/docs` (Swagger UI).
    *   Spec URL: `/api/v1/openapi.json`.

### Repository Implementation (concrete, no traits)

There are **no repository traits**. Each `src/database/<entity>.rs` file implements `impl PostgresRepository { ... }`.
All repository methods receive `&current_user.id` and scope every query to that user.

### Authentication

*   Cookie-based authentication (`src/auth.rs`) via `CurrentUser` request guard.
*   Sessions are `Secure`, `HttpOnly`, `SameSite=Lax`.

### Pagination

List endpoints use keyset (cursor-based) pagination via `CursorParams`.
Query params: `cursor` (UUID) and `limit` (default 50, max 200).

## Infrastructure & Deployment

The project supports a hardened single-VM production setup using the following tools:

*   **Provisioning:** Terraform (`infra/hetzner`).
*   **Configuration:** Ansible (`ansible/`).
*   **Orchestration:** Docker Compose (`deploy/production`).
*   **CI/CD:** Drone CI (managed in `.drone.yml`) and GitHub Workflows.

### CI Discipline

Always run the full PR check suite locally before pushing:
*   `cargo fmt --check`
*   `cargo clippy --workspace --all-targets -- -D warnings`
*   `cargo build --verbose`
*   `cargo test --verbose`
*   `cargo audit`

### VPS Facts (Production)

*   **OS:** Ubuntu 24.04 LTS.
*   **Docker:** rootful Docker daemon via `/var/run/docker.sock`.
*   **Deploy User:** `deploy` (key-only SSH, member of `docker` group).
*   **Location:** `/opt/piggypulse/budget`.

### Key Implementation Patterns

**Adding a New Endpoint:**
1.  Add `#[openapi(tag = "...")]` annotation.
2.  Register handler in `rocket_okapi::openapi_get_routes_spec![]` in `src/routes/<entity>.rs`.
3.  Ensure response types derive `JsonSchema`.

**Error Handling:**
*   `AppError` handles DB errors, validation, etc.
*   `JsonBody<T>` provides detailed validation errors (422).
