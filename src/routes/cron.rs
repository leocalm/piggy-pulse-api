use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::models::budget_period::AutoPeriodGenerationResponse;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{FromRequest, Outcome as RequestOutcome, Request};
use rocket::serde::json::Json;
use rocket::{State, post, routes};
use sqlx::PgPool;

pub(crate) struct CronAuth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CronAuth {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let config = match req.rocket().state::<Config>() {
            Some(config) => config,
            None => return Outcome::Error((Status::InternalServerError, AppError::Unauthorized)),
        };

        if config.cron.auth_token.is_empty() {
            return Outcome::Error((Status::BadRequest, AppError::BadRequest("Cron auth token is not configured".to_string())));
        }

        let incoming = req.headers().get_one("x-cron-token");
        match incoming {
            Some(token) if token == config.cron.auth_token => Outcome::Success(CronAuth),
            _ => Outcome::Error((Status::Forbidden, AppError::Forbidden)),
        }
    }
}

#[post("/generate-periods")]
pub async fn generate_automatic_periods(pool: &State<PgPool>, _cron_auth: CronAuth) -> Result<Json<AutoPeriodGenerationResponse>, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let result = repo.generate_automatic_budget_periods().await?;
    Ok(Json(result))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![generate_automatic_periods]
}
