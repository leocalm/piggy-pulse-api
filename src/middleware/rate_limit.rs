use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::{RateLimitBackend, RateLimitConfig};
use tokio::sync::Mutex;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(not(test), allow(dead_code))]
enum RateLimitBucket {
    Read,
    Mutation,
    Auth,
}

#[cfg_attr(not(test), allow(dead_code))]
impl RateLimitBucket {
    fn as_str(&self) -> &'static str {
        match self {
            RateLimitBucket::Read => "read",
            RateLimitBucket::Mutation => "mutation",
            RateLimitBucket::Auth => "auth",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(not(test), allow(dead_code))]
enum RateLimitIdentity {
    Ip(String),
    User(String),
}

#[cfg_attr(not(test), allow(dead_code))]
impl RateLimitIdentity {
    fn key(&self) -> String {
        match self {
            RateLimitIdentity::Ip(ip) => format!("ip:{}", ip),
            RateLimitIdentity::User(user) => format!("user:{}", user),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(not(test), allow(dead_code))]
struct RateLimitKey {
    identity: RateLimitIdentity,
    bucket: RateLimitBucket,
}

#[derive(Debug, Clone)]
#[cfg_attr(not(test), allow(dead_code))]
struct Counter {
    window_start: Instant,
    count: u32,
}

pub(crate) struct RateLimiter {
    #[cfg_attr(not(test), allow(dead_code))]
    config: RateLimitConfig,
    window: Duration,
    cleanup_interval: Duration,
    backend: RateLimiterBackendImpl,
}

enum RateLimiterBackendImpl {
    Redis {
        #[allow(dead_code)]
        manager: redis::aio::ConnectionManager,
        #[allow(dead_code)]
        key_prefix: String,
    },
    InMemory {
        counters: Mutex<HashMap<RateLimitKey, Counter>>,
    },
}

impl RateLimiter {
    pub async fn new(config: RateLimitConfig) -> Result<Self, redis::RedisError> {
        let window = Duration::from_secs(config.window_seconds.max(1));
        let cleanup_interval = Duration::from_secs(config.cleanup_interval_seconds.max(1));

        let backend = match config.backend {
            RateLimitBackend::Redis => {
                let client = redis::Client::open(config.redis_url.as_str())?;
                let mut manager = redis::aio::ConnectionManager::new(client).await?;
                let _: () = redis::cmd("PING").query_async(&mut manager).await?;
                RateLimiterBackendImpl::Redis {
                    manager,
                    key_prefix: config.redis_key_prefix.clone(),
                }
            }
            RateLimitBackend::InMemory => RateLimiterBackendImpl::InMemory {
                counters: Mutex::new(HashMap::new()),
            },
        };

        Ok(Self {
            config,
            window,
            cleanup_interval,
            backend,
        })
    }

    pub fn spawn_cleanup_task(self: Arc<Self>) {
        if !matches!(self.backend, RateLimiterBackendImpl::InMemory { .. }) {
            return;
        }
        let cleanup_interval = self.cleanup_interval;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(cleanup_interval);
            loop {
                ticker.tick().await;
                let now = Instant::now();
                let window = self.window;
                if let RateLimiterBackendImpl::InMemory { counters } = &self.backend {
                    let mut counters = counters.lock().await;
                    counters.retain(|_, counter| now.duration_since(counter.window_start) < window);
                }
            }
        });
    }

