# Budget API

Rust backend for a budgeting tool, using Rocket for the HTTP server and PostgreSQL via SQLx.

## Prerequisites

- Rust (stable)
- PostgreSQL running locally or remotely

Set `DATABASE_URL` in your environment. `ROCKET_SECRET_KEY` is required in non-debug profiles (recommended for local testing of auth flows). Example:

```bash
export DATABASE_URL=postgres://user:password@localhost:5432/budget_db
# Optional for local debug; required for non-debug profiles:
# export ROCKET_SECRET_KEY=$(openssl rand -base64 32)
```

Or copy the example config file and edit as needed:

```bash
cp Budget.toml.example Budget.toml
```

See `CONFIGURATION.md` for the full configuration reference (including rate limiting).

## Running the API

```bash
cargo run
```

The server will start (by default) on `http://127.0.0.1:8000`.

## Running Cron Jobs

Periodic jobs are executed by a dedicated binary instead of an HTTP endpoint.

Run automatic budget period generation:

```bash
cargo run --bin cron -- generate-periods
```

### Endpoints

All API endpoints are versioned and mounted under `/api/v1` by default. The current version is `v1` (configurable via `api.base_path` or `BUDGET_API__BASE_PATH`). You can also expose the same routes under additional base paths via `api.additional_base_paths`. The examples below assume the default base path.

- `GET /api/v1/health` – simple health check.
- `POST /api/v1/budgets` – create a budget.
- `GET /api/v1/budgets` – list budgets.

#### API Documentation

- Swagger/OpenAPI documentation is disabled by default.
- Enable docs explicitly with `api.expose_docs = true` (or `BUDGET_API__EXPOSE_DOCS=true`).
- When enabled, docs are available at `/api/v1/docs` and spec at `/api/v1/openapi.json`.

For complete API lifecycle information including deprecation policies and migration guides, see [API_LIFECYCLE.md](API_LIFECYCLE.md).

## Production Security Baseline

For production deployments, configure these minimum settings:

- Set a strong `ROCKET_SECRET_KEY` (required in non-debug profiles).
- Restrict CORS origins (do not use wildcard origins with credentials).
- Configure allowed origins explicitly, for example:

```toml
[cors]
allowed_origins = ["https://app.example.com"]
allow_credentials = true
```

```bash
export BUDGET_CORS__ALLOWED_ORIGINS='["https://app.example.com"]'
export BUDGET_CORS__ALLOW_CREDENTIALS=true
```

- Keep `api.expose_docs = false` unless docs are intentionally public.
- If running behind a reverse proxy/load balancer, enable forwarded IP rate limiting:
  - `BUDGET_RATE_LIMIT__USE_FORWARDED_IP=true`
  - `BUDGET_RATE_LIMIT__FORWARDED_IP_HEADER=x-forwarded-for`
- Use Redis-backed rate limiting in production:
  - `BUDGET_RATE_LIMIT__BACKEND=redis`

Cookie/session hardening is enabled in code (`Secure`, `HttpOnly`, `SameSite=Lax`).

## Dependency Audit

Run dependency vulnerability checks with:

```bash
cargo audit
```

This repository includes `.cargo/audit.toml` with a documented temporary ignore for `RUSTSEC-2023-0071` (transitive `rsa` via `sqlx` macro toolchain, no upstream fix available as of 2026-02-06).

## Database schema & migrations

Migrations are managed by [sqlx-cli](https://github.com/jmoiron/sqlx) and live under `migrations/`.
Each migration is its own directory containing `up.sql` (apply) and `down.sql` (rollback):

```
migrations/
├── 0001_init/
│   ├── up.sql      # create all tables
│   └── down.sql    # drop all tables
└── 0002_add_transaction_indexes/
    ├── up.sql      # add indexes & schema fixes
    └── down.sql    # remove indexes & revert fixes
```

Install `sqlx-cli` once:

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
```

Then set `DATABASE_URL` and apply all pending migrations:

```bash
export DATABASE_URL=postgres://user:password@localhost:5432/budget_db
sqlx migrate run
```

To roll back the last migration:

```bash
sqlx migrate revert
```

To check migration status:

```bash
sqlx migrate info
```
