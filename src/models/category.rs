use rocket::serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, Default, sqlx::Type)]
#[sqlx(type_name = "category_type", rename_all = "PascalCase")]
pub enum CategoryType {
    #[default]
    Incoming,
    Outgoing,
    Transfer,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, Eq, PartialEq, sqlx::Type)]
#[sqlx(type_name = "category_behavior", rename_all = "lowercase")]
pub enum CategoryBehavior {
    Fixed,
    Variable,
    Subscription,
}

/// Raw category row. Structural fields (type, behavior, parent_id,
/// is_system, is_archived) stay plaintext; label/presentation fields
/// (name, color, icon, description) are encrypted.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Category {
    pub id: Uuid,
    pub category_type: CategoryType,
    pub behavior: Option<CategoryBehavior>,
    pub parent_id: Option<Uuid>,
    pub is_system: bool,
    pub is_archived: bool,
    pub name_enc: Vec<u8>,
    pub color_enc: Option<Vec<u8>>,
    pub icon_enc: Option<Vec<u8>>,
    pub description_enc: Option<Vec<u8>>,
}
