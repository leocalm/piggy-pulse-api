use crate::Config;
use crate::auth::{CurrentUser, parse_session_cookie_value};
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::{AuthRateLimit, RateLimit};
use crate::middleware::{ClientIp, UserAgent};
use crate::models::audit::audit_events;
use crate::models::user::{LoginRequest, UserRequest, UserResponse, UserUpdateRequest};
use crate::service::auth::{AuthService, LoginOutcome};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::serde::json::Json;
use rocket::time::Duration;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new user (sign up)
#[openapi(tag = "Users")]
#[post("/", data = "<payload>")]
pub async fn post_user(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: AuthRateLimit,
    cookies: &CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    payload: Json<UserRequest>,
) -> Result<(Status, Json<UserResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };

    // Attempt the insert directly and let the DB unique constraint on email
    // handle duplicates. This avoids a separate SELECT that would leak timing
    // information about whether an account exists.
    match repo.create_user(&payload.name, &payload.email, &payload.password).await {
        Ok(user) => {
            // Create default settings for the new user (best-effort, non-critical)
            if let Err(e) = repo.create_default_settings(&user.id).await {
                tracing::warn!("Failed to create default settings for user {}: {}", user.id, e);
            }

            let ttl_seconds = config.session.ttl_seconds.max(60);
            let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds);
            let session = repo
                .create_session(&user.id, expires_at, user_agent.0.as_deref(), client_ip.0.as_deref())
                .await?;
            let value = format!("{}:{}", session.id, user.id);
            cookies.add_private(
                Cookie::build(("user", value))
                    .path("/")
                    .secure(config.session.cookie_secure)
                    .http_only(true)
                    .same_site(SameSite::Lax)
                    .max_age(Duration::seconds(ttl_seconds))
                    .build(),
            );
            Ok((Status::Created, Json(UserResponse::from(&user))))
        }
        Err(AppError::Db { ref source, .. }) if is_unique_violation(source) => Err(AppError::BadRequest("Unable to create account".to_string())),
        Err(e) => Err(e),
    }
}

/// Update a user by ID
#[openapi(tag = "Users")]
#[put("/<id>", data = "<payload>")]
pub async fn put_user(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    client_ip: ClientIp,
    user_agent: UserAgent,
    id: &str,
    payload: Json<UserUpdateRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid user id", e))?;
    if uuid != current_user.id {
        return Err(AppError::Forbidden);
    }
    payload.validate()?;
    let user = repo.update_user(&uuid, &payload.name, &payload.email, payload.password.as_deref()).await?;
    let changed_fields = payload.changed_fields();
    let _ = repo
        .create_security_audit_log(
            Some(&current_user.id),
            audit_events::ACCOUNT_UPDATED,
            true,
            client_ip.0.clone(),
            user_agent.0.clone(),
            Some(json!({"changed_fields": changed_fields})),
        )
        .await;
    Ok(Json(UserResponse::from(&user)))
}

/// Delete a user by ID
#[openapi(tag = "Users")]
#[delete("/<id>")]
pub async fn delete_user_route(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid user id", e))?;
    if uuid != current_user.id {
        return Err(AppError::Forbidden);
    }
    repo.delete_user(&uuid).await?;
    Ok(Status::Ok)
}

/// Log in a user and set authentication cookie
#[openapi(tag = "Users")]
#[post("/login", data = "<payload>")]
pub async fn post_user_login(
    pool: &State<PgPool>,
    config: &State<Config>,
    _rate_limit: AuthRateLimit,
    cookies: &CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    payload: Json<LoginRequest>,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let ip = client_ip.0.as_deref().unwrap_or("unknown");
    let auth = AuthService::new(&repo, config);

    match auth.login(&payload, ip, client_ip.0.clone(), user_agent.0.clone()).await? {
        LoginOutcome::TwoFactorRequired => Err(AppError::TwoFactorRequired),
        LoginOutcome::Success { session_id, user_id } => {
            let ttl_seconds = config.session.ttl_seconds.max(60);
            let value = format!("{}:{}", session_id, user_id);
            cookies.add_private(
                Cookie::build(("user", value))
                    .path("/")
                    .secure(config.session.cookie_secure)
                    .http_only(true)
                    .same_site(SameSite::Lax)
                    .max_age(Duration::seconds(ttl_seconds))
                    .build(),
            );
            Ok(Status::Ok)
        }
    }
}

