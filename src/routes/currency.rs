use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::currency::{CurrencyRequest, CurrencyResponse};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, delete, get, post, put};
use rocket_okapi::openapi;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

/// Create a new currency
#[openapi(tag = "Currencies")]
#[post("/", data = "<payload>")]
pub async fn create_currency(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    payload: Json<CurrencyRequest>,
) -> Result<(Status, Json<CurrencyResponse>), AppError> {
    payload.validate()?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let currency = repo.create_currency(&payload, &current_user.id).await?;
    Ok((Status::Created, Json(CurrencyResponse::from(&currency))))
}

/// Get a currency by its code (e.g., USD, EUR)
#[openapi(tag = "Currencies")]
#[get("/<code>")]
pub async fn get_currency(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, code: &str) -> Result<Json<CurrencyResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    if let Some(currency) = repo.get_currency_by_code(code, &current_user.id).await? {
        Ok(Json(CurrencyResponse::from(&currency)))
    } else {
        Err(AppError::NotFound(format!("Currency not found: {}", code)))
    }
}

/// Get currencies by name
#[openapi(tag = "Currencies")]
#[get("/name/<name>")]
pub async fn get_currencies(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    name: &str,
) -> Result<Json<Vec<CurrencyResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let currencies = repo.get_currencies(name, &current_user.id).await?;
    Ok(Json(currencies.iter().map(CurrencyResponse::from).collect()))
}

/// Delete a currency by ID
#[openapi(tag = "Currencies")]
#[delete("/<id>")]
pub async fn delete_currency(pool: &State<PgPool>, _rate_limit: RateLimit, current_user: CurrentUser, id: &str) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid currency id", e))?;
    repo.delete_currency(&uuid, &current_user.id).await?;
    Ok(Status::Ok)
}

/// Update a currency by ID
#[openapi(tag = "Currencies")]
#[put("/<id>", data = "<payload>")]
pub async fn put_currency(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    current_user: CurrentUser,
    id: &str,
    payload: Json<CurrencyRequest>,
) -> Result<Json<CurrencyResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let uuid = Uuid::parse_str(id).map_err(|e| AppError::uuid("Invalid currency id", e))?;
    let currency = repo.update_currency(&uuid, &payload, &current_user.id).await?;
    Ok(Json(CurrencyResponse::from(&currency)))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![create_currency, get_currency, get_currencies, delete_currency, put_currency]
}
