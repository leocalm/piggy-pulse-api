# Logging System Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement comprehensive logging — standard log format, audit trail for security events, enhanced request logging with timing/user context/body sizes, and slow query warnings.

**Architecture:** Extend existing `tracing` + `RequestLogger` fairing + `security_audit_log` table. Extract audit logic into dedicated modules. Configure SQLx built-in slow query logging. No new crates needed.

**Tech Stack:** Rust, Rocket, tracing/tracing-subscriber, SQLx (PgConnectOptions), PostgreSQL

**Design doc:** `docs/plans/2026-02-25-logging-design.md`

---

### Task 1: Add config fields for slow_request_ms and slow_query_ms

**Files:**
- Modify: `src/config.rs`
- Modify: `PiggyPulse.toml.example`

**Step 1: Add fields to LoggingConfig struct**

In `src/config.rs`, add two fields to the `LoggingConfig` struct (around line 85-88):

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub json_format: bool,
    pub slow_request_ms: u64,
    pub slow_query_ms: u64,
}
```

**Step 2: Update LoggingConfig Default impl**

Update the `Default` impl for `LoggingConfig` (around line 173-180):

```rust
impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            json_format: false,
            slow_request_ms: 500,
            slow_query_ms: 100,
        }
    }
}
```

**Step 3: Update PiggyPulse.toml.example**

Add the new fields to the `[logging]` section:

```toml
[logging]
level = "info"  # trace, debug, info, warn, error
json_format = false
slow_request_ms = 500   # warn threshold for slow requests (ms)
slow_query_ms = 100     # warn threshold for slow DB queries (ms)
```

**Step 4: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build (no errors)

**Step 5: Commit**

```bash
git add src/config.rs PiggyPulse.toml.example
git commit -m "feat(logging): add slow_request_ms and slow_query_ms config fields"
```

---

### Task 2: Configure SQLx slow query logging

**Files:**
- Modify: `src/db.rs`

**Step 1: Update init_pool to use PgConnectOptions**

Replace the current `init_pool` function in `src/db.rs` with one that uses `PgConnectOptions` for slow query logging. The function currently connects using `.connect(&db_config.url)`. Change it to parse `PgConnectOptions` from the URL and configure statement logging:

```rust
use crate::config::DatabaseConfig;
use log::LevelFilter;
use rocket::fairing::AdHoc;
use sqlx::PgPool;
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use std::str::FromStr;
use std::time::Duration;

// Embed migrations into the binary at compile time.
static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

