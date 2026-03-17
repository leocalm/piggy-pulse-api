use rocket::State;
use rocket::http::Status;
use rocket::post;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::database::postgres_repository::PostgresRepository;
use crate::dto::categories::CategoryType as V2CategoryType;
use crate::dto::common::Date;
use crate::dto::transactions::{AccountRef, CategoryRef, CreateTransactionRequest, TransactionKind, TransactionResponse, VendorRef};
use crate::error::app_error::AppError;
use crate::models::category::CategoryType as V1CategoryType;
use crate::models::transaction::{Transaction, TransactionRequest as V1TransactionRequest};

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

#[post("/", data = "<payload>")]
pub async fn create_transaction(
    pool: &State<PgPool>,
    user: CurrentUser,
    payload: Json<CreateTransactionRequest>,
) -> Result<(Status, Json<TransactionResponse>), AppError> {
    // Extract fields and validate
    let (date, description, amount, from_account_id, category_id, vendor_id, to_account_id) = match &*payload {
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

    let v1_request = V1TransactionRequest {
        amount,
        description: description.clone(),
        occurred_at: date.0,
        category_id,
        from_account_id,
        to_account_id,
        vendor_id,
    };

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let tx = repo.create_transaction(&v1_request, &user.id).await?;

    Ok((Status::Created, Json(to_v2_response(&tx))))
}
