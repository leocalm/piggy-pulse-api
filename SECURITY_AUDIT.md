# Security Audit Report — PiggyPulse API

**Date:** 2026-02-06
**Scope:** Full codebase review for cloud deployment readiness (Hetzner)
**Verdict:** Conditionally ready. Fix the CRITICAL and HIGH items before going live.

---

## Executive Summary

The application has a solid security foundation. Rust's memory safety eliminates entire vulnerability classes (buffer overflows, use-after-free). The use of `sqlx` with parameterized queries prevents SQL injection. Argon2 password hashing is industry-standard. Data is properly scoped per-user at the query level.

However, there are several issues that range from critical to informational that must be addressed before exposing this to the internet.

---

## CRITICAL Findings

### C1. Broken Access Control on User Update and Delete (IDOR)

**Files:** `src/routes/user.rs:33-53`

`put_user` and `delete_user_route` accept an `id` path parameter and use `_current_user: CurrentUser` — the underscore prefix means the current user's identity is **verified but never checked against the target `id`**. Any authenticated user can update or delete any other user's account by supplying their UUID.

```rust
// src/routes/user.rs:33 — current_user.id is never compared to `id`
pub async fn put_user(..., _current_user: CurrentUser, id: &str, ...) -> ... {
    let uuid = Uuid::parse_str(id)...;
    let user = repo.update_user(&uuid, ...).await?;  // no ownership check
}
```

The underlying `update_user` and `delete_user` database methods (`src/database/user.rs:69,91`) operate on the provided `id` directly with no `WHERE user_id = $current_user` filter.

**Impact:** Account takeover. Any authenticated user can change another user's password/email or delete their account.
**Severity:** CRITICAL

### C2. Login Endpoint Leaks User Existence (Timing Oracle + Behavior)

**File:** `src/routes/user.rs:59-73`

The login handler returns `200 OK` regardless of whether the email exists. This seems like a mitigation, but it creates two problems:

1. **Timing side-channel:** When the user exists, `verify_password` runs Argon2 (expensive, ~hundreds of ms). When the user doesn't exist, the response is immediate. An attacker can distinguish existing vs. non-existing accounts by measuring response time.

2. **Signup endpoint confirms existence:** `post_user` (`src/routes/user.rs:17-28`) returns `UserAlreadyExists` with the email in the error message, directly confirming account existence.

**Impact:** User enumeration enables targeted credential stuffing.
**Severity:** CRITICAL (in combination with C1)

### C3. No Rocket Secret Key Configuration

**Files:** `Rocket.toml`, `PiggyPulse.toml.example`, `src/config.rs`

Rocket's private cookies (used for authentication) are encrypted using a `secret_key` configured in `Rocket.toml`. The current `Rocket.toml` only sets `cli_colors = false` — no secret key. In debug mode, Rocket generates a volatile random key (changes every restart, invalidating all sessions). In **release mode, Rocket refuses to start without a secret key**, or historically has used an insecure default.

There is no mention of `secret_key` anywhere in the codebase. If you deploy this as-is in release mode, either:
- It won't start, or
- If an old Rocket version defaults to something, the cookies are trivially forgeable.

**Impact:** Session forgery — an attacker could forge the `user` cookie to impersonate any user.
**Severity:** CRITICAL

---

## HIGH Findings

### H1. CORS Allows All Origins by Default

**File:** `src/config.rs:95` (default), `src/lib.rs:46-81`

The default CORS configuration is `allowed_origins: ["*"]`. While `allow_credentials` is `false` by default (preventing cookie-bearing cross-origin requests), this means any website can make unauthenticated API calls to your endpoints (health checks, signup, login).

More critically, if someone changes `allow_credentials` to `true` without also restricting origins, the app panics — but that's a runtime check, not a compile-time guarantee. The default should be restrictive, not permissive.

**Impact:** Cross-origin attacks, signup abuse.
**Severity:** HIGH

### H2. No HTTPS/TLS Enforcement

There is no TLS configuration in the application. The server listens on plain HTTP. If deployed behind a reverse proxy (nginx, Caddy, Traefik), this is acceptable — but:

- There are no `Secure` or `SameSite` attributes explicitly set on the authentication cookie (`src/routes/user.rs:69`).
- Without `Secure`, the cookie will be sent over plain HTTP if the user is tricked into visiting an HTTP URL.
- Without `SameSite=Lax` or `Strict`, CSRF attacks are possible.

