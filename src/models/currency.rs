use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Debug, Clone, Default)]
pub struct Currency {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub currency: String,
    pub decimal_places: i32,
    pub created_at: DateTime<Utc>,
}

impl From<&Currency> for CurrencyResponse {
    fn from(value: &Currency) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            symbol: value.symbol.clone(),
            currency: value.currency.clone(),
            decimal_places: value.decimal_places,
        }
    }
}

#[derive(Serialize, Debug)]
pub struct CurrencyResponse {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub currency: String,
    pub decimal_places: i32,
}

#[derive(Deserialize, Debug, Validate)]
pub struct CurrencyRequest {
    #[validate(length(min = 3))]
    pub name: String,
    #[validate(length(max = 3))]
    pub symbol: String,
    #[validate(length(equal = 3))]
    pub currency: String,
    #[validate(range(min = 0))]
    pub decimal_places: i32,
}
