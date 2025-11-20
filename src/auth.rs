use crate::error::app_error::AppError;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{FromRequest, Outcome as RequestOutcome, Request};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let cookies = req.cookies();
        if let Some(cookie) = cookies.get_private("user")
            && let Some((id_str, username)) = cookie.value().split_once(':')
            && let Ok(id) = Uuid::parse_str(id_str)
        {
            return Outcome::Success(CurrentUser {
                id,
                username: username.to_string(),
            });
        }

        Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials))
    }
}
