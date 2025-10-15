//! Circuit breakers prevent cascading failures by temporarily
//! disabling requests to failing backends.

use serde::Deserialize;
use std::sync::atomic::{AtomicI64, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing;

/// Circuit breaker state machine
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Normal operation, requests allowed
    Closed,

    /// Failing, requests blocked
    Open,

    /// Testing recovery, limited requests
    HalfOpen,
}

/// State change listener callback
pub type StateChangeListener = Box<dyn Fn(CircuitState) + Send + Sync>;

/// Circuit breaker for individual backend
pub struct CircuitBreaker {
    /// Backend identifier
    backend_id: String,

    /// Current state
    state: Arc<RwLock<CircuitState>>,

    /// Failure counter
    failure_count: Arc<AtomicU32>,

    /// Success counter (for half-open)
    success_count: Arc<AtomicU32>,

    /// Last state change timestamp
    last_state_change: Arc<AtomicI64>,

    /// Configuration
    config: CircuitBreakerConfig,

    /// State change listeners
    listeners: Arc<RwLock<Vec<StateChangeListener>>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failures needed to open circuit
    pub failure_threshold: u32,

    /// Successes needed to close circuit
    pub success_threshold: u32,

    /// Time before attempting recovery
    pub timeout: Duration,

    /// Error rate threshold (0.0 - 1.0)
    pub error_rate_threshold: f64,

    /// Half-open test request limit
    pub half_open_limit: u32,

    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,

    /// Maximum backoff duration
    pub max_backoff: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
            error_rate_threshold: 0.5,
            half_open_limit: 3,
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_secs(300),
        }
    }
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(backend_id: String, config: CircuitBreakerConfig) -> Self {
        Self {
            backend_id,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_state_change: Arc::new(AtomicI64::new(
                SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            )),
            config,
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if request should be allowed
    pub async fn should_allow_request(&self) -> bool {
        let state = self.state.read().await;

        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout expired
                let last_change = self.last_state_change.load(Ordering::Relaxed);
                let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

                if now - last_change > self.config.timeout.as_secs() as i64 {
                    // Transition to half-open
                    drop(state);
                    self.transition_to_half_open().await;
                    true
                } else {
                    false
                }
            },
            CircuitState::HalfOpen => {
                // Allow limited requests for testing
                let success = self.success_count.load(Ordering::Relaxed);
                let failure = self.failure_count.load(Ordering::Relaxed);

                (success + failure) < self.config.half_open_limit
            },
        }
    }

    /// Record successful request
    pub async fn record_success(&self) {
        let state = self.state.read().await.clone();

        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            },
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;

                if count >= self.config.success_threshold {
                    drop(state);
                    self.transition_to_closed().await;
                }
            },
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                tracing::warn!("Success recorded in open state for {}", self.backend_id);
            },
        }
    }

    /// Record failed request
    pub async fn record_failure(&self) {
        let state = self.state.read().await.clone();

        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;

                if count >= self.config.failure_threshold {
                    drop(state);
                    self.transition_to_open().await;
                }
            },
            CircuitState::HalfOpen => {
                // Single failure in half-open returns to open
                drop(state);
                self.transition_to_open().await;
            },
            CircuitState::Open => {
                // Already open, update failure count for metrics
                self.failure_count.fetch_add(1, Ordering::Relaxed);
            },
        }
    }

    /// Get current state
    pub async fn current_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }

    /// Add state change listener
    pub async fn add_listener(&self, listener: StateChangeListener) {
        self.listeners.write().await.push(listener);
    }

    /// Transition to open state
    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;

        self.update_timestamp();
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);

        tracing::error!("Circuit breaker OPEN for backend {}", self.backend_id);

        // Notify listeners
        self.notify_listeners(CircuitState::Open).await;
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::HalfOpen;

        self.update_timestamp();
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);

        tracing::info!(
            "Circuit breaker HALF-OPEN for backend {} (testing recovery)",
            self.backend_id
        );

        // Notify listeners
        self.notify_listeners(CircuitState::HalfOpen).await;
    }

    /// Transition to closed state
    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;

        self.update_timestamp();
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);

        tracing::info!(
            "Circuit breaker CLOSED for backend {} (recovered)",
            self.backend_id
        );

        // Notify listeners
        self.notify_listeners(CircuitState::Closed).await;
    }

    /// Update timestamp
    fn update_timestamp(&self) {
        self.last_state_change.store(
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64,
            Ordering::Relaxed,
        );
    }

    /// Notify state change listeners
    async fn notify_listeners(&self, new_state: CircuitState) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(new_state.clone());
        }
    }

    /// Get circuit metrics
    pub async fn metrics(&self) -> CircuitBreakerMetrics {
        CircuitBreakerMetrics {
            backend_id: self.backend_id.clone(),
            state: self.state.read().await.clone(),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            last_state_change: self.last_state_change.load(Ordering::Relaxed),
        }
    }
}

