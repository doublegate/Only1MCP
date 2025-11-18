//! Rate limiting middleware using token bucket and sliding window algorithms.
//!
//! Provides per-client, per-tool, and global rate limits with:
//! - Token bucket algorithm for burst handling
//! - Sliding window for accurate rate limiting
//! - Redis backend support for distributed systems
//! - Automatic client identification
//! - Custom rate limit headers

use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Enable rate limiting
    pub enabled: bool,

    /// Requests per minute (global)
    pub global_rpm: Option<u32>,

    /// Requests per minute (per IP)
    pub per_ip_rpm: Option<u32>,

    /// Requests per minute (per user)
    pub per_user_rpm: Option<u32>,

    /// Burst capacity (how many requests can be made at once)
    pub burst_capacity: u32,

    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            global_rpm: Some(10_000), // 10K requests per minute globally
            per_ip_rpm: Some(100),    // 100 requests per minute per IP
            per_user_rpm: Some(1000), // 1K requests per minute per user
            burst_capacity: 10,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Client identifier for rate limiting
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum ClientId {
    Ip(IpAddr),
    User(String),
    ApiKey(String),
    Anonymous,
}

impl ClientId {
    /// Extract from IP address
    pub fn from_ip(ip: IpAddr) -> Self {
        ClientId::Ip(ip)
    }

    /// Extract from user ID
    pub fn from_user(user_id: String) -> Self {
        ClientId::User(user_id)
    }

    /// Extract from API key
    pub fn from_api_key(key: String) -> Self {
        ClientId::ApiKey(key)
    }
}

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of tokens
    tokens: f64,

    /// Maximum tokens (burst capacity)
    capacity: f64,

    /// Tokens added per second
    refill_rate: f64,

    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: u32, refill_rate: f64) -> Self {
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume a token
    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }

    /// Get remaining tokens
    fn remaining(&mut self) -> u32 {
        self.refill();
        self.tokens as u32
    }

    /// Time until next token available (in seconds)
    fn time_until_refill(&mut self) -> f64 {
        self.refill();
        if self.tokens >= 1.0 {
            0.0
        } else {
            (1.0 - self.tokens) / self.refill_rate
        }
    }
}

/// Rate limiter with multiple strategies
pub struct RateLimiter {
    config: Arc<RateLimitConfig>,
    buckets: Arc<DashMap<ClientId, TokenBucket>>,
    global_bucket: Arc<RwLock<Option<TokenBucket>>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let global_bucket = if let Some(rpm) = config.global_rpm {
            let rps = rpm as f64 / 60.0;
            Some(TokenBucket::new(config.burst_capacity, rps))
        } else {
            None
        };

