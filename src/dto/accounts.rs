use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use uuid::Uuid;
use validator::Validate;
use crate::dto::misc::CurrencyResponse;
use crate::models::currency::Currency;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountType {
    #[default]
    Checking,
    Savings,
    CreditCard,
    Wallet,
    Allowance,
}

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AccountStatus {
    #[default]
    Active,
    Inactive,
}

#[derive(Serialize, Debug, JsonSchema)]
#[serde(tag = "accountType", rename_all = "camelCase")]
pub enum AccountResponse {
    Checking(AccountResponseBase),
    Savings(AccountResponseBase),
    CreditCard(AccountResponseBaseWithSpendLimit),
    Wallet(AccountResponseBase),
    Allowance(AccountResponseBaseWithSpendLimit),
}

#[derive(Serialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponseBase {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub status: AccountStatus,
    pub currency: CurrencyResponse,
}

#[derive(Serialize, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponseBaseWithSpendLimit {
    #[serde(flatten)]
    pub base: AccountResponseBase,
    pub spend_limit: Option<i64>,
}


#[derive(Deserialize, Debug, JsonSchema)]
#[serde(tag = "accountType", rename_all = "camelCase")]
pub enum CreateAccountRequest {
    Checking(AccountRequestBase),
    Savings(AccountRequestBase),
    CreditCard(AccountRequestBaseWithSpendLimit),
    Wallet(AccountRequestBase),
    Allowance(AccountRequestBaseWithSpendLimit),
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountRequestBase {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(equal = 7))]
    pub color: String,
    pub initial_balance: i64,
    pub currency_id: Uuid,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AccountRequestBaseWithSpendLimit {
    #[serde(flatten)]
    #[validate(nested)]
    pub base: AccountRequestBase,
    pub spend_limit: Option<i64>,
}

pub type UpdateAccountRequest = CreateAccountRequest;