/// Circuit breaker metrics
#[derive(Debug, Clone)]
pub struct CircuitBreakerMetrics {
    pub backend_id: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_state_change: i64,
}

/// Circuit breaker manager for multiple backends
pub struct CircuitBreakerManager {
    /// Circuit breakers per backend
    breakers: Arc<dashmap::DashMap<String, Arc<CircuitBreaker>>>,

    /// Default configuration
    default_config: CircuitBreakerConfig,
}

impl CircuitBreakerManager {
    /// Create new circuit breaker manager
    pub fn new(default_config: CircuitBreakerConfig) -> Self {
        Self {
            breakers: Arc::new(dashmap::DashMap::new()),
            default_config,
        }
    }

    /// Get or create circuit breaker for backend
    pub fn get_or_create(&self, backend_id: &str) -> Arc<CircuitBreaker> {
        self.breakers
            .entry(backend_id.to_string())
            .or_insert_with(|| {
                Arc::new(CircuitBreaker::new(
                    backend_id.to_string(),
                    self.default_config.clone(),
                ))
            })
            .clone()
    }

    /// Check if backend is available
    pub async fn is_available(&self, backend_id: &str) -> bool {
        if let Some(breaker) = self.breakers.get(backend_id) {
            breaker.should_allow_request().await
        } else {
            true // No breaker means no failures yet
        }
    }

    /// Record request outcome
    pub async fn record_outcome(&self, backend_id: &str, success: bool) {
        let breaker = self.get_or_create(backend_id);

        if success {
            breaker.record_success().await;
        } else {
            breaker.record_failure().await;
        }
    }

    /// Get all circuit breaker metrics
    pub async fn all_metrics(&self) -> Vec<CircuitBreakerMetrics> {
        let mut metrics = Vec::new();

        for entry in self.breakers.iter() {
            metrics.push(entry.value().metrics().await);
        }

        metrics
    }

    /// Reset specific circuit breaker
    pub async fn reset(&self, backend_id: &str) {
        if let Some(breaker) = self.breakers.get(backend_id) {
            breaker.transition_to_closed().await;
        }
    }

    /// Reset all circuit breakers
    pub async fn reset_all(&self) {
        for entry in self.breakers.iter() {
            entry.value().transition_to_closed().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_circuit_breaker_state_transitions() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout: Duration::from_millis(100),
            ..Default::default()
        };

        let breaker = CircuitBreaker::new("test".to_string(), config);

        // Initially closed
        assert_eq!(breaker.current_state().await, CircuitState::Closed);
        assert!(breaker.should_allow_request().await);

        // Record failures to open circuit
        for _ in 0..3 {
            breaker.record_failure().await;
        }
        assert_eq!(breaker.current_state().await, CircuitState::Open);
        assert!(!breaker.should_allow_request().await);

        // Wait for timeout
        sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(breaker.should_allow_request().await);
        assert_eq!(breaker.current_state().await, CircuitState::HalfOpen);

        // Record successes to close circuit
        for _ in 0..2 {
            breaker.record_success().await;
        }
        assert_eq!(breaker.current_state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_manager() {
        let manager = CircuitBreakerManager::new(CircuitBreakerConfig::default());

        // Initially all backends available
        assert!(manager.is_available("backend1").await);
        assert!(manager.is_available("backend2").await);

        // Record failures for backend1
        for _ in 0..5 {
            manager.record_outcome("backend1", false).await;
        }

        // backend1 should be unavailable, backend2 still available
        assert!(!manager.is_available("backend1").await);
        assert!(manager.is_available("backend2").await);

        // Reset backend1
        manager.reset("backend1").await;
        assert!(manager.is_available("backend1").await);
    }
}