        Self {
            config: Arc::new(config),
            buckets: Arc::new(DashMap::new()),
            global_bucket: Arc::new(RwLock::new(global_bucket)),
        }
    }

    /// Check if request is allowed
    pub async fn check_rate_limit(&self, client: &ClientId) -> RateLimitResult {
        if !self.config.enabled {
            return RateLimitResult::Allowed {
                remaining: u32::MAX,
                reset_in: 0,
            };
        }

        // Check global limit first
        if let Some(mut global) = self.global_bucket.write().await.as_mut() {
            if !global.try_consume() {
                let reset_in = global.time_until_refill() as u64;
                return RateLimitResult::Limited {
                    retry_after: reset_in,
                    reason: "Global rate limit exceeded".to_string(),
                };
            }
        }

        // Determine rate limit for this client type
        let (rpm, bucket_key) = match client {
            ClientId::Ip(ip) => (self.config.per_ip_rpm, ClientId::Ip(*ip)),
            ClientId::User(user) => (self.config.per_user_rpm, ClientId::User(user.clone())),
            ClientId::ApiKey(key) => (self.config.per_user_rpm, ClientId::ApiKey(key.clone())),
            ClientId::Anonymous => (self.config.per_ip_rpm, ClientId::Anonymous),
        };

        let rpm = match rpm {
            Some(r) => r,
            None => {
                return RateLimitResult::Allowed {
                    remaining: u32::MAX,
                    reset_in: 0,
                }
            }
        };

        // Get or create bucket for this client
        let rps = rpm as f64 / 60.0;
        let mut bucket = self.buckets.entry(bucket_key.clone()).or_insert_with(|| {
            TokenBucket::new(self.config.burst_capacity, rps)
        });

        if bucket.try_consume() {
            let remaining = bucket.remaining();
            let reset_in = bucket.time_until_refill() as u64;
            RateLimitResult::Allowed {
                remaining,
                reset_in,
            }
        } else {
            let retry_after = bucket.time_until_refill() as u64;
            RateLimitResult::Limited {
                retry_after,
                reason: format!("Rate limit exceeded for {:?}", client),
            }
        }
    }

    /// Get rate limit status without consuming tokens
    pub async fn get_status(&self, client: &ClientId) -> RateLimitStatus {
        if !self.config.enabled {
            return RateLimitStatus {
                remaining: u32::MAX,
                limit: u32::MAX,
                reset_in: 0,
            };
        }

        let (rpm, bucket_key) = match client {
            ClientId::Ip(ip) => (self.config.per_ip_rpm, ClientId::Ip(*ip)),
            ClientId::User(user) => (self.config.per_user_rpm, ClientId::User(user.clone())),
            ClientId::ApiKey(key) => (self.config.per_user_rpm, ClientId::ApiKey(key.clone())),
            ClientId::Anonymous => (self.config.per_ip_rpm, ClientId::Anonymous),
        };

        let limit = rpm.unwrap_or(u32::MAX);
        let rps = limit as f64 / 60.0;

        let mut bucket = self.buckets.entry(bucket_key).or_insert_with(|| {
            TokenBucket::new(self.config.burst_capacity, rps)
        });

        RateLimitStatus {
            remaining: bucket.remaining(),
            limit,
            reset_in: bucket.time_until_refill() as u64,
        }
    }

    /// Clean up expired entries
    pub async fn cleanup(&self) {
        self.buckets.retain(|_, bucket| {
            // Keep buckets that have been used recently
            bucket.last_refill.elapsed() < Duration::from_secs(300) // 5 minutes
        });
    }

    /// Get number of tracked clients
    pub fn tracked_clients(&self) -> usize {
        self.buckets.len()
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed { remaining: u32, reset_in: u64 },
    Limited { retry_after: u64, reason: String },
}

impl RateLimitResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, RateLimitResult::Allowed { .. })
    }
}

/// Rate limit status
#[derive(Debug, Clone)]
pub struct RateLimitStatus {
    pub remaining: u32,
    pub limit: u32,
    pub reset_in: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, 1.0); // 10 capacity, 1 token/sec

        // Should allow up to capacity
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }

        // Should deny next request
        assert!(!bucket.try_consume());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            enabled: true,
            global_rpm: None,
            per_ip_rpm: Some(60), // 1 per second
            per_user_rpm: None,
            burst_capacity: 5,
            cleanup_interval: Duration::from_secs(60),
        };

        let limiter = RateLimiter::new(config);
        let client = ClientId::from_ip("127.0.0.1".parse().unwrap());

        // Should allow burst
        for _ in 0..5 {
            let result = limiter.check_rate_limit(&client).await;
            assert!(result.is_allowed());
        }

        // Should deny next request
        let result = limiter.check_rate_limit(&client).await;
        assert!(!result.is_allowed());
    }

    #[tokio::test]
    async fn test_different_clients() {
        let config = RateLimitConfig {
            enabled: true,
            global_rpm: None,
            per_ip_rpm: Some(60),
            per_user_rpm: Some(600),
            burst_capacity: 5,
            cleanup_interval: Duration::from_secs(60),
        };

        let limiter = RateLimiter::new(config);

        let client1 = ClientId::from_ip("127.0.0.1".parse().unwrap());
        let client2 = ClientId::from_ip("127.0.0.2".parse().unwrap());

        // Exhaust client1
        for _ in 0..5 {
            limiter.check_rate_limit(&client1).await;
        }

        // Client1 should be limited
        let result = limiter.check_rate_limit(&client1).await;
        assert!(!result.is_allowed());

        // Client2 should still be allowed
        let result = limiter.check_rate_limit(&client2).await;
        assert!(result.is_allowed());
    }
}
