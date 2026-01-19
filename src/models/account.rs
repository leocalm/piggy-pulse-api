use crate::models::currency::{Currency, CurrencyResponse};
use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum AccountType {
    Checking,
    Savings,
    CreditCard,
    Wallet,
    Allowance,
}

#[derive(Serialize, Debug, Clone)]
pub struct Account {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: Currency,
    pub balance: i64,
    pub created_at: DateTime<Utc>,
    pub spend_limit: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct AccountRequest {
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: String,
    pub balance: i64,
    pub spend_limit: Option<i32>,
}

#[derive(Serialize, Debug)]
pub struct AccountResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: CurrencyResponse,
    pub balance: i64,
    pub spend_limit: Option<i32>,
}

impl From<&Account> for AccountResponse {
    fn from(account: &Account) -> Self {
        AccountResponse {
            id: account.id,
            name: account.name.clone(),
            color: account.color.clone(),
            icon: account.icon.clone(),
            account_type: account.account_type,
            currency: CurrencyResponse {
                id: account.currency.id,
                name: account.currency.name.clone(),
                symbol: account.currency.symbol.clone(),
                currency: account.currency.currency.clone(),
                decimal_places: account.currency.decimal_places,
            },
            balance: account.balance,
            spend_limit: account.spend_limit,
        }
    }
}
