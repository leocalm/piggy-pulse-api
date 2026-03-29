#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dto::categories::CategoryType;
use crate::dto::common::{Date, PaginatedResponse};

// ===== Embedded refs (response) =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountRef {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryRef {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    #[serde(rename = "type")]
    pub category_type: CategoryType,
}

pub use crate::dto::common::VendorMinimal as VendorRef;

// ===== TransactionResponse =====

/// Discriminated variant flattened into TransactionResponse.
/// The `transactionType` tag acts as the discriminator.
#[derive(Serialize, Debug)]
#[serde(tag = "transactionType", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TransactionKind {
    Regular { to_account: Option<AccountRef> },
    Transfer { to_account: AccountRef },
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResponse {
    pub id: Uuid,
    pub date: Date,
    pub description: String,
    pub amount: i64, // INVARIANT: amount >= 0, validated in route layer
    pub from_account: AccountRef,
    pub category: CategoryRef,
    pub vendor: Option<VendorRef>,
    #[serde(flatten)]
    pub kind: TransactionKind,
}

pub type TransactionListResponse = PaginatedResponse<TransactionResponse>;

// ===== Requests =====

/// Top-level internally-tagged enum avoids the serde flatten+tag limitation on the Deserialize path.
/// validator 0.20 does not support #[derive(Validate)] on enums; range/length validation for
/// shared fields must be enforced explicitly by the route layer.
#[derive(Deserialize, Debug)]
#[serde(tag = "transactionType")]
pub enum CreateTransactionRequest {
    Regular {
        date: Date,
        description: String,
        amount: i64, // INVARIANT: amount >= 0, validated in route layer
        #[serde(rename = "fromAccountId")]
        from_account_id: Uuid,
        #[serde(rename = "categoryId")]
        category_id: Uuid,
        #[serde(rename = "vendorId")]
        vendor_id: Option<Uuid>,
    },
    Transfer {
        date: Date,
        description: String,
        amount: i64, // INVARIANT: amount >= 0, validated in route layer
        #[serde(rename = "fromAccountId")]
        from_account_id: Uuid,
        #[serde(rename = "categoryId")]
        category_id: Uuid,
        #[serde(rename = "vendorId")]
        vendor_id: Option<Uuid>,
        #[serde(rename = "toAccountId")]
        to_account_id: Uuid,
    },
}

pub type UpdateTransactionRequest = CreateTransactionRequest;

// ===== Stats =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStatsResponse {
    pub total_inflows: i64,
    pub total_outflows: i64,
    pub net_amount: i64,
    pub transaction_count: i64,
}
