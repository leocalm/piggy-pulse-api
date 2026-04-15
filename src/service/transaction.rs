//! Transaction service layer — encryption-at-rest edition.
//!
//! Phase 2b of the encryption refactor. The service layer converts V2 DTO
//! requests to the internal `TransactionRequest` model, hands them to the
//! repository's encrypted write path, and maps the returned
//! `LedgerInsertResult` into the wire `EncryptedTransactionResponse`.
//!
//! Read paths (list / stats / has-any / detail) are deleted; see Phase 3
//! which introduces `GET /v2/transactions` returning the full period as
//! ciphertext and retires the dashboard/stats/detail endpoints.

use crate::crypto::Dek;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::transactions::{CreateTransactionRequest, EncryptedTransactionResponse};
use crate::error::app_error::AppError;
use crate::models::pagination::TransactionDirection;
use crate::models::transaction::TransactionRequest as V1TransactionRequest;
use chrono::NaiveDate;
use uuid::Uuid;

pub struct TransactionService<'a> {
    repository: &'a PostgresRepository,
}

impl<'a> TransactionService<'a> {
    pub fn new(repository: &'a PostgresRepository) -> Self {
        TransactionService { repository }
    }

    pub async fn create_transaction(&self, request: &CreateTransactionRequest, user_id: &Uuid, dek: &Dek) -> Result<EncryptedTransactionResponse, AppError> {
        let v1_request = to_v1_request(request)?;
        let result = self.repository.create_transaction(&v1_request, user_id, dek).await?;
        Ok(result.into())
    }

    pub async fn batch_create_transactions(
        &self,
        requests: &[CreateTransactionRequest],
        user_id: &Uuid,
        dek: &Dek,
    ) -> Result<Vec<EncryptedTransactionResponse>, AppError> {
        let v1_requests: Result<Vec<_>, _> = requests.iter().map(to_v1_request).collect();
        let v1_requests = v1_requests?;
        let results = self.repository.batch_create_transactions(&v1_requests, user_id, dek).await?;
        Ok(results.into_iter().map(EncryptedTransactionResponse::from).collect())
    }

    pub async fn update_transaction(
        &self,
        id: &Uuid,
        request: &CreateTransactionRequest,
        user_id: &Uuid,
        dek: &Dek,
    ) -> Result<EncryptedTransactionResponse, AppError> {
        let v1_request = to_v1_request(request)?;
        let result = self.repository.update_transaction(id, &v1_request, user_id, dek).await?;
        Ok(result.into())
    }

    pub async fn delete_transaction(&self, id: &Uuid, user_id: &Uuid, dek: &Dek) -> Result<(), AppError> {
        self.repository.delete_transaction(id, user_id, dek).await
    }
}

/// Converts the V2 direction string (from query param) to the V1 TransactionDirection
/// which maps to DB category_type values. Kept for the filter-plumbing in
/// dto/pagination until Phase 3 retires the filter machinery entirely.
#[allow(dead_code)]
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

    if amount < 0 {
        return Err(AppError::BadRequest("amount must be >= 0".to_string()));
    }
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
#[allow(dead_code)]
pub fn parse_date(s: &str) -> Result<NaiveDate, AppError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| AppError::BadRequest(format!("Invalid date format '{}'. Expected YYYY-MM-DD", s)))
}
