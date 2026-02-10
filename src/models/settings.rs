use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Debug, Clone, sqlx::FromRow)]
pub struct Settings {
    pub id: Uuid,
    pub user_id: Uuid,
    pub theme: String,
    pub language: String,
    pub default_currency_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct SettingsResponse {
    pub id: Uuid,
    pub theme: String,
    pub language: String,
    pub default_currency_id: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
}

impl From<&Settings> for SettingsResponse {
    fn from(value: &Settings) -> Self {
        Self {
            id: value.id,
            theme: value.theme.clone(),
            language: value.language.clone(),
            default_currency_id: value.default_currency_id,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct SettingsRequest {
    #[validate(length(min = 1))]
    #[schemars(regex(pattern = r"^(light|dark|auto)$"))]
    pub theme: String,

    #[validate(length(equal = 2))]
    #[schemars(regex(pattern = r"^(en|es|pt|fr|de)$"))]
    pub language: String,

    pub default_currency_id: Option<Uuid>,
}
