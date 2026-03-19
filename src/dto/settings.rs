use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::dto::common::{BCP_47_REGEX, ISO_4217_REGEX};

// ===== Profile =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProfileResponse {
    pub name: String,
    pub currency: String,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(regex(path = *ISO_4217_REGEX))]
    pub currency: String,
}

// ===== Preferences =====

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Theme {
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "dark")]
    Dark,
    #[serde(rename = "system")]
    System,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum DateFormat {
    #[serde(rename = "DD/MM/YYYY")]
    DdMmYyyy,
    #[serde(rename = "MM/DD/YYYY")]
    MmDdYyyy,
    #[serde(rename = "YYYY-MM-DD")]
    YyyyMmDd,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq)]
pub enum NumberFormat {
    #[serde(rename = "1,234.56")]
    CommaPeriod,
    #[serde(rename = "1.234,56")]
    PeriodComma,
    #[serde(rename = "1 234,56")]
    SpaceComma,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesResponse {
    pub theme: Theme,
    pub date_format: DateFormat,
    pub number_format: NumberFormat,
    pub language: String,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferencesRequest {
    pub theme: Theme,
    pub date_format: DateFormat,
    pub number_format: NumberFormat,
    #[validate(regex(path = *BCP_47_REGEX))]
    pub language: String,
}

// ===== Sessions =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub is_current: bool,
}

pub type SessionListResponse = Vec<SessionResponse>;

// ===== Account Actions =====

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeleteAccountRequest {
    #[validate(length(min = 1))]
    pub password: String,
}

pub type ResetStructureRequest = DeleteAccountRequest;
