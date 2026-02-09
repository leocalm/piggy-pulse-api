use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::overlay::{OverlayRequest, OverlayResponse, TransactionWithMembership};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// List all overlays for the current user
#[openapi(tag = "Overlays")]
#[get("/")]
pub async fn list_overlays(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser) -> Result<Json<Vec<OverlayResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let overlays = repo.list_overlays(&current_user.id).await?;
    let responses: Vec<OverlayResponse> = overlays.iter().map(OverlayResponse::from).collect();
    Ok(Json(responses))
}

/// Create a new overlay
#[openapi(tag = "Overlays")]
#[post("/", data = "<payload>")]
pub async fn create_overlay(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<OverlayRequest>,
) -> Result<Json<OverlayResponse>, AppError> {
    payload.validate()?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let overlay = repo.create_overlay(&payload, &current_user.id).await?;
    Ok(Json(OverlayResponse::from(&overlay)))
}

/// Get a specific overlay by ID
#[openapi(tag = "Overlays")]
#[get("/<id>")]
pub async fn get_overlay(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: String) -> Result<Json<OverlayResponse>, AppError> {
    let overlay_id = Uuid::parse_str(&id)?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let overlay = repo.get_overlay(&overlay_id, &current_user.id).await?;
    Ok(Json(OverlayResponse::from(&overlay)))
}

/// Update an overlay by ID
#[openapi(tag = "Overlays")]
#[put("/<id>", data = "<payload>")]
pub async fn update_overlay(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: String,
    payload: Json<OverlayRequest>,
) -> Result<Json<OverlayResponse>, AppError> {
    payload.validate()?;
    let overlay_id = Uuid::parse_str(&id)?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let overlay = repo.update_overlay(&overlay_id, &payload, &current_user.id).await?;
    Ok(Json(OverlayResponse::from(&overlay)))
}

/// Delete an overlay by ID
#[openapi(tag = "Overlays")]
#[delete("/<id>")]
pub async fn delete_overlay(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: String) -> Result<Status, AppError> {
    let overlay_id = Uuid::parse_str(&id)?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.delete_overlay(&overlay_id, &current_user.id).await?;
    Ok(Status::NoContent)
}

/// Get transactions for an overlay with membership information
#[openapi(tag = "Overlays")]
#[get("/<id>/transactions")]
pub async fn get_overlay_transactions(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: String,
) -> Result<Json<Vec<TransactionWithMembership>>, AppError> {
    let overlay_id = Uuid::parse_str(&id)?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let transactions = repo.get_overlay_transactions(&overlay_id, &current_user.id).await?;
    Ok(Json(transactions))
}

/// Manually include a transaction in an overlay
#[openapi(tag = "Overlays")]
#[post("/<id>/transactions/<tx_id>/include")]
pub async fn include_transaction(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: String,
    tx_id: String,
) -> Result<Status, AppError> {
    let overlay_id = Uuid::parse_str(&id)?;
    let transaction_id = Uuid::parse_str(&tx_id)?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.include_transaction(&overlay_id, &transaction_id, &current_user.id).await?;
    Ok(Status::NoContent)
}

/// Manually exclude a transaction from an overlay
#[openapi(tag = "Overlays")]
#[delete("/<id>/transactions/<tx_id>/exclude")]
pub async fn exclude_transaction(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: String,
    tx_id: String,
) -> Result<Status, AppError> {
    let overlay_id = Uuid::parse_str(&id)?;
    let transaction_id = Uuid::parse_str(&tx_id)?;
    let repo = PostgresRepository { pool: pool.inner().clone() };
    repo.exclude_transaction(&overlay_id, &transaction_id, &current_user.id).await?;
    Ok(Status::NoContent)
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![
        list_overlays,
        create_overlay,
        get_overlay,
        update_overlay,
        delete_overlay,
        get_overlay_transactions,
        include_transaction,
        exclude_transaction
    ]
}
