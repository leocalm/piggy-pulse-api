#![allow(dead_code)]

use serde::Serialize;

// ===== Health =====

#[derive(Serialize, Debug)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
}
