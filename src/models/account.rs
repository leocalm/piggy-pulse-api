use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub enum AccountType {
    Checking,
    Savings,
    CreditCard,
}

#[derive(Serialize, Debug)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: Currency,
    pub balance: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct AccountRequest {
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: String,
    pub balance: i64,
}

#[derive(Serialize, Debug)]
pub struct AccountResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: String,
    pub balance: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Currency {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub currency: String,
    pub decimal_places: usize,
}
