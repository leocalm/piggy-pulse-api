use crate::error::app_error::AppError;
use chrono::NaiveDate;
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;

/// Cursor-based pagination parameters.
/// `cursor` is the `id` of the last item seen on the previous page.
/// When `None`, the query starts from the beginning of the result set.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(crate = "rocket::serde")]
pub struct CursorParams {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

impl CursorParams {
    pub const DEFAULT_LIMIT: i64 = 200;
    pub const MAX_LIMIT: i64 = 200;

    /// Parse a cursor from the raw query-string value (which Rocket gives us as `Option<String>`).
    #[allow(clippy::result_large_err)] // AppError is the project-wide error type; boxing it here would be inconsistent
    pub fn from_query(cursor: Option<String>, limit: Option<i64>) -> Result<Self, AppError> {
        let cursor = match cursor {
            Some(s) if s.is_empty() => None,
            Some(s) => Some(Uuid::parse_str(&s).map_err(|e| AppError::uuid("Invalid cursor", e))?),
            None => None,
        };
        Ok(Self { cursor, limit })
    }

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

/// Cursor-paginated response wrapper.
/// `next_cursor` is `Some(id)` when there is another page, `None` on the last page.
#[derive(Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[serde(crate = "rocket::serde")]
pub struct CursorPaginatedResponse<T> {
    pub data: Vec<T>,
    pub next_cursor: Option<Uuid>,
}

impl<T> CursorPaginatedResponse<T> {
    /// Build the response from a raw result set that was fetched with `limit + 1` rows.
    /// If `rows` contains more than `limit` items the extra trailing row is dropped and
    /// `next_cursor` is set to the `id` of the last item that *is* included.
    pub fn from_rows<F>(mut rows: Vec<T>, limit: i64, id_of: F) -> Self
    where
        F: Fn(&T) -> Uuid,
    {
        if rows.len() as i64 > limit {
            rows.truncate(limit as usize);
            let next_cursor = Some(id_of(rows.last().unwrap()));
            Self { data: rows, next_cursor }
        } else {
            Self { data: rows, next_cursor: None }
        }
    }
}

/// Optional filter parameters for transaction queries.
#[derive(Debug, Clone, Default, Deserialize, Serialize, JsonSchema)]
#[serde(crate = "rocket::serde")]
pub struct TransactionFilters {
    pub account_ids: Vec<Uuid>,
    pub category_ids: Vec<Uuid>,
    pub direction: Option<String>,   // "Incoming" | "Outgoing" | "Transfer"
    pub vendor_ids: Vec<Uuid>,
    pub date_from: Option<NaiveDate>,
    pub date_to: Option<NaiveDate>,
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
    fn test_from_rows_no_next_page() {
        let ids: Vec<Uuid> = (0..3).map(|_| Uuid::new_v4()).collect();
        let resp = CursorPaginatedResponse::from_rows(ids.clone(), 5, |id| *id);
        assert_eq!(resp.data.len(), 3);
        assert!(resp.next_cursor.is_none());
    }

    #[test]
    fn test_from_rows_has_next_page() {
        let ids: Vec<Uuid> = (0..6).map(|_| Uuid::new_v4()).collect();
        let resp = CursorPaginatedResponse::from_rows(ids.clone(), 5, |id| *id);
        assert_eq!(resp.data.len(), 5);
        assert_eq!(resp.next_cursor, Some(ids[4]));
    }

    #[test]
    fn test_from_rows_exact_page_boundary() {
        // Exactly limit rows returned → no next page (the extra row wasn't there)
        let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();
        let resp = CursorPaginatedResponse::from_rows(ids.clone(), 5, |id| *id);
        assert_eq!(resp.data.len(), 5);
        assert!(resp.next_cursor.is_none());
    }

    #[test]
    fn test_transaction_filters_is_empty_default() {
        let f = TransactionFilters::default();
        assert!(f.is_empty());
    }

    #[test]
    fn test_transaction_filters_is_empty_with_direction() {
        let f = TransactionFilters {
            direction: Some("Incoming".to_string()),
            ..Default::default()
        };
        assert!(!f.is_empty());
    }
}
