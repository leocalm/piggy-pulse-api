use crate::models::currency::{Currency, CurrencyResponse};
use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum AccountType {
    #[default]
    Checking,
    Savings,
    CreditCard,
    Wallet,
    Allowance,
}

#[derive(Serialize, Debug, Clone, Default)]
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

#[derive(Deserialize, Debug, Validate)]
pub struct AccountRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(min = 3))]
    pub color: String,
    #[validate(length(min = 3))]
    pub icon: String,
    pub account_type: AccountType,
    #[validate(length(equal = 3))]
    pub currency: String,
    #[validate(range(min = 0))]
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
