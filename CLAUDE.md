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

The codebase follows a clean separation of concerns with three main layers:

1. **Routes Layer** (`src/routes/`): HTTP handlers that receive requests, validate input, and return responses
2. **Service Layer** (`src/service/`): Business logic layer (currently minimal, with `account` and `dashboard` services)
3. **Database Layer** (`src/database/`): Data access layer using trait-based repository pattern

### Repository Pattern

Each domain entity has a corresponding repository trait defined in `src/database/<entity>.rs`:

```rust
#[async_trait::async_trait]
pub trait EntityRepository {
    async fn create_entity(&self, request: &EntityRequest) -> Result<Entity, AppError>;
    async fn get_entity_by_id(&self, id: &Uuid) -> Result<Option<Entity>, AppError>;
    async fn list_entities(&self) -> Result<Vec<Entity>, AppError>;
    async fn delete_entity(&self, id: &Uuid) -> Result<(), AppError>;
    async fn update_entity(&self, id: &Uuid, request: &EntityRequest) -> Result<Entity, AppError>;
}
```

All repository traits are implemented for `PostgresRepository<'a>` which wraps a `deadpool_postgres::Client`.

### Database Connection Management

- Uses `deadpool-postgres` for connection pooling (configured in `src/db.rs`)
- Pool is initialized at application startup with hardcoded credentials (localhost, postgres:example)
- Pool is managed by Rocket's state system and injected into route handlers
- Routes call `get_client(pool)` to obtain a connection, then wrap it in `PostgresRepository { client: &client }`

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

- Test utilities in `src/test_utils.rs` provide `MockRepository` implementations for unit testing
- Some routes have test modules (e.g., `src/routes/health.rs`, `src/service/dashboard.rs`)
- Mock repositories implement the same traits as `PostgresRepository` for dependency injection in tests

## Key Implementation Patterns

### Adding a New Entity

When adding a new entity (e.g., "Payment"), follow this pattern:

1. Add database table in a new migration file
2. Create model struct in `src/models/payment.rs` and add to `src/models.rs`
3. Create repository trait and PostgresRepository impl in `src/database/payment.rs` and add to `src/database.rs`
4. Create route handlers in `src/routes/payment.rs` and add to `src/routes.rs`
5. Mount routes in `src/lib.rs` `build_rocket()` function
6. Add mock implementation to `src/test_utils.rs` if needed for testing

### Route Handler Pattern

All route handlers follow this structure:

```rust
pub async fn handler(
    pool: &State<Pool>,
    _current_user: CurrentUser,
    // other params
) -> Result<Json<Response>, AppError> {
    let client = get_client(pool).await?;
    let repo = PostgresRepository { client: &client };
    let result = repo.some_operation().await?;
    Ok(Json(Response::from(&result)))
}
```

### Database Query Pattern

Repository implementations use raw SQL with tokio-postgres:

```rust
let rows = self .client.query(
"SELECT ... FROM table WHERE id = $1",
& [ & id]
).await?;
```

Results are mapped using helper functions like `map_row_to_entity(row)`.

## Important Notes

- PostgreSQL connection details are hardcoded in `src/db.rs` (localhost, user: postgres, password: example, db: budget_db)
- Authentication is currently stubbed out and not enforced
- The codebase uses `async-trait` for async trait methods
- All IDs are UUIDs generated by PostgreSQL's `gen_random_uuid()`
- Amounts are stored as `BIGINT` (cents) in the database but exposed as `i64` in Rust
- Timestamps use PostgreSQL `TIMESTAMPTZ` with `chrono::DateTime<Utc>`
