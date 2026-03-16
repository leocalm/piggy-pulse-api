#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::{Date, HEX_COLOR_REGEX, PaginatedResponse};
use crate::dto::misc::CurrencyResponse;

// ===== Enums =====

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
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

// ===== Account Response =====

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum AccountResponse {
    Checking(AccountResponseFields),
    Savings(AccountResponseFields),
    CreditCard(AccountResponseFields),
    Wallet(AccountResponseFields),
    Allowance(AccountResponseFields),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponseFields {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub status: AccountStatus,
    pub initial_balance: i64,
    pub currency: CurrencyResponse,
    pub spend_limit: Option<i64>,
}

pub type AccountListResponse = PaginatedResponse<AccountResponse>;

// ===== Account Option =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountOptionResponse {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

pub type AccountOptionListResponse = Vec<AccountOptionResponse>;

// ===== Account Summary =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummaryResponse {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub account_type: AccountType,
    pub color: String,
    pub status: AccountStatus,
    pub current_balance: i64,
    pub net_change_this_period: i64,
    pub next_transfer: Option<Date>,
    pub balance_after_next_transfer: Option<i64>,
    pub number_of_transactions: i64,
}

pub type AccountSummaryListResponse = PaginatedResponse<AccountSummaryResponse>;

// ===== Account Details =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LargestOutflow {
    pub category_name: String,
    pub value: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StabilityContext {
    pub periods_on_target: i64,
    pub average_closing_balance: i64,
    pub highest_closing_balance: i64,
    pub lowest_closing_balance: i64,
    pub largest_single_outflow: Option<LargestOutflow>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CategoryBreakdownItem {
    pub category_id: Uuid,
    pub category_name: String,
    pub value: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TransactionBreakdownItem {
    pub date: Date,
    pub description: String,
    pub category_name: String,
    pub amount: i64,
    pub balance: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountDetailsResponse {
    #[serde(flatten)]
    pub base: AccountSummaryResponse,
    pub inflow: i64,
    pub outflow: i64,
    pub stability_context: StabilityContext,
    pub categories_breakdown: Vec<CategoryBreakdownItem>,
    pub transactions_breakdown: Vec<TransactionBreakdownItem>,
}

// ===== Account Balance History =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountBalanceHistoryPoint {
    pub date: Date,
    pub balance: i64,
    pub transaction_count: i64,
}

pub type AccountBalanceHistoryResponse = Vec<AccountBalanceHistoryPoint>;

// ===== Account Requests =====

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum CreateAccountRequest {
    Checking(AccountRequestFields),
    Savings(AccountRequestFields),
    CreditCard(AccountRequestFields),
    Wallet(AccountRequestFields),
    Allowance(AccountRequestFields),
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AccountRequestFields {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(regex(path = *HEX_COLOR_REGEX))]
    pub color: String,
    pub initial_balance: i64,
    pub currency_id: Uuid,
    pub spend_limit: Option<i64>,
}

pub type UpdateAccountRequest = CreateAccountRequest;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AdjustBalanceRequest {
    pub new_balance: i64,
}

// ===== Conversions from domain models to V2 DTOs =====

use crate::dto::misc::SymbolPosition as DtoSymbolPosition;
use crate::models::account::{Account, AccountType as ModelAccountType};
use crate::models::currency::SymbolPosition as ModelSymbolPosition;

impl CreateAccountRequest {
    pub fn fields(&self) -> &AccountRequestFields {
        match self {
            Self::Checking(f) | Self::Savings(f) | Self::CreditCard(f) | Self::Wallet(f) | Self::Allowance(f) => f,
        }
    }

    pub fn model_account_type(&self) -> ModelAccountType {
        match self {
            Self::Checking(_) => ModelAccountType::Checking,
            Self::Savings(_) => ModelAccountType::Savings,
            Self::CreditCard(_) => ModelAccountType::CreditCard,
            Self::Wallet(_) => ModelAccountType::Wallet,
            Self::Allowance(_) => ModelAccountType::Allowance,
        }
    }
}

fn convert_symbol_position(pos: ModelSymbolPosition) -> DtoSymbolPosition {
    match pos {
        ModelSymbolPosition::Before => DtoSymbolPosition::Before,
        ModelSymbolPosition::After => DtoSymbolPosition::After,
    }
}

fn convert_account_type(t: ModelAccountType) -> AccountType {
    match t {
        ModelAccountType::Checking => AccountType::Checking,
        ModelAccountType::Savings => AccountType::Savings,
        ModelAccountType::CreditCard => AccountType::CreditCard,
        ModelAccountType::Wallet => AccountType::Wallet,
        ModelAccountType::Allowance => AccountType::Allowance,
    }
}

impl From<&Account> for AccountResponse {
    fn from(account: &Account) -> Self {
        let fields = AccountResponseFields {
            id: account.id,
            name: account.name.clone(),
            color: account.color.clone(),
            status: if account.is_archived {
                AccountStatus::Inactive
            } else {
                AccountStatus::Active
            },
            initial_balance: account.balance,
            currency: CurrencyResponse {
                id: account.currency.id,
                name: account.currency.name.clone(),
                symbol: account.currency.symbol.clone(),
                code: account.currency.currency.clone(),
                decimal_places: account.currency.decimal_places,
                symbol_position: convert_symbol_position(account.currency.symbol_position),
            },
            spend_limit: account.spend_limit.map(|s| s as i64),
        };
        match account.account_type {
            ModelAccountType::Checking => Self::Checking(fields),
            ModelAccountType::Savings => Self::Savings(fields),
            ModelAccountType::CreditCard => Self::CreditCard(fields),
            ModelAccountType::Wallet => Self::Wallet(fields),
            ModelAccountType::Allowance => Self::Allowance(fields),
        }
    }
}

impl From<&Account> for AccountSummaryResponse {
    fn from(account: &Account) -> Self {
        Self {
            id: account.id,
            name: account.name.clone(),
            account_type: convert_account_type(account.account_type),
            color: account.color.clone(),
            status: if account.is_archived {
                AccountStatus::Inactive
            } else {
                AccountStatus::Active
            },
            current_balance: account.balance,
            net_change_this_period: 0,
            next_transfer: None,
            balance_after_next_transfer: None,
            number_of_transactions: 0,
        }
    }
}
