use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::AccountListResponse;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[get("/")]
pub async fn list_accounts(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<AccountListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    Ok(Json(service.list_accounts(&user.id).await?))
}
