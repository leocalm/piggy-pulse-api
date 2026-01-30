use rocket::serde::{Deserialize, Serialize};

/// Pagination parameters for list queries
/// Both page and limit are optional to maintain backwards compatibility
/// When not provided, returns all results (no pagination)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PaginationParams {
    /// Page number (1-indexed). When None, returns all results.
    pub page: Option<i64>,
    /// Number of items per page. When None, uses default or returns all.
    pub limit: Option<i64>,
}

impl PaginationParams {
    /// Default limit when limit is provided but not specified
    pub const DEFAULT_LIMIT: i64 = 50;
    /// Maximum allowed limit
    pub const MAX_LIMIT: i64 = 200;

    /// Calculate the SQL OFFSET value based on page and limit
    pub fn offset(&self) -> Option<i64> {
        match (self.page, self.limit) {
            (Some(page), Some(limit)) => Some((page - 1) * limit),
            (Some(page), None) => Some((page - 1) * Self::DEFAULT_LIMIT),
            _ => None,
        }
    }

    /// Get the effective limit, applying defaults and max constraints
    pub fn effective_limit(&self) -> Option<i64> {
        match self.limit {
            Some(limit) => Some(limit.min(Self::MAX_LIMIT)),
            None if self.page.is_some() => Some(Self::DEFAULT_LIMIT),
            None => None, // No pagination when both are None
        }
    }

    /// Check if pagination is requested
    pub fn is_paginated(&self) -> bool {
        self.page.is_some() || self.limit.is_some()
    }
}

/// Paginated response wrapper with metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct PaginatedResponse<T> {
    /// The actual data items
    pub data: Vec<T>,
    /// Current page number (1-indexed)
    pub page: i64,
    /// Number of items per page
    pub limit: i64,
    /// Total number of items across all pages
    pub total_items: i64,
    /// Total number of pages
    pub total_pages: i64,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: i64, limit: i64, total_items: i64) -> Self {
        let total_pages = if limit > 0 { (total_items + limit - 1) / limit } else { 1 };

        Self { data, page, limit, total_items, total_pages }
    }
}
