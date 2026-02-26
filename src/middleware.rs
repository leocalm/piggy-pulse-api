pub mod rate_limit;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Data, Response};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

/// Request ID that is attached to every request for tracking
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn new() -> Self {
        RequestId(Uuid::new_v4().to_string())
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RequestId {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try to get request_id from local_cache (set by fairing)
        if let Some(request_id) = request.local_cache(|| None::<RequestId>).as_ref() {
            return Outcome::Success(request_id.clone());
        }

        // Fallback: create a new one if fairing hasn't run
        Outcome::Success(RequestId::new())
    }
}

/// Stores the request start time for duration calculation
#[derive(Debug, Clone, Copy)]
struct RequestStartTime(Instant);

/// Fairing that adds request ID to all requests and logs request/response information
pub struct RequestLogger;

#[rocket::async_trait]
impl Fairing for RequestLogger {
    fn info(&self) -> Info {
        Info {
            name: "Request Logger",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, request: &mut Request<'_>, _: &mut Data<'_>) {
        let request_id = RequestId::new();
        let method = request.method().to_string();
        let uri = request.uri().to_string();
        let request_bytes = request.headers().get_one("Content-Length").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);

        // Extract user_id from cookie without DB hit
        let user_id = request.cookies().get_private("user").and_then(|c| {
            let value = c.value().to_string();
            crate::auth::parse_session_cookie_value(&value).map(|(_, uid)| uid.to_string())
        });

        // Store start time, request_id, and request_bytes in local cache
        request.local_cache(|| RequestStartTime(Instant::now()));
        request.local_cache(|| Some(request_id.clone()));
        request.local_cache(|| request_bytes);

        info!(
            request_id = %request_id.0,
            method = %method,
            uri = %uri,
            user_id = user_id.as_deref().unwrap_or("-"),
            request_bytes = request_bytes,
            "incoming request"
        );
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let request_id = request
            .local_cache(|| None::<RequestId>)
            .as_ref()
            .map(|r| r.0.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let duration_ms = request.local_cache(|| RequestStartTime(Instant::now())).0.elapsed().as_millis() as u64;

        let request_bytes = *request.local_cache(|| 0u64);

        let status = response.status();
        let method = request.method();
        let uri = request.uri();

        let response_bytes = response
            .headers()
            .get_one("Content-Length")
            .and_then(|v| v.parse::<u64>().ok())
            .or_else(|| response.body().preset_size().map(|s| s as u64))
            .unwrap_or(0);

        // Get user_id from CurrentUser cached by auth guard
        let user_id = request.local_cache(|| None::<crate::auth::CurrentUser>).as_ref().map(|u| u.id.to_string());

        // Add request_id to response headers for client tracking
        response.set_header(Header::new("X-Request-Id", request_id.clone()));

        // Add security headers
        response.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        response.set_header(Header::new("X-Frame-Options", "DENY"));
        response.set_header(Header::new("Cache-Control", "no-store"));

        // Get slow_request_ms threshold from managed state
        let slow_request_ms = request
            .rocket()
            .state::<crate::config::Config>()
            .map(|c| c.logging.slow_request_ms)
            .unwrap_or(500);

        // Only escalate 5xx to WARN; 4xx responses (401, 403, 422, etc.) are
        // routine for a REST API and log at INFO to avoid WARN noise.
        let is_error = status.class().is_server_error();
        let is_slow = duration_ms > slow_request_ms;

        if is_error || is_slow {
            warn!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = status.code,
                duration_ms = duration_ms,
                request_bytes = request_bytes,
                response_bytes = response_bytes,
                user_id = user_id.as_deref().unwrap_or("-"),
                slow = if is_slow { Some(true) } else { None },
                "request completed{}{}",
                if is_error { " with error" } else { "" },
                if is_slow { " (slow)" } else { "" },
            );
        } else {
            info!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = status.code,
                duration_ms = duration_ms,
                request_bytes = request_bytes,
                response_bytes = response_bytes,
                user_id = user_id.as_deref().unwrap_or("-"),
                "request completed"
            );
        }
    }
}

// ── UserAgent guard ───────────────────────────────────────────────────────────

/// Extracts the `User-Agent` header value from the incoming request.
pub struct UserAgent(pub Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for UserAgent {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let ua = req.headers().get_one("User-Agent").map(|s| s.to_string());
        Outcome::Success(UserAgent(ua))
    }
}

impl<'a> OpenApiFromRequest<'a> for UserAgent {
    fn from_request_input(_gen: &mut OpenApiGenerator, _name: String, _required: bool) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}

// ── ClientIp guard ────────────────────────────────────────────────────────────

/// Extracts the client IP address from the incoming request.
pub struct ClientIp(pub Option<String>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientIp {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, ()> {
        let ip = req.client_ip().map(|ip| ip.to_string());
        Outcome::Success(ClientIp(ip))
    }
}

impl<'a> OpenApiFromRequest<'a> for ClientIp {
    fn from_request_input(_gen: &mut OpenApiGenerator, _name: String, _required: bool) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id_creation() {
        let request_id = RequestId::new();
        assert!(!request_id.0.is_empty());
        // Verify it's a valid UUID format
        assert!(Uuid::parse_str(&request_id.0).is_ok());
    }

    #[test]
    fn test_request_id_default() {
        let request_id = RequestId::default();
        assert!(!request_id.0.is_empty());
        assert!(Uuid::parse_str(&request_id.0).is_ok());
    }

    #[test]
    fn test_request_ids_are_unique() {
        let id1 = RequestId::new();
        let id2 = RequestId::new();
        assert_ne!(id1.0, id2.0);
    }

    #[test]
    fn test_server_errors_are_warn() {
        use rocket::http::Status;
        let cases = [Status::InternalServerError, Status::BadGateway, Status::ServiceUnavailable];
        for status in cases {
            assert!(status.class().is_server_error(), "{} should be server error", status.code);
        }
    }

    #[test]
    fn test_client_errors_are_not_warn() {
        use rocket::http::Status;
        let cases = [Status::Unauthorized, Status::Forbidden, Status::UnprocessableEntity, Status::NotFound];
        for status in cases {
            assert!(!status.class().is_server_error(), "{} should not trigger WARN", status.code);
        }
    }
}
