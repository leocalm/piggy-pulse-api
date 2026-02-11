use chrono::{DateTime, Utc};
use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};
use zxcvbn::zxcvbn;

#[derive(Serialize, Debug, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub name: String,
    pub email: String,
}

#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct UserRequest {
    #[validate(length(min = 8))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    #[validate(custom(function = "validate_password_strength"))]
    pub password: String,
}

#[derive(Deserialize, Debug, JsonSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

impl From<&User> for UserResponse {
    fn from(user: &User) -> Self {
        Self {
            id: user.id,
            name: user.name.clone(),
            email: user.email.clone(),
        }
    }
}

pub fn validate_password_strength(password: &str) -> Result<(), ValidationError> {
    let estimate = zxcvbn(password, &[]).map_err(|_| ValidationError::new("password_strength"))?;
    if estimate.score() < 3 {
        let mut error = ValidationError::new("password_strength");
        error.message = Some("Password is too weak".into());
        return Err(error);
    }
    Ok(())
}
