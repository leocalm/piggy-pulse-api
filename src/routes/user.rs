use crate::Config;
use crate::auth::{CurrentUser, parse_session_cookie_value};
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::{AuthRateLimit, RateLimit};
use crate::models::user::{LoginRequest, UserRequest, UserResponse};
use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::serde::json::Json;
use rocket::time::Duration;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
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
            let session = repo.create_session(&user.id, expires_at).await?;
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
    id: &str,
    payload: Json<UserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid user id", e))?;
    if uuid != current_user.id {
        return Err(AppError::Forbidden);
    }
    payload.validate()?;
    let user = repo.update_user(&uuid, &payload.name, &payload.email, &payload.password).await?;
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
    payload: Json<LoginRequest>,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    match repo.get_user_by_email(&payload.email).await? {
        Some(user) => {
            // Verify password first
            if repo.verify_password(&user, &payload.password).await.is_err() {
                return Err(AppError::InvalidCredentials);
            }

            // Check if user has 2FA enabled
            let two_factor = repo.get_two_factor_by_user(&user.id).await?;
            let has_2fa = two_factor.as_ref().map(|tf| tf.is_enabled).unwrap_or(false);

            if has_2fa {
                // 2FA is enabled - check if code was provided
                if payload.two_factor_code.is_none() {
                    // Return 428 Precondition Required with JSON response
                    return Err(AppError::TwoFactorRequired);
                }

                // Code was provided - verify it
                let code = payload.two_factor_code.as_ref().unwrap();

                // Check rate limit
                if repo.check_rate_limit(&user.id).await? {
                    return Err(AppError::BadRequest("Too many failed attempts. Please try again later.".to_string()));
                }

                // Parse encryption key
                let encryption_key = config.two_factor.parse_encryption_key().map_err(AppError::BadRequest)?;

                // Decrypt and verify in blocking task
                let two_factor_data = two_factor.unwrap();
                let encrypted_secret = two_factor_data.encrypted_secret.clone();
                let encryption_nonce = two_factor_data.encryption_nonce.clone();
                let code_clone = code.clone();

                let totp_valid = tokio::task::spawn_blocking(move || {
                    // Decrypt the secret
                    let secret = PostgresRepository::decrypt_secret(&encrypted_secret, &encryption_nonce, &encryption_key)?;

                    // Verify TOTP code
                    PostgresRepository::verify_totp_code(&secret, &code_clone)
                })
                .await
                .map_err(|e| AppError::BadRequest(format!("Task join error: {}", e)))??;

                // If TOTP failed, try backup code
                let backup_valid = if !totp_valid { repo.verify_backup_code(&user.id, code).await? } else { false };

                if !totp_valid && !backup_valid {
                    // Record failed attempt
                    repo.record_failed_attempt(&user.id).await?;
                    return Err(AppError::BadRequest("Invalid two-factor authentication code.".to_string()));
                }

                // Success - reset rate limit
                repo.reset_rate_limit(&user.id).await?;
            }

            // Create session (either no 2FA or 2FA passed)
            let ttl_seconds = config.session.ttl_seconds.max(60);
            let expires_at = chrono::Utc::now() + chrono::Duration::seconds(ttl_seconds);
            let session = repo.create_session(&user.id, expires_at).await?;
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

            Ok(Status::Ok)
        }
        None => {
            // Equalize response timing so attackers cannot distinguish
            // existing from non-existing accounts by measuring latency.
            PostgresRepository::dummy_verify(&payload.password);
            Err(AppError::InvalidCredentials)
        }
    }
}

/// Log out the current user
#[openapi(tag = "Users")]
#[post("/logout")]
pub async fn post_user_logout(pool: &State<PgPool>, _rate_limit: RateLimit, cookies: &CookieJar<'_>) -> Status {
    if let Some(cookie) = cookies.get_private("user")
        && let Some((session_id, _)) = parse_session_cookie_value(cookie.value())
    {
        let repo = PostgresRepository { pool: pool.inner().clone() };
        let _ = repo.delete_session(&session_id).await;
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
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");

        let response = client.get("/api/v1/users/me").dispatch().await;

        assert_eq!(response.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn test_get_me_returns_current_user() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

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
        config.database.url = "postgresql://test:test@localhost/test".to_string();

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
        config.database.url = "postgresql://test:test@localhost/test".to_string();

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
}
