use rocket::State;
use rocket::http::{Cookie, Status};
use rocket::post;
use sqlx::PgPool;

use crate::auth::CurrentUser;
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::service::auth::AuthService;

#[post("/logout")]
pub async fn logout(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &rocket::http::CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    user: CurrentUser,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    auth.logout(&user.id, user.session_id, client_ip.0.clone(), user_agent.0.clone()).await?;

    cookies.remove_private(Cookie::build("user").build());

    Ok(Status::Ok)
}
