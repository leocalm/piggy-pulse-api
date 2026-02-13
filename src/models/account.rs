use crate::models::currency::{Currency, CurrencyResponse};
use crate::models::dashboard::BudgetPerDayResponse;
use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default, JsonSchema)]
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
    pub user_id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: Currency,
    pub balance: i64,
    pub created_at: DateTime<Utc>,
    pub spend_limit: Option<i32>,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct AccountRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(min = 3))]
    pub color: String,
    #[validate(length(min = 3))]
    pub icon: String,
    pub account_type: AccountType,
    #[validate(range(min = 0))]
    pub balance: i64,
    pub spend_limit: Option<i32>,
}

#[derive(Serialize, Debug, JsonSchema)]
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
                symbol_position: account.currency.symbol_position,
            },
            balance: account.balance,
            spend_limit: account.spend_limit,
        }
    }
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct AccountListResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub icon: String,
    pub account_type: AccountType,
    pub currency: CurrencyResponse,
    pub balance: i64,
    pub spend_limit: Option<i32>,
    pub balance_per_day: Vec<BudgetPerDayResponse>,
    pub balance_change_this_period: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Clone)]
pub struct AccountWithMetrics {
    pub account: Account,
    pub current_balance: i64,
    pub balance_change_this_period: i64,
    pub transaction_count: i64,
}

#[derive(Debug, Clone)]
pub struct AccountBalancePerDay {
    pub account_id: Uuid,
    pub account_name: String,
    pub date: String,
    pub balance: i64,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct AccountsSummaryResponse {
    pub total_net_worth: i64,
    pub total_assets: i64,
    pub total_liabilities: i64,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct AccountOptionResponse {
    pub id: Uuid,
    pub name: String,
    pub icon: String,
}
