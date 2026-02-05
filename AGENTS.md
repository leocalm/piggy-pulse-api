# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Project Overview

This is a Rust-based budgeting API built with Rocket web framework and PostgreSQL. The application provides a RESTful API for managing personal finances,
including budgets, transactions, accounts, categories, and vendors.

## Build and Development Commands

```bash
# Run the API server (default port 8000)
cargo run

# Build the project
cargo build

# Run in release mode
cargo build --release && cargo run --release

# Format code (max line width: 160)
cargo fmt

# Lint code
cargo clippy --workspace --all-targets -- -D warnings

# Run tests
cargo test

# Run specific test
cargo test <test_name>
```

## Configuration

Configuration is loaded via `figment` in priority order (highest wins):

1. Environment variables prefixed with `BUDGET_` — use `__` to separate nested keys (e.g. `BUDGET_DATABASE__URL`)
2. `Budget.toml` in the project root
3. Compiled-in defaults

Key sections and their defaults:

| Section | Key | Default |
|---|---|---|
| `[database]` | `url` | `postgres://localhost/budget_db` |
| | `max_connections` | 16 |
| | `min_connections` | 4 |
| | `connection_timeout` | 5 s |
| | `acquire_timeout` | 5 s |
| `[server]` | `port` | 8000 |
| | `address` | `127.0.0.1` |
| `[logging]` | `level` | `info` |
| | `json_format` | `false` |
| `[cors]` | `allowed_origins` | `["*"]` |
| | `allow_credentials` | `false` |
| `[rate_limit]` | `read_limit` | 300 |
| | `mutation_limit` | 60 |
| | `auth_limit` | 10 |
| | `window_seconds` | 60 |
| | `cleanup_interval_seconds` | 60 |
| | `require_client_ip` | `true` |

> Wildcard origins (`*`) combined with `allow_credentials = true` is an invalid combination and will panic at startup.

## Database Setup

