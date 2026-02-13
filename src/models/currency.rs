use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, JsonSchema, Default)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SymbolPosition {
    #[default]
    Before,
    After,
}

#[derive(Debug, Clone, Default, sqlx::FromRow)]
pub struct Currency {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub currency: String,
    pub decimal_places: i32,
    pub symbol_position: SymbolPosition,
}

impl From<&Currency> for CurrencyResponse {
    fn from(value: &Currency) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            symbol: value.symbol.clone(),
            currency: value.currency.clone(),
            decimal_places: value.decimal_places,
            symbol_position: value.symbol_position,
        }
    }
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct CurrencyResponse {
    pub id: Uuid,
    pub name: String,
    pub symbol: String,
    pub currency: String,
    pub decimal_places: i32,
    pub symbol_position: SymbolPosition,
}
