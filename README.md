# Budget API

Rust backend for a budgeting tool, using Rocket for the HTTP server and PostgreSQL via SQLx.

## Prerequisites

- Rust (stable)
- PostgreSQL running locally or remotely

Set `DATABASE_URL` in your environment, e.g.:

```bash
export DATABASE_URL=postgres://user:password@localhost:5432/budget_db
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

- `GET /api/health` – simple health check.
- `POST /api/budgets` – create a budget.
- `GET /api/budgets` – list budgets.

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
