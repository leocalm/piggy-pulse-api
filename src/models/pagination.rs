use chrono::NaiveDate;
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Allowed values for the transaction direction filter.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(crate = "rocket::serde")]
pub enum TransactionDirection {
    Incoming,
    Outgoing,
    Transfer,
}

impl TransactionDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionDirection::Incoming => "Incoming",
            TransactionDirection::Outgoing => "Outgoing",
            TransactionDirection::Transfer => "Transfer",
        }
    }
}

/// Cursor-based pagination parameters.
/// `cursor` is the `id` of the last item seen on the previous page.
/// When `None`, the query starts from the beginning of the result set.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct CursorParams {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

impl CursorParams {
    pub const DEFAULT_LIMIT: i64 = 200;
    pub const MAX_LIMIT: i64 = 200;

    /// The number of rows to actually fetch from the database.
    /// One extra row is requested so we can determine whether a next page exists.
    pub fn fetch_limit(&self) -> i64 {
        self.effective_limit() + 1
    }

    /// The limit that will be reported back to the caller (capped at MAX_LIMIT).
    pub fn effective_limit(&self) -> i64 {
        self.limit.unwrap_or(Self::DEFAULT_LIMIT).min(Self::MAX_LIMIT)
    }
}

/// Optional filter parameters for transaction queries.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TransactionFilters {
    pub account_ids: Vec<Uuid>,
    pub category_ids: Vec<Uuid>,
    pub direction: Option<TransactionDirection>,
    pub vendor_ids: Vec<Uuid>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
    /// Free-text search: matches against description (ILIKE) or amount (LIKE).
    pub search: Option<String>,
}

impl TransactionFilters {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.account_ids.is_empty()
            && self.category_ids.is_empty()
            && self.direction.is_none()
            && self.vendor_ids.is_empty()
            && self.date_from.is_none()
            && self.date_to.is_none()
            && self.search.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effective_limit_default() {
        let params = CursorParams { cursor: None, limit: None };
        assert_eq!(params.effective_limit(), 200);
        assert_eq!(params.fetch_limit(), 201);
    }

    #[test]
    fn test_effective_limit_capped() {
        let params = CursorParams {
            cursor: None,
            limit: Some(500),
        };
        assert_eq!(params.effective_limit(), 200);
        assert_eq!(params.fetch_limit(), 201);
    }

    #[test]
    fn test_effective_limit_explicit() {
        let params = CursorParams { cursor: None, limit: Some(10) };
        assert_eq!(params.effective_limit(), 10);
        assert_eq!(params.fetch_limit(), 11);
    }

    #[test]
    fn test_transaction_filters_is_empty_default() {
        let f = TransactionFilters::default();
        assert!(f.is_empty());
    }

    #[test]
    fn test_transaction_filters_is_empty_with_direction() {
        let f = TransactionFilters {
            direction: Some(TransactionDirection::Incoming),
            ..Default::default()
        };
        assert!(!f.is_empty());
    }
}
