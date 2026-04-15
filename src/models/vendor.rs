use uuid::Uuid;

/// Raw vendor row. Structural fields (archived) stay plaintext;
/// label fields (name, description) are encrypted.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Vendor {
    pub id: Uuid,
    pub archived: bool,
    pub name_enc: Vec<u8>,
    pub description_enc: Option<Vec<u8>>,
}
