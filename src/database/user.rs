use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::user::User;
use argon2::Argon2;
use password_hash::rand_core::OsRng;
use password_hash::{PasswordHash, PasswordVerifier, Salt, SaltString};
use tokio_postgres::Row;
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
impl<'a> UserRepository for PostgresRepository<'a> {
    async fn create_user(&self, name: &str, email: &str, password: &str) -> Result<User, AppError> {
        let (salt, password_hash) = password_hash(password);

        let result = self
            .client
            .query(
                r#"
        INSERT INTO users (name, email, salt, password_hash)
        VALUES($1, $2, $3, $4)
        RETURNING *
        "#,
                &[&name, &email, &salt, &password_hash],
            )
            .await?;

        if let Some(row) = result.first() {
            Ok(map_response_to_model(row))
        } else {
            Err(AppError::Db("Error creating user".to_string()))
        }
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let result = self
            .client
            .query(
                r#"
    SELECT * FROM users
    WHERE email = $1
    "#,
                &[&email],
            )
            .await?;

        if let Some(row) = result.first() {
            Ok(Some(map_response_to_model(row)))
        } else {
            Ok(None)
        }
    }

    async fn verify_password(&self, user: &User, password: &str) -> Result<(), AppError> {
        let password_hash = PasswordHash::new(&user.password_hash)?;
        Argon2::default()
            .verify_password(password.as_bytes(), &password_hash)
            .map_err(|_| AppError::InvalidCredentials)?;

        Ok(())
    }

    async fn update_user(&self, id: &Uuid, name: &str, email: &str, password: &str) -> Result<User, AppError> {
        let (salt, password_hash) = password_hash(password);

        let result = self
            .client
            .query(
                r#"
        UPDATE users
        SET name = $1, email = $2, salt = $3, password_hash = $4
        WHERE id = $5
        RETURNING *
        "#,
                &[&name, &email, &salt, &password_hash, &id],
            )
            .await?;

        if let Some(row) = result.first() {
            Ok(map_response_to_model(row))
        } else {
            Err(AppError::NotFound("User not found".to_string()))
        }
    }

    async fn delete_user(&self, id: &Uuid) -> Result<(), AppError> {
        self.client
            .execute(
                r#"
        DELETE FROM users
        WHERE id = $1
        "#,
                &[&id],
            )
            .await?;

        Ok(())
    }
}

fn map_response_to_model(user: &Row) -> User {
    User {
        id: user.get("id"),
        name: user.get("name"),
        email: user.get("email"),
        password_hash: user.get("password_hash"),
        created_at: user.get("created_at"),
    }
}

fn password_hash(password: &str) -> (String, String) {
    let salt_string = SaltString::generate(&mut OsRng);
    let salt = Salt::from(&salt_string);
    let password_hash = PasswordHash::generate(Argon2::default(), password.as_bytes(), salt).unwrap();

    (salt.to_string(), password_hash.to_string())
}
