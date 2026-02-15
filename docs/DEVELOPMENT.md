# Development Guide

This document describes how to run, test, and develop the PiggyPulse API locally.

The project is designed to run in Docker (recommended) but can also be run directly with Cargo.

---

## Requirements

- Rust (stable toolchain)
- Docker + Docker Compose
- PostgreSQL (if not using Docker)

Make sure `cargo`, `rustup`, and `docker` are available in PATH.

---

## Project Structure

The backend follows a layered structure:

- HTTP layer (Rocket routes)
- API DTOs (request/response types)
- Domain models
- Data access (SQLx)
- PostgreSQL

Configuration is handled via:

- `.env`
- `dotenv`
- `figment`
- Optional `PiggyPulse.toml`

---

# Running Locally (Recommended: Docker)

The recommended setup mirrors production.

## 1. Start PostgreSQL

```bash
    docker compose up db
```

Required environment variable:

```bash
    POSTGRES_PASSWORD=<your_password>
```

This uses the same `docker-compose.yml` as production.

---

## 2. Run the API (locally via Cargo)

Create a `.env` file with:

```bash
    PIGGY_PULSE_DATABASE__URL=postgres://user:password@localhost:5432/piggy_pulse
    ROCKET_SECRET_KEY=<generate_secure_key>

    PIGGY_PULSE_CORS__ALLOWED_ORIGINS=["http://localhost:5173"]
    PIGGY_PULSE_CORS__ALLOW_CREDENTIALS=true
    PIGGY_PULSE_API__EXPOSE_DOCS=true
    PIGGY_PULSE_SESSION__COOKIE_SECURE=false

    PIGGY_PULSE_EMAIL__ENABLED=false
```

Then run:

```bash
    cargo run
```

The API will be available at:

```bash
    http://localhost:8000/api/v1
```

---

# Running Fully via Docker

Alternatively:

```bash
    docker compose up --build
```

This starts:

- PostgreSQL
- API container
- Runs migrations automatically

---

# Database Migrations

Migrations are handled using SQLx:

```bash
    sqlx migrate run
```

When running in Docker, migrations are executed automatically on startup.

No seed scripts are used. Required base data is created via migrations. Additional data should be created through normal API flows.

---

# Configuration

Configuration uses:

- `.env`
- Environment variables
- Optional `PiggyPulse.toml`

The system uses nested config structure via environment variable names.

Example:

```bash
    PIGGY_PULSE_DATABASE__URL=
    PIGGY_PULSE_CORS__ALLOWED_ORIGINS=
    PIGGY_PULSE_EMAIL__SMTP_HOST=
```

---

## Required Variables (Local Development)

Minimum required:

```bash
    PIGGY_PULSE_DATABASE__URL
    ROCKET_SECRET_KEY
    PIGGY_PULSE_CORS__ALLOWED_ORIGINS=["<your_local_frontend_url>"]
    PIGGY_PULSE_CORS__ALLOW_CREDENTIALS=true
```

Optional (email support):

```bash
    PIGGY_PULSE_EMAIL__SMTP_HOST
    PIGGY_PULSE_EMAIL__SMTP_PORT
    PIGGY_PULSE_EMAIL__SMTP_USERNAME
    PIGGY_PULSE_EMAIL__SMTP_PASSWORD
    PIGGY_PULSE_EMAIL__FROM_ADDRESS
    PIGGY_PULSE_EMAIL__FROM_NAME
    PIGGY_PULSE_EMAIL__ENABLED
```

---

# Testing

Run all tests:

```bash
    cargo test
```

CI enforces:

```bash
    cargo fmt --check
    cargo clippy -- -D warnings
    cargo test
```

Database-dependent tests are currently disabled pending a reliable isolated test DB setup.

---

# OpenAPI / Swagger

OpenAPI documentation is generated automatically at runtime.

To use the documentation UI locally, add the following variables to `.env`:

```bash
    PIGGY_PULSE_API__EXPOSE_DOCS=true
    PIGGY_PULSE_SESSION__COOKIE_SECURE=false
```

Local endpoint:

```bash
    http://localhost:8000/api/v1/openapi.json
```

Documentation UI is hosted separately and consumes this endpoint.

---

# Linting & Formatting

Format:

```bash
    cargo fmt
```

Lint:

```bash
    cargo clippy -- -D warnings
```

CI will fail on formatting or lint warnings.

---

# Conventional Commits (Required)

CI enforces **Conventional Commits** for both:

- PR titles
- Commit subjects in the PR (merge commits are ignored)

Required format:

- `type(scope)!: description`
- `type: description`

Allowed `type` values:

- `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, `test`

Examples:

- `feat(api): add cursor pagination`
- `fix(auth)!: reject invalid session cookie`
- `docs: update local dev steps`

Fixing failures:

- Reword commits: `git rebase -i origin/main` then change `pick` to `reword`
- Squash commits: interactive rebase and squash into a single Conventional Commit

---

# Deployment Model (High-Level)

Production deployment:

- Dockerized service
- Hosted on Hetzner VPS
- CI via self-hosted Drone

Deployment command:

```bash
    docker compose pull
    docker compose down cron piggy-pulse
    docker compose up -d cron piggy-pulse
```

More details available in `docs/DEPLOYMENT.md`.

---

# Development Principles

When contributing:

- Do not expose internal models in API responses
- All public-facing structs must be DTOs
- Manual mapping between domain models and DTOs is required
- Breaking API changes require version bump
- Authentication and rate limiting must remain enforced
- OpenAPI changes should be reviewed before release

The API contract is treated as a public boundary.
