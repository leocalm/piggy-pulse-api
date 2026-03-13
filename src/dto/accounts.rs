#![allow(unused)]

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

static HEX_COLOR_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").unwrap());

use crate::dto::common::{Date, PaginatedResponse};
use crate::dto::misc::CurrencyResponse;

// ===== Enums =====

#[derive(Serialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
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
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AccountResponse {
    Checking(AccountResponseBase),
    Savings(AccountResponseBase),
    CreditCard(AccountResponseBaseWithSpendLimit),
    Wallet(AccountResponseBase),
    Allowance(AccountResponseBaseWithSpendLimit),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponseBase {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub status: AccountStatus,
    pub initial_balance: i64,
    pub currency: CurrencyResponse,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountResponseBaseWithSpendLimit {
    #[serde(flatten)]
    pub base: AccountResponseBase,
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
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CreateAccountRequest {
    Checking(AccountRequestBase),
    Savings(AccountRequestBase),
    CreditCard(AccountRequestBaseWithSpendLimit),
    Wallet(AccountRequestBase),
    Allowance(AccountRequestBaseWithSpendLimit),
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AccountRequestBase {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(regex(path = *HEX_COLOR_REGEX))]
    pub color: String,
    pub initial_balance: i64,
    pub currency_id: Uuid,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AccountRequestBaseWithSpendLimit {
    #[serde(flatten)]
    #[validate(nested)]
    pub base: AccountRequestBase,
    pub spend_limit: Option<i64>,
}

pub type UpdateAccountRequest = CreateAccountRequest;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AdjustBalanceRequest {
    pub new_balance: i64,
}
