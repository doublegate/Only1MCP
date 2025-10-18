//! Comprehensive metrics following Prometheus naming conventions

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use lazy_static::lazy_static;
use prometheus::{
    histogram_opts, opts, register_counter_vec, register_gauge_vec, register_histogram_vec,
    CounterVec, Encoder, GaugeVec, HistogramVec, Registry, TextEncoder,
};
use std::sync::Arc;
use std::time::Duration;

lazy_static! {
    // Request metrics
    pub static ref MCP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        opts!(
            "only1mcp_mcp_requests_total",
            "Total number of MCP requests processed"
        ),
        &["server_id", "method", "status"]
    ).unwrap();

    pub static ref MCP_REQUEST_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        histogram_opts!(
            "only1mcp_mcp_request_duration_seconds",
            "MCP request duration in seconds",
            vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
        ),
        &["server_id", "method"]
    ).unwrap();

    // Context optimization metrics
    pub static ref CONTEXT_TOKENS_SAVED: CounterVec = register_counter_vec!(
        opts!(
            "only1mcp_context_tokens_saved_total",
            "Total tokens saved through optimization"
        ),
        &["optimization_type"]  // cache_hit, deduplication, compression
    ).unwrap();

    pub static ref CONTEXT_CACHE_HIT_RATIO: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_context_cache_hit_ratio",
            "Cache hit ratio for context optimization (0-1)"
        ),
        &["cache_type"]  // tool_result, resource_fetch, prompt_template
    ).unwrap();

    // Backend server health
    pub static ref BACKEND_HEALTH_STATUS: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_backend_health_status",
            "Health status of backend servers (0=down, 1=up)"
        ),
        &["server_id", "transport_type"]
    ).unwrap();

    pub static ref BACKEND_LATENCY_SECONDS: HistogramVec = register_histogram_vec!(
        histogram_opts!(
            "only1mcp_backend_latency_seconds",
            "Backend server response latency",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
        ),
        &["server_id", "endpoint"]
    ).unwrap();

    // Connection pool metrics
    pub static ref CONNECTION_POOL_SIZE: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_connection_pool_size",
            "Current size of connection pool"
        ),
        &["server_id", "state"]  // active, idle, pending
    ).unwrap();

    pub static ref CONNECTION_REUSE_RATIO: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_connection_reuse_ratio",
            "Connection reuse ratio (0-1)"
        ),
        &["server_id"]
    ).unwrap();

    // Cost tracking metrics
    pub static ref API_COST_DOLLARS: CounterVec = register_counter_vec!(
        opts!(
            "only1mcp_api_cost_dollars_total",
            "Cumulative API costs in dollars"
        ),
        &["provider", "model", "operation"]
    ).unwrap();

    // System resource metrics
    pub static ref MEMORY_USAGE_BYTES: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_memory_usage_bytes",
            "Memory usage in bytes"
        ),
        &["type"]  // heap, stack, cache
    ).unwrap();

    pub static ref CPU_USAGE_PERCENT: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_cpu_usage_percent",
            "CPU usage percentage (0-100)"
        ),
        &["core"]
    ).unwrap();

    // Circuit breaker metrics
    pub static ref CIRCUIT_BREAKER_STATE: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_circuit_breaker_state",
            "Circuit breaker state (0=closed, 1=open, 2=half-open)"
        ),
        &["server_id"]
    ).unwrap();

    pub static ref CIRCUIT_BREAKER_FAILURES: CounterVec = register_counter_vec!(
        opts!(
            "only1mcp_circuit_breaker_failures_total",
            "Total circuit breaker failures"
        ),
        &["server_id"]
    ).unwrap();

    // Configuration hot-reload metrics
    pub static ref CONFIG_RELOAD_TOTAL: prometheus::IntCounter = prometheus::register_int_counter!(
        "only1mcp_config_reload_total",
        "Total number of successful configuration reloads"
    ).unwrap();

    pub static ref CONFIG_RELOAD_ERRORS: prometheus::IntCounter = prometheus::register_int_counter!(
        "only1mcp_config_reload_errors_total",
        "Total number of configuration reload errors"
    ).unwrap();

    // Rate limiting metrics
    pub static ref RATE_LIMIT_EXCEEDED: CounterVec = register_counter_vec!(
        opts!(
            "only1mcp_rate_limit_exceeded_total",
            "Total rate limit exceeded events"
        ),
        &["client_id", "limit_type"]
    ).unwrap();

    pub static ref RATE_LIMIT_REMAINING: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_rate_limit_remaining",
            "Remaining rate limit capacity"
        ),
        &["client_id", "limit_type"]
    ).unwrap();

    // Health check metrics
    pub static ref HEALTH_CHECK_TOTAL: CounterVec = register_counter_vec!(
        opts!(
            "only1mcp_health_check_total",
            "Total number of health checks performed"
        ),
        &["server_id", "result"]  // result: success, failure
    ).unwrap();

    pub static ref HEALTH_CHECK_DURATION_SECONDS: HistogramVec = register_histogram_vec!(
        histogram_opts!(
            "only1mcp_health_check_duration_seconds",
            "Health check duration in seconds",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
        ),
        &["server_id"]
    ).unwrap();

    pub static ref SERVER_HEALTH_STATUS: GaugeVec = register_gauge_vec!(
        opts!(
            "only1mcp_server_health_status",
            "Server health status (0=unhealthy, 1=healthy)"
        ),
        &["server_id"]
    ).unwrap();

    // Cache metrics (Feature 3)
    pub static ref CACHE_HITS_TOTAL: prometheus::IntCounter = prometheus::register_int_counter!(
        "only1mcp_cache_hits_total",
        "Total number of cache hits"
    ).unwrap();

    pub static ref CACHE_MISSES_TOTAL: prometheus::IntCounter = prometheus::register_int_counter!(
        "only1mcp_cache_misses_total",
        "Total number of cache misses"
    ).unwrap();

    pub static ref CACHE_SIZE_ENTRIES: prometheus::IntGauge = prometheus::register_int_gauge!(
        "only1mcp_cache_size_entries",
        "Current number of entries in cache"
    ).unwrap();

    pub static ref CACHE_EVICTIONS_TOTAL: prometheus::IntCounter = prometheus::register_int_counter!(
        "only1mcp_cache_evictions_total",
        "Total number of cache evictions"
    ).unwrap();

    // Batching metrics (Feature 4)
    pub static ref BATCH_REQUESTS_TOTAL: prometheus::IntCounter = prometheus::register_int_counter!(
        "only1mcp_batch_requests_total",
        "Total number of requests submitted to batching"
    ).unwrap();

    pub static ref BATCH_SIZE: prometheus::Histogram = prometheus::register_histogram!(
        prometheus::histogram_opts!(
            "only1mcp_batch_size",
            "Distribution of batch sizes (number of requests per batch)",
            vec![1.0, 2.0, 3.0, 5.0, 10.0, 20.0, 50.0]
        )
    ).unwrap();

    pub static ref BATCH_WAIT_TIME_SECONDS: prometheus::Histogram = prometheus::register_histogram!(
        prometheus::histogram_opts!(
            "only1mcp_batch_wait_time_seconds",
            "Time requests wait in batch before processing",
            vec![0.01, 0.025, 0.05, 0.075, 0.1, 0.15, 0.2, 0.5]
        )
    ).unwrap();

    pub static ref BATCHING_EFFICIENCY_RATIO: prometheus::Gauge = prometheus::register_gauge!(
        "only1mcp_batching_efficiency_ratio",
        "Batching efficiency ratio: backend_calls / total_requests (lower is better)"
    ).unwrap();

    // Registry for all metrics
    pub static ref REGISTRY: Registry = {
        let registry = Registry::new();
        registry.register(Box::new(MCP_REQUESTS_TOTAL.clone())).unwrap();
        registry.register(Box::new(MCP_REQUEST_DURATION_SECONDS.clone())).unwrap();
        registry.register(Box::new(CONTEXT_TOKENS_SAVED.clone())).unwrap();
        registry.register(Box::new(CONTEXT_CACHE_HIT_RATIO.clone())).unwrap();
        registry.register(Box::new(BACKEND_HEALTH_STATUS.clone())).unwrap();
        registry.register(Box::new(BACKEND_LATENCY_SECONDS.clone())).unwrap();
        registry.register(Box::new(CONNECTION_POOL_SIZE.clone())).unwrap();
        registry.register(Box::new(CONNECTION_REUSE_RATIO.clone())).unwrap();
        registry.register(Box::new(API_COST_DOLLARS.clone())).unwrap();
        registry.register(Box::new(MEMORY_USAGE_BYTES.clone())).unwrap();
        registry.register(Box::new(CPU_USAGE_PERCENT.clone())).unwrap();
        registry.register(Box::new(CIRCUIT_BREAKER_STATE.clone())).unwrap();
        registry.register(Box::new(CIRCUIT_BREAKER_FAILURES.clone())).unwrap();
        registry.register(Box::new(RATE_LIMIT_EXCEEDED.clone())).unwrap();
        registry.register(Box::new(RATE_LIMIT_REMAINING.clone())).unwrap();
        registry.register(Box::new(HEALTH_CHECK_TOTAL.clone())).unwrap();
        registry.register(Box::new(HEALTH_CHECK_DURATION_SECONDS.clone())).unwrap();
        registry.register(Box::new(SERVER_HEALTH_STATUS.clone())).unwrap();
        registry.register(Box::new(CACHE_HITS_TOTAL.clone())).unwrap();
        registry.register(Box::new(CACHE_MISSES_TOTAL.clone())).unwrap();
        registry.register(Box::new(CACHE_SIZE_ENTRIES.clone())).unwrap();
        registry.register(Box::new(CACHE_EVICTIONS_TOTAL.clone())).unwrap();
        registry.register(Box::new(BATCH_REQUESTS_TOTAL.clone())).unwrap();
        registry.register(Box::new(BATCH_SIZE.clone())).unwrap();
        registry.register(Box::new(BATCH_WAIT_TIME_SECONDS.clone())).unwrap();
        registry.register(Box::new(BATCHING_EFFICIENCY_RATIO.clone())).unwrap();
        registry
    };
}

