use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

// ── Card size enum ───────────────────────────────────────────────────────────

#[derive(sqlx::Type, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[sqlx(type_name = "card_size", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum CardSize {
    Half,
    Full,
}

// ── Card type constants & helpers ────────────────────────────────────────────

pub const GLOBAL_CARD_TYPES: &[&str] = &[
    "current_period",
    "budget_stability",
    "net_position",
    "recent_transactions",
    "top_categories",
    "budget_per_day",
    "monthly_burn_rate",
    "remaining_budget",
    "month_progress",
    "balance_over_time",
];

pub const ENTITY_CARD_TYPES: &[&str] = &["account_summary", "category_breakdown", "vendor_spend"];

pub const MAX_CARDS_PER_USER: i64 = 20;
pub const MAX_ENTITY_CARDS_PER_TYPE: i64 = 5;

/// Returns the fixed size for a given card type, or None if the card type is unknown.
pub fn default_size_for_card_type(card_type: &str) -> Option<CardSize> {
    match card_type {
        "current_period" => Some(CardSize::Full),
        "budget_stability" => Some(CardSize::Half),
        "net_position" => Some(CardSize::Half),
        "recent_transactions" => Some(CardSize::Full),
        "top_categories" => Some(CardSize::Half),
        "budget_per_day" => Some(CardSize::Half),
        "monthly_burn_rate" => Some(CardSize::Half),
        "remaining_budget" => Some(CardSize::Half),
        "month_progress" => Some(CardSize::Half),
        "balance_over_time" => Some(CardSize::Full),
        "account_summary" => Some(CardSize::Half),
        "category_breakdown" => Some(CardSize::Half),
        "vendor_spend" => Some(CardSize::Half),
        _ => None,
    }
}

pub fn is_valid_card_type(card_type: &str) -> bool {
    GLOBAL_CARD_TYPES.contains(&card_type) || ENTITY_CARD_TYPES.contains(&card_type)
}

pub fn is_entity_card_type(card_type: &str) -> bool {
    ENTITY_CARD_TYPES.contains(&card_type)
}

pub fn entity_table_for_card_type(card_type: &str) -> Option<&'static str> {
    match card_type {
        "account_summary" => Some("account"),
        "category_breakdown" => Some("category"),
        "vendor_spend" => Some("vendor"),
        _ => None,
    }
}

// ── DB row struct ────────────────────────────────────────────────────────────

#[derive(Serialize, Debug, Clone, sqlx::FromRow)]
pub struct DashboardCard {
    pub id: Uuid,
    pub user_id: Uuid,
    pub card_type: String,
    pub entity_id: Option<Uuid>,
    pub size: CardSize,
    pub position: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ── API response ─────────────────────────────────────────────────────────────

#[derive(Serialize, Debug, JsonSchema)]
pub struct DashboardCardResponse {
    pub id: Uuid,
    pub card_type: String,
    pub entity_id: Option<Uuid>,
    pub size: CardSize,
    pub position: i32,
    pub enabled: bool,
}

impl From<&DashboardCard> for DashboardCardResponse {
    fn from(card: &DashboardCard) -> Self {
        Self {
            id: card.id,
            card_type: card.card_type.clone(),
            entity_id: card.entity_id,
            size: card.size,
            position: card.position,
            enabled: card.enabled,
        }
    }
}

// ── API requests ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct CreateDashboardCardRequest {
    pub card_type: String,
    pub entity_id: Option<Uuid>,
    pub position: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct UpdateDashboardCardRequest {
    pub position: Option<i32>,
    pub enabled: Option<bool>,
    pub entity_id: Option<Uuid>,
}

#[derive(Deserialize, Debug, JsonSchema)]
pub struct ReorderEntry {
    pub id: Uuid,
    pub position: i32,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct ReorderRequest {
    pub order: Vec<ReorderEntry>,
}

// ── Available cards response ─────────────────────────────────────────────────

#[derive(Serialize, Debug, JsonSchema)]
pub struct AvailableGlobalCard {
    pub card_type: String,
    pub default_size: CardSize,
    pub already_added: bool,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct AvailableEntity {
    pub id: Uuid,
    pub name: String,
    pub already_added: bool,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct AvailableEntityCardType {
    pub card_type: String,
    pub default_size: CardSize,
    pub available_entities: Vec<AvailableEntity>,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct AvailableCardsResponse {
    pub global_cards: Vec<AvailableGlobalCard>,
    pub entity_cards: Vec<AvailableEntityCardType>,
}

// ── Default seed cards ───────────────────────────────────────────────────────

pub struct DefaultCard {
    pub card_type: &'static str,
    pub size: CardSize,
    pub position: i32,
}

pub const DEFAULT_CARDS: &[DefaultCard] = &[
    DefaultCard {
        card_type: "current_period",
        size: CardSize::Full,
        position: 0,
    },
    DefaultCard {
        card_type: "budget_stability",
        size: CardSize::Half,
        position: 1,
    },
    DefaultCard {
        card_type: "net_position",
        size: CardSize::Half,
        position: 2,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_card_types_have_default_sizes() {
        for ct in GLOBAL_CARD_TYPES.iter().chain(ENTITY_CARD_TYPES.iter()) {
            assert!(default_size_for_card_type(ct).is_some(), "Missing default size for card type: {ct}");
        }
    }

    #[test]
    fn unknown_card_type_returns_none() {
        assert!(default_size_for_card_type("unknown").is_none());
    }

    #[test]
    fn entity_card_types_are_entity() {
        for ct in ENTITY_CARD_TYPES {
            assert!(is_entity_card_type(ct));
            assert!(entity_table_for_card_type(ct).is_some());
        }
    }

    #[test]
    fn global_card_types_are_not_entity() {
        for ct in GLOBAL_CARD_TYPES {
            assert!(!is_entity_card_type(ct));
        }
    }
}