    #[cfg_attr(not(test), allow(dead_code))]
    async fn check(&self, identities: &[RateLimitIdentity], bucket: RateLimitBucket) -> RateLimitDecision {
        if identities.is_empty() {
            return RateLimitDecision::Allow;
        }

        match &self.backend {
            RateLimiterBackendImpl::Redis { manager, key_prefix } => self.check_redis(manager, key_prefix, identities, bucket).await,
            RateLimiterBackendImpl::InMemory { counters } => self.check_in_memory(counters, identities, bucket).await,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn limit_for_bucket(&self, bucket: RateLimitBucket) -> u32 {
        match bucket {
            RateLimitBucket::Read => self.config.read_limit,
            RateLimitBucket::Mutation => self.config.mutation_limit,
            RateLimitBucket::Auth => self.config.auth_limit,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    async fn check_in_memory(
        &self,
        counters: &Mutex<HashMap<RateLimitKey, Counter>>,
        identities: &[RateLimitIdentity],
        bucket: RateLimitBucket,
    ) -> RateLimitDecision {
        let limit = self.limit_for_bucket(bucket);
        let now = Instant::now();
        let mut counters = counters.lock().await;
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

    #[cfg_attr(not(test), allow(dead_code))]
    async fn check_redis(
        &self,
        manager: &redis::aio::ConnectionManager,
        key_prefix: &str,
        identities: &[RateLimitIdentity],
        bucket: RateLimitBucket,
    ) -> RateLimitDecision {
        let limit = self.limit_for_bucket(bucket);
        let window_secs = self.window.as_secs().max(1);
        let keys: Vec<String> = identities
            .iter()
            .map(|identity| format!("{}{}:{}", key_prefix, bucket.as_str(), identity.key()))
            .collect();

        let mut conn = manager.clone();

        let counts: Vec<Option<u32>> = match redis::cmd("MGET").arg(&keys).query_async(&mut conn).await {
            Ok(counts) => counts,
            Err(err) => {
                warn!(error = %err, bucket = %bucket.as_str(), "rate limiter redis lookup failed");
                return self.redis_failure_decision(bucket);
            }
        };

        let mut retry_after: Option<Duration> = None;
        for (idx, count_opt) in counts.iter().enumerate() {
            let count = count_opt.unwrap_or(0);
            if count >= limit {
                let ttl_ms: i64 = match redis::cmd("PTTL").arg(&keys[idx]).query_async(&mut conn).await {
                    Ok(ttl_ms) => ttl_ms,
                    Err(err) => {
                        warn!(error = %err, bucket = %bucket.as_str(), "rate limiter redis ttl lookup failed");
                        return self.redis_failure_decision(bucket);
                    }
                };

                let ttl_secs = match ttl_ms {
                    -1 => {
                        warn!(
                            key = %keys[idx],
                            "rate limiter redis key has no expiry after EXPIRE NX; using window_secs as fallback"
                        );
                        window_secs
                    }
                    _ if ttl_ms <= 0 => window_secs,
                    _ => (ttl_ms as u64).div_ceil(1000),
                };

                let ttl = Duration::from_secs(ttl_secs.max(1));
                retry_after = Some(retry_after.map_or(ttl, |current| current.max(ttl)));
            }
        }

        if let Some(retry_after) = retry_after {
            return RateLimitDecision::Limited { retry_after };
        }

        let mut pipe = redis::pipe();
        for key in &keys {
            pipe.cmd("INCR").arg(key).ignore();
            pipe.cmd("EXPIRE").arg(key).arg(window_secs as usize).arg("NX").ignore();
        }

        if let Err(err) = pipe.query_async::<()>(&mut conn).await {
            warn!(error = %err, bucket = %bucket.as_str(), "rate limiter redis increment failed");
            return self.redis_failure_decision(bucket);
        }

        RateLimitDecision::Allow
    }

    #[cfg_attr(not(test), allow(dead_code))]
    fn redis_failure_decision(&self, bucket: RateLimitBucket) -> RateLimitDecision {
        if bucket == RateLimitBucket::Auth {
            return RateLimitDecision::Limited { retry_after: self.window };
        }

        RateLimitDecision::Allow
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
enum RateLimitDecision {
    Allow,
    Limited { retry_after: Duration },
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RateLimitRetryAfter(pub u64);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RateLimitConfig;
    use std::time::Duration;

    fn in_memory_config() -> RateLimitConfig {
        RateLimitConfig {
            backend: RateLimitBackend::InMemory,
            redis_key_prefix: "test:rate_limit:".to_string(),
            ..RateLimitConfig::default()
        }
    }

    #[rocket::async_test]
    async fn rate_limiter_blocks_after_limit() {
        let mut config = in_memory_config();
        config.read_limit = 2;
        config.mutation_limit = 1;
        config.auth_limit = 1;
        config.window_seconds = 60;
        config.cleanup_interval_seconds = 60;
        config.require_client_ip = false;

        let limiter = RateLimiter::new(config).await.expect("rate limiter");

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
        let mut config = in_memory_config();
        config.read_limit = 1;
        config.mutation_limit = 1;
        config.auth_limit = 1;
        config.window_seconds = 1;
        config.cleanup_interval_seconds = 60;
        config.require_client_ip = false;

        let limiter = RateLimiter::new(config).await.expect("rate limiter");

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
        let mut config = in_memory_config();
        config.read_limit = 10;
        config.mutation_limit = 1;
        config.auth_limit = 1;
        config.window_seconds = 60;
        config.cleanup_interval_seconds = 60;
        config.require_client_ip = false;

        let limiter = RateLimiter::new(config).await.expect("rate limiter");

        let identities = vec![RateLimitIdentity::Ip("127.0.0.1".to_string())];

        assert!(matches!(limiter.check(&identities, RateLimitBucket::Mutation).await, RateLimitDecision::Allow));
        assert!(matches!(
            limiter.check(&identities, RateLimitBucket::Mutation).await,
            RateLimitDecision::Limited { .. }
        ));
    }

    #[rocket::async_test]
    async fn rate_limiter_does_not_increment_when_limited() {
        let mut config = in_memory_config();
        config.read_limit = 1;
        config.mutation_limit = 1;
        config.auth_limit = 1;
        config.window_seconds = 60;
        config.cleanup_interval_seconds = 60;
        config.require_client_ip = false;

        let limiter = RateLimiter::new(config).await.expect("rate limiter");

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

    #[cfg(test)]
    impl RateLimiter {
        async fn count_for(&self, identity: RateLimitIdentity, bucket: RateLimitBucket) -> u32 {
            match &self.backend {
                RateLimiterBackendImpl::InMemory { counters } => {
                    let counters = counters.lock().await;
                    counters.get(&RateLimitKey { identity, bucket }).map(|counter| counter.count).unwrap_or(0)
                }
                RateLimiterBackendImpl::Redis { .. } => 0,
            }
        }
    }
}
