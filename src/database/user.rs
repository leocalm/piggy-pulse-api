use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::user::User;
use argon2::Argon2;
use password_hash::rand_core::OsRng;
use password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, Salt, SaltString};
use std::sync::LazyLock;
use uuid::Uuid;

/// A real Argon2 hash generated once at startup, used as a timing decoy
/// so that login requests for non-existent users take the same time as
/// requests for existing users.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(b"dummy-never-matches", Salt::from(&salt))
        .expect("failed to generate dummy hash")
        .to_string()
});

impl PostgresRepository {
    pub async fn create_user(&self, name: &str, email: &str, password: &str) -> Result<User, AppError> {
        let (salt, password_hash) = password_hash(password);

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, email, salt, password_hash)
            VALUES($1, $2, $3, $4)
            RETURNING id, name, email, password_hash
            "#,
        )
        .bind(name)
        .bind(email)
        .bind(&salt)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, name, email, password_hash
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_user_by_id(&self, id: &Uuid) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, name, email, password_hash
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn verify_password(&self, user: &User, password: &str) -> Result<(), AppError> {
        let password_hash = PasswordHash::new(&user.password_hash).map_err(|e| AppError::password_hash("Failed to parse stored password hash", e))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &password_hash)
            .map_err(|_| AppError::InvalidCredentials)?;

        Ok(())
    }

    /// Perform a throwaway Argon2 verification to equalize response timing
    /// regardless of whether the target account exists. This prevents attackers
    /// from distinguishing existing vs non-existing accounts by measuring
    /// response latency.
    pub fn dummy_verify(password: &str) {
        let hash = PasswordHash::new(&DUMMY_HASH).expect("invalid dummy hash");
        let _ = Argon2::default().verify_password(password.as_bytes(), &hash);
    }

    pub async fn update_user(&self, id: &Uuid, name: &str, email: &str, new_password: Option<&str>) -> Result<User, AppError> {
        let user = if let Some(password) = new_password {
            let (salt, hash) = password_hash(password);
            sqlx::query_as::<_, User>(
                r#"
                UPDATE users
                SET name = $1, email = $2, salt = $3, password_hash = $4
                WHERE id = $5
                RETURNING id, name, email, password_hash
                "#,
            )
            .bind(name)
            .bind(email)
            .bind(&salt)
            .bind(&hash)
            .bind(id)
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, User>(
                r#"
                UPDATE users
                SET name = $1, email = $2
                WHERE id = $3
                RETURNING id, name, email, password_hash
                "#,
            )
            .bind(name)
            .bind(email)
            .bind(id)
            .fetch_one(&self.pool)
            .await?
        };

        Ok(user)
    }

    pub async fn delete_user(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }

    /// Verifies the current password and updates it to the new one.
    pub async fn change_password(&self, user_id: &Uuid, current_password: &str, new_password: &str) -> Result<(), AppError> {
        let user = self.get_user_by_id(user_id).await?.ok_or(AppError::UserNotFound)?;
        self.verify_password(&user, current_password)
            .await
            .map_err(|_| AppError::BadRequest("Current password is incorrect".to_string()))?;

        let (salt, new_hash) = password_hash(new_password);
        sqlx::query("UPDATE users SET salt = $1, password_hash = $2 WHERE id = $3")
            .bind(&salt)
            .bind(&new_hash)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

pub(crate) fn password_hash(password: &str) -> (String, String) {
    let salt_string = SaltString::generate(&mut OsRng);
    let salt = Salt::from(&salt_string);
    let password_hash = PasswordHash::generate(Argon2::default(), password.as_bytes(), salt).unwrap();

    (salt.to_string(), password_hash.to_string())
}
