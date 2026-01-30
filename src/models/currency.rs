use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Deserialize, Debug)]
pub struct CurrencyRequest {
    pub name: String,
    pub symbol: String,
    pub currency: String,
    pub decimal_places: i32,
}
