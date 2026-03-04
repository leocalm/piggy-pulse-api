use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::middleware::{ClientIp, UserAgent};
use crate::models::category::{CategoryResponse, CategoryType};
use crate::models::settings::{
    DeleteAccountRequest, PasswordChangeRequest, PeriodModelRequest, PeriodModelResponse, PreferencesRequest, PreferencesResponse, ProfileRequest,
    ProfileResponse, ResetStructureRequest, SessionInfoResponse, SettingsRequest, SettingsResponse,
};
use crate::models::transaction::TransactionResponse;
use crate::models::vendor::VendorResponse;
use rocket::http::{Cookie, CookieJar, Header, Status};
use rocket::request::Request;
use rocket::response::{Responder, Response};
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::openapi;
use rocket_okapi::response::OpenApiResponderInner;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// A typed file download response (used for CSV and JSON exports).
pub(crate) struct FileDownload {
    bytes: Vec<u8>,
    content_type: rocket::http::ContentType,
    filename: &'static str,
}

impl<'r> Responder<'r, 'static> for FileDownload {
    fn respond_to(self, _req: &'r Request<'_>) -> rocket::response::Result<'static> {
        Response::build()
            .header(self.content_type)
            .header(Header::new("Content-Disposition", format!("attachment; filename=\"{}\"", self.filename)))
            .sized_body(self.bytes.len(), std::io::Cursor::new(self.bytes))
            .ok()
    }
}

impl OpenApiResponderInner for FileDownload {
    fn responses(_gen: &mut OpenApiGenerator) -> Result<Responses, rocket_okapi::OpenApiError> {
        use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiResponse};
        let mut responses = Responses::default();
        responses.responses.insert(
            "200".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "File download".to_string(),
                ..Default::default()
            }),
        );
        Ok(responses)
    }
}

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
    client_ip: ClientIp,
    user_agent: UserAgent,
    payload: Json<PasswordChangeRequest>,
) -> Result<Status, AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.change_password(&current_user.id, &payload.current_password, &payload.new_password).await?;
    // Invalidate all other sessions so stolen session tokens are no longer usable
    match current_user.session_id {
        Some(ref sid) => repo.delete_other_sessions_for_user(&current_user.id, sid).await?,
        None => repo.delete_all_sessions_for_user(&current_user.id).await?,
    }
    let _ = repo
        .create_security_audit_log(
            Some(&current_user.id),
            crate::models::audit::audit_events::PASSWORD_CHANGED,
            true,
            client_ip.0.clone(),
            user_agent.0.clone(),
            None,
        )
        .await;
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
            ip_address: s.ip_address,
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
    payload.validate()?;

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
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.delete_user(&current_user.id).await?;
    cookies.remove_private(Cookie::build("user").build());
    Ok(Status::Ok)
}

// ── Export ────────────────────────────────────────────────────────────────────

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Export all transactions as CSV
#[openapi(tag = "Settings")]
#[get("/export/transactions")]
pub async fn get_export_transactions(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<FileDownload, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let transactions = repo.list_all_transactions(&current_user.id).await?;

    let mut csv = String::from("date,description,amount,currency,category,type,from_account,to_account,vendor\n");

    for t in &transactions {
        let tx = TransactionResponse::from(t);
        let decimal_places = tx.from_account.currency.decimal_places as u32;
        let divisor = 10i64.pow(decimal_places);
        let whole = tx.amount / divisor;
        let frac = (tx.amount % divisor).abs();
        let tx_type = if tx.to_account.is_some() {
            "transfer"
        } else if matches!(tx.category.category_type, CategoryType::Incoming) {
            "incoming"
        } else {
            "outgoing"
        };
        // lgtm[rust/cleartext-logging] - intentional: authenticated export endpoint returning user's own data
        let to_account = tx.to_account.as_ref().map(|a| a.name.as_str()).unwrap_or("");
        let vendor = tx.vendor.as_ref().map(|v| v.name.as_str()).unwrap_or("");

        csv.push_str(&format!(
            "{},{},{}.{:0>prec$},{},{},{},{},{},{}\n",
            tx.occurred_at,
            escape_csv(&tx.description),
            whole,
            frac,
            tx.from_account.currency.currency,
            escape_csv(&tx.category.name),
            tx_type,
            escape_csv(&tx.from_account.name),
            escape_csv(to_account),
            escape_csv(vendor),
            prec = decimal_places as usize,
        ));
    }

    Ok(FileDownload {
        bytes: csv.into_bytes(),
        content_type: rocket::http::ContentType::new("text", "csv"),
        filename: "transactions.csv",
    })
}

/// Export full user dataset as JSON (GDPR data portability)
#[openapi(tag = "Settings")]
#[get("/export/full")]
pub async fn get_export_full(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<FileDownload, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let transactions = repo.list_all_transactions(&current_user.id).await?;
    let transaction_responses: Vec<TransactionResponse> = transactions.iter().map(TransactionResponse::from).collect();

    let accounts = repo.list_accounts_management(&current_user.id).await?;
    let categories = repo
        .list_all_categories(&current_user.id)
        .await?
        .iter()
        .map(CategoryResponse::from)
        .collect::<Vec<_>>();
    let vendors = repo
        .list_all_vendors(&current_user.id)
        .await?
        .iter()
        .map(VendorResponse::from)
        .collect::<Vec<_>>();
    let profile = repo.get_profile(&current_user.id).await?;
    let preferences = repo.get_preferences(&current_user.id).await?;
    let settings = repo.get_settings(&current_user.id).await?;

    let export = serde_json::json!({
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "profile": ProfileResponse::from(&profile),
        "preferences": PreferencesResponse::from(&preferences),
        "settings": SettingsResponse::from(&settings),
        "accounts": accounts,
        "categories": categories,
        "vendors": vendors,
        "transactions": transaction_responses,
    });

    let json_str = serde_json::to_string_pretty(&export).map_err(|e| AppError::PasswordHash {
        message: format!("JSON serialization error: {e}"),
    })?;

    Ok(FileDownload {
        bytes: json_str.into_bytes(),
        content_type: rocket::http::ContentType::JSON,
        filename: "piggy-pulse-export.json",
    })
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
        get_export_transactions,
        get_export_full,
    ]
}