/// Record metrics for an MCP request
pub fn record_mcp_request(server_id: &str, method: &str, status: &str, duration: Duration) {
    MCP_REQUESTS_TOTAL.with_label_values(&[server_id, method, status]).inc();

    MCP_REQUEST_DURATION_SECONDS
        .with_label_values(&[server_id, method])
        .observe(duration.as_secs_f64());
}

/// Record context optimization metrics
pub fn record_context_optimization(optimization_type: &str, tokens_saved: u64) {
    CONTEXT_TOKENS_SAVED
        .with_label_values(&[optimization_type])
        .inc_by(tokens_saved as f64);
}

/// Update cache hit ratio
pub fn update_cache_hit_ratio(cache_type: &str, ratio: f64) {
    CONTEXT_CACHE_HIT_RATIO.with_label_values(&[cache_type]).set(ratio);
}

/// Update backend health status
pub fn update_backend_health(server_id: &str, transport_type: &str, is_healthy: bool) {
    BACKEND_HEALTH_STATUS
        .with_label_values(&[server_id, transport_type])
        .set(if is_healthy { 1.0 } else { 0.0 });
}

/// Record backend latency
pub fn record_backend_latency(server_id: &str, endpoint: &str, duration: Duration) {
    BACKEND_LATENCY_SECONDS
        .with_label_values(&[server_id, endpoint])
        .observe(duration.as_secs_f64());
}

