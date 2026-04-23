use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::user::User;
use argon2::Argon2;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, Salt, SaltString};
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
    pub async fn create_user(
        &self,
        name: &str,
        email: &str,
        password: &str,
        wrapped_dek: Option<&[u8]>,
        dek_wrap_params: Option<&serde_json::Value>,
    ) -> Result<User, AppError> {
        let (salt, password_hash) = password_hash(password);

        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (name, email, salt, password_hash, wrapped_dek, dek_wrap_params)
            VALUES($1, $2, $3, $4, $5, $6)
            RETURNING id, name, email, password_hash
            "#,
        )
        .bind(name)
        .bind(email)
        .bind(&salt)
        .bind(&password_hash)
        .bind(wrapped_dek)
        .bind(dek_wrap_params)
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    pub async fn get_wrapped_dek(&self, user_id: &Uuid) -> Result<(Option<Vec<u8>>, Option<serde_json::Value>), AppError> {
        let row: (Option<Vec<u8>>, Option<serde_json::Value>) = sqlx::query_as("SELECT wrapped_dek, dek_wrap_params FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn update_wrapped_dek(&self, user_id: &Uuid, wrapped_dek: &[u8], dek_wrap_params: &serde_json::Value) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET wrapped_dek = $1, dek_wrap_params = $2 WHERE id = $3")
            .bind(wrapped_dek)
            .bind(dek_wrap_params)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        Ok(())
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

    /// Timing-equalization helper for flows where no real password is available
    /// (e.g. forgot-password for a non-existent email). Uses a fixed input to
    /// burn the same CPU time as a real verification.
    pub fn dummy_verify_no_input() {
        // The input value is irrelevant — we only need to spend Argon2 CPU time.
        let hash = PasswordHash::new(&DUMMY_HASH).expect("invalid dummy hash");
        let _ = Argon2::default().verify_password(b"timing-equalization", &hash);
    }

    pub async fn delete_user(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }
}

pub(crate) fn password_hash(password: &str) -> (String, String) {
    let salt_string = SaltString::generate(&mut OsRng);
    let salt = Salt::from(&salt_string);
    let password_hash = PasswordHash::generate(Argon2::default(), password.as_bytes(), salt).unwrap();

    (salt.to_string(), password_hash.to_string())
}
