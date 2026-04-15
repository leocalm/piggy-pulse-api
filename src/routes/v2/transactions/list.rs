use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::EncryptedTransactionResponse;
use crate::error::app_error::AppError;
use crate::service::transaction::TransactionService;

#[get("/?<periodId>")]
#[allow(non_snake_case)]
pub async fn list_transactions(pool: &State<PgPool>, user: CurrentUser, periodId: Option<String>) -> Result<Json<Vec<EncryptedTransactionResponse>>, AppError> {
    let period_uuid = match periodId {
        Some(ref s) if !s.is_empty() && s != "null" => Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid periodId", e))?,
        _ => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);
    Ok(Json(service.list_by_period(&period_uuid, &user.id).await?))
}