/// Log out the current user
#[openapi(tag = "Users")]
#[post("/logout")]
pub async fn post_user_logout(pool: &State<PgPool>, _rate_limit: RateLimit, cookies: &CookieJar<'_>, user_agent: UserAgent, client_ip: ClientIp) -> Status {
    if let Some(cookie) = cookies.get_private("user")
        && let Some((session_id, user_id)) = parse_session_cookie_value(cookie.value())
    {
        let repo = PostgresRepository { pool: pool.inner().clone() };
        let _ = repo.delete_session(&session_id).await;
        let _ = repo
            .create_security_audit_log(Some(&user_id), audit_events::LOGOUT, true, client_ip.0.clone(), user_agent.0.clone(), None)
            .await;
    }
    cookies.remove_private(Cookie::build("user").build());
    Status::Ok
}

/// Get the currently authenticated user
#[openapi(tag = "Users")]
#[get("/me")]
pub async fn get_me(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<UserResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    if let Some(user) = repo.get_user_by_id(&current_user.id).await? {
        Ok(Json(UserResponse::from(&user)))
    } else {
        Err(AppError::NotFound(current_user.id.to_string()))
    }
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![post_user, post_user_login, post_user_logout, put_user, delete_user_route, get_me]
}

/// Check whether a sqlx error is a PostgreSQL unique-constraint violation (error code 23505).
fn is_unique_violation(err: &sqlx::error::Error) -> bool {
    if let sqlx::error::Error::Database(db_err) = err {
        return db_err.code().is_some_and(|code| code == "23505");
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{Config, build_rocket};
    use rocket::http::{ContentType, Cookie, Status};
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_me_unauthorized_without_cookie() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/users/me").dispatch().await;

        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_me_returns_current_user() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let payload = serde_json::json!({
            "name": "Test User",
            "email": "test.user@example.com",
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_id = user_json["id"].as_str().expect("user id");
        let user_email = user_json["email"].as_str().expect("user email");

        let login_payload = serde_json::json!({
            "email": user_email,
            "password": "CorrectHorseBatteryStaple!2026"
        });

        let login_response = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(login_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(login_response.status(), Status::Ok);

        let response = client.get("/api/v1/users/me").dispatch().await;

        assert_eq!(response.status(), Status::Ok);

        let body = response.into_string().await.expect("me response body");
        let me_json: Value = serde_json::from_str(&body).expect("valid me json");

        assert_eq!(me_json["id"].as_str().unwrap(), user_id);
        assert_eq!(me_json["email"].as_str().unwrap(), user_email);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_put_user_forbidden_when_wrong_user() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let payload = serde_json::json!({
            "name": "User A",
            "email": "user.a@example.com",
            "password": "password123"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_id = user_json["id"].as_str().expect("user id");
        let user_email = user_json["email"].as_str().expect("user email");

        let cookie_value = format!("{}:{}", user_id, user_email);
        client.cookies().add_private(Cookie::build(("user", cookie_value)).path("/").build());

        let other_id = uuid::Uuid::new_v4().to_string();
        let update_payload = serde_json::json!({
            "name": "Updated Name",
            "email": "updated@example.com",
            "password": "newpass123"
        });

        let response = client
            .put(format!("/api/v1/users/{}", other_id))
            .header(ContentType::JSON)
            .body(update_payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_delete_user_forbidden_when_wrong_user() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let payload = serde_json::json!({
            "name": "User B",
            "email": "user.b@example.com",
            "password": "password123"
        });

        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;

        assert_eq!(response.status(), Status::Created);

        let body = response.into_string().await.expect("user response body");
        let user_json: Value = serde_json::from_str(&body).expect("valid user json");
        let user_id = user_json["id"].as_str().expect("user id");
        let user_email = user_json["email"].as_str().expect("user email");

        let cookie_value = format!("{}:{}", user_id, user_email);
        client.cookies().add_private(Cookie::build(("user", cookie_value)).path("/").build());

        let other_id = uuid::Uuid::new_v4().to_string();

        let response = client.delete(format!("/api/v1/users/{}", other_id)).dispatch().await;

        assert_eq!(response.status(), Status::Forbidden);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_login_returns_429_after_excessive_failed_attempts() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;
        // Use tight limits for testing
        config.login_rate_limit.free_attempts = 2;
        config.login_rate_limit.delay_seconds = vec![5, 30];
        config.login_rate_limit.lockout_attempts = 5;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Register user
        let payload = serde_json::json!({
            "name": "Rate Limit User",
            "email": "rate.limit.test@example.com",
            "password": "CorrectHorseBatteryStaple!2026"
        });
        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        // Log out
        let _ = client.post("/api/v1/users/logout").dispatch().await;

        let wrong_creds = serde_json::json!({
            "email": "rate.limit.test@example.com",
            "password": "WrongPassword123"
        });

        // Consume free attempts
        for _ in 0..2 {
            let r = client
                .post("/api/v1/users/login")
                .header(ContentType::JSON)
                .body(wrong_creds.to_string())
                .dispatch()
                .await;
            assert_eq!(r.status(), Status::Unauthorized);
        }

        // Next attempt after free_attempts should be rate-limited
        let r = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(wrong_creds.to_string())
            .dispatch()
            .await;
        assert_eq!(r.status(), Status::TooManyRequests);

        let body = r.into_string().await.expect("response body");
        let json: Value = serde_json::from_str(&body).expect("valid json");
        assert!(json.get("retry_after_seconds").is_some(), "Should include retry_after_seconds");
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_login_returns_423_after_lockout() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;
        // free_attempts=2 < lockout_attempts=3: attempt 3 locks directly, no delay step.
        // This ensures we can reach lockout without having to wait out a delay period.
        config.login_rate_limit.free_attempts = 2;
        config.login_rate_limit.delay_seconds = vec![5];
        config.login_rate_limit.lockout_attempts = 3;
        config.login_rate_limit.lockout_duration_minutes = 1;
        config.login_rate_limit.enable_email_unlock = false; // No email in tests

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        // Register user
        let payload = serde_json::json!({
            "name": "Lockout Test User",
            "email": "lockout.test@example.com",
            "password": "CorrectHorseBatteryStaple!2026"
        });
        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let _ = client.post("/api/v1/users/logout").dispatch().await;

        let wrong_creds = serde_json::json!({
            "email": "lockout.test@example.com",
            "password": "WrongPassword123"
        });

        // Exhaust attempts up to lockout threshold
        for _ in 0..3 {
            let _ = client
                .post("/api/v1/users/login")
                .header(ContentType::JSON)
                .body(wrong_creds.to_string())
                .dispatch()
                .await;
        }

        // Account should now be locked (423)
        let r = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(wrong_creds.to_string())
            .dispatch()
            .await;
        assert_eq!(r.status(), Status { code: 423 });

        let body = r.into_string().await.expect("response body");
        let json: Value = serde_json::from_str(&body).expect("valid json");
        assert!(json.get("locked_until").is_some(), "Should include locked_until");
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_successful_login_resets_rate_limit() {
        let mut config = Config::default();
        config.database.url = "postgres://postgres:example@127.0.0.1:5432/piggy_pulse_db".to_string();
        config.session.cookie_secure = false;
        config.login_rate_limit.free_attempts = 3;

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let password = "CorrectHorseBatteryStaple!2026";
        let payload = serde_json::json!({
            "name": "Reset Test User",
            "email": "reset.test@example.com",
            "password": password
        });
        let response = client
            .post("/api/v1/users/")
            .header(ContentType::JSON)
            .body(payload.to_string())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Created);

        let _ = client.post("/api/v1/users/logout").dispatch().await;

        let wrong_creds = serde_json::json!({
            "email": "reset.test@example.com",
            "password": "WrongPassword123"
        });
        let correct_creds = serde_json::json!({
            "email": "reset.test@example.com",
            "password": password
        });

        // Two failed attempts (within free_attempts = 3)
        for _ in 0..2 {
            let r = client
                .post("/api/v1/users/login")
                .header(ContentType::JSON)
                .body(wrong_creds.to_string())
                .dispatch()
                .await;
            assert_eq!(r.status(), Status::Unauthorized);
        }

        // Successful login should clear the counter
        let r = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(correct_creds.to_string())
            .dispatch()
            .await;
        assert_eq!(r.status(), Status::Ok);

        let _ = client.post("/api/v1/users/logout").dispatch().await;

        // After reset, wrong attempts should start fresh (Unauthorized, not rate limited)
        let r = client
            .post("/api/v1/users/login")
            .header(ContentType::JSON)
            .body(wrong_creds.to_string())
            .dispatch()
            .await;
        assert_eq!(r.status(), Status::Unauthorized);
    }
}