**Impact:** Session hijacking via network sniffing; CSRF.
**Severity:** HIGH

### H3. Swagger/OpenAPI Docs Exposed in Production

**File:** `src/lib.rs:180-182`

The Swagger UI is mounted unconditionally at `/api/v1/docs` with the full OpenAPI specification at `/api/v1/openapi.json`. This gives attackers a complete map of every endpoint, parameter, and data model.

**Impact:** Information disclosure accelerates attack planning.
**Severity:** HIGH

### H4. Docker Compose Uses Default Credentials

**File:** `docker-compose.yaml`

PostgreSQL password is `example`. Adminer is exposed on port 8080 with no authentication beyond the database credentials. If this compose file is used in production (even accidentally), the database is wide open.

**Impact:** Full database compromise.
**Severity:** HIGH (if used in production)

---

## MEDIUM Findings

### M1. No Session Expiration or Invalidation

The encrypted cookie has no TTL, expiration, or server-side session tracking. Once issued, a cookie is valid until:
- The user explicitly logs out
- The server's secret key changes
- The browser discards it

There is no way to force-logout a compromised session. No session revocation mechanism exists.

**Impact:** Stolen sessions remain valid indefinitely.
**Severity:** MEDIUM

### M2. Rate Limiter Uses In-Memory Storage

**File:** `src/middleware/rate_limit.rs`

The rate limiter stores counters in a `HashMap` behind a `tokio::Mutex`. This means:
- Counters are lost on restart (attacker can bypass by waiting for deploys)
- If you scale to multiple instances, each has independent counters (rate limit is effectively multiplied)
- The fixed-window algorithm allows burst-at-boundary attacks (up to 2x the limit)

**Impact:** Rate limiting can be bypassed in multi-instance deployments.
**Severity:** MEDIUM

### M3. Password Policy Is Weak

**File:** `src/models/user.rs` (via validator)

Minimum password length is 8 characters with no complexity requirements. No check against common/breached passwords.

**Impact:** Weak passwords are allowed.
**Severity:** MEDIUM

### M4. Global Unique Constraints Leak Cross-User Information

**File:** `migrations/0001_init/up.sql`

`account.name`, `category.name`, `vendor.name`, `piggy-pulse_period.name` are globally UNIQUE — not scoped to `(user_id, name)`. This means:
- User A creates account "Main Checking"
- User B tries to create "Main Checking" → gets a database constraint error

This leaks the existence of other users' data names.

**Impact:** Information disclosure (entity name enumeration across users).
**Severity:** MEDIUM

### M5. Error Responses Can Leak Internal Details

**File:** `src/error/app_error.rs:135-142`

The error responder logs the full error (including DB errors, stack traces, etc.) server-side, which is correct. However, the `AppError::Db` variant's `Display` impl just says "Internal server error" — that's good. But `ValidationError` returns the full validation error structure to the client, which can reveal internal field names and validation rules.

The `error/json.rs` logs the request body preview (up to 500 chars) to the server log — make sure server logs are not exposed.

**Impact:** Low-grade information disclosure.
**Severity:** MEDIUM

---

## LOW / INFORMATIONAL Findings

### L1. Adminer Included in docker-compose

Adminer is a full database management UI. Even for development, this increases attack surface. It should be a separate profile or opt-in.

### L2. No Security Headers

