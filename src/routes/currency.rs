use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::rate_limit::RateLimit;
use crate::models::currency::CurrencyResponse;
use rocket::serde::json::Json;
use rocket::{State, get};
use rocket_okapi::openapi;
use sqlx::PgPool;

/// Get all currencies
#[openapi(tag = "Currencies")]
#[get("/")]
pub async fn get_all_currencies(pool: &State<PgPool>, _rate_limit: RateLimit, _current_user: CurrentUser) -> Result<Json<Vec<CurrencyResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let currencies = repo.get_all_currencies().await?;
    Ok(Json(currencies.iter().map(CurrencyResponse::from).collect()))
}

/// Get a currency by its code (e.g., USD, EUR)
#[openapi(tag = "Currencies")]
#[get("/<code>")]
pub async fn get_currency(pool: &State<PgPool>, _rate_limit: RateLimit, _current_user: CurrentUser, code: &str) -> Result<Json<CurrencyResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    if let Some(currency) = repo.get_currency_by_code(code).await? {
        Ok(Json(CurrencyResponse::from(&currency)))
    } else {
        Err(AppError::NotFound(format!("Currency not found: {}", code)))
    }
}

/// Get currencies by name
#[openapi(tag = "Currencies")]
#[get("/name/<name>")]
pub async fn get_currencies_by_name(
    pool: &State<PgPool>,
    _rate_limit: RateLimit,
    _current_user: CurrentUser,
    name: &str,
) -> Result<Json<Vec<CurrencyResponse>>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let currencies = repo.get_currencies_by_name(name).await?;
    Ok(Json(currencies.iter().map(CurrencyResponse::from).collect()))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![get_all_currencies, get_currency, get_currencies_by_name]
}
