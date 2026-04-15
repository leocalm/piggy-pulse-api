use chrono::NaiveDate;
use rocket::serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

/// Plaintext transaction fields as they arrive from the v2 route
/// handler. The service layer encrypts amount + description with
/// the caller's DEK before handing off to the repository.
#[derive(Deserialize, Debug, Validate)]
pub struct TransactionRequest {
    #[validate(range(min = 0))]
    pub amount: i64,
    #[validate(length(min = 3))]
    pub description: String,
    pub occurred_at: NaiveDate,
    pub category_id: Uuid,
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub vendor_id: Option<Uuid>,
}
