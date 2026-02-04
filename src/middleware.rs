use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Data, Response};
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
        let method = request.method();
        let uri = request.uri();

        // Store request_id in local_cache for later retrieval
        request.local_cache(|| Some(request_id.clone()));

        info!(
            request_id = %request_id.0,
            method = %method,
            uri = %uri,
            "incoming request"
        );
    }

    async fn on_response<'r>(&self, request: &'r Request<'_>, response: &mut Response<'r>) {
        let request_id = request
            .local_cache(|| None::<RequestId>)
            .as_ref()
            .map(|r| r.0.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let status = response.status();
        let method = request.method();
        let uri = request.uri();

        // Add request_id to response headers for client tracking
        response.set_header(Header::new("X-Request-Id", request_id.clone()));

        // Log response with appropriate level based on status
        if status.class().is_server_error() || status.class().is_client_error() {
            warn!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = %status.code,
                "request completed with error"
            );
        } else {
            info!(
                request_id = %request_id,
                method = %method,
                uri = %uri,
                status = %status.code,
                "request completed"
            );
        }
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
}
