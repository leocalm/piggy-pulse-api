use crate::error::app_error::AppError;
use crate::models::user::User;
use argon2::Argon2;
use deadpool_postgres::Client;
use password_hash::rand_core::OsRng;
use password_hash::{PasswordHash, PasswordVerifier, Salt, SaltString};
use tokio_postgres::Row;

pub async fn create_user(
    client: &Client,
    name: &str,
    email: &str,
    password: &str,
) -> Result<Option<User>, AppError> {
    let (salt, password_hash) = password_hash(password);

    let result = client
        .query(
            r#"
        INSERT INTO users (name, email, salt, password_hash)
        VALUES($1, $2, $3, $4)
        RETURNING *
        "#,
            &[&name, &email, &salt, &password_hash],
        )
        .await?;

    Ok(map_response_to_model(&result))
}

pub async fn get_user_by_email(client: &Client, email: &str) -> Result<Option<User>, AppError> {
    let result = client
        .query(
            r#"
    SELECT * FROM users
    WHERE email = $1
    "#,
            &[&email],
        )
        .await?;

    Ok(map_response_to_model(&result))
}

pub async fn verify_password(user: &User, password: &str) -> Result<(), AppError> {
    let password_hash = PasswordHash::new(&user.password_hash)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &password_hash)
        .map_err(|_| AppError::InvalidCredentials)?;

    Ok(())
}

fn map_response_to_model(result: &[Row]) -> Option<User> {
    result.first().map(|user| User {
        id: user.get("id"),
        name: user.get("name"),
        email: user.get("email"),
        password_hash: user.get("password_hash"),
        created_at: user.get("created_at"),
        updated_at: user.get("updated_at"),
    })
}

fn password_hash(password: &str) -> (String, String) {
    let salt_string = SaltString::generate(&mut OsRng);
    let salt = Salt::from(&salt_string);
    let password_hash =
        PasswordHash::generate(Argon2::default(), password.as_bytes(), salt).unwrap();

    (salt.to_string(), password_hash.to_string())
}
