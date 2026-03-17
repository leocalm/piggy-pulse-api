use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::AccountListResponse;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[get("/?<cursor>&<limit>")]
pub async fn list_accounts(pool: &State<PgPool>, user: CurrentUser, cursor: Option<String>, limit: Option<u32>) -> Result<Json<AccountListResponse>, AppError> {
    let cursor_uuid = match cursor {
        Some(ref s) if !s.is_empty() => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid cursor", e))?),
        _ => None,
    };
    let effective_limit = limit.unwrap_or(50).min(200) as i64;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);

    let response = service.list_accounts_v2(cursor_uuid, effective_limit, &user.id).await?;
    Ok(Json(response))
}
