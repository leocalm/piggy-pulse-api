use crate::models::account::{Account, AccountResponse};
use crate::models::category::{Category, CategoryResponse};
use crate::models::vendor::{Vendor, VendorResponse};
use chrono::NaiveDate;
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Default)]
pub struct Transaction {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: i64,
    pub description: String,
    pub occurred_at: NaiveDate,
    pub category: Category,
    pub from_account: Account,
    pub to_account: Option<Account>,
    pub vendor: Option<Vendor>,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct TransactionRequest {
    #[validate(range(min = 0))]
    pub amount: i64,
    #[validate(length(min = 3))]
    pub description: String,
    pub occurred_at: NaiveDate,
    pub category_id: Uuid,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub amount: i64,
    pub description: String,
    pub occurred_at: NaiveDate,
    pub category: CategoryResponse,
    pub from_account: AccountResponse,
    pub to_account: Option<AccountResponse>,
    pub vendor: Option<VendorResponse>,
}

impl From<&Transaction> for TransactionResponse {
    fn from(transaction: &Transaction) -> Self {
        Self {
            id: transaction.id,
            amount: transaction.amount,
            description: transaction.description.clone(),
            occurred_at: transaction.occurred_at,
            category: CategoryResponse::from(&transaction.category),
            from_account: AccountResponse::from(&transaction.from_account),
            to_account: transaction.to_account.as_ref().map(AccountResponse::from),
            vendor: transaction.vendor.as_ref().map(VendorResponse::from),
        }
    }
}
