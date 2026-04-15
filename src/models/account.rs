use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default, sqlx::Type)]
#[sqlx(type_name = "text", rename_all = "PascalCase")]
pub enum AccountType {
    #[default]
    Checking,
    Savings,
    CreditCard,
    Wallet,
    Allowance,
}

/// Raw account row as read from postgres. All monetary and label
/// fields are AES-GCM envelopes; structural fields stay plaintext so
/// the server can still route by account_type, resolve FKs, and
/// enforce allowance/credit-card schedule semantics.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub account_type: AccountType,
    pub currency_id: Uuid,
    pub is_archived: bool,
    pub name_enc: Vec<u8>,
    pub color_enc: Vec<u8>,
    pub icon_enc: Vec<u8>,
    pub current_balance_enc: Vec<u8>,
    pub spend_limit_enc: Option<Vec<u8>>,
    pub next_transfer_amount_enc: Option<Vec<u8>>,
    pub top_up_amount_enc: Option<Vec<u8>>,
    pub top_up_cycle: Option<String>,
    pub top_up_day: Option<i32>,
    pub statement_close_day: Option<i32>,
    pub payment_due_day: Option<i32>,
}