/// Update connection pool metrics
pub fn update_connection_pool(server_id: &str, active: usize, idle: usize, pending: usize) {
    CONNECTION_POOL_SIZE
        .with_label_values(&[server_id, "active"])
        .set(active as f64);
    CONNECTION_POOL_SIZE.with_label_values(&[server_id, "idle"]).set(idle as f64);
    CONNECTION_POOL_SIZE
        .with_label_values(&[server_id, "pending"])
        .set(pending as f64);
}

/// Record API cost
pub fn record_api_cost(provider: &str, model: &str, operation: &str, cost: f64) {
    API_COST_DOLLARS.with_label_values(&[provider, model, operation]).inc_by(cost);
}

/// Update circuit breaker state
pub fn update_circuit_breaker_state(server_id: &str, state: CircuitBreakerState) {
    let state_value = match state {
        CircuitBreakerState::Closed => 0.0,
        CircuitBreakerState::Open => 1.0,
        CircuitBreakerState::HalfOpen => 2.0,
    };
    CIRCUIT_BREAKER_STATE.with_label_values(&[server_id]).set(state_value);
}

/// Record circuit breaker failure
pub fn record_circuit_breaker_failure(server_id: &str) {
    CIRCUIT_BREAKER_FAILURES.with_label_values(&[server_id]).inc();
}

/// Record rate limit exceeded
pub fn record_rate_limit_exceeded(client_id: &str, limit_type: &str) {
    RATE_LIMIT_EXCEEDED.with_label_values(&[client_id, limit_type]).inc();
}

/// Update remaining rate limit
pub fn update_rate_limit_remaining(client_id: &str, limit_type: &str, remaining: u64) {
    RATE_LIMIT_REMAINING
        .with_label_values(&[client_id, limit_type])
        .set(remaining as f64);
}

/// Circuit breaker state enum
#[derive(Debug, Clone, Copy)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

/// Metrics exporter for Prometheus scraping
pub struct MetricsExporter {
    registry: Arc<Registry>,
}

impl Default for MetricsExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsExporter {
    /// Create new metrics exporter
    pub fn new() -> Self {
        Self {
            registry: Arc::new(REGISTRY.clone()),
        }
    }

