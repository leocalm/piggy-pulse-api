use rocket::State;
use rocket::http::{Cookie, Status};
use rocket::post;
use sqlx::PgPool;

use crate::auth::{AuthMethod, CurrentUser};
use crate::config::Config;
use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use crate::middleware::{ClientIp, UserAgent};
use crate::service::auth::AuthService;
use crate::session_dek::SessionDekStore;

#[post("/logout")]
pub async fn logout(
    pool: &State<PgPool>,
    config: &State<Config>,
    cookies: &rocket::http::CookieJar<'_>,
    user_agent: UserAgent,
    client_ip: ClientIp,
    dek_store: &State<SessionDekStore>,
    user: CurrentUser,
) -> Result<Status, AppError> {
    let repo = PostgresRepository { pool: pool.inner().clone() };
    let auth = AuthService::new(&repo, config);

    // Revoke the bearer token if authenticated via Bearer
    if user.auth_method == AuthMethod::Bearer
        && let Some(token_id) = user.api_token_id
    {
        repo.revoke(&token_id).await?;
    }

    auth.logout(&user.id, user.session_id, client_ip.0.clone(), user_agent.0.clone()).await?;

    if let Some(principal_id) = user.principal_id() {
        dek_store.remove(&principal_id).await;
    }

    cookies.remove_private(Cookie::build("user").build());

    Ok(Status::Ok)
}
