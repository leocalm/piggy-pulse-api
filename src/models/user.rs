use uuid::Uuid;
use validator::ValidationError;
use zxcvbn::{Score, zxcvbn};

#[derive(sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
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
