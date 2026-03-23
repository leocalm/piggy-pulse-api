use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::CurrencyResponse;
use crate::error::app_error::AppError;
use crate::service::currency::CurrencyService;

#[get("/<code>")]
pub async fn get_currency(pool: &State<PgPool>, code: &str) -> Result<Json<CurrencyResponse>, AppError> {
    if !code.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(AppError::NotFound("Currency not found".to_string()));
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CurrencyService::new(&repo);
    let currency = service.get_currency_by_code(code).await?;
    Ok(Json(currency))
}
