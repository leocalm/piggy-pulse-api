# Budget API

Rust backend for a budgeting tool, using Rocket for the HTTP server and PostgreSQL via SQLx.

## Prerequisites

- Rust (stable)
- PostgreSQL running locally or remotely

Set `DATABASE_URL` and `ROCKET_SECRET_KEY` in your environment, e.g.:

```bash
export DATABASE_URL=postgres://user:password@localhost:5432/budget_db
export ROCKET_SECRET_KEY=$(openssl rand -base64 32)
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

### Endpoints

All API endpoints are versioned and mounted under `/api/v1` by default. The current version is `v1` (configurable via `api.base_path` or `BUDGET_API__BASE_PATH`). You can also expose the same routes under additional base paths via `api.additional_base_paths`. The examples below assume the default base path.

- `GET /api/v1/health` – simple health check.
- `POST /api/v1/budgets` – create a budget.
- `GET /api/v1/budgets` – list budgets.

#### API Documentation

- Swagger/OpenAPI documentation is hidden by default
- Enable it by setting `BUDGET_API__ENABLE_SWAGGER=true` (then `/api/v1/docs` and `/api/v1/openapi.json` are available)

For complete API lifecycle information including deprecation policies and migration guides, see [API_LIFECYCLE.md](API_LIFECYCLE.md).

### Security Defaults

- CORS is denied by default (empty `allowed_origins`). Set `BUDGET_CORS__ALLOWED_ORIGINS` to your frontend origin(s).
- Auth cookies are `HttpOnly` always. In `release` profile they are also `Secure` and `SameSite=Strict`. In non-release profiles they use `SameSite=Lax`.

### Docker Compose (Development)

The `docker-compose.yaml` file only provisions PostgreSQL and requires explicit environment variables:

- `POSTGRES_USER`
- `POSTGRES_PASSWORD`
- `POSTGRES_DB`

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
