use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::CurrencyListResponse;
use crate::error::app_error::AppError;
use crate::service::currency::CurrencyService;

#[get("/")]
pub async fn list_currencies(pool: &State<PgPool>) -> Result<Json<CurrencyListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = CurrencyService::new(&repo);
    let currencies = service.list_currencies().await?;
    Ok(Json(currencies))
}
