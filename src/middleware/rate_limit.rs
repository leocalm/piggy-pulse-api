use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::auth::parse_user_cookie_value;
use crate::config::RateLimitConfig;
use rocket::http::{Method, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket_okapi::r#gen::OpenApiGenerator;
use rocket_okapi::okapi::openapi3::{RefOr, Response as OpenApiResponse, Responses};
use rocket_okapi::request::{OpenApiFromRequest, RequestHeaderInput};
use tokio::sync::Mutex;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RateLimitBucket {
    Read,
    Mutation,
    Auth,
}

impl RateLimitBucket {
    fn from_method(method: Method) -> Self {
        match method {
            Method::Post | Method::Put | Method::Patch | Method::Delete => RateLimitBucket::Mutation,
            Method::Get | Method::Head | Method::Options | Method::Trace | Method::Connect => RateLimitBucket::Read,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum RateLimitIdentity {
    Ip(String),
    User(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RateLimitKey {
    identity: RateLimitIdentity,
    bucket: RateLimitBucket,
}

#[derive(Debug, Clone)]
struct Counter {
    window_start: Instant,
    count: u32,
}

#[derive(Debug)]
pub(crate) struct RateLimiter {
    config: RateLimitConfig,
    window: Duration,
    cleanup_interval: Duration,
    counters: Mutex<HashMap<RateLimitKey, Counter>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let window = Duration::from_secs(config.window_seconds.max(1));
        let cleanup_interval = Duration::from_secs(config.cleanup_interval_seconds.max(1));

        Self {
            config,
            window,
            cleanup_interval,
            counters: Mutex::new(HashMap::new()),
        }
    }

    pub fn spawn_cleanup_task(self: Arc<Self>) {
        let cleanup_interval = self.cleanup_interval;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(cleanup_interval);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                let window = self.window;
                let mut counters = self.counters.lock().await;
                counters.retain(|_, counter| now.duration_since(counter.window_start) < window);
            }
        });
    }

    async fn check(&self, identities: &[RateLimitIdentity], bucket: RateLimitBucket) -> RateLimitDecision {
        if identities.is_empty() {
            return RateLimitDecision::Allow;
        }

        // NOTE: This is a fixed-window counter; bursts can exceed the limit near window boundaries.
        let limit = self.limit_for_bucket(bucket);
        let now = Instant::now();
        let mut counters = self.counters.lock().await;
        let mut retry_after: Option<Duration> = None;

        for identity in identities {
            let key = RateLimitKey {
                identity: identity.clone(),
                bucket,
            };
            let counter = counters.entry(key).or_insert_with(|| Counter { window_start: now, count: 0 });

            if now.duration_since(counter.window_start) >= self.window {
                counter.window_start = now;
                counter.count = 0;
            }

            if counter.count >= limit {
                let elapsed = now.duration_since(counter.window_start);
                let remaining = self.window.saturating_sub(elapsed);
                retry_after = Some(retry_after.map_or(remaining, |current| current.max(remaining)));
            }
        }

        if let Some(retry_after) = retry_after {
            return RateLimitDecision::Limited { retry_after };
        }

        for identity in identities {
            let key = RateLimitKey {
                identity: identity.clone(),
                bucket,
            };
            if let Some(counter) = counters.get_mut(&key) {
                counter.count += 1;
            }
        }

        RateLimitDecision::Allow
    }

    fn limit_for_bucket(&self, bucket: RateLimitBucket) -> u32 {
        match bucket {
            RateLimitBucket::Read => self.config.read_limit,
            RateLimitBucket::Mutation => self.config.mutation_limit,
            RateLimitBucket::Auth => self.config.auth_limit,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateLimitDecision {
    Allow,
    Limited { retry_after: Duration },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RateLimit;

#[derive(Debug, Clone, Copy)]
pub(crate) struct AuthRateLimit;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RateLimitRetryAfter(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RateLimitError {
    TooManyRequests,
    MissingClientIp,
}

impl RateLimitError {
    fn status(self) -> Status {
        match self {
            RateLimitError::TooManyRequests => Status::TooManyRequests,
            RateLimitError::MissingClientIp => Status::BadRequest,
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RateLimit {
    type Error = RateLimitError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match rate_limit_request(request, RateLimitBucket::from_method(request.method())).await {
            Outcome::Success(_) => Outcome::Success(RateLimit),
            Outcome::Error(error) => Outcome::Error(error),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthRateLimit {
    type Error = RateLimitError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match rate_limit_request(request, RateLimitBucket::Auth).await {
            Outcome::Success(_) => Outcome::Success(AuthRateLimit),
            Outcome::Error(error) => Outcome::Error(error),
            Outcome::Forward(status) => Outcome::Forward(status),
        }
    }
}

impl<'a> OpenApiFromRequest<'a> for RateLimit {
    fn from_request_input(_gen: &mut OpenApiGenerator, _name: String, _required: bool) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }

    fn get_responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        too_many_requests_response()
    }
}

impl<'a> OpenApiFromRequest<'a> for AuthRateLimit {
    fn from_request_input(_gen: &mut OpenApiGenerator, _name: String, _required: bool) -> rocket_okapi::Result<RequestHeaderInput> {
        Ok(RequestHeaderInput::None)
    }

    fn get_responses(_gen: &mut OpenApiGenerator) -> rocket_okapi::Result<Responses> {
        too_many_requests_response()
    }
}

async fn rate_limit_request(request: &Request<'_>, bucket: RateLimitBucket) -> Outcome<(), RateLimitError> {
    let limiter = match request.rocket().state::<Arc<RateLimiter>>() {
        Some(limiter) => limiter,
        None => return Outcome::Success(()),
    };

    let request_id = request
        .local_cache(|| None::<crate::middleware::RequestId>)
        .as_ref()
        .map(|r| r.0.as_str())
        .unwrap_or("unknown");

    let ip = request.client_ip().map(|addr| addr.to_string());
    if ip.is_none() {
        warn!(
            request_id = %request_id,
            method = %request.method(),
            uri = %request.uri(),
            "client ip unavailable for rate limiting"
        );
    }

    let mut identities = Vec::new();
    if let Some(ip) = ip {
        identities.push(RateLimitIdentity::Ip(ip));
    }
    if let Some(user_id) = extract_user_id(request) {
        identities.push(RateLimitIdentity::User(user_id));
    }

    if identities.is_empty() {
        if limiter.config.require_client_ip {
            return Outcome::Error((RateLimitError::MissingClientIp.status(), RateLimitError::MissingClientIp));
        }
        identities.push(RateLimitIdentity::Ip("missing-ip".to_string()));
    }

    match limiter.check(&identities, bucket).await {
        RateLimitDecision::Allow => Outcome::Success(()),
        RateLimitDecision::Limited { retry_after } => {
            let retry_after_secs = retry_after.as_secs().max(1);
            request.local_cache(|| Some(RateLimitRetryAfter(retry_after_secs)));
            warn!(
                request_id = %request_id,
                method = %request.method(),
                uri = %request.uri(),
                retry_after_secs = %retry_after_secs,
                "rate limit exceeded"
            );
            Outcome::Error((RateLimitError::TooManyRequests.status(), RateLimitError::TooManyRequests))
        }
    }
}

fn extract_user_id(request: &Request<'_>) -> Option<String> {
    let cookie = request.cookies().get_private("user")?;
    let (id, _) = parse_user_cookie_value(cookie.value())?;
    Some(id.to_string())
}

fn too_many_requests_response() -> rocket_okapi::Result<Responses> {
    let mut responses = Responses::default();
    responses.responses.insert(
        "429".to_string(),
        RefOr::Object(OpenApiResponse {
            description: "Too Many Requests".to_string(),
            ..Default::default()
        }),
    );
    Ok(responses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routes::error::too_many_requests;
    use rocket::http::{ContentType, Status};
    use rocket::local::asynchronous::Client;
    use rocket::{catchers, get, routes};

    #[get("/limited")]
    async fn limited(_rate_limit: RateLimit) -> Status {
        Status::Ok
    }

    #[rocket::async_test]
    async fn rate_limiter_blocks_after_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            read_limit: 2,
            mutation_limit: 1,
            auth_limit: 1,
            window_seconds: 60,
            cleanup_interval_seconds: 60,
            require_client_ip: false,
        });

        let identities = vec![RateLimitIdentity::Ip("127.0.0.1".to_string())];

        assert!(matches!(limiter.check(&identities, RateLimitBucket::Read).await, RateLimitDecision::Allow));
        assert!(matches!(limiter.check(&identities, RateLimitBucket::Read).await, RateLimitDecision::Allow));
        assert!(matches!(
            limiter.check(&identities, RateLimitBucket::Read).await,
            RateLimitDecision::Limited { .. }
        ));
    }

    #[rocket::async_test]
    async fn rate_limiter_resets_after_window() {
        let limiter = RateLimiter::new(RateLimitConfig {
            read_limit: 1,
            mutation_limit: 1,
            auth_limit: 1,
            window_seconds: 1,
            cleanup_interval_seconds: 60,
            require_client_ip: false,
        });

        let identities = vec![RateLimitIdentity::Ip("127.0.0.1".to_string())];
        assert!(matches!(limiter.check(&identities, RateLimitBucket::Read).await, RateLimitDecision::Allow));
        assert!(matches!(
            limiter.check(&identities, RateLimitBucket::Read).await,
            RateLimitDecision::Limited { .. }
        ));

        tokio::time::sleep(Duration::from_millis(1100)).await;

        assert!(matches!(limiter.check(&identities, RateLimitBucket::Read).await, RateLimitDecision::Allow));
    }

    #[rocket::async_test]
    async fn mutation_bucket_enforced() {
        let limiter = RateLimiter::new(RateLimitConfig {
            read_limit: 10,
            mutation_limit: 1,
            auth_limit: 1,
            window_seconds: 60,
            cleanup_interval_seconds: 60,
            require_client_ip: false,
        });

        let identities = vec![RateLimitIdentity::Ip("127.0.0.1".to_string())];

        assert!(matches!(limiter.check(&identities, RateLimitBucket::Mutation).await, RateLimitDecision::Allow));
        assert!(matches!(
            limiter.check(&identities, RateLimitBucket::Mutation).await,
            RateLimitDecision::Limited { .. }
        ));
    }

    #[rocket::async_test]
    async fn rate_limiter_does_not_increment_when_limited() {
        let limiter = RateLimiter::new(RateLimitConfig {
            read_limit: 1,
            mutation_limit: 1,
            auth_limit: 1,
            window_seconds: 60,
            cleanup_interval_seconds: 60,
            require_client_ip: false,
        });

        let ip = RateLimitIdentity::Ip("10.0.0.1".to_string());
        let user = RateLimitIdentity::User("user-1".to_string());
        let identities = vec![ip.clone(), user.clone()];

        assert!(matches!(limiter.check(&identities, RateLimitBucket::Read).await, RateLimitDecision::Allow));
        assert!(matches!(
            limiter.check(&identities, RateLimitBucket::Read).await,
            RateLimitDecision::Limited { .. }
        ));

        let ip_count = limiter.count_for(ip, RateLimitBucket::Read).await;
        let user_count = limiter.count_for(user, RateLimitBucket::Read).await;

        assert_eq!(ip_count, 1);
        assert_eq!(user_count, 1);
    }

    #[test]
    fn rate_limit_bucket_from_method() {
        assert_eq!(RateLimitBucket::from_method(Method::Get), RateLimitBucket::Read);
        assert_eq!(RateLimitBucket::from_method(Method::Head), RateLimitBucket::Read);
        assert_eq!(RateLimitBucket::from_method(Method::Options), RateLimitBucket::Read);
        assert_eq!(RateLimitBucket::from_method(Method::Trace), RateLimitBucket::Read);
        assert_eq!(RateLimitBucket::from_method(Method::Connect), RateLimitBucket::Read);
        assert_eq!(RateLimitBucket::from_method(Method::Post), RateLimitBucket::Mutation);
        assert_eq!(RateLimitBucket::from_method(Method::Put), RateLimitBucket::Mutation);
        assert_eq!(RateLimitBucket::from_method(Method::Patch), RateLimitBucket::Mutation);
        assert_eq!(RateLimitBucket::from_method(Method::Delete), RateLimitBucket::Mutation);
    }

    #[rocket::async_test]
    async fn rate_limit_retry_after_header_is_set() {
        let limiter = Arc::new(RateLimiter::new(RateLimitConfig {
            read_limit: 0,
            mutation_limit: 0,
            auth_limit: 0,
            window_seconds: 60,
            cleanup_interval_seconds: 60,
            require_client_ip: false,
        }));

        let rocket = rocket::build()
            .manage(limiter)
            .mount("/", routes![limited])
            .register("/", catchers![too_many_requests]);

        let client = Client::tracked(rocket).await.expect("valid rocket instance");
        let response = client.get("/limited").dispatch().await;

        assert_eq!(response.status(), Status::TooManyRequests);
        assert_eq!(response.headers().get_one("Retry-After"), Some("60"));
        assert_eq!(response.content_type(), Some(ContentType::JSON));
    }

    #[cfg(test)]
    impl RateLimiter {
        async fn count_for(&self, identity: RateLimitIdentity, bucket: RateLimitBucket) -> u32 {
            let counters = self.counters.lock().await;
            counters.get(&RateLimitKey { identity, bucket }).map(|counter| counter.count).unwrap_or(0)
        }
    }
}
