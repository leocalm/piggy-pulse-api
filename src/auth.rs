use crate::error::app_error::AppError;
use rocket::http::Status;
use rocket::outcome::Outcome;
use rocket::request::{FromRequest, Outcome as RequestOutcome, Request};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::{Object, Responses, SecurityRequirement, SecurityScheme, SecuritySchemeData};
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
}

pub(crate) fn parse_user_cookie_value(value: &str) -> Option<(Uuid, String)> {
    let (id_str, username) = value.split_once(':')?;
    let id = Uuid::parse_str(id_str).ok()?;
    Some((id, username.to_string()))
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = AppError;

    async fn from_request(req: &'r Request<'_>) -> RequestOutcome<Self, Self::Error> {
        let cookies = req.cookies();
        if let Some(cookie) = cookies.get_private("user")
            && let Some((id, username)) = parse_user_cookie_value(cookie.value())
        {
            return Outcome::Success(CurrentUser { id, username });
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
    use super::parse_user_cookie_value;
    use uuid::Uuid;

    #[test]
    fn parse_user_cookie_value_valid() {
        let id = Uuid::new_v4();
        let value = format!("{}:user@example.com", id);
        let parsed = parse_user_cookie_value(&value);
        assert!(matches!(parsed, Some((parsed_id, username)) if parsed_id == id && username == "user@example.com"));
    }

    #[test]
    fn parse_user_cookie_value_invalid_uuid() {
        let parsed = parse_user_cookie_value("not-a-uuid:user@example.com");
        assert!(parsed.is_none());
    }

    #[test]
    fn parse_user_cookie_value_missing_delimiter() {
        let parsed = parse_user_cookie_value("missing-delimiter");
        assert!(parsed.is_none());
    }
}
