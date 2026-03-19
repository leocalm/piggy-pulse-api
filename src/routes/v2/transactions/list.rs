use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::TransactionListResponse;
use crate::error::app_error::AppError;
use crate::models::pagination::TransactionFilters;
use crate::service::transaction::{TransactionService, parse_date, parse_direction};

#[get("/?<periodId>&<cursor>&<limit>&<direction>&<accountId>&<categoryId>&<vendorId>&<fromDate>&<toDate>")]
#[allow(clippy::too_many_arguments)]
#[allow(non_snake_case)]
pub async fn list_transactions(
    pool: &State<PgPool>,
    user: CurrentUser,
    periodId: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
    direction: Option<String>,
    accountId: Option<String>,
    categoryId: Option<String>,
    vendorId: Option<String>,
    fromDate: Option<String>,
    toDate: Option<String>,
) -> Result<Json<TransactionListResponse>, AppError> {
    // periodId is required
    let period_id_str = periodId.ok_or_else(|| AppError::BadRequest("periodId is required".to_string()))?;
    let period_uuid = Uuid::parse_str(&period_id_str).map_err(|e| AppError::uuid("Invalid periodId", e))?;

    let cursor_uuid = match cursor {
        Some(ref s) if !s.is_empty() => Some(Uuid::parse_str(s).map_err(|e| AppError::uuid("Invalid cursor", e))?),
        _ => None,
    };

    let effective_limit = limit.unwrap_or(50).min(200) as i64;

    // Build filters
    let mut filters = TransactionFilters::default();

    if let Some(ref dir) = direction {
        filters.direction = Some(parse_direction(dir)?);
    }

    if let Some(ref aid) = accountId {
        let uuid = Uuid::parse_str(aid).map_err(|e| AppError::uuid("Invalid accountId", e))?;
        filters.account_ids = vec![uuid];
    }

    if let Some(ref cid) = categoryId {
        let uuid = Uuid::parse_str(cid).map_err(|e| AppError::uuid("Invalid categoryId", e))?;
        filters.category_ids = vec![uuid];
    }

    if let Some(ref vid) = vendorId {
        let uuid = Uuid::parse_str(vid).map_err(|e| AppError::uuid("Invalid vendorId", e))?;
        filters.vendor_ids = vec![uuid];
    }

    if let Some(ref fd) = fromDate {
        filters.date_from = Some(parse_date(fd)?);
    }

    if let Some(ref td) = toDate {
        filters.date_to = Some(parse_date(td)?);
    }

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = TransactionService::new(&repo);

    let response = service.list_transactions(&period_uuid, cursor_uuid, effective_limit, filters, &user.id).await?;

    Ok(Json(response))
}
