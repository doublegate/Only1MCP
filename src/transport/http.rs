//! HTTP transport implementation with connection pooling
//!
//! Handles HTTP-based MCP servers with connection pooling using bb8.
//! Features include request forwarding, response streaming, timeout handling,
//! and retry logic with exponential backoff.

use async_trait::async_trait;
use bb8::{ManageConnection, Pool};
use reqwest::{Client, StatusCode};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::types::{McpRequest, McpResponse};

/// HTTP transport errors
#[derive(Error, Debug)]
pub enum HttpError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Health check failed with status: {0}")]
    HealthCheckFailed(StatusCode),

    #[error("Connection expired after 5 minutes")]
    ConnectionExpired,

    #[error("Connection exhausted after 1000 requests")]
    ConnectionExhausted,

    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Server error: {0}")]
    ServerError(String),
}

/// HTTP transport configuration
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// Base URL of the HTTP server
    pub base_url: String,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Request timeout
    pub request_timeout: Duration,

    /// Maximum retries
    pub max_retries: u32,

    /// Keep-alive duration
    pub keep_alive: Duration,

    /// Maximum connections per host
    pub max_connections_per_host: usize,

    /// Enable compression
    pub compression: bool,

    /// Custom HTTP headers
    pub headers: std::collections::HashMap<String, String>,
}

impl Default for HttpTransportConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            connection_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_retries: 3,
            keep_alive: Duration::from_secs(90),
            max_connections_per_host: 10,
            compression: true,
            headers: std::collections::HashMap::new(),
        }
    }
}

/// HTTP connection manager for bb8 pool
pub struct HttpConnectionManager {
    /// Base URL for the backend
    base_url: String,

    /// HTTP client for connections
    client: Client,

    /// Connection timeout
    timeout: Duration,

    /// Custom HTTP headers
    headers: std::collections::HashMap<String, String>,
}

impl HttpConnectionManager {
    /// Create new HTTP connection manager
    pub fn new(config: HttpTransportConfig) -> Self {
        let client = Client::builder()
            .timeout(config.request_timeout)
            .connect_timeout(config.connection_timeout)
            .tcp_keepalive(Some(config.keep_alive))
            .pool_max_idle_per_host(config.max_connections_per_host)
            // Note: gzip/brotli compression is enabled by default in reqwest
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url: config.base_url,
            client,
            timeout: config.connection_timeout,
            headers: config.headers,
        }
    }
}

#[async_trait]
impl ManageConnection for HttpConnectionManager {
    type Connection = HttpConnection;
    type Error = HttpError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        // Try health check, but don't fail if endpoint doesn't exist (404)
        // Some MCP servers like Context7 don't have a /health endpoint
        let health_url = format!("{}/health", self.base_url);

        if let Ok(response) = self.client.get(&health_url).timeout(self.timeout).send().await {
            // If health endpoint exists but returns error (not 404), that's a problem
            if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
                return Err(HttpError::HealthCheckFailed(response.status()));
            }
        }
        // If health check fails entirely or returns 404, proceed anyway

        Ok(HttpConnection {
            base_url: self.base_url.clone(),
            client: self.client.clone(),
            created_at: Instant::now(),
            request_count: Arc::new(AtomicU64::new(0)),
            headers: self.headers.clone(),
        })
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Check if connection is still valid
        if conn.created_at.elapsed() > Duration::from_secs(300) {
            // Connection too old (5 minutes)
            return Err(HttpError::ConnectionExpired);
        }

        if conn.request_count.load(Ordering::Relaxed) > 1000 {
            // Too many requests on this connection
            return Err(HttpError::ConnectionExhausted);
        }

        // Perform quick health check
        let response = conn
            .client
            .head(format!("{}/health", conn.base_url))
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .map_err(|_| HttpError::HealthCheckFailed(StatusCode::REQUEST_TIMEOUT))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(HttpError::HealthCheckFailed(response.status()))
        }
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        // Check if connection is broken
        conn.request_count.load(Ordering::Relaxed) > 10000
            || conn.created_at.elapsed() > Duration::from_secs(600)
    }
}

/// Pooled HTTP connection wrapper
pub struct HttpConnection {
    /// Base URL for requests
    base_url: String,

    /// Reusable HTTP client
    client: Client,

    /// Connection creation time
    created_at: Instant,

    /// Number of requests sent
    request_count: Arc<AtomicU64>,

    /// Custom HTTP headers
    headers: std::collections::HashMap<String, String>,
}

impl HttpConnection {
    /// Send a request using this connection
    pub async fn send(&self, request: McpRequest) -> Result<McpResponse, HttpError> {
        self.request_count.fetch_add(1, Ordering::Relaxed);

        let response =
            self.client.post(format!("{}/mcp", self.base_url)).json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(HttpError::ServerError(format!("{}: {}", status, body)));
        }

        let mcp_response: McpResponse =
            response.json().await.map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

