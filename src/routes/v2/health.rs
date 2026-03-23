use rocket::State;
use rocket::get;
use rocket::http::Status;
use rocket::serde::json::Json;
use sqlx::PgPool;

use crate::dto::health::HealthResponse;

#[get("/")]
pub async fn health_check(pool: &State<PgPool>) -> Result<Json<HealthResponse>, Status> {
    sqlx::query("SELECT 1").execute(pool.inner()).await.map_err(|_| Status::ServiceUnavailable)?;

    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        database: "connected".to_string(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![health_check]
}
