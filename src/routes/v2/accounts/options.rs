use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::AccountOptionListResponse;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[get("/options")]
pub async fn get_account_options(pool: &State<PgPool>, user: CurrentUser) -> Result<Json<AccountOptionListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);
    Ok(Json(service.list_account_options(&user.id).await?))
}
