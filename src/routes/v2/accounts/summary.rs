use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::AccountSummaryListResponse;
use crate::error::app_error::AppError;
use crate::service::account::AccountService;

#[get("/summary?<periodId>&<cursor>&<limit>")]
pub async fn list_account_summaries(
    pool: &State<PgPool>,
    user: CurrentUser,
    #[allow(non_snake_case)] periodId: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
) -> Result<Json<AccountSummaryListResponse>, AppError> {
    let cursor_uuid = match cursor {
        Some(ref s) if !s.is_empty() && s != "null" => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid cursor", e))?),
        _ => None,
    };
    let effective_limit = limit.unwrap_or(50).min(200) as i64;
    let period_uuid = match periodId {
        Some(ref s) => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid period id", e))?),
        None => return Err(AppError::BadRequest("periodId is required".to_string())),
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = AccountService::new(&repo);

    let response = service.list_account_summaries_v2(cursor_uuid, effective_limit, period_uuid, &user.id).await?;
    Ok(Json(response))
}
