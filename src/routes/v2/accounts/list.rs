use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::accounts::{AccountListResponse, AccountResponse};
use crate::dto::common::PaginatedResponse;
use crate::error::app_error::AppError;

#[get("/?<cursor>&<limit>")]
pub async fn list_accounts(pool: &State<PgPool>, user: CurrentUser, cursor: Option<String>, limit: Option<u32>) -> Result<Json<AccountListResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };

    let cursor_uuid = match cursor {
        Some(ref s) if !s.is_empty() => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid cursor", e))?),
        _ => None,
    };
    let effective_limit = limit.unwrap_or(50).min(200) as i64;

    let (mut accounts, total_count) = repo.list_accounts_v2(cursor_uuid, effective_limit, &user.id).await?;

    let has_more = accounts.len() as i64 > effective_limit;
    if has_more {
        accounts.truncate(effective_limit as usize);
    }
    let next_cursor = if has_more { accounts.last().map(|a| a.id.to_string()) } else { None };

    let data: Vec<AccountResponse> = accounts.iter().map(AccountResponse::from).collect();

    Ok(Json(PaginatedResponse {
        data,
        total_count,
        has_more,
        next_cursor,
    }))
}
