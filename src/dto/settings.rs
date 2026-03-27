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
    pub avatar: String,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProfileRequest {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(regex(path = *ISO_4217_REGEX))]
    pub currency: String,
    // max 64 chars — matches the maxLength constraint in openapi/schemas/Settings.yaml
    #[validate(length(max = 64))]
    pub avatar: String,
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

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum ColorTheme {
    #[default]
    #[serde(rename = "nebula")]
    Nebula,
    #[serde(rename = "sunrise")]
    Sunrise,
    #[serde(rename = "sage_stone")]
    SageStone,
    #[serde(rename = "deep_ocean")]
    DeepOcean,
    #[serde(rename = "warm_rose")]
    WarmRose,
    #[serde(rename = "moonlit")]
    Moonlit,
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardLayout {
    pub widget_order: Vec<String>,
    pub hidden_widgets: Vec<String>,
    /// Account IDs to display as individual cards on the dashboard.
    /// If omitted (empty), all active accounts are shown.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visible_account_ids: Vec<Uuid>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesResponse {
    pub theme: Theme,
    pub date_format: DateFormat,
    pub number_format: NumberFormat,
    pub language: String,
    pub compact_mode: bool,
    pub dashboard_layout: DashboardLayout,
    pub color_theme: ColorTheme,
}

#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePreferencesRequest {
    pub theme: Theme,
    pub date_format: DateFormat,
    pub number_format: NumberFormat,
    #[validate(regex(path = *BCP_47_REGEX))]
    pub language: String,
    #[serde(default)]
    pub compact_mode: bool,
    #[serde(default)]
    pub dashboard_layout: DashboardLayout,
    #[serde(default)]
    pub color_theme: ColorTheme,
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
