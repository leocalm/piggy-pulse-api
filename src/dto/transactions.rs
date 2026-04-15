#![allow(dead_code)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::dto::common::Date;

// ─────────────────────────────────────────────────────────────────────
// Encrypted transaction response
// ─────────────────────────────────────────────────────────────────────
//
// Under the encryption-at-rest design the server cannot decrypt the
// transaction's amount or description to join them with plaintext entity
// metadata. Instead the server returns the raw ciphertext envelopes plus
// plaintext foreign-key ids. The client decrypts locally and joins
// against its cached account/category/vendor lists.

/// Server response shape for a newly-created or newly-corrected
/// transaction. All numeric amounts and free-text descriptions are
/// base64-encoded AES-GCM envelopes.
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EncryptedTransactionResponse {
    pub id: Uuid,
    pub seq: i64,
    pub date: Date,
    /// ISO-8601 timestamp of the first insert for this logical id.
    /// Stable across corrections.
    pub first_created_at: String,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
    /// Base64-encoded AES-256-GCM envelope (12-byte nonce + ciphertext + 16-byte tag).
    pub amount_enc: String,
    /// Base64-encoded AES-256-GCM envelope.
    pub description_enc: String,
}

impl From<crate::database::transaction::LedgerInsertResult> for EncryptedTransactionResponse {
    fn from(r: crate::database::transaction::LedgerInsertResult) -> Self {
        Self {
            id: r.id,
            seq: r.seq,
            date: Date(r.occurred_at),
            first_created_at: r.first_created_at.to_rfc3339(),
            from_account_id: r.from_account_id,
            to_account_id: r.to_account_id,
            category_id: r.category_id,
            vendor_id: r.vendor_id,
            amount_enc: BASE64.encode(&r.amount_enc),
            description_enc: BASE64.encode(&r.description_enc),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Requests
// ─────────────────────────────────────────────────────────────────────
//
// Request bodies still carry plaintext amount + description inside the
// authenticated session. The server encrypts on write with the session
// DEK before touching the database.

#[derive(Deserialize, Debug)]
#[serde(tag = "transactionType")]
pub enum CreateTransactionRequest {
    Regular {
        date: Date,
        description: String,
        amount: i64,
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
        amount: i64,
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
