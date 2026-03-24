use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryType as V2CategoryType;
use crate::dto::common::Date;
use crate::dto::transactions::{AccountRef, CategoryRef, CreateTransactionRequest, TransactionKind, TransactionListResponse, TransactionResponse, VendorRef};
use crate::error::app_error::AppError;
use crate::models::category::CategoryType as V1CategoryType;
use crate::models::pagination::{CursorParams, TransactionDirection, TransactionFilters};
use crate::models::transaction::{Transaction, TransactionRequest as V1TransactionRequest};
use chrono::NaiveDate;
use uuid::Uuid;

pub struct TransactionService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> TransactionService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        TransactionService { repository }
    }

    pub async fn create_transaction(&self, request: &CreateTransactionRequest, user_id: &Uuid) -> Result<TransactionResponse, AppError> {
        let v1_request = to_v1_request(request)?;
        let tx = self.repository.create_transaction(&v1_request, user_id).await?;
        Ok(to_v2_response(&tx))
    }

    /// Creates all transactions in a single atomic DB transaction.
    /// Returns all created transactions or rolls back entirely if any item fails.
    pub async fn batch_create_transactions(&self, requests: &[CreateTransactionRequest], user_id: &Uuid) -> Result<Vec<TransactionResponse>, AppError> {
        let v1_requests: Result<Vec<_>, _> = requests.iter().map(to_v1_request).collect();
        let v1_requests = v1_requests?;
        let txs = self.repository.batch_create_transactions(&v1_requests, user_id).await?;
        Ok(txs.iter().map(to_v2_response).collect())
    }

    pub async fn update_transaction(&self, id: &Uuid, request: &CreateTransactionRequest, user_id: &Uuid) -> Result<TransactionResponse, AppError> {
        // Check existence first to return 404 instead of RowNotFound
        let existing = self.repository.get_transaction_by_id(id, user_id).await?;
        if existing.is_none() {
            return Err(AppError::NotFound("Transaction not found".to_string()));
        }

        let v1_request = to_v1_request(request)?;
        let tx = self.repository.update_transaction(id, &v1_request, user_id).await?;
        Ok(to_v2_response(&tx))
    }

    pub async fn delete_transaction(&self, id: &Uuid, user_id: &Uuid) -> Result<(), AppError> {
        let existing = self.repository.get_transaction_by_id(id, user_id).await?;
        if existing.is_none() {
            return Err(AppError::NotFound("Transaction not found".to_string()));
        }
        self.repository.delete_transaction(id, user_id).await
    }

    pub async fn list_transactions(
        &self,
        period_id: &Uuid,
        cursor: Option<Uuid>,
        limit: i64,
        filters: TransactionFilters,
        user_id: &Uuid,
    ) -> Result<TransactionListResponse, AppError> {
        let params = CursorParams { cursor, limit: Some(limit) };

        let mut rows = self.repository.get_transactions_for_period(period_id, &params, &filters, user_id).await?;

        let has_more = rows.len() as i64 > limit;
        if has_more {
            rows.truncate(limit as usize);
        }
        let next_cursor = if has_more { rows.last().map(|t| t.id.to_string()) } else { None };

        // Count total for the period (without pagination)
        let total_count = rows.len() as i64;

        let data: Vec<TransactionResponse> = rows.iter().map(to_v2_response).collect();

        Ok(TransactionListResponse {
            data,
            total_count,
            has_more,
            next_cursor,
        })
    }
}

/// Converts the V2 direction string (from query param) to the V1 TransactionDirection
/// which maps to DB category_type values.
pub fn parse_direction(direction: &str) -> Result<TransactionDirection, AppError> {
    match direction {
        "income" => Ok(TransactionDirection::Incoming),
        "expense" => Ok(TransactionDirection::Outgoing),
        "transfer" => Ok(TransactionDirection::Transfer),
        _ => Err(AppError::BadRequest(format!(
            "Invalid direction '{}'. Must be one of: income, expense, transfer",
            direction
        ))),
    }
}

fn to_v2_category_type(ct: V1CategoryType) -> V2CategoryType {
    match ct {
        V1CategoryType::Incoming => V2CategoryType::Income,
        V1CategoryType::Outgoing => V2CategoryType::Expense,
        V1CategoryType::Transfer => V2CategoryType::Transfer,
    }
}

fn to_v2_response(tx: &Transaction) -> TransactionResponse {
    let from_account = AccountRef {
        id: tx.from_account.id,
        name: tx.from_account.name.clone(),
        color: tx.from_account.color.clone(),
    };

    let category = CategoryRef {
        id: tx.category.id,
        name: tx.category.name.clone(),
        color: tx.category.color.clone(),
        icon: tx.category.icon.clone(),
        category_type: to_v2_category_type(tx.category.category_type),
    };

    let vendor = tx.vendor.as_ref().map(|v| VendorRef {
        id: v.id,
        name: v.name.clone(),
    });

    let kind = match &tx.to_account {
        Some(to_acc) => TransactionKind::Transfer {
            to_account: AccountRef {
                id: to_acc.id,
                name: to_acc.name.clone(),
                color: to_acc.color.clone(),
            },
        },
        None => TransactionKind::Regular { to_account: None },
    };

    TransactionResponse {
        id: tx.id,
        date: Date(tx.occurred_at),
        description: tx.description.clone(),
        amount: tx.amount,
        from_account,
        category,
        vendor,
        kind,
    }
}

/// Validates and converts a V2 CreateTransactionRequest into a V1 TransactionRequest.
fn to_v1_request(request: &CreateTransactionRequest) -> Result<V1TransactionRequest, AppError> {
    let (date, description, amount, from_account_id, category_id, vendor_id, to_account_id) = match request {
        CreateTransactionRequest::Regular {
            date,
            description,
            amount,
            from_account_id,
            category_id,
            vendor_id,
        } => (date, description, *amount, *from_account_id, *category_id, vendor_id.as_ref().copied(), None),
        CreateTransactionRequest::Transfer {
            date,
            description,
            amount,
            from_account_id,
            category_id,
            vendor_id,
            to_account_id,
        } => (
            date,
            description,
            *amount,
            *from_account_id,
            *category_id,
            vendor_id.as_ref().copied(),
            Some(*to_account_id),
        ),
    };

    // Validate amount >= 0
    if amount < 0 {
        return Err(AppError::BadRequest("amount must be >= 0".to_string()));
    }

    // Validate description length
    if description.len() < 3 {
        return Err(AppError::BadRequest("description must be at least 3 characters".to_string()));
    }

    Ok(V1TransactionRequest {
        amount,
        description: description.clone(),
        occurred_at: date.0,
        category_id,
        from_account_id,
        to_account_id,
        vendor_id,
    })
}

/// Parse a date string in YYYY-MM-DD format.
pub fn parse_date(s: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| AppError::BadRequest(format!("Invalid date format '{}'. Expected YYYY-MM-DD", s)))
}
