use rocket::http::Status;
use rocket::response::Responder;
use rocket::serde::json::serde_json;
use rocket::{Request, Response};
use rocket_okapi::OpenApiError;
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::Responses;
use rocket_okapi::response::OpenApiResponderInner;
use serde::Serialize;
use std::io::Cursor;
use thiserror::Error;
use tracing::error;
use validator::ValidationErrors;

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct ErrorResponse {
    message: String,
    request_id: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Internal server error")]
    Db {
        message: String,
        #[source]
        source: sqlx::error::Error,
    },
    #[error("User not found")]
    UserNotFound,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Internal server error")]
    PasswordHash { message: String },
    #[error("User {0} already exists")]
    UserAlreadyExists(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Currency not found: {0}")]
    CurrencyDoesNotExist(String),
    #[error("Internal server error")]
    UuidError {
        message: String,
        #[source]
        source: uuid::Error,
    },
    #[error("Validation error: {0}")]
    ValidationError(#[from] ValidationErrors),
    #[error("Internal server error")]
    ConfigurationError {
        message: String,
        #[source]
        source: Box<figment::Error>,
    },
    #[error("Email error: {0}")]
    EmailError(String),
    #[error("Two-factor authentication required")]
    TwoFactorRequired,
}

impl AppError {
    pub fn db(message: impl Into<String>, source: sqlx::error::Error) -> Self {
        Self::Db {
            message: message.into(),
            source,
        }
    }

    pub fn uuid(message: impl Into<String>, source: uuid::Error) -> Self {
        Self::UuidError {
            message: message.into(),
            source,
        }
    }

    pub fn password_hash(message: impl Into<String>, source: password_hash::Error) -> Self {
        Self::PasswordHash {
            message: format!("{}: {}", message.into(), source),
        }
    }

    pub fn email(message: impl Into<String>) -> Self {
        Self::EmailError(message.into())
    }
}

impl From<password_hash::Error> for AppError {
    fn from(e: password_hash::Error) -> Self {
        AppError::password_hash("Password hashing failed", e)
    }
}

impl From<uuid::Error> for AppError {
    fn from(e: uuid::Error) -> Self {
        AppError::uuid("Invalid UUID", e)
    }
}

impl From<&AppError> for Status {
    fn from(e: &AppError) -> Self {
        match e {
            AppError::UserNotFound => Status::NotFound,
            AppError::InvalidCredentials => Status::Unauthorized,
            AppError::PasswordHash { .. } => Status::InternalServerError,
            AppError::Db { .. } => Status::InternalServerError,
            AppError::Unauthorized => Status::Unauthorized,
            AppError::Forbidden => Status::Forbidden,
            AppError::UserAlreadyExists(_) => Status::Conflict,
            AppError::BadRequest(_) => Status::BadRequest,
            AppError::NotFound(_) => Status::NotFound,
            AppError::CurrencyDoesNotExist(_) => Status::BadRequest,
            AppError::UuidError { .. } => Status::BadRequest,
            AppError::ValidationError(_) => Status::BadRequest,
            AppError::ConfigurationError { .. } => Status::InternalServerError,
            AppError::EmailError(_) => Status::InternalServerError,
            AppError::TwoFactorRequired => Status::PreconditionRequired,
        }
    }
}

impl<'r> Responder<'r, 'static> for AppError {
    fn respond_to(self, req: &Request<'_>) -> rocket::response::Result<'static> {
        // Extract request context for better error logging
        let method = req.method();
        let uri = req.uri();

        // Try to get request_id from local_cache
        let request_id = req
            .local_cache(|| None::<crate::middleware::RequestId>)
            .as_ref()
            .map(|r| r.0.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Try to get user from auth
        let user_id = req
            .local_cache(|| None::<crate::auth::CurrentUser>)
            .as_ref()
            .map(|u| u.id.to_string())
            .unwrap_or_else(|| "anonymous".to_string());

        error!(
            error = ?self,
            request_id = %request_id,
            user_id = %user_id,
            method = %method,
            uri = %uri,
            "request failed"
        );

        let status = Status::from(&self);
        let body = match &self {
            AppError::TwoFactorRequired => {
                // Special case for 2FA required - return custom JSON
                serde_json::json!({"two_factor_required": true}).to_string()
            }
            _ => {
                let error_message = match &self {
                    AppError::ValidationError(_) => "Invalid request".to_string(),
                    _ => self.to_string(),
                };
                let error_response = ErrorResponse {
                    message: error_message.clone(),
                    request_id: request_id.clone(),
                };
                serde_json::to_string(&error_response).unwrap_or_else(|e| {
                    error!(
                        request_id = %request_id,
                        error = %e,
                        "Failed to serialize error response"
                    );
                    format!(r#"{{"message":"Error serialization failed","request_id":"{}"}}"#, request_id)
                })
            }
        };

        Response::build()
            .status(status)
            .header(rocket::http::ContentType::JSON)
            .sized_body(body.len(), Cursor::new(body))
            .ok()
    }
}

impl OpenApiResponderInner for AppError {
    fn responses(_gen: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
        use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiResponse};
        let mut responses = Responses::default();
        responses.responses.insert(
            "400".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Bad Request".to_string(),
                ..Default::default()
            }),
        );
        responses.responses.insert(
            "401".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Unauthorized".to_string(),
                ..Default::default()
            }),
        );
        responses.responses.insert(
            "403".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Forbidden".to_string(),
                ..Default::default()
            }),
        );
        responses.responses.insert(
            "404".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Not Found".to_string(),
                ..Default::default()
            }),
        );
        responses.responses.insert(
            "500".to_string(),
            RefOr::Object(OpenApiResponse {
                description: "Internal Server Error".to_string(),
                ..Default::default()
            }),
        );
        Ok(responses)
    }
}

impl From<figment::Error> for AppError {
    fn from(e: figment::Error) -> Self {
        AppError::ConfigurationError {
            message: "Failed to read configuration".to_string(),
            source: Box::new(e),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::NotFound("Resource not found".to_string()),
            _ => AppError::db("Database error", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket::{get, routes};

    #[get("/test-error")]
    #[allow(clippy::result_large_err)]
    fn test_error_route() -> Result<(), AppError> {
        Err(AppError::NotFound("Test resource".to_string()))
    }

    #[test]
    fn test_error_response_includes_request_id() {
        // In `cargo test --release`, Rocket's default profile is `release`, which rejects the default insecure secret key.
        // Use a deterministic (but non-default) secret key so this test passes under CI's release-mode test run.
        let rocket = rocket::custom(rocket::Config::figment().merge(("secret_key", "MDEyMzQ1Njc4OWFiY2RlZjAxMjM0NTY3ODlhYmNkZWY=")))
            .attach(crate::middleware::RequestLogger)
            .mount("/", routes![test_error_route]);

        let client = Client::tracked(rocket).expect("valid rocket instance");
        let response = client.get("/test-error").dispatch();

        assert_eq!(response.status(), Status::NotFound);

        // Verify X-Request-Id header is present
        let request_id_header = response.headers().get_one("X-Request-Id");
        assert!(request_id_header.is_some(), "X-Request-Id header should be present");

        // Verify response body includes request_id
        let body = response.into_string().expect("response body");
        assert!(body.contains("request_id"), "Response body should contain request_id field");
        assert!(body.contains("Test resource"), "Response body should contain error message");

        // Verify it's valid JSON with both fields
        let json: serde_json::Value = serde_json::from_str(&body).expect("valid JSON");
        assert!(json.get("message").is_some(), "Response should have message field");
        assert!(json.get("request_id").is_some(), "Response should have request_id field");
    }
}
