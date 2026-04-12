use chrono::{DateTime, Utc};
use rocket::serde::Serialize;
use uuid::Uuid;

// ── Existing general settings ─────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone, sqlx::FromRow)]
pub struct Settings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub theme: String,
    pub language: String,
    pub default_currency_id: Option<Uuid>,
    pub budget_stability_tolerance_basis_points: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
