#![allow(unused)]

use std::sync::LazyLock;

use chrono::NaiveDate;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub static HEX_COLOR_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#[0-9A-Fa-f]{6}$").unwrap());
pub static ISO_4217_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[A-Z]{3}$").unwrap());
pub static BCP_47_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-zA-Z]{2,3}(-[a-zA-Z0-9]{2,8})*$").unwrap());

// ===== Date =====

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Date(pub NaiveDate);

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.format("%Y-%m-%d").to_string())
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse::<NaiveDate>().map(Date).map_err(serde::de::Error::custom)
    }
}

// ===== Pagination =====

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total_count: i64,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

// ===== Error Responses =====

#[derive(Serialize, Deserialize, Debug)]
pub struct ErrorResponse {
    pub message: String,
    pub code: Option<String>,
}

pub type UnauthorizedErrorResponse = ErrorResponse;
pub type BadRequestErrorResponse = ErrorResponse;
pub type NotFoundErrorResponse = ErrorResponse;
pub type InternalServerErrorResponse = ErrorResponse;
pub type ServiceUnavailableErrorResponse = ErrorResponse;
pub type ConflictErrorResponse = ErrorResponse;