    /// Export metrics in Prometheus format
    pub fn export(&self) -> Result<Vec<u8>, PrometheusError> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();

        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;

        Ok(buffer)
    }
}

/// Prometheus error wrapper
#[derive(Debug, thiserror::Error)]
pub enum PrometheusError {
    #[error("Failed to encode metrics: {0}")]
    Encode(#[from] prometheus::Error),
}

/// HTTP handler for /metrics endpoint
pub async fn metrics_handler(
    State(state): State<crate::proxy::server::AppState>,
) -> impl IntoResponse {
    match state.metrics.exporter.export() {
        Ok(metrics) => (
            StatusCode::OK,
            [("Content-Type", "text/plain; version=0.0.4")],
            metrics,
        ),
        Err(e) => {
            tracing::error!("Failed to export metrics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("Content-Type", "text/plain")],
                b"Failed to export metrics".to_vec(),
            )
        },
    }
}

/// System metrics collector
pub struct SystemMetricsCollector;

impl SystemMetricsCollector {
    /// Start collecting system metrics
    pub fn start(interval: Duration) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);

            loop {
                interval.tick().await;
                Self::collect_metrics();
            }
        });
    }

    /// Collect current system metrics
    fn collect_metrics() {
        // Memory metrics
        if let Ok(mem_info) = sys_info::mem_info() {
            MEMORY_USAGE_BYTES
                .with_label_values(&["heap"])
                .set((mem_info.total - mem_info.avail) as f64 * 1024.0);
        }

        // CPU metrics
        if let Ok(cpu_usage) = sys_info::loadavg() {
            CPU_USAGE_PERCENT.with_label_values(&["all"]).set(cpu_usage.one * 100.0);
        }
    }
}

/// Initialize metrics system
pub fn init(metrics_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Start system metrics collector
    SystemMetricsCollector::start(Duration::from_secs(10));

    // Log metrics initialization
    tracing::info!(
        "Metrics initialized, Prometheus endpoint available at :{}{}",
        metrics_port,
        "/metrics"
    );

    Ok(())
}

/// Convenient metrics wrapper for application state
#[derive(Clone)]
pub struct Metrics {
    exporter: Arc<MetricsExporter>,
}

impl Metrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self {
            exporter: Arc::new(MetricsExporter::new()),
        }
    }

    /// Record MCP request
    pub fn record_request(&self, server_id: &str, method: &str, status: &str, duration: Duration) {
        record_mcp_request(server_id, method, status, duration);
    }

    /// Increment cache hits
    pub fn cache_hits(&self) -> CacheHitCounter {
        CacheHitCounter
    }

    /// Record for specific tools/list duration
    pub fn tools_list_duration(&self) -> DurationRecorder {
        DurationRecorder {
            metric_name: "tools_list",
        }
    }

    /// Record for tools/call duration
    pub fn tools_call_duration(&self) -> DurationRecorder {
        DurationRecorder {
            metric_name: "tools_call",
        }
    }

    /// Record for resources/list duration
    pub fn resources_list_duration(&self) -> DurationRecorder {
        DurationRecorder {
            metric_name: "resources_list",
        }
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper struct for cache hit counting
pub struct CacheHitCounter;

impl CacheHitCounter {
    pub fn inc(&self) {
        CONTEXT_TOKENS_SAVED.with_label_values(&["cache_hit"]).inc();
    }
}

/// Helper struct for duration recording
pub struct DurationRecorder {
    metric_name: &'static str,
}

impl DurationRecorder {
    pub fn record(&self, duration: f64) {
        // Record to appropriate histogram
        MCP_REQUEST_DURATION_SECONDS
            .with_label_values(&["proxy", self.metric_name])
            .observe(duration);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_mcp_request() {
        record_mcp_request(
            "server1",
            "tools.list",
            "success",
            Duration::from_millis(50),
        );

        // Verify metric was recorded
        let metric_families = REGISTRY.gather();
        assert!(!metric_families.is_empty());
    }

    #[test]
    fn test_metrics_exporter() {
        let exporter = MetricsExporter::new();
        let result = exporter.export();

        assert!(result.is_ok());
        let metrics = result.unwrap();
        assert!(!metrics.is_empty());

        // Should contain Prometheus text format
        let metrics_str = String::from_utf8(metrics).unwrap();
        assert!(metrics_str.contains("# TYPE"));
    }

    #[test]
    fn test_circuit_breaker_metrics() {
        update_circuit_breaker_state("server1", CircuitBreakerState::Open);
        record_circuit_breaker_failure("server1");

        // Verify metrics were recorded
        let metric_families = REGISTRY.gather();
        assert!(!metric_families.is_empty());
    }
}
