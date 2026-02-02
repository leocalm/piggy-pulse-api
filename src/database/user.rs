use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::user::User;
use argon2::Argon2;
use password_hash::rand_core::OsRng;
use password_hash::{PasswordHash, PasswordVerifier, Salt, SaltString};
use uuid::Uuid;

#[async_trait::async_trait]
pub trait UserRepository {
    async fn create_user(&self, name: &str, email: &str, password: &str) -> Result<User, AppError>;
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn verify_password(&self, user: &User, password: &str) -> Result<(), AppError>;
    async fn update_user(&self, id: &Uuid, name: &str, email: &str, password: &str) -> Result<User, AppError>;
    async fn delete_user(&self, id: &Uuid) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl UserRepository for PostgresRepository {
    async fn create_user(&self, name: &str, email: &str, password: &str) -> Result<User, AppError> {
        let (salt, password_hash) = password_hash(password);

        let user = sqlx::query_as!(
            User,
            r#"
            INSERT INTO users (name, email, salt, password_hash)
            VALUES($1, $2, $3, $4)
            RETURNING id, name, email, password_hash, created_at
            "#,
            name,
            email,
            &salt,
            &password_hash
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as!(
            User,
            r#"
            SELECT id, name, email, password_hash, created_at
            FROM users
            WHERE email = $1
            "#,
            email
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    async fn verify_password(&self, user: &User, password: &str) -> Result<(), AppError> {
        let password_hash = PasswordHash::new(&user.password_hash).map_err(|e| AppError::password_hash("Failed to parse stored password hash", e))?;
        Argon2::default()
            .verify_password(password.as_bytes(), &password_hash)
            .map_err(|_| AppError::InvalidCredentials)?;

        Ok(())
    }

    async fn update_user(&self, id: &Uuid, name: &str, email: &str, password: &str) -> Result<User, AppError> {
        let (salt, password_hash) = password_hash(password);

        let user = sqlx::query_as!(
            User,
            r#"
            UPDATE users
            SET name = $1, email = $2, salt = $3, password_hash = $4
            WHERE id = $5
            RETURNING id, name, email, password_hash, created_at
            "#,
            name,
            email,
            &salt,
            &password_hash,
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    async fn delete_user(&self, id: &Uuid) -> Result<(), AppError> {
        sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(&self.pool).await?;

        Ok(())
    }
}

fn password_hash(password: &str) -> (String, String) {
    let salt_string = SaltString::generate(&mut OsRng);
    let salt = Salt::from(&salt_string);
    let password_hash = PasswordHash::generate(Argon2::default(), password.as_bytes(), salt).unwrap();

    (salt.to_string(), password_hash.to_string())
}
