use crate::models::account::{Account, AccountResponse};
use crate::models::category::{Category, CategoryResponse};
use crate::models::vendor::{Vendor, VendorResponse};
use chrono::NaiveDate;
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum TransactionType {
    Incoming,
    Outgoing,
    Transfer,
}

#[derive(Serialize, Debug, Clone)]
pub struct Transaction {
    pub id: Uuid,
    pub amount: i32,
    pub description: String,
    pub occurred_at: NaiveDate,
    pub transaction_type: TransactionType,
    pub category: Category,
    pub from_account: Account,
    pub to_account: Option<Account>,
    pub vendor: Option<Vendor>,
}

#[derive(Deserialize, Debug)]
pub struct TransactionRequest {
    pub amount: i32,
    pub description: String,
    pub occurred_at: NaiveDate,
    pub transaction_type: TransactionType,
    pub category_id: Uuid,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
}

#[derive(Serialize, Debug)]
pub struct TransactionResponse {
    pub id: Uuid,
    pub amount: i32,
    pub description: String,
    pub occurred_at: NaiveDate,
    pub transaction_type: TransactionType,
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
            transaction_type: transaction.transaction_type,
            category: CategoryResponse::from(&transaction.category),
            from_account: AccountResponse::from(&transaction.from_account),
            to_account: transaction.to_account.as_ref().map(AccountResponse::from),
            vendor: transaction.vendor.as_ref().map(VendorResponse::from),
        }
    }
}

// Helper method for TransactionRequest to map to DB enum/text value
pub trait TransactionRequestDbExt {
    fn transaction_type_to_db(&self) -> String;
}

impl TransactionRequestDbExt for TransactionRequest {
    fn transaction_type_to_db(&self) -> String {
        match self.transaction_type {
            TransactionType::Incoming => "Incoming".to_string(),
            TransactionType::Outgoing => "Outgoing".to_string(),
            TransactionType::Transfer => "Transfer".to_string(),
        }
    }
}
