//! Health checking combines active probes with passive request monitoring
//! to quickly detect failures while minimizing overhead.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::{interval, MissedTickBehavior};
use tracing::{debug, error, info, warn};

/// Comprehensive health check manager
pub struct HealthCheckManager {
    /// Active health checkers per backend (for future active health checking feature)
    _checkers: Arc<DashMap<String, Arc<HealthChecker>>>,

    /// Health status cache (for future health status caching feature)
    _status_cache: Arc<DashMap<String, HealthStatus>>,

    /// Configuration (for future dynamic health check configuration)
    _config: HealthConfig,

    /// Metrics (for future health check metrics)
    _metrics: Arc<HealthMetrics>,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall health state
    pub state: HealthState,

    /// Last successful check
    pub last_success: Instant,

    /// Last failed check
    pub last_failure: Option<Instant>,

    /// Consecutive successes
    pub success_count: u32,

    /// Consecutive failures
    pub failure_count: u32,

    /// Average latency (ms)
    pub avg_latency: f64,

    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,

    /// Resource usage
    pub resources: ResourceMetrics,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthStatus {
    /// Create a new health status
    pub fn new() -> Self {
        Self {
            state: HealthState::Unknown,
            last_success: Instant::now(),
            last_failure: None,
            success_count: 0,
            failure_count: 0,
            avg_latency: 0.0,
            error_rate: 0.0,
            resources: ResourceMetrics::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthState {
    /// Service is healthy and accepting requests
    Healthy,

    /// Service is degraded but operational
    Degraded,

    /// Service is unhealthy and not accepting requests
    Unhealthy,

    /// Health status unknown (not checked yet)
    Unknown,
}

impl HealthState {
    pub fn is_healthy(&self) -> bool {
        matches!(self, HealthState::Healthy | HealthState::Degraded)
    }
}

/// Transport type for health checking
#[derive(Debug, Clone)]
pub enum HealthCheckTransport {
    Http { endpoint: String },
    Stdio { command: String, args: Vec<String> },
}

/// Individual backend health checker
pub struct HealthChecker {
    /// Backend identifier
    backend_id: String,

    /// Transport configuration
    transport: HealthCheckTransport,

    /// Health check path (for HTTP)
    health_path: String,

    /// Check interval
    interval: Duration,

    /// Check timeout
    timeout: Duration,

    /// Failure threshold (fall)
    failure_threshold: u32,

    /// Success threshold (rise)
    success_threshold: u32,

    /// Current status
    status: Arc<RwLock<HealthStatus>>,

    /// HTTP client for checks
    http_client: reqwest::Client,

    /// Circuit breaker integration (optional)
    circuit_breaker: Option<Arc<crate::health::circuit_breaker::CircuitBreakerManager>>,

    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
}

impl HealthChecker {
    /// Create a new health checker from config module's HealthCheckConfig
    pub fn from_config(
        backend_id: String,
        transport: HealthCheckTransport,
        config: crate::config::HealthCheckConfig,
    ) -> Self {
        Self {
            backend_id,
            transport,
            health_path: config.path,
            interval: Duration::from_secs(config.interval_seconds),
            timeout: Duration::from_secs(config.timeout_seconds),
            failure_threshold: config.unhealthy_threshold,
            success_threshold: config.healthy_threshold,
            status: Arc::new(RwLock::new(HealthStatus::new())),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(config.timeout_seconds))
                .build()
                .unwrap(),
            circuit_breaker: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new health checker (legacy method using local HealthCheckConfig)
    pub fn new(
        backend_id: String,
        transport: HealthCheckTransport,
        config: HealthCheckConfig,
    ) -> Self {
        Self {
            backend_id,
            transport,
            health_path: "/health".to_string(), // Default path
            interval: config.interval,
            timeout: config.timeout,
            failure_threshold: config.failure_threshold,
            success_threshold: config.success_threshold,
            status: Arc::new(RwLock::new(HealthStatus::new())),
            http_client: reqwest::Client::builder().timeout(config.timeout).build().unwrap(),
            circuit_breaker: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set circuit breaker for integration
    pub fn with_circuit_breaker(
        mut self,
        circuit_breaker: Arc<crate::health::circuit_breaker::CircuitBreakerManager>,
    ) -> Self {
        self.circuit_breaker = Some(circuit_breaker);
        self
    }

    /// Start continuous health checking
    pub async fn start(self: Arc<Self>) {
        let mut interval = interval(self.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        while !self.shutdown.load(Ordering::Relaxed) {
            interval.tick().await;

            // Perform health check
            let result = self.perform_check().await;

            // Update status
            self.update_status(result).await;
        }
    }

    /// Perform single health check
    async fn perform_check(&self) -> HealthCheckResult {
        let start = Instant::now();

        match &self.transport {
            HealthCheckTransport::Http { endpoint } => {
                self.perform_http_check(endpoint, start).await
            },
            HealthCheckTransport::Stdio { command, args } => {
                self.perform_stdio_check(command, args, start).await
            },
        }
    }

    /// Perform HTTP health check
    async fn perform_http_check(&self, endpoint: &str, start: Instant) -> HealthCheckResult {
        // Build health endpoint URL
        let health_url = if endpoint.ends_with('/') {
            format!("{}{}", endpoint, self.health_path.trim_start_matches('/'))
        } else {
            format!("{}{}", endpoint, self.health_path)
        };

        // Determine if MCP-specific health check or generic HTTP
        let request = if health_url.contains("/mcp/health") || health_url.contains("/health") {
            // Try MCP-specific JSON-RPC health check first
            self.http_client
                .post(&health_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "health/check",
                    "params": {}
                }))
                .timeout(self.timeout)
        } else {
            // Generic HTTP GET health check
            self.http_client.get(&health_url).timeout(self.timeout)
        };

        match request.send().await {
            Ok(response) => {
                let latency = start.elapsed();

                if response.status().is_success() {
                    // Parse health response if available
                    if let Ok(body) = response.json::<HealthResponse>().await {
                        HealthCheckResult::Success {
                            latency,
                            details: Some(body),
                        }
                    } else {
                        HealthCheckResult::Success {
                            latency,
                            details: None,
                        }
                    }
                } else {
                    HealthCheckResult::Failure {
                        reason: format!("HTTP {}", response.status()),
                        _latency: Some(latency),
                    }
                }
            },
            Err(e) => {
                let latency = start.elapsed();

                HealthCheckResult::Failure {
                    reason: if e.is_timeout() {
                        format!("Timeout after {:?}", self.timeout)
                    } else if e.is_connect() {
                        "Connection refused".to_string()
                    } else {
                        e.to_string()
                    },
                    _latency: Some(latency),
                }
            },
        }
    }

    /// Perform STDIO process health check
    async fn perform_stdio_check(
        &self,
        command: &str,
        args: &[String],
        start: Instant,
    ) -> HealthCheckResult {
        use tokio::process::Command;

        // Check if command exists using 'which' crate
        if which::which(command).is_err() {
            return HealthCheckResult::Failure {
                reason: format!("Command '{}' not found in PATH", command),
                _latency: Some(start.elapsed()),
            };
        }

        // Try to execute a simple test command (e.g., --version or --help)
        // This validates that the process can be spawned
        let test_result = tokio::time::timeout(
            self.timeout,
            Command::new(command).args(args).arg("--version").output(),
        )
        .await;

        match test_result {
            Ok(Ok(output)) => {
                let latency = start.elapsed();

                if output.status.success() {
                    HealthCheckResult::Success {
                        latency,
                        details: None,
                    }
                } else {
                    // Process exists but returned non-zero exit code
                    // This is still considered "alive" for STDIO processes
                    HealthCheckResult::Success {
                        latency,
                        details: None,
                    }
                }
            },
            Ok(Err(e)) => HealthCheckResult::Failure {
                reason: format!("Failed to spawn process: {}", e),
                _latency: Some(start.elapsed()),
            },
            Err(_) => HealthCheckResult::Failure {
                reason: format!("Process health check timed out after {:?}", self.timeout),
                _latency: Some(start.elapsed()),
            },
        }
    }

    /// Update health status based on check result
    async fn update_status(&self, result: HealthCheckResult) {
        use crate::metrics::HEALTH_CHECK_TOTAL;

        let mut status = self.status.write().await;

        match result {
            HealthCheckResult::Success { latency, details } => {
                // Record successful check
                HEALTH_CHECK_TOTAL.with_label_values(&[&self.backend_id, "success"]).inc();

                status.last_success = Instant::now();
                status.success_count += 1;
                status.failure_count = 0;

                // Update latency moving average
                status.avg_latency =
                    (status.avg_latency * 0.9) + (latency.as_millis() as f64 * 0.1);

                // Update error rate (exponential decay)
                status.error_rate *= 0.95;

                // Parse resource metrics if available
                if let Some(details) = details {
                    status.resources = details.resources;
                }

                // Determine state based on thresholds
                let previous_state = status.state.clone();
                if status.success_count >= self.success_threshold
                    && status.state != HealthState::Healthy
                {
                    info!("Backend {} is now healthy", self.backend_id);
                    status.state = HealthState::Healthy;
                }

                // Notify circuit breaker of successful check
                if let Some(cb) = &self.circuit_breaker {
                    cb.record_outcome(&self.backend_id, true).await;

                    // Reset circuit breaker when transitioning to healthy
                    if previous_state != HealthState::Healthy
                        && status.state == HealthState::Healthy
                    {
                        cb.reset(&self.backend_id).await;
                    }
                }
            },
            HealthCheckResult::Failure {
                reason,
                _latency: _,
            } => {
                // Record failed check
                HEALTH_CHECK_TOTAL.with_label_values(&[&self.backend_id, "failure"]).inc();

                status.last_failure = Some(Instant::now());
                status.failure_count += 1;
                status.success_count = 0;

                // Update error rate
                status.error_rate = (status.error_rate * 0.9) + 0.1;

                // Log failure reason
                warn!("Health check failed for {}: {}", self.backend_id, reason);

                // Determine state based on thresholds
                let previous_state = status.state.clone();
                if status.failure_count >= self.failure_threshold {
                    if status.state != HealthState::Unhealthy {
                        error!("Backend {} is now unhealthy", self.backend_id);
                        status.state = HealthState::Unhealthy;

                        // Notify circuit breaker to open (block traffic)
                        // Nesting required for: threshold check → state change → circuit breaker notification
                        #[allow(clippy::excessive_nesting)]
                        if let Some(cb) = &self.circuit_breaker {
                            cb.trip(&self.backend_id).await;
                        }
                    }
                } else if status.failure_count > 0 {
                    status.state = HealthState::Degraded;
                }

                // Record failure with circuit breaker
                if let Some(cb) = &self.circuit_breaker {
                    cb.record_outcome(&self.backend_id, false).await;

                    // Trip circuit breaker when transitioning to unhealthy
                    if previous_state != HealthState::Unhealthy
                        && status.state == HealthState::Unhealthy
                    {
                        cb.trip(&self.backend_id).await;
                    }
                }
            },
        }

        // Emit metrics
        self.emit_metrics(&status);
    }

    /// Emit health metrics
    fn emit_metrics(&self, status: &HealthStatus) {
        use crate::metrics::{HEALTH_CHECK_DURATION_SECONDS, SERVER_HEALTH_STATUS};

        // Record health status gauge (0 = unhealthy, 1 = healthy)
        let health_value = if status.state.is_healthy() { 1.0 } else { 0.0 };
        SERVER_HEALTH_STATUS.with_label_values(&[&self.backend_id]).set(health_value);

        // Record latency
        HEALTH_CHECK_DURATION_SECONDS
            .with_label_values(&[&self.backend_id])
            .observe(status.avg_latency / 1000.0); // Convert ms to seconds

        debug!(
            "Health status for {}: {:?} (error_rate: {:.2}%, latency: {:.2}ms)",
            self.backend_id,
            status.state,
            status.error_rate * 100.0,
            status.avg_latency
        );
    }

    /// Stop health checking
    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Get current health status
    pub async fn get_status(&self) -> HealthStatus {
        self.status.read().await.clone()
    }
}

/// Passive health monitoring through request analysis
pub struct PassiveHealthMonitor {
    /// Request success/failure tracking
    request_stats: Arc<DashMap<String, RequestStats>>,

    /// Configuration
    config: PassiveHealthConfig,

    /// Circuit breaker manager
    circuit_breakers: Arc<crate::health::circuit_breaker::CircuitBreakerManager>,
}

impl PassiveHealthMonitor {
    /// Create a new passive health monitor
    pub fn new(config: PassiveHealthConfig) -> Self {
        Self {
            request_stats: Arc::new(DashMap::new()),
            config,
            circuit_breakers: Arc::new(crate::health::circuit_breaker::CircuitBreakerManager::new()),
        }
    }

    /// Record request outcome for passive monitoring
    pub async fn record_request(&self, backend_id: &str, success: bool, latency: Duration) {
        let stats = self.request_stats.entry(backend_id.to_string()).or_default();

        stats.record(success, latency);

        // Check if circuit breaker should trip
        if stats.should_trip_circuit_breaker(&self.config) {
            self.circuit_breakers.trip(backend_id).await;

            let p99 = stats.p99_latency().await;
            warn!(
                "Circuit breaker tripped for {} (error_rate: {:.2}%, latency: {:?})",
                backend_id,
                stats.error_rate() * 100.0,
                p99
            );
        }
    }

    /// Get current passive health assessment
    pub async fn assess_health(&self, backend_id: &str) -> HealthState {
        if let Some(stats) = self.request_stats.get(backend_id) {
            if stats.error_rate() > self.config.unhealthy_threshold {
                HealthState::Unhealthy
            } else if stats.error_rate() > self.config.degraded_threshold {
                HealthState::Degraded
            } else {
                HealthState::Healthy
            }
        } else {
            HealthState::Unknown
        }
    }
}

// Supporting types

#[derive(Debug)]
enum HealthCheckResult {
    Success {
        latency: Duration,
        details: Option<HealthResponse>,
    },
    Failure {
        reason: String,
        _latency: Option<Duration>,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct HealthResponse {
    status: String,
    resources: ResourceMetrics,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ResourceMetrics {
    pub cpu_usage: f64,
    pub memory_mb: u64,
    pub active_connections: u32,
}

#[derive(Debug, Clone)]
pub struct HealthConfig {
    pub check_interval: Duration,
    pub check_timeout: Duration,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub interval: Duration,
    pub timeout: Duration,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

#[derive(Debug, Clone)]
pub struct PassiveHealthConfig {
    pub unhealthy_threshold: f64,
    pub degraded_threshold: f64,
    pub window_size: Duration,
}

#[derive(Debug, Default)]
pub struct HealthMetrics {
    pub checks_total: AtomicU64,
    pub checks_failed: AtomicU64,
    pub backends_healthy: AtomicU32,
    pub backends_unhealthy: AtomicU32,
}

/// Request statistics for passive monitoring
pub struct RequestStats {
    success_count: AtomicU64,
    failure_count: AtomicU64,
    total_latency_ms: AtomicU64,
    latencies: RwLock<Vec<Duration>>,
    _window_start: RwLock<Instant>,
}

impl Default for RequestStats {
    fn default() -> Self {
        Self::new()
    }
}

impl RequestStats {
    pub fn new() -> Self {
        Self {
            success_count: AtomicU64::new(0),
            failure_count: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            latencies: RwLock::new(Vec::new()),
            _window_start: RwLock::new(Instant::now()),
        }
    }

    pub fn record(&self, success: bool, latency: Duration) {
        if success {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failure_count.fetch_add(1, Ordering::Relaxed);
        }

        self.total_latency_ms.fetch_add(latency.as_millis() as u64, Ordering::Relaxed);

        // Store latency for percentile calculations
        if let Ok(mut latencies) = self.latencies.try_write() {
            latencies.push(latency);

            // Keep only recent latencies (last 1000)
            if latencies.len() > 1000 {
                latencies.remove(0);
            }
        }
    }

    pub fn error_rate(&self) -> f64 {
        let total =
            self.success_count.load(Ordering::Relaxed) + self.failure_count.load(Ordering::Relaxed);

        if total == 0 {
            0.0
        } else {
            self.failure_count.load(Ordering::Relaxed) as f64 / total as f64
        }
    }

    pub async fn p99_latency(&self) -> Duration {
        let latencies = self.latencies.read().await;
        if latencies.is_empty() {
            return Duration::ZERO;
        }

        let mut sorted = latencies.clone();
        sorted.sort();

        let index = ((sorted.len() as f64 * 0.99) as usize).min(sorted.len() - 1);
        sorted[index]
    }

    pub fn should_trip_circuit_breaker(&self, config: &PassiveHealthConfig) -> bool {
        self.error_rate() > config.unhealthy_threshold
    }
}