        Ok(mcp_response)
    }

    /// Get connection statistics
    pub fn stats(&self) -> ConnectionStats {
        ConnectionStats {
            created_at: self.created_at,
            request_count: self.request_count.load(Ordering::Relaxed),
            age: self.created_at.elapsed(),
        }
    }
}

/// Connection statistics
#[derive(Debug)]
pub struct ConnectionStats {
    pub created_at: Instant,
    pub request_count: u64,
    pub age: Duration,
}

/// HTTP transport implementation
pub struct HttpTransport {
    /// Connection pool
    pool: Pool<HttpConnectionManager>,

    /// Configuration
    config: HttpTransportConfig,

    /// Metrics
    metrics: Arc<TransportMetrics>,
}

impl HttpTransport {
    /// Create new HTTP transport
    pub async fn new(config: HttpTransportConfig) -> Result<Self, HttpError> {
        let manager = HttpConnectionManager::new(config.clone());

        let pool = Pool::builder()
            .max_size(config.max_connections_per_host as u32)
            .min_idle(Some(1))
            .max_lifetime(Some(Duration::from_secs(300)))
            .idle_timeout(Some(Duration::from_secs(60)))
            .connection_timeout(config.connection_timeout)
            .build(manager)
            .await
            .map_err(|e| HttpError::ConnectionFailed(e.to_string()))?;

        Ok(Self {
            pool,
            config,
            metrics: Arc::new(TransportMetrics::new()),
        })
    }

    /// Send MCP request
    pub async fn send(&self, request: McpRequest) -> Result<McpResponse, HttpError> {
        let start = Instant::now();

        // Get connection from pool
        let conn = self.pool.get().await.map_err(|e| HttpError::ConnectionFailed(e.to_string()))?;

        // Send request
        let response = self.retry_with_backoff(|| conn.send(request.clone())).await?;

        // Update metrics
        self.metrics.record_request(start.elapsed());

        Ok(response)
    }

    /// Retry with exponential backoff
    async fn retry_with_backoff<F, Fut, T>(&self, f: F) -> Result<T, HttpError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, HttpError>>,
    {
        let mut delay = Duration::from_millis(100);
        let mut attempts = 0;

        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if attempts >= self.config.max_retries => return Err(e),
                Err(e) => {
                    tracing::warn!("Request failed (attempt {}): {}", attempts + 1, e);
                    tokio::time::sleep(delay).await;
                    delay *= 2;
                    attempts += 1;
                },
            }
        }
    }

    /// Get pool statistics
    pub fn pool_stats(&self) -> PoolStats {
        let state = self.pool.state();
        PoolStats {
            connections: state.connections,
            idle_connections: state.idle_connections,
            pending_connections: 0, // bb8 0.8 doesn't expose pending connections
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<(), HttpError> {
        let conn = self.pool.get().await.map_err(|e| HttpError::ConnectionFailed(e.to_string()))?;

        let response = conn
            .client
            .get(format!("{}/health", conn.base_url))
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(HttpError::HealthCheckFailed(response.status()))
        }
    }

    /// Send request to a specific endpoint (convenience method for handler.rs)
    pub async fn send_request(
        &self,
        endpoint: &str,
        request: McpRequest,
    ) -> Result<McpResponse, HttpError> {
        let start = Instant::now();

        // Get pooled connection
        let conn = self.pool.get().await.map_err(|e| HttpError::ConnectionFailed(e.to_string()))?;

        // Record attempt
        self.metrics.request_count.fetch_add(1, Ordering::Relaxed);

        // Send request with custom headers
        let mut request_builder =
            conn.client.post(endpoint).json(&request).timeout(self.config.request_timeout);

        // Apply custom headers from config
        for (key, value) in &conn.headers {
            request_builder = request_builder.header(key, value);
        }

        let result = request_builder.send().await;

        match result {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let body =
                        response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    self.metrics.error_count.fetch_add(1, Ordering::Relaxed);
                    return Err(HttpError::ServerError(format!("{}: {}", status, body)));
                }

                let mcp_response: McpResponse =
                    response.json().await.map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

                let elapsed = start.elapsed().as_micros() as u64;
                self.metrics.total_latency_us.fetch_add(elapsed, Ordering::Relaxed);

                Ok(mcp_response)
            },
            Err(e) => {
                self.metrics.error_count.fetch_add(1, Ordering::Relaxed);
                Err(e.into())
            },
        }
    }
}

/// Pool statistics
#[derive(Debug, Default)]
pub struct PoolStats {
    pub connections: u32,
    pub idle_connections: u32,
    pub pending_connections: u32,
}

/// Transport metrics
pub struct TransportMetrics {
    request_count: AtomicU64,
    total_latency_us: AtomicU64,
    error_count: AtomicU64,
}

impl Default for TransportMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl TransportMetrics {
    pub fn new() -> Self {
        Self {
            request_count: AtomicU64::new(0),
            total_latency_us: AtomicU64::new(0),
            error_count: AtomicU64::new(0),
        }
    }

