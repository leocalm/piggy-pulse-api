use rocket::serde::{Deserialize, Serialize};
use rocket_okapi::JsonSchema;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, JsonSchema, Default)]
pub enum SymbolPosition {
    #[default]
    Before,
    After,
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
