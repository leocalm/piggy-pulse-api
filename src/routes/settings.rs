use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::settings::{
    DeleteAccountRequest, PasswordChangeRequest, PeriodModelRequest, PeriodModelResponse, PreferencesRequest, PreferencesResponse, ProfileRequest,
    ProfileResponse, ResetStructureRequest, SessionInfoResponse, SettingsRequest, SettingsResponse,
};
use rocket::http::{Cookie, CookieJar, Status};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

// ── General settings ──────────────────────────────────────────────────────────

/// Get current user's settings
#[openapi(tag = "Settings")]
#[get("/")]
pub async fn get_settings(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<SettingsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let settings = repo.get_settings(&current_user.id).await?;
    Ok(Json(SettingsResponse::from(&settings)))
}

/// Update current user's settings (creates if not exists)
#[openapi(tag = "Settings")]
#[put("/", data = "<payload>")]
pub async fn put_settings(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<SettingsRequest>,
) -> Result<Json<SettingsResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let settings = repo.upsert_settings(&payload, &current_user.id).await?;
    Ok(Json(SettingsResponse::from(&settings)))
}

// ── Profile ───────────────────────────────────────────────────────────────────

/// Get the authenticated user's profile (name, masked email, timezone, currency)
#[openapi(tag = "Settings")]
#[get("/profile")]
pub async fn get_profile(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<ProfileResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let profile = repo.get_profile(&current_user.id).await?;
    Ok(Json(ProfileResponse::from(&profile)))
}

/// Update the authenticated user's profile (name, timezone, currency)
#[openapi(tag = "Settings")]
#[put("/profile", data = "<payload>")]
pub async fn put_profile(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<ProfileRequest>,
) -> Result<Json<ProfileResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let profile = repo.update_profile(&current_user.id, &payload).await?;
    Ok(Json(ProfileResponse::from(&profile)))
}

// ── Preferences ───────────────────────────────────────────────────────────────

/// Get the authenticated user's UI preferences (theme, date/number format, compact mode)
#[openapi(tag = "Settings")]
#[get("/preferences")]
pub async fn get_preferences(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<PreferencesResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let prefs = repo.get_preferences(&current_user.id).await?;
    Ok(Json(PreferencesResponse::from(&prefs)))
}

/// Update the authenticated user's UI preferences
#[openapi(tag = "Settings")]
#[put("/preferences", data = "<payload>")]
pub async fn put_preferences(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<PreferencesRequest>,
) -> Result<Json<PreferencesResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let prefs = repo
        .update_preferences(
            &current_user.id,
            &payload.theme,
            &payload.date_format,
            &payload.number_format,
            payload.compact_mode,
        )
        .await?;
    Ok(Json(PreferencesResponse::from(&prefs)))
}

// ── Security: password ────────────────────────────────────────────────────────

/// Change the authenticated user's password (requires current password verification)
#[openapi(tag = "Settings")]
#[post("/security/password", data = "<payload>")]
pub async fn post_change_password(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<PasswordChangeRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.change_password(&current_user.id, &payload.current_password, &payload.new_password).await?;
    Ok(Status::Ok)
}

// ── Security: sessions ────────────────────────────────────────────────────────

/// List all active sessions for the authenticated user
#[openapi(tag = "Settings")]
#[get("/security/sessions")]
pub async fn list_sessions(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<Vec<SessionInfoResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let sessions = repo.list_sessions_for_user(&current_user.id).await?;
    let responses = sessions
        .into_iter()
        .map(|s| SessionInfoResponse {
            id: s.id,
            created_at: s.created_at,
            expires_at: s.expires_at,
            user_agent: s.user_agent,
        })
        .collect();
    Ok(Json(responses))
}

/// Revoke a session by ID. Revoking the current session logs out immediately.
#[openapi(tag = "Settings")]
#[delete("/security/sessions/<id>")]
pub async fn revoke_session(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    cookies: &CookieJar<'_>,
    id: &str,
) -> Result<Status, AppError> {
    let session_id = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid session id", e))?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.delete_session_for_user(&session_id, &current_user.id).await?;

    // Clear the cookie if the revoked session is the current one
    if let Some(cookie) = cookies.get_private("user")
        && let Some((current_session_id, _)) = crate::auth::parse_session_cookie_value(cookie.value())
        && current_session_id == session_id
    {
        cookies.remove_private(Cookie::build("user").build());
    }

    Ok(Status::Ok)
}

// ── Period model ──────────────────────────────────────────────────────────────

/// Get the authenticated user's period model configuration
#[openapi(tag = "Settings")]
#[get("/period-model")]
pub async fn get_period_model(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<PeriodModelResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let model = repo.get_period_model(&current_user.id).await?;
    Ok(Json(model))
}

/// Update the authenticated user's period model configuration
#[openapi(tag = "Settings")]
#[put("/period-model", data = "<payload>")]
pub async fn put_period_model(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<PeriodModelRequest>,
) -> Result<Json<PeriodModelResponse>, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let model = repo.upsert_period_model(&current_user.id, &payload).await?;
    Ok(Json(model))
}

// ── Danger zone ───────────────────────────────────────────────────────────────

/// Reset the user's financial structure (accounts, categories, budget periods, period schedule).
/// All linked transactions are also removed due to database cascade rules.
/// Requires `confirmation` = "RESET".
#[openapi(tag = "Settings")]
#[post("/danger/reset-structure", data = "<payload>")]
pub async fn post_reset_structure(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<ResetStructureRequest>,
) -> Result<Status, AppError> {
    if payload.confirmation != "RESET" {
        return Err(AppError::BadRequest("confirmation must equal 'RESET'".to_string()));
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.reset_structure(&current_user.id).await?;
    Ok(Status::Ok)
}

/// Permanently delete the authenticated user's account.
/// Requires `confirmation` = "DELETE".
#[openapi(tag = "Settings")]
#[post("/danger/delete-account", data = "<payload>")]
pub async fn post_delete_account(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    cookies: &CookieJar<'_>,
    payload: Json<DeleteAccountRequest>,
) -> Result<Status, AppError> {
    if payload.confirmation != "DELETE" {
        return Err(AppError::BadRequest("confirmation must equal 'DELETE'".to_string()));
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.delete_user(&current_user.id).await?;
    cookies.remove_private(Cookie::build("user").build());
    Ok(Status::Ok)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        get_settings,
        put_settings,
        get_profile,
        put_profile,
        get_preferences,
        put_preferences,
        post_change_password,
        list_sessions,
        revoke_session,
        get_period_model,
        put_period_model,
        post_reset_structure,
        post_delete_account,
    ]
}
