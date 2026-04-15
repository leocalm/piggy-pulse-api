use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dto::common::PaginatedResponse;

// ===== Account type / status =====

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    #[default]
    Checking,
    Savings,
    CreditCard,
    Wallet,
    Allowance,
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountStatus {
    #[default]
    Active,
    Inactive,
}

// ===== Encrypted response =====
//
// Monetary and label fields are returned as base64-encoded AES-GCM
// ciphertext; the client decrypts with its DEK. Structural fields
// (type, status, currency_id, allowance/credit-card schedule) stay
// plaintext so the client can bucket accounts without decrypting.

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedAccountResponse {
    pub id: Uuid,
    pub account_type: AccountType,
    pub status: AccountStatus,
    pub currency_id: Uuid,
    pub name_enc: String,
    pub color_enc: String,
    pub icon_enc: String,
    pub current_balance_enc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spend_limit_enc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_transfer_amount_enc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_up_amount_enc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_up_cycle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_up_day: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement_close_day: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_due_day: Option<i32>,
}

pub type AccountListResponse = PaginatedResponse<EncryptedAccountResponse>;

// ===== Lightweight option (used by selectors) =====
//
// The option endpoint also returns only ciphertext for the label. The
// client decrypts and renders. No unencrypted "display name" leaks.

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountOptionResponse {
    pub id: Uuid,
    pub account_type: AccountType,
    pub name_enc: String,
    pub color_enc: String,
}

pub type AccountOptionListResponse = Vec<AccountOptionResponse>;

// ===== Requests =====
//
// Create/update carry the plaintext fields: the server holds the DEK
// for the session and encrypts before writing. All monetary amounts
// are integer cents.

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    pub account_type: AccountType,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub currency_id: Uuid,
    pub initial_balance: i64,
    pub spend_limit: Option<i64>,
    pub next_transfer_amount: Option<i64>,
    pub top_up_amount: Option<i64>,
    pub top_up_cycle: Option<String>,
    pub top_up_day: Option<i32>,
    pub statement_close_day: Option<i32>,
    pub payment_due_day: Option<i32>,
}

pub type UpdateAccountRequest = CreateAccountRequest;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AdjustBalanceRequest {
    pub new_balance: i64,
}

pub fn b64(bytes: &[u8]) -> String {
    B64.encode(bytes)
}
