use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::HasTransactionsResponse;
use crate::error::app_error::AppError;

#[get("/has-any")]
pub async fn has_any_transactions(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<HasTransactionsResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let has = repo.has_any_transactions(&user.id).await?;
    Ok(Json(HasTransactionsResponse { has_transactions: has }))
}