    pub fn record_request(&self, duration: Duration) {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> MetricsStats {
        let count = self.request_count.load(Ordering::Relaxed);
        let total_latency = self.total_latency_us.load(Ordering::Relaxed);

        MetricsStats {
            request_count: count,
            average_latency_us: if count > 0 { total_latency / count } else { 0 },
            error_count: self.error_count.load(Ordering::Relaxed),
        }
    }
}

/// Metrics statistics
#[derive(Debug)]
pub struct MetricsStats {
    pub request_count: u64,
    pub average_latency_us: u64,
    pub error_count: u64,
}

/// Multi-backend HTTP transport pool manager
pub struct HttpTransportPool {
    /// Transports per endpoint (lazy initialization)
    transports: dashmap::DashMap<String, Arc<HttpTransport>>,
    /// Default configuration for new transports
    default_config: HttpTransportConfig,
}

impl Default for HttpTransportPool {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTransportPool {
    /// Create a new transport pool with default configuration
    pub fn new() -> Self {
        Self {
            transports: dashmap::DashMap::new(),
            default_config: HttpTransportConfig::default(),
        }
    }

    /// Get or create an HTTP transport for a specific endpoint
    async fn get_or_create(&self, endpoint: &str) -> Result<Arc<HttpTransport>, HttpError> {
        // Check if we already have a transport for this endpoint
        if let Some(transport) = self.transports.get(endpoint) {
            return Ok(transport.clone());
        }

        // Extract base URL from endpoint
        let base_url = if let Ok(url) = url::Url::parse(endpoint) {
            format!(
                "{}://{}",
                url.scheme(),
                url.host_str().unwrap_or("localhost")
            )
        } else {
            endpoint.to_string()
        };

        // Create new transport
        let config = HttpTransportConfig {
            base_url: base_url.clone(),
            ..self.default_config.clone()
        };

        let transport = Arc::new(HttpTransport::new(config).await?);

        // Store for reuse
        self.transports.insert(base_url, transport.clone());

        Ok(transport)
    }

    /// Send request to a specific endpoint
    pub async fn send_request(
        &self,
        endpoint: &str,
        request: crate::types::McpRequest,
    ) -> Result<crate::types::McpResponse, HttpError> {
        self.send_request_with_headers(endpoint, request, std::collections::HashMap::new())
            .await
    }

    /// Send request to a specific endpoint with custom headers
    pub async fn send_request_with_headers(
        &self,
        endpoint: &str,
        request: crate::types::McpRequest,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<crate::types::McpResponse, HttpError> {
        // Get or create base transport
        let transport = self.get_or_create(endpoint).await?;

        // If no headers provided, use existing transport logic
        if headers.is_empty() {
            return transport.send_request(endpoint, request).await;
        }

        // For requests with headers, we need to override the connection's headers
        // This is a simplified approach - get pooled connection and send with custom headers
        let start = Instant::now();

        // Get pooled connection
        let conn = transport
            .pool
            .get()
            .await
            .map_err(|e| HttpError::ConnectionFailed(e.to_string()))?;

        // Record attempt
        transport.metrics.request_count.fetch_add(1, Ordering::Relaxed);

        // Build request with custom headers
        let mut request_builder = conn
            .client
            .post(endpoint)
            .json(&request)
            .timeout(transport.config.request_timeout);

        // Apply custom headers (these override any default headers)
        for (key, value) in &headers {
            request_builder = request_builder.header(key, value);
        }

        let result = request_builder.send().await;

        match result {
            Ok(response) => {
                if !response.status().is_success() {
                    let status = response.status();
                    let body =
                        response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    transport.metrics.error_count.fetch_add(1, Ordering::Relaxed);
                    return Err(HttpError::ServerError(format!("{}: {}", status, body)));
                }

                let mcp_response: crate::types::McpResponse =
                    response.json().await.map_err(|e| HttpError::InvalidResponse(e.to_string()))?;

                let elapsed = start.elapsed().as_micros() as u64;
                transport.metrics.total_latency_us.fetch_add(elapsed, Ordering::Relaxed);

                Ok(mcp_response)
            },
            Err(e) => {
                transport.metrics.error_count.fetch_add(1, Ordering::Relaxed);
                Err(e.into())
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = HttpTransportConfig::default();
        assert_eq!(config.connection_timeout, Duration::from_secs(10));
        assert_eq!(config.max_retries, 3);
        assert!(config.compression);
    }

    #[tokio::test]
    async fn test_connection_manager() {
        let config = HttpTransportConfig {
            base_url: "http://localhost:8080".to_string(),
            ..Default::default()
        };

        let manager = HttpConnectionManager::new(config);
        assert_eq!(manager.base_url, "http://localhost:8080");
    }

    #[test]
    fn test_metrics() {
        let metrics = TransportMetrics::new();
        metrics.record_request(Duration::from_millis(100));
        metrics.record_request(Duration::from_millis(200));
        metrics.record_error();

        let stats = metrics.get_stats();
        assert_eq!(stats.request_count, 2);
        assert_eq!(stats.error_count, 1);
        assert!(stats.average_latency_us > 0);
    }
}
