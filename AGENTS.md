# AGENTS.md

## Purpose

This repo is the PiggyPulse Rust backend API. Agents should keep changes security-first, API-contract aware, and grounded in AgentBrain plus current source. Do not rely on deleted or stale `CLAUDE.md`-style instructions.

## Memory-first workflow

AgentBrain is the durable project memory source. Before meaningful work:

1. Read the memory manifest.
2. Read PiggyPulse project context.
3. Read relevant project memory files.
4. Search decisions.
5. Check open questions.
6. Inspect this repo.

If AgentBrain and code disagree, stop and report the mismatch. This repo has a known API path mismatch in docs: some README/docs mention `/api/v1`, while current config and routes use `/v2`.

## Required memory reads

- Always: `Context.md`, `ArchitectureOverview.md`, `KnownIssues.md`.
- API/backend work: `Backend.md`, `APIConventions.md`, `SecurityModel.md`, `DataModel.md`.
- Feature work: relevant `Features/*`, especially `Auth.md`, `BudgetPeriods.md`, `Transactions.md`, `Accounts.md`, `Categories.md`, `Dashboard.md`, `Projections.md`.
- Deployment/config work: `Deployment.md`, relevant `Integrations/*`.
- Tests: `Testing.md`, `KnownIssues.md`.

## Memory write-back rules

After meaningful work, record durable decisions, open questions, interaction/session summaries, and requested daily/global summaries.

Use MCP tools if available: `record_decision`, `upsert_open_question`, `record_interaction`, `append_daily_log`, `append_global_daily_summary`.

If MCP is unavailable, write to `AgentBrain/10_Projects/PiggyPulse/`. Do not store secrets, `.env` contents, credentials, private keys, tokens, service-account files, or raw chain-of-thought.

## Repo overview

Rust 2024 backend for PiggyPulse using Rocket 0.5.1, SQLx 0.8, PostgreSQL, Figment configuration, Argon2 auth, optional TOTP 2FA, Bearer tokens, HttpOnly cookies, AES-256-GCM encryption-at-rest, and Docker deployment.

## Important directories

- `src/routes/v2/` - API v2 route handlers.
- `src/dto/` - public request/response DTOs.
- `src/models/` - internal domain models; do not expose directly.
- `src/service/` - business logic.
- `src/database/` - SQLx data access.
- `src/auth.rs`, `src/crypto.rs`, `src/session_dek.rs`, `src/middleware.rs` - auth, crypto, sessions, request logging.
- `migrations/` - SQLx migrations.
- `docs/` - development, deployment, and 2FA docs.
- `.github/workflows/` - CI checks.

## Commands

Verified from `README.md`, `docs/DEVELOPMENT.md`, `docs/DEPLOYMENT.md`, and GitHub workflows.

### Install

No repo-specific install command is required beyond Rust/Cargo and optional `sqlx-cli`.

### Development

- `cargo run`
- `docker compose up db`
- `docker compose up --build`

### Build

- `cargo build --verbose`
- `cargo build --bin piggy-pulse`

### Test

- `cargo test`
- `cargo test --verbose`
- `cargo test --test data_integrity --test 'v2_*' -- --ignored --test-threads=1`
- `cargo test two_factor -- --ignored`

### Lint / format

- `cargo fmt`
- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo audit` when dependencies change.

### Database / migrations

- `sqlx migrate run`
- `docker compose -f docker-compose.test.yaml up -d --wait`

### Mobile platform commands

Not applicable.

## Conventions

- API routes are under `src/routes/v2/`; verify `PiggyPulse.toml` before citing public paths.
- Keep public API structs in `src/dto/`; never return database/domain structs directly.
- Validate at DTO boundaries using `validator` and existing helper patterns.
- Scope all user financial data by `user_id`.
- Preserve cursor-based pagination response shape from `src/dto/common.rs`.
- Keep transaction ledger and encryption-at-rest semantics intact.
- Use SQLx with explicit queries and migrations; avoid broad ORM-style abstractions.
- Breaking API changes require versioning and OpenAPI review.

## Testing expectations

- Small pure Rust/domain changes: run `cargo fmt --check`, relevant `cargo test`, and `cargo clippy --workspace --all-targets -- -D warnings` when practical.
- API/DTO changes: also inspect OpenAPI impact and run route/integration tests if available.
- Migration/data-access changes: run `sqlx migrate run` against a safe local/test database and relevant database tests.
- Security/auth changes: run targeted auth/2FA tests and manually inspect rate limiting, cookie/token, and error behavior.

## Security / privacy rules

- Never commit secrets or `.env` values.
- Never store passwords, API tokens, SMTP credentials, database URLs with credentials, private keys, or raw encryption keys in docs or memory.
- Follow `SecurityModel.md` for backend/security changes.
- Do not expose internal database models in public APIs.
- Keep Argon2, rate limiting, dummy-hash timing protection, 2FA, cookie security, Bearer token handling, and CORS behavior aligned with existing code.
- Swagger UI must not be exposed from production runtime unless a recorded decision changes that.
- Follow `PrivacyRules.md` only if the task touches personal/career/user memory; otherwise project security rules apply.

## Environment variables

Document names only, never values. Verified names include:

- `PIGGY_PULSE_DATABASE__URL`
- `PIGGY_PULSE_DATABASE__MAX_CONNECTIONS`
- `PIGGY_PULSE_DATABASE__MIN_CONNECTIONS`
- `PIGGY_PULSE_DATABASE__CONNECTION_TIMEOUT`
- `PIGGY_PULSE_DATABASE__ACQUIRE_TIMEOUT`
- `PIGGY_PULSE_SERVER__PORT`
- `PIGGY_PULSE_SERVER__ADDRESS`
- `PIGGY_PULSE_API__BASE_PATH`
- `PIGGY_PULSE_API__EXPOSE_DOCS`
- `PIGGY_PULSE_CORS__ALLOWED_ORIGINS`
- `PIGGY_PULSE_CORS__ALLOW_CREDENTIALS`
- `PIGGY_PULSE_SESSION__COOKIE_SECURE`
- `PIGGY_PULSE_TWO_FACTOR__ENCRYPTION_KEY`
- `PIGGY_PULSE_EMAIL__SMTP_HOST`
- `PIGGY_PULSE_EMAIL__SMTP_PORT`
- `PIGGY_PULSE_EMAIL__SMTP_USERNAME`
- `PIGGY_PULSE_EMAIL__SMTP_PASSWORD`
- `PIGGY_PULSE_EMAIL__FROM_ADDRESS`
- `PIGGY_PULSE_EMAIL__FROM_NAME`
- `PIGGY_PULSE_EMAIL__ENABLED`
- `ROCKET_SECRET_KEY`
- `DATABASE_URL`
- `POSTGRES_PASSWORD`

## When to stop and ask/report

Stop and report if memory contradicts source code, required commands are missing, tests fail for unrelated reasons, secrets are needed, a destructive migration is needed, public API/security behavior changes, or the requested change conflicts with recorded decisions.

## Completion checklist

Before final response, verify:

- [ ] Relevant AgentBrain memory was read.
- [ ] Relevant source files were inspected.
- [ ] Existing decisions and open questions were checked.
- [ ] Commands run are listed.
- [ ] Tests/lint/build were run where appropriate, or skipped with reason.
- [ ] Durable decisions were recorded or proposed.
- [ ] Open questions were recorded or proposed.
- [ ] No secrets or `.env` values were exposed.
- [ ] Any memory/code contradictions were reported.
