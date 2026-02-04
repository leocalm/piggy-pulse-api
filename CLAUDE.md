# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
cargo clippy -- -D warnings

# Run tests
cargo test

# Run specific test
cargo test <test_name>
```

## Database Setup

Set the `DATABASE_URL` environment variable (or use `.env` file):

```bash
export DATABASE_URL=postgres://user:password@localhost:5432/budget_db
```

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

- Uses `sqlx::PgPool` (see `Config` / Rocket state) for pooling.
- Routes receive `&State<PgPool>`, then construct `PostgresRepository { pool: pool.inner().clone() }`.
- No `deadpool-postgres` or trait objects involved.

### Authentication

- Cookie-based authentication implemented in `src/auth.rs` via `CurrentUser` request guard
- Currently returns a stub user (bypasses auth) - authentication is not fully enforced
- User ID and username are extracted from encrypted "user" cookie with format `id:username`

### Domain Models

Models are split into two types in `src/models/<entity>.rs`:

- Domain models (e.g., `Budget`, `Transaction`, `Account`) representing database entities
- Request/Response DTOs (e.g., `BudgetRequest`, `BudgetResponse`) for API serialization

### API Endpoints Structure

All endpoints are mounted under `/api` prefix:

- `/api/health` - Health check
- `/api/users` - User management
- `/api/accounts` - Account CRUD
- `/api/currency` - Currency operations
- `/api/categories` - Category management
- `/api/budgets` - Budget CRUD
- `/api/budget-categories` - Budget category associations
- `/api/budget_period` - Budget periods
- `/api/transactions` - Transaction CRUD
- `/api/vendors` - Vendor management
- `/api/dashboard` - Dashboard aggregations

### Error Handling

Custom error types in `src/error/`:

- `AppError` enum covers DB errors, validation, not found, invalid credentials, etc.
- Implements `Responder` trait to automatically convert to HTTP responses
- Route handlers return `Result<T, AppError>`

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

- PostgreSQL connection details come from config/ENV (see `Config` / Rocket state).
- Authentication remains a light stub via `CurrentUser`.
- IDs are UUIDs from PostgreSQL `gen_random_uuid()`.
- Amounts are stored as `BIGINT` (cents) in the database but exposed as `i64` in Rust.
- Timestamps use `TIMESTAMPTZ` with `chrono::DateTime<Utc>`.

## CI Discipline

Always run the full PR check suite locally before pushing:
- `cargo fmt --check`
- `cargo clippy --no-deps -- -D warnings`
- `cargo build --verbose`
- `cargo test --verbose`

This mirrors `.github/workflows/rust.yml` and keeps PR checks green.