pub async fn init_pool(db_config: &DatabaseConfig, slow_query_ms: u64) -> Result<PgPool, sqlx::Error> {
    let connect_options = PgConnectOptions::from_str(&db_config.url)?
        .log_statements(LevelFilter::Debug)
        .log_slow_statements(LevelFilter::Warn, Duration::from_millis(slow_query_ms));

    let pool = PgPoolOptions::new()
        .max_connections(db_config.max_connections)
        .min_connections(db_config.min_connections)
        .acquire_timeout(Duration::from_secs(db_config.acquire_timeout))
        .idle_timeout(Duration::from_secs(30))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(connect_options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(pool)
}
```

**Step 2: Update stage_db to pass slow_query_ms**

Update the `stage_db` function signature and call site:

```rust
pub fn stage_db(db_config: DatabaseConfig, slow_query_ms: u64) -> AdHoc {
    AdHoc::try_on_ignite("Postgres (sqlx)", |rocket| async move {
        match init_pool(&db_config, slow_query_ms).await {
            Ok(pool) => {
                tracing::info!("Database pool initialized successfully");
                Ok(rocket.manage(pool))
            }
            Err(e) => {
                tracing::error!("Failed to initialize database pool: {}", e);
                Err(rocket)
            }
        }
    })
}
```

**Step 3: Update the call site in lib.rs**

In `src/lib.rs` line 250, update the `stage_db` call to pass the slow query threshold:

```rust
.attach(stage_db(config.database, config.logging.slow_query_ms))
```

**Step 4: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 5: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass

**Step 6: Commit**

```bash
git add src/db.rs src/lib.rs
git commit -m "feat(logging): configure SQLx slow query logging via PgConnectOptions"
```

---

### Task 3: Enhance RequestLogger with timing, user context, and body sizes

**Files:**
- Modify: `src/middleware.rs`

**Step 1: Add RequestTimer to store start time**

Add a new struct to store timing info in request-local state, and add `std::time::Instant` import. Place this near the top of the file after the existing `RequestId` struct:

```rust
use std::time::Instant;

/// Stores the request start time for duration calculation
#[derive(Debug, Clone, Copy)]
struct RequestStartTime(Instant);
```

**Step 2: Update on_request to capture start time, request bytes, and create a tracing span**

Replace the `on_request` method of the `RequestLogger` fairing. The new version:
- Records `Instant::now()` in local cache
- Captures `Content-Length` header as request_bytes
- Extracts user_id from cookie (lightweight parse, no DB hit)

```rust
async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
    let request_id = RequestId::new();
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let request_bytes = request
        .headers()
        .get_one("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);

    // Extract user_id from cookie without DB hit
    let user_id = request
        .cookies()
        .get_private("user")
        .and_then(|c| {
            let value = c.value().to_string();
            crate::auth::parse_session_cookie_value(&value).map(|(_, uid)| uid.to_string())
        });

    // Store start time and request_id in local cache
    request.local_cache(|| RequestStartTime(Instant::now()));
    request.local_cache(|| Some(request_id.clone()));
    request.local_cache(|| request_bytes);

    info!(
        request_id = %request_id.0,
        method = %method,
        uri = %uri,
        user_id = user_id.as_deref().unwrap_or("-"),
        request_bytes = request_bytes,
        "incoming request"
    );
}
```

**Step 3: Update on_response to include duration, user_id, and body sizes**

Replace the `on_response` method. The new version:
- Computes `duration_ms` from the cached start time
- Reads `Content-Length` from response or body size
- Extracts `user_id` from the cached `CurrentUser` (set by auth guard)
- Logs at WARN if duration exceeds slow_request_ms threshold

```rust
async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
    let request_id = request
        .local_cache(|| None::<RequestId>)
        .as_ref()
        .map(|r| r.0.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let duration_ms = request
        .local_cache(|| RequestStartTime(Instant::now()))
        .0
        .elapsed()
        .as_millis() as u64;

    let request_bytes = *request.local_cache(|| 0u64);

    let status = response.status();
    let method = request.method();
    let uri = request.uri();

    let response_bytes = response
        .headers()
        .get_one("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .or_else(|| response.body().preset_size().map(|s| s as u64))
        .unwrap_or(0);

    // Get user_id from CurrentUser cached by auth guard
    let user_id = request
        .local_cache(|| None::<crate::auth::CurrentUser>)
        .as_ref()
        .map(|u| u.id.to_string());

    // Add request_id to response headers for client tracking
    response.set_header(Header::new("X-Request-Id", request_id.clone()));

    // Add security headers
    response.set_header(Header::new("X-Content-Type-Options", "nosniff"));
    response.set_header(Header::new("X-Frame-Options", "DENY"));
    response.set_header(Header::new("Cache-Control", "no-store"));

    // Get slow_request_ms threshold from managed state
    let slow_request_ms = request
        .rocket()
        .state::<crate::config::Config>()
        .map(|c| c.logging.slow_request_ms)
        .unwrap_or(500);

    // Log response with appropriate level
    let is_error = status.class().is_server_error() || status.class().is_client_error();
    let is_slow = duration_ms > slow_request_ms;

    if is_error || is_slow {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = status.code,
            duration_ms = duration_ms,
            request_bytes = request_bytes,
            response_bytes = response_bytes,
            user_id = user_id.as_deref().unwrap_or("-"),
            slow = is_slow,
            "request completed{}{}",
            if is_error { " with error" } else { "" },
            if is_slow { " (slow)" } else { "" },
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = status.code,
            duration_ms = duration_ms,
            request_bytes = request_bytes,
            response_bytes = response_bytes,
            user_id = user_id.as_deref().unwrap_or("-"),
            "request completed"
        );
    }
}
```

**Step 4: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 5: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass

**Step 6: Commit**

```bash
git add src/middleware.rs
git commit -m "feat(logging): enhance RequestLogger with timing, user context, and body sizes"
```

---

### Task 4: Enhance init_tracing with standard format

**Files:**
- Modify: `src/lib.rs`

**Step 1: Update init_tracing for consistent field output**

The current `init_tracing` already uses `with_target(true)` and `with_line_number(true)`. Add `with_thread_ids(false)`, `with_file(false)`, and ensure timestamps use RFC 3339. The tracing-subscriber `fmt` layer already outputs timestamp + level + target + message + fields by default, so the main change is adding `with_timer` for UTC ISO 8601:

```rust
fn init_tracing(log_level: &str, json_format: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339());

    if json_format {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}
```

**Note:** `tracing_subscriber::fmt::time::UtcTime::rfc_3339()` requires the `time` feature of tracing-subscriber. Check if this is already available — if `tracing-subscriber` is imported with `features = ["env-filter", "json"]`, we may need to add `"time"` as well. If `UtcTime` is not available, use `.with_timer(tracing_subscriber::fmt::time::SystemTime)` which is the default and already outputs ISO 8601 timestamps. The existing default timer is sufficient — only switch if you need strict UTC.

**Alternative (simpler, no new features):** If `UtcTime` requires additional features, keep the existing timer (it already outputs timestamps) and just add `with_thread_ids(false)`:

```rust
fn init_tracing(log_level: &str, json_format: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_line_number(true)
        .with_thread_ids(false);

    if json_format {
        subscriber.json().init();
    } else {
        subscriber.init();
    }
}
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "feat(logging): standardize tracing format output"
```

---

### Task 5: Create centralized audit_events module and extract audit repo method

**Files:**
- Create: `src/models/audit.rs`
- Modify: `src/models.rs` (add `pub mod audit;`)
- Create: `src/database/audit.rs`
- Modify: `src/database.rs` (add `pub mod audit;`)
- Modify: `src/models/password_reset.rs` (remove `audit_events` module)
- Modify: `src/database/password_reset.rs` (remove `create_security_audit_log` method)

**Step 1: Create src/models/audit.rs**

```rust
/// Event types for security audit log
pub mod audit_events {
    // Authentication events
    pub const LOGIN_SUCCESS: &str = "login_success";
    pub const LOGIN_FAILED: &str = "login_failed";
    pub const LOGOUT: &str = "logout";
    pub const SESSION_EXPIRED: &str = "session_expired";

    // Two-factor authentication events
    pub const TWO_FACTOR_ENABLED: &str = "2fa_enabled";
    pub const TWO_FACTOR_DISABLED: &str = "2fa_disabled";
    pub const TWO_FACTOR_BACKUP_USED: &str = "2fa_backup_used";

    // Account events
    pub const PASSWORD_CHANGED: &str = "password_changed";
    pub const ACCOUNT_UPDATED: &str = "account_updated";

    // Password reset events (moved from password_reset.rs)
    pub const PASSWORD_RESET_REQUESTED: &str = "password_reset_requested";
    pub const PASSWORD_RESET_TOKEN_VALIDATED: &str = "password_reset_token_validated";
    pub const PASSWORD_RESET_COMPLETED: &str = "password_reset_completed";
    pub const PASSWORD_RESET_FAILED: &str = "password_reset_failed";
    pub const PASSWORD_RESET_TOKEN_EXPIRED: &str = "password_reset_token_expired";
    pub const PASSWORD_RESET_TOKEN_INVALID: &str = "password_reset_token_invalid";
}
```

**Step 2: Create src/database/audit.rs**

Move the `create_security_audit_log` method from `src/database/password_reset.rs` into its own file. Also add a helper that both persists to DB and logs to tracing:

```rust
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use serde_json::Value as JsonValue;
use uuid::Uuid;

impl PostgresRepository {
    /// Create a security audit log entry and log it to tracing
    pub async fn create_security_audit_log(
        &self,
        user_id: Option<&Uuid>,
        event_type: &str,
        success: bool,
        ip_address: Option<String>,
        user_agent: Option<String>,
        metadata: Option<JsonValue>,
    ) -> Result<(), AppError> {
        // Log to tracing (stdout) as well for operational visibility
        if success {
            tracing::info!(
                category = "audit",
                event_type = event_type,
                success = success,
                user_id = user_id.map(|u| u.to_string()).as_deref().unwrap_or("-"),
                ip = ip_address.as_deref().unwrap_or("-"),
                "security audit event"
            );
        } else {
            tracing::warn!(
                category = "audit",
                event_type = event_type,
                success = success,
                user_id = user_id.map(|u| u.to_string()).as_deref().unwrap_or("-"),
                ip = ip_address.as_deref().unwrap_or("-"),
                "security audit event (failure)"
            );
        }

        sqlx::query(
            r#"
            INSERT INTO security_audit_log (user_id, event_type, success, ip_address, user_agent, metadata)
            VALUES ($1, $2, $3, $4::inet, $5, $6)
            "#,
        )
        .bind(user_id)
        .bind(event_type)
        .bind(success)
        .bind(ip_address)
        .bind(user_agent)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
```

**Step 3: Register the new modules**

In `src/models.rs`, add after the existing modules:
```rust
pub mod audit;
```

In `src/database.rs`, add after the existing modules:
```rust
pub mod audit;
```

**Step 4: Remove the old audit_events module from password_reset.rs**

In `src/models/password_reset.rs`, delete the entire `pub mod audit_events { ... }` block (lines 70-78).

**Step 5: Remove create_security_audit_log from database/password_reset.rs**

In `src/database/password_reset.rs`, delete the `create_security_audit_log` method (lines 134-160).

**Step 6: Update import paths in routes/password_reset.rs**

In `src/routes/password_reset.rs` line 6, change:
```rust
use crate::models::password_reset::{
    PasswordResetConfirmRequest, PasswordResetRequest, PasswordResetResponse, PasswordResetValidateRequest, PasswordResetValidateResponse, audit_events,
};
```
to:
```rust
use crate::models::audit::audit_events;
use crate::models::password_reset::{
    PasswordResetConfirmRequest, PasswordResetRequest, PasswordResetResponse, PasswordResetValidateRequest, PasswordResetValidateResponse,
};
```

**Step 7: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 8: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass

**Step 9: Commit**

```bash
git add src/models/audit.rs src/models.rs src/database/audit.rs src/database.rs src/models/password_reset.rs src/database/password_reset.rs src/routes/password_reset.rs
git commit -m "refactor(logging): extract audit events and audit repo into dedicated modules"
```

---

### Task 6: Add audit logging to login success/failure

**Files:**
- Modify: `src/routes/user.rs`

**Step 1: Add audit imports**

Add to the imports at the top of `src/routes/user.rs`:
```rust
use crate::models::audit::audit_events;
```

**Step 2: Add login_failed audit for wrong password**

In `post_user_login`, after the password verification failure (around line 115-117), add audit logging before returning the error. Replace:

```rust
if repo.verify_password(&user, &payload.password).await.is_err() {
    return Err(AppError::InvalidCredentials);
}
```

with:

```rust
if repo.verify_password(&user, &payload.password).await.is_err() {
    let _ = repo
        .create_security_audit_log(
            Some(&user.id),
            audit_events::LOGIN_FAILED,
            false,
            client_ip.0.clone(),
            user_agent.0.clone(),
            Some(serde_json::json!({"email": &payload.email, "reason": "invalid_password"})),
        )
        .await;
    return Err(AppError::InvalidCredentials);
}
```

**Step 3: Add login_failed audit for failed 2FA**

In `post_user_login`, where the 2FA code is invalid (around line 160-163), add audit logging before the `record_failed_attempt` call. Replace:

```rust
if !totp_valid && !backup_valid {
    // Record failed attempt
    repo.record_failed_attempt(&user.id).await?;
    return Err(AppError::BadRequest("Invalid two-factor authentication code.".to_string()));
}
```

with:

```rust
if !totp_valid && !backup_valid {
    repo.record_failed_attempt(&user.id).await?;
    let _ = repo
        .create_security_audit_log(
            Some(&user.id),
            audit_events::LOGIN_FAILED,
            false,
            client_ip.0.clone(),
            user_agent.0.clone(),
            Some(serde_json::json!({"email": &payload.email, "reason": "invalid_2fa_code"})),
        )
        .await;
    return Err(AppError::BadRequest("Invalid two-factor authentication code.".to_string()));
}
```

**Step 4: Add login_success audit and 2fa_backup_used**

After the session is created successfully (around line 185, just before `Ok(Status::Ok)`), add:

```rust
// Determine if backup code was used
let two_factor_info = if has_2fa {
    let backup_used = !totp_valid; // If totp wasn't valid but we got here, backup was used
    if backup_used {
        let _ = repo
            .create_security_audit_log(
                Some(&user.id),
                audit_events::TWO_FACTOR_BACKUP_USED,
                true,
                client_ip.0.clone(),
                user_agent.0.clone(),
                None,
            )
            .await;
    }
    Some(serde_json::json!({"2fa_used": true, "backup_code_used": backup_used}))
} else {
    None
};

let _ = repo
    .create_security_audit_log(
        Some(&user.id),
        audit_events::LOGIN_SUCCESS,
        true,
        client_ip.0.clone(),
        user_agent.0.clone(),
        Some(serde_json::json!({
            "email": &payload.email,
            "2fa_used": has_2fa,
        })),
    )
    .await;
```

**Important:** The variable `totp_valid` is only in scope inside the `if has_2fa` block. You'll need to hoist a `backup_code_used` bool out of the 2FA block so it's accessible here. Add `let mut backup_code_used = false;` before the `if has_2fa` block, and set `backup_code_used = true;` when `backup_valid` is true.

**Step 5: Add login_failed audit for unknown email**

In the `None` branch (around line 189-194), add audit logging after the dummy_verify:

```rust
None => {
    PostgresRepository::dummy_verify(&payload.password);
    let _ = repo
        .create_security_audit_log(
            None,
            audit_events::LOGIN_FAILED,
            false,
            client_ip.0.clone(),
            user_agent.0.clone(),
            Some(serde_json::json!({"email": &payload.email, "reason": "user_not_found"})),
        )
        .await;
    Err(AppError::InvalidCredentials)
}
```

**Step 6: Add logout audit**

In `post_user_logout` (around line 201-210), add audit logging. The current function doesn't have `ClientIp` or `UserAgent` guards — add them to the function signature:

```rust
pub async fn post_user_logout(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    cookies: &CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
) -> Status {
    if let Some(cookie) = cookies.get_private("user")
        && let Some((session_id, user_id)) = parse_session_cookie_value(cookie.value())
    {
        let repo = PostgresRepository { pool: pool.inner().clone() };
        let _ = repo.delete_session(&session_id).await;
        let _ = repo
            .create_security_audit_log(
                Some(&user_id),
                audit_events::LOGOUT,
                true,
                client_ip.0.clone(),
                user_agent.0.clone(),
                None,
            )
            .await;
    }
    cookies.remove_private(Cookie::build("user").build());
    Status::Ok
}
```

**Step 7: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 8: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass

**Step 9: Commit**

```bash
git add src/routes/user.rs
git commit -m "feat(logging): add audit logging for login success/failure and logout"
```

---

### Task 7: Add audit logging to 2FA enable/disable

**Files:**
- Modify: `src/routes/two_factor.rs`

**Step 1: Add audit import**

Add to the imports at the top of `src/routes/two_factor.rs`:
```rust
use crate::middleware::{ClientIp, UserAgent};
use crate::models::audit::audit_events;
```

**Step 2: Add 2fa_enabled audit to verify_two_factor**

Add `client_ip: ClientIp` and `user_agent: UserAgent` parameters to the `verify_two_factor` function signature. After the `repo.verify_and_enable_two_factor` call (around line 102), add:

```rust
let _ = repo
    .create_security_audit_log(
        Some(&current_user.id),
        audit_events::TWO_FACTOR_ENABLED,
        true,
        client_ip.0.clone(),
        user_agent.0.clone(),
        None,
    )
    .await;
```

**Step 3: Add 2fa_disabled audit to disable_two_factor**

Add `client_ip: ClientIp` and `user_agent: UserAgent` parameters to the `disable_two_factor` function signature. After the `repo.disable_two_factor` call (around line 154), add:

```rust
let _ = repo
    .create_security_audit_log(
        Some(&current_user.id),
        audit_events::TWO_FACTOR_DISABLED,
        true,
        client_ip.0.clone(),
        user_agent.0.clone(),
        Some(serde_json::json!({"method": "normal"})),
    )
    .await;
```

**Step 4: Add 2fa_disabled audit to emergency_disable_confirm**

Add `client_ip: ClientIp` and `user_agent: UserAgent` parameters to `emergency_disable_confirm`. After the `repo.disable_two_factor` call (around line 295), add:

```rust
let _ = repo
    .create_security_audit_log(
        Some(&user_id),
        audit_events::TWO_FACTOR_DISABLED,
        true,
        client_ip.0.clone(),
        user_agent.0.clone(),
        Some(serde_json::json!({"method": "emergency"})),
    )
    .await;
```

**Step 5: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 6: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass

**Step 7: Commit**

```bash
git add src/routes/two_factor.rs
git commit -m "feat(logging): add audit logging for 2FA enable/disable events"
```

---

### Task 8: Add audit logging for account updates and password changes

**Files:**
- Modify: `src/routes/user.rs`

**Step 1: Add account_updated audit to put_user**

Add `user_agent: UserAgent` and `client_ip: ClientIp` parameters to the `put_user` function signature. After the `repo.update_user` call (around line 81), add:

```rust
let _ = repo
    .create_security_audit_log(
        Some(&current_user.id),
        audit_events::ACCOUNT_UPDATED,
        true,
        client_ip.0.clone(),
        user_agent.0.clone(),
        Some(serde_json::json!({"changed_fields": ["name", "email", "password"]})),
    )
    .await;
```

**Note:** Since the `put_user` endpoint currently takes the full `UserRequest` payload (name, email, password), we log all fields. If you want to track which fields actually changed, you'd need to compare old vs new values — that's a future enhancement, not needed now.

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 3: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass

**Step 4: Commit**

```bash
git add src/routes/user.rs
git commit -m "feat(logging): add audit logging for account updates"
```

---

### Task 9: Add session_expired audit to auth guard

**Files:**
- Modify: `src/auth.rs`

**Step 1: Add audit logging when session is expired/invalid**

In the `CurrentUser` `from_request` implementation, the `Ok(None)` branch (around line 51-53) means the session was not found or expired. Add audit logging there. The tricky part is that `from_request` doesn't have easy access to `ClientIp` / `UserAgent` guards (you can't call other guards from a guard without recursion risk). Instead, extract the IP and user-agent directly from the request headers:

Replace the `Ok(None)` arm:

```rust
Ok(None) => {
    let _ = repo.delete_session_if_expired(&session_id).await;

    // Log session expired event
    let ip = req.client_ip().map(|ip| ip.to_string());
    let ua = req.headers().get_one("User-Agent").map(|s| s.to_string());
    let _ = repo
        .create_security_audit_log(
            Some(&user_id),
            crate::models::audit::audit_events::SESSION_EXPIRED,
            false,
            ip,
            ua,
            Some(serde_json::json!({"session_id": session_id.to_string()})),
        )
        .await;

    return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
}
```

**Step 2: Verify it compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 3: Run tests**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass

**Step 4: Commit**

```bash
git add src/auth.rs
git commit -m "feat(logging): add session_expired audit event in auth guard"
```

---

### Task 10: Final verification and cleanup

**Step 1: Run full lint check**

Run: `cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -20`
Expected: no warnings or errors

**Step 2: Run full test suite**

Run: `cargo test 2>&1 | tail -20`
Expected: all tests pass

**Step 3: Run cargo fmt**

Run: `cargo fmt`
Expected: code formatted

**Step 4: Verify the complete build**

Run: `cargo build 2>&1 | tail -5`
Expected: successful build

**Step 5: Commit any formatting changes**

```bash
git add -A
git commit -m "style(logging): apply cargo fmt"
```

**Step 6: Review the full diff**

Run: `git log --oneline -10` to verify all commits look correct.