The application sets no security headers beyond `X-Request-Id`. For a pure API this is less critical than for a web app, but you should consider:
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Cache-Control: no-store` on authenticated responses
- `Strict-Transport-Security` (if handling TLS)

### L3. `create_account` Binds Parameters in Non-Positional Order

`src/database/account.rs:126-151` — the SQL has `VALUES($1, $2, ...)` where `$1` is `name` but `.bind(user_id)` is called second after `.bind(&request.name)`. This actually works correctly because `bind` maps sequentially, but the SQL placeholders and bind order don't match the column list (`user_id` is column 1, `name` is column 2 in the INSERT, but `$1` maps to the first `.bind()` which is `name`). This is confusing and error-prone but not currently exploitable.

### L4. Transaction Amount Is `INTEGER` in DB but `i32` in Request

The migration uses `INTEGER` for `transaction.amount` but `BIGINT` for `account.balance`. Large transaction volumes or amounts could overflow. The model uses `i32` for the request DTO but the domain concern is consistency.

### L5. No Audit Trail for Financial Operations

For a piggy-pulseing app handling financial data, there is no audit log of who changed what and when. The `created_at` field exists but there's no `updated_at`, no change history, and no immutable transaction log.

---

## Deployment Recommendations for Hetzner

### Must-Do Before Going Live

1. **Fix C1 (IDOR):** Add ownership checks to `put_user` and `delete_user_route`. Compare `current_user.id` against the target `id` and return 403 if they don't match.

2. **Fix C3 (Secret Key):** Generate a 256-bit secret key and configure it:
   ```toml
   # Rocket.toml [release]
   [release]
   secret_key = "<generate with: openssl rand -base64 32>"
   ```

3. **Fix C2 (User Enumeration):** On signup, return a generic "check your email" response instead of `UserAlreadyExists`. On login, add a dummy Argon2 hash verification when the user doesn't exist to equalize timing.

4. **Restrict CORS (H1):** Set `allowed_origins` to your specific frontend domain(s).

5. **Set Cookie Attributes (H2):** Add `Secure`, `SameSite=Lax`, and `HttpOnly` to the auth cookie:
   ```rust
   Cookie::build(("user", value))
       .path("/")
       .secure(true)
       .same_site(SameSite::Lax)
       .http_only(true)
       .build()
   ```

6. **Disable Swagger in Production (H3):** Gate the Swagger mount behind a config flag or environment check.

7. **Do NOT use docker-compose.yaml in production (H4).**

### Infrastructure (Hetzner-Specific)

8. **Reverse Proxy:** Place Caddy or nginx in front of the Rocket server for TLS termination. Caddy auto-provisions Let's Encrypt certificates.

9. **Firewall:** Use Hetzner's firewall to only expose ports 80/443. Block direct access to port 8000 (Rocket) and 5432 (PostgreSQL).

10. **Database:** Use Hetzner's managed PostgreSQL or at minimum set strong credentials, enable SSL connections, and restrict `pg_hba.conf` to your server's IP.

11. **Secrets Management:** Use environment variables (not files in the repo) for `PIGGY_PULSE_DATABASE__URL` and `ROCKET_SECRET_KEY`. Consider Hetzner's cloud-init or a secrets manager.

12. **Backups:** Configure automated PostgreSQL backups. This is financial data.

13. **Monitoring:** Enable JSON logging (`logging.json_format = true`) and ship logs to a monitoring stack. The request IDs are already in place for correlation.

### Fix M4 (Global Unique Constraints)

Migrate the UNIQUE constraints on `account.name`, `category.name`, `vendor.name`, and `piggy-pulse_period.name` to be composite unique on `(user_id, name)` instead of just `(name)`. This is a data model bug that will cause issues at scale even without the security implications.

---

## What's Already Good

- **Rust + memory safety**: No buffer overflows, no null pointer dereferences, no data races.
- **sqlx parameterized queries**: SQL injection is not possible through the normal code paths.
- **Argon2 with OsRng salt**: Password hashing is state-of-the-art.
- **Per-user data scoping**: Every database query (except user CRUD) includes `WHERE user_id = $N`.
- **Rate limiting**: Present and correctly implemented for its scope.
- **Input validation**: Multi-layer validation (JSON parsing, struct validation, UUID parsing, DB constraints).
- **Request ID tracking**: Full request traceability for incident response.
- **1 MiB JSON body limit**: Prevents request body DoS.
- **UUIDs for identifiers**: Not enumerable, not sequential.

---

## Risk Summary

| ID | Finding | Severity | Exploitable Today? |
|----|---------|----------|-------------------|
| C1 | IDOR on user update/delete | CRITICAL | Yes |
| C2 | User enumeration via timing + signup | CRITICAL | Yes |
| C3 | No Rocket secret_key configured | CRITICAL | On deploy |
| H1 | CORS allows all origins | HIGH | Yes |
| H2 | No Secure/SameSite on auth cookie | HIGH | On deploy |
| H3 | Swagger exposed unconditionally | HIGH | On deploy |
| H4 | Docker compose default credentials | HIGH | If reused |
| M1 | No session expiration | MEDIUM | Yes |
| M2 | In-memory rate limiter | MEDIUM | On scale-out |
| M3 | Weak password policy | MEDIUM | Yes |
| M4 | Global unique constraints | MEDIUM | Yes |
| M5 | Validation errors leak internals | MEDIUM | Yes |
