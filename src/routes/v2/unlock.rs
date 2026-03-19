use rocket::State;
use rocket::get;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::database::postgres_repository::PostgresRepository;
use crate::dto::misc::UnlockResponse;
use crate::error::app_error::AppError;
use crate::middleware::ClientIp;
use crate::service::unlock::UnlockService;

#[get("/unlock?<token>")]
pub async fn unlock(pool: &State<PgPool>, client_ip: ClientIp, token: Option<String>) -> Result<Json<UnlockResponse>, AppError> {
    let token = token
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AppError::BadRequest("Missing required query parameter: token".to_string()))?;

    let repo = PostgresRepository { pool: pool.inner().clone() };
    let service = UnlockService::new(&repo);
    let ip = client_ip.0.as_deref().unwrap_or("unknown");
    let response = service.unlock_by_token(&token, ip).await?;
    Ok(Json(response))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![unlock]
}
