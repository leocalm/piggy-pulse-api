use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};
use rocket::{State, get, http::Status};
use rocket_okapi::openapi;
use schemars::JsonSchema;
use sqlx::PgPool;

/// Check API and database health
#[openapi(tag = "Health")]
#[get("/")]
pub async fn healthcheck(pool: &State<PgPool>) -> Result<Json<HealthResponse>, Status> {
    sqlx::query("SELECT 1").execute(pool.inner()).await.map_err(|_| Status::ServiceUnavailable)?;

    Ok(Json(HealthResponse {
        status: "ok".to_string(),
        database: "connected".to_string(),
    }))
}

pub fn routes() -> (Vec<rocket::Route>, okapi::openapi3::OpenApi) {
    rocket_okapi::openapi_get_routes_spec![healthcheck]
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, JsonSchema)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
}

#[cfg(test)]
mod tests {
    //! Integration tests for routes require a running PostgreSQL database
    //! configured at the DATABASE_URL specified in the test Config.
    //! These tests verify the full request/response cycle including database connections.
    //!
    //! To run integration tests, ensure PostgreSQL is running with appropriate test credentials.

    use crate::{Config, build_rocket};
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;

    #[rocket::async_test]
    #[ignore = "requires database"]
    async fn health_check_works() {
        let mut config = Config::default();
        config.database.url = "postgresql://test:test@localhost/test".to_string();

        let client = Client::tracked(build_rocket(config)).await.expect("valid rocket instance");
        let response = client.get("/api/v1/health").dispatch().await;
        assert_eq!(response.status(), Status::Ok);

        let body = response.into_json::<super::HealthResponse>().await.expect("health response json");
        assert_eq!(
            body,
            super::HealthResponse {
                status: "ok".to_string(),
                database: "connected".to_string(),
            }
        );
    }
}
