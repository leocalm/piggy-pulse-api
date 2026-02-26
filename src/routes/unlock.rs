use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::ClientIp;
use rocket::serde::json::Json;
use rocket::{State, get};
use rocket_okapi::openapi;
use schemars::JsonSchema;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, JsonSchema)]
pub struct UnlockResponse {
    pub message: String,
}

/// Unlock an account using an emailed token
///
/// Called when a user clicks their account unlock email link.
/// Validates the token and clears both the user-based and IP-based rate limit
/// records so the user is not immediately re-blocked after unlocking.
#[openapi(tag = "Authentication")]
#[get("/unlock?<token>&<user>")]
pub async fn get_unlock(pool: &State<PgPool>, client_ip: ClientIp, token: String, user: String) -> Result<Json<UnlockResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let ip = client_ip.0.as_deref().unwrap_or("unknown");

    let user_id = Uuid::parse_str(&user).map_err(|e| AppError::uuid("Invalid user ID", e))?;

    if repo.verify_and_apply_unlock_token(&user_id, &token, ip).await? {
        Ok(Json(UnlockResponse {
            message: "Account unlocked successfully. You can now log in.".to_string(),
        }))
    } else {
        Err(AppError::BadRequest("Invalid or expired unlock token".to_string()))
    }
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![get_unlock]
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_unlock_module_exists() {
        // Endpoint exists and compiles
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_unlock_with_valid_token() {
        // Requires a running PostgreSQL at DATABASE_URL
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn test_unlock_with_invalid_token_returns_error() {
        // Requires a running PostgreSQL at DATABASE_URL
    }
}
