#![allow(unused)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ===== Currency =====

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SymbolPosition {
    #[default]
    Before,
    After,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyResponse {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub code: String,
    pub decimal_places: i32,
    pub symbol_position: SymbolPosition,
}

pub type CurrencyListResponse = Vec<CurrencyResponse>;

// ===== Onboarding =====

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStatus {
    NotStarted,
    InProgress,
    Completed,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OnboardingStep {
    Period,
    Accounts,
    Categories,
    Summary,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingStatusResponse {
    pub status: OnboardingStatus,
    pub current_step: Option<OnboardingStep>,
}

// ===== Unlock =====

#[derive(Serialize, Debug)]
pub struct UnlockResponse {
    pub message: String,
}
