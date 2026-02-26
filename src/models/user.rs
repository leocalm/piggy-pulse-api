use rocket::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use uuid::Uuid;
use validator::{Validate, ValidationError};
use zxcvbn::{Score, zxcvbn};

#[derive(Debug, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
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

/// Request payload for updating an existing user account.
/// Password is optional — omit to leave the current password unchanged.
#[derive(Deserialize, Debug, Validate, JsonSchema)]
pub struct UserUpdateRequest {
    #[validate(length(min = 8))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8))]
    #[validate(custom(function = "validate_password_strength"))]
    pub password: Option<String>,
}

impl UserUpdateRequest {
    /// Returns the subset of top-level fields that are being changed.
    /// `name` and `email` are always included; `password` is included only
    /// when the caller explicitly provided a new value.
    pub fn changed_fields(&self) -> Vec<&'static str> {
        let mut fields = vec!["name", "email"];
        if self.password.is_some() {
            fields.push("password");
        }
        fields
    }
}

#[derive(Deserialize, Debug, JsonSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    /// Optional two-factor authentication code (6 digits or backup code)
    pub two_factor_code: Option<String>,
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
    let estimate = zxcvbn(password, &[]);
    if estimate.score() < Score::Three {
        let mut error = ValidationError::new("password_strength");
        error.message = Some("Password is too weak".into());
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_fields_without_password() {
        let req = UserUpdateRequest {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            password: None,
        };
        assert_eq!(req.changed_fields(), vec!["name", "email"]);
    }

    #[test]
    fn changed_fields_with_password() {
        let req = UserUpdateRequest {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            password: Some("NewStrongPass!99".to_string()),
        };
        assert_eq!(req.changed_fields(), vec!["name", "email", "password"]);
    }
}
