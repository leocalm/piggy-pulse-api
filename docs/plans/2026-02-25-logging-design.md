# Logging Design: Audit, Operational, and Standard Format

**Date:** 2026-02-25
**Status:** Approved

## Goals

1. Comprehensive audit trail for all security-relevant actions
2. Operational logging for slow queries and slow requests
3. Standard log format across the entire application
4. Design for future centralized logging (stdout JSON)

## Standard Log Format

Every log line includes a consistent set of fields.

**Fields:**

| Field | Source | Always present |
|-------|--------|----------------|
| `timestamp` | tracing-subscriber | Yes |
| `level` | tracing | Yes |
| `request_id` | RequestLogger span | Yes (in request context) |
| `user_id` | CurrentUser / cookie | No (only if authenticated) |
| `target` | tracing module path | Yes |
| `message` | log statement | Yes |

**Context fields** vary by log category:

- **request:** `method`, `uri`, `status`, `duration_ms`, `request_bytes`, `response_bytes`
- **db:** `query_time_ms`, `query`
- **audit:** `event_type`, `success`, `ip`, `user_agent`

**Human-readable (dev):**
```
2026-02-25T14:30:00.123Z  INFO [req:550e8400] user:abc123 piggy_pulse::routes::user — Request completed method=GET uri=/api/v1/budgets status=200 duration_ms=42 res_bytes=1024
```

**JSON (production):**
```json
{"timestamp":"2026-02-25T14:30:00.123Z","level":"INFO","request_id":"550e8400...","user_id":"abc123","target":"piggy_pulse::routes::user","message":"Request completed","method":"GET","uri":"/api/v1/budgets","status":200,"duration_ms":42,"response_bytes":1024}
```

**Log categories:**

| Category | Purpose | Min Level |
|----------|---------|-----------|
| `request` | HTTP request lifecycle | INFO |
| `audit` | Security events (also persisted to DB) | INFO |
| `db` | Slow query warnings | WARN |
| `app` | General application logic | INFO |
| `error` | Unhandled errors, panics | ERROR |

## Audit Events

All audit events are persisted to the existing `security_audit_log` table AND logged to stdout.

**New events:**

| Event Type | Trigger | Key Metadata |
|------------|---------|--------------|
| `login_success` | Successful login | `email`, `2fa_used` |
| `login_failed` | Wrong password or unknown email | `email`, `reason` |
| `logout` | User logs out | — |
| `session_expired` | Guard detects expired session | `session_id` |
| `2fa_enabled` | User enables 2FA | — |
| `2fa_disabled` | User disables 2FA | `method` (normal/emergency) |
| `2fa_backup_used` | Backup code used for login | — |
| `password_changed` | User changes password (not reset) | — |
| `account_updated` | Profile/email changes | `changed_fields` |

**Existing events (unchanged):** `password_reset_requested`, `password_reset_token_validated`, `password_reset_completed`, `password_reset_failed`, `password_reset_token_expired`, `password_reset_token_invalid`.

## Enhanced RequestLogger

Upgrade the existing `RequestLogger` fairing:

1. Record `Instant::now()` on request, compute `duration_ms` on response
2. Capture `Content-Length` from request and response headers
3. Extract `user_id` from session cookie (lightweight parse, no DB hit)
4. Wrap request in a tracing span with `request_id` and `user_id` so all log lines within the request inherit these fields
5. Slow request warning: log at WARN if `duration_ms` exceeds `slow_request_ms` threshold

## Slow Query Logging

Use SQLx's built-in `PgConnectOptions` configuration:

```rust
let options = PgConnectOptions::from_str(&db_config.url)?
    .log_statements(LevelFilter::Debug)
    .log_slow_statements(LevelFilter::Warn, Duration::from_millis(slow_query_ms));
```

- Development (`RUST_LOG=debug`): All queries with timing
- Production (`RUST_LOG=info`): Only slow queries (>threshold) as WARN

## Configuration

New fields in `[logging]`:

```toml
[logging]
level = "info"
json_format = false
slow_request_ms = 500
slow_query_ms = 100
```

## Files to Modify

| File | Change |
|------|--------|
| `src/config.rs` | Add `slow_request_ms`, `slow_query_ms` to `LoggingConfig` |
| `src/lib.rs` | Enhanced `init_tracing` with standard format |
| `src/middleware.rs` | Timing, user context, body sizes, request span |
| `src/db.rs` | Configure `PgConnectOptions` slow query logging |
| `src/models/audit.rs` | **New** — centralized `audit_events` constants |
| `src/models/password_reset.rs` | Remove `audit_events` module (moved) |
| `src/database/audit.rs` | **New** — extract `create_security_audit_log` into dedicated repo file |
| `src/database/password_reset.rs` | Remove audit log method (moved) |
| `src/routes/user.rs` | Add login success/failure/logout audit calls |
| `src/routes/two_factor.rs` | Add 2FA enable/disable/backup audit calls |
| `src/auth.rs` | Add session expired audit event |

## What Stays Unchanged

- `security_audit_log` DB table schema (no migration needed)
- Existing password reset audit events (import paths updated)
- Rate limiting, CORS, error handling
