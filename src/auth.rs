use crate::database::postgres_repository::PostgresRepository;
use crate::error::app_error::AppError;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{FromRequest, Outcome as RequestOutcome, Request};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::{Object, Responses, SecurityRequirement, SecurityScheme, SecuritySchemeData};
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
}

pub(crate) fn parse_session_cookie_value(value: &str) -> Option<(Uuid, Uuid)> {
    let (session_id_str, user_id_str) = value.split_once(':')?;
    let session_id = Uuid::parse_str(session_id_str).ok()?;
    let user_id = Uuid::parse_str(user_id_str).ok()?;
    Some((session_id, user_id))
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let cookies = req.cookies();
        if let Some(cookie) = cookies.get_private("user")
            && let Some((session_id, user_id)) = parse_session_cookie_value(cookie.value())
        {
            let pool = match req.rocket().state::<PgPool>() {
                Some(pool) => pool,
                None => return Outcome::Error((Status::InternalServerError, AppError::Unauthorized)),
            };

            let repo = PostgresRepository { pool: pool.clone() };

            match repo.get_active_session_user(&session_id, &user_id).await {
                Ok(Some(user)) => {
                    let current_user = CurrentUser {
                        id: user.id,
                        username: user.email,
                    };
                    req.local_cache(|| Some(current_user.clone()));
                    return Outcome::Success(current_user);
                }
                Ok(None) => {
                    let _ = repo.delete_session_if_expired(&session_id).await;
                    return Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials));
                }
                Err(err) => return Outcome::Error((Status::InternalServerError, err)),
            }
        }

        Outcome::Error((Status::Unauthorized, AppError::InvalidCredentials))
    }
}

impl<'a> OpenApiFromRequest<'a> for CurrentUser {
    fn from_request_input(_gen: &mut OpenApiGenerator, _name: String, _required: bool) -> rocket_okapi::Result<RequestHeaderInput> {
        // Document the cookie-based authentication requirement
        let security_scheme = SecurityScheme {
            description: Some("Cookie-based authentication. Log in via POST /api/users/login to obtain the session cookie.".to_string()),
            data: SecuritySchemeData::ApiKey {
                name: "user".to_string(),
                location: "cookie".to_string(),
            },
            extensions: Object::default(),
        };

        let mut security_req = SecurityRequirement::new();
        security_req.insert("cookieAuth".to_string(), Vec::new());

        Ok(RequestHeaderInput::Security("cookieAuth".to_string(), security_scheme, security_req))
    }

    fn get_responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        use rocket_okapi::okapi::openapi3::{RefOr, Response};
        let mut responses = Responses::default();
        responses.responses.insert(
            "401".to_string(),
            RefOr::Object(Response {
                description: "Unauthorized - Authentication required".to_string(),
                ..Default::default()
            }),
        );
        Ok(responses)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_session_cookie_value;
    use uuid::Uuid;

    #[test]
    fn parse_session_cookie_value_valid() {
        let session_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let value = format!("{}:{}", session_id, user_id);
        let parsed = parse_session_cookie_value(&value);
        assert!(matches!(parsed, Some((parsed_session_id, parsed_user_id)) if parsed_session_id == session_id && parsed_user_id == user_id));
    }

    #[test]
    fn parse_session_cookie_value_invalid_uuid() {
        let parsed = parse_session_cookie_value("not-a-uuid:user@example.com");
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_session_cookie_value_missing_delimiter() {
        let parsed = parse_session_cookie_value("missing-delimiter");
        assert!(parsed.is_none());
    }
}