Migrations are managed by sqlx-cli. Each migration lives in its own directory under `migrations/`
with `up.sql` (apply) and `down.sql` (rollback). Install sqlx-cli and apply:

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
sqlx migrate run       # apply all pending migrations
sqlx migrate revert    # roll back the last migration
sqlx migrate info      # show migration status
```

When adding a new migration:

```bash
sqlx migrate add <description>   # creates migrations/NNNN_description/{up,down}.sql
```

## Architecture

### Layered Architecture Pattern

The codebase keeps a simple separation of concerns:

1. **Routes Layer** (`src/routes/`): Rocket handlers for HTTP I/O.
2. **Service Layer** (`src/service/`): Light business logic helpers (e.g., account aggregation, dashboard calculations).
3. **Database Layer** (`src/database/`): Concrete data access methods implemented directly on `PostgresRepository`.

### Repository Implementation (concrete, no traits)

There are **no repository traits**. Each `src/database/<entity>.rs` file implements `impl PostgresRepository { ... }` with async methods for that entity (CRUD, queries, helpers).

Benefits:
- Less boilerplate and indirection.
- Callers (routes/services) use the concrete repository directly.
- Tests rely on pure helper functions and sample data instead of mock trait impls.

### Database Connection Management

- Uses `sqlx::PgPool` configured in `src/db.rs` via a Rocket `AdHoc` fairing (`stage_db`).
- Pool options (`max_connections`, `min_connections`, `acquire_timeout`) come from `DatabaseConfig`. Additional hard-coded limits: idle timeout 30 s, max lifetime 1800 s.
- Routes receive `&State<PgPool>`, then construct `PostgresRepository { pool: pool.inner().clone() }`.
- No `deadpool-postgres` or trait objects involved.
- All repository methods receive `&current_user.id` and scope every query to that user.

### Authentication

- Cookie-based authentication implemented in `src/auth.rs` via the `CurrentUser` request guard (`FromRequest`).
- The guard reads the private (encrypted) `user` cookie. Expected format: `<uuid>:<username>`. Returns `401 Unauthorized` if the cookie is missing or unparseable.
- `CurrentUser.id` is threaded into every repository call to scope queries to the authenticated user.

### Domain Models

Models are split into two types in `src/models/<entity>.rs`:

- Domain models (e.g., `Budget`, `Transaction`, `Account`) representing database entities
- Request/Response DTOs (e.g., `BudgetRequest`, `BudgetResponse`) for API serialization

### API Endpoints Structure

All endpoints are mounted under `/api/v1`. List endpoints use cursor-based pagination (see Pagination below).

- `/api/v1/health` — `GET /` runs `SELECT 1` against the pool; returns `{"status":"ok","database":"connected"}` or `503`
- `/api/v1/users` — create, login, logout, update, delete, `GET /me`
- `/api/v1/accounts` — CRUD + cursor-paginated list
- `/api/v1/currency` — CRUD; lookup by code (`GET /<code>`) or name (`GET /name/<name>`)
- `/api/v1/categories` — CRUD + cursor-paginated list; `GET /not-in-budget` returns Outgoing categories not yet associated with a budget
- `/api/v1/budgets` — CRUD + cursor-paginated list
- `/api/v1/budget-categories` — CRUD + cursor-paginated list
- `/api/v1/budget_period` — CRUD + cursor-paginated list; `GET /current` returns the period whose date range covers today
- `/api/v1/transactions` — CRUD + cursor-paginated list; list accepts optional `period_id` query filter
- `/api/v1/vendors` — CRUD + cursor-paginated list; `GET /with_status?order_by=<name|most_used|more_recent>` returns vendors enriched with transaction-count stats
- `/api/v1/dashboard` — `budget-per-day`, `spent-per-category`, `monthly-burn-in`, `month-progress`, `recent-transactions`, `dashboard` (all accept `period_id`)
  `spent-per-category` returns `percentage_spent` in basis points (percent * 100). Example: 2534 = 25.34%.

404 and 409 responses are caught under `/api/v1` and returned as `{"message":"..."}` JSON.

### Pagination

List endpoints use keyset (cursor-based) pagination via `CursorParams` (`src/models/pagination.rs`):

- Query params: `cursor` (UUID of the last item on the previous page) and `limit` (default **50**, max **200**).
- Responses are wrapped in `CursorPaginatedResponse<T>` with `data` and `next_cursor` (`null` on the last page).
- The DB layer fetches `limit + 1` rows; if an extra row exists it is dropped and `next_cursor` is set to the `id` of the last returned item.
- Indexes on `(user_id, created_at DESC, id DESC)` (and `start_date` for budget periods) back the cursor queries.

### Error Handling

`src/error/app_error.rs` — `AppError` enum covers DB errors, validation, not found, invalid credentials, UUID parse, password-hash, and configuration errors. Implements `Responder`: logs via `tracing::error!`, maps to the appropriate HTTP status, and returns the error message as plain-text body. Route handlers return `Result<T, AppError>`.

`src/error/json.rs` — `JsonBody<T>` is a custom `FromData` extractor used instead of Rocket's built-in `Json<T>`. On a parse failure it logs the serde error location (line/column), the error category, and a preview of the request body (up to 500 chars), then returns **422 Unprocessable Entity**.

### Testing

- Test utilities in `src/test_utils.rs` provide **sample data helpers** (`sample_account`, `sample_transaction`, etc.) and conversions from request structs to models.
- Services expose pure helper functions for deterministic unit tests (e.g., dashboard helpers).
- Most route tests that hit the database remain `#[ignore]` unless a DB is available.

## Key Implementation Patterns

### Adding a New Entity

1. Add DB table via migration.
2. Create model structs in `src/models/<entity>.rs`.
3. Add concrete methods on `PostgresRepository` in `src/database/<entity>.rs`.
4. Add route handlers in `src/routes/<entity>.rs` and mount in `src/lib.rs`.
5. Add any needed sample data helpers in `src/test_utils.rs` for unit tests.

### Route Handler Pattern

Routes construct the concrete repository directly from the pooled `PgPool`:

```rust
pub async fn handler(
    pool: &State<PgPool>,
    current_user: CurrentUser,
) -> Result<Json<Response>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let result = repo.some_operation(&current_user.id).await?;
    Ok(Json(Response::from(&result)))
}
```

### Database Query Pattern

Repository methods use `sqlx` with `PgPool` (no trait objects, no deadpool). Mapping is usually done with `sqlx::FromRow` structs or manual conversions.

## Important Notes

- PostgreSQL connection details come from `Config` (see Configuration section above).
- IDs are UUIDs generated by PostgreSQL `gen_random_uuid()`.
- Amounts are stored as `BIGINT` (cents) in the database and exposed as `i64` in Rust.
- Timestamps use `TIMESTAMPTZ` with `chrono::DateTime<Utc>`.
- Every query is scoped to the authenticated user via `user_id`.

## CI Discipline

Always run the full PR check suite locally before pushing:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --verbose`
- `cargo test --verbose`

This mirrors `.github/workflows/rust.yml` and keeps PR checks green.
