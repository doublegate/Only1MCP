//! Streamable HTTP transport implementation for MCP 2025-03-26 specification.
//!
//! This transport supports modern MCP servers that use the Streamable HTTP protocol,
//! which replaces the legacy HTTP+SSE transport. Key features:
//!
//! - Single `/mcp` endpoint for all operations
//! - Session management via `mcp-session-id` header
//! - Support for both JSON and SSE response formats
//! - Stateful bidirectional communication
//!
//! # Protocol Flow
//!
//! 1. **Initialization**: Client sends `initialize` request without session ID
//! 2. **Session Creation**: Server responds with `mcp-session-id` header
//! 3. **Subsequent Requests**: Client includes session ID in all future requests
//! 4. **Session Expiry**: 400/401 errors trigger reinitialization
//!
//! # Example
//!
//! ```no_run
//! use only1mcp::transport::streamable_http::{StreamableHttpTransport, StreamableHttpConfig};
//! use std::collections::HashMap;
//!
//! let config = StreamableHttpConfig {
//!     url: "http://localhost:8124/mcp".to_string(),
//!     headers: {
//!         let mut h = HashMap::new();
//!         h.insert("Accept".into(), "application/json, text/event-stream".into());
//!         h
//!     },
//!     timeout_ms: 30000,
//! };
//!
//! let transport = StreamableHttpTransport::new(config);
//! ```

use crate::error::Error;
use crate::types::{McpRequest, McpResponse};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Streamable HTTP transport implementing MCP 2025-03-26 specification.
///
/// Manages session-based communication with modern MCP servers using
/// the Streamable HTTP protocol. Handles both JSON and SSE response formats.
#[derive(Clone)]
pub struct StreamableHttpTransport {
    /// HTTP client for requests
    client: Client,

    /// Base endpoint URL (e.g., "http://localhost:8124/mcp")
    endpoint: String,

    /// Session ID from server (stored after initialization)
    session_id: Arc<RwLock<Option<String>>>,

    /// Custom headers per configuration
    headers: HashMap<String, String>,

    /// Connection timeout
    timeout: Duration,
}

/// Configuration for Streamable HTTP transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamableHttpConfig {
    /// Endpoint URL (e.g., "http://localhost:8124/mcp")
    pub url: String,

    /// Custom HTTP headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Timeout in milliseconds
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_timeout_ms() -> u64 {
    30000
}

/// Error type for Streamable HTTP transport operations
#[derive(Debug, thiserror::Error)]
pub enum StreamableHttpError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Invalid session ID: {0}")]
    InvalidSession(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),

    #[error("Failed to parse response: {0}")]
    ParseError(String),

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

impl StreamableHttpTransport {
    /// Create a new Streamable HTTP transport with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Transport configuration with URL, headers, and timeout
    ///
    /// # Returns
    ///
    /// Initialized transport ready to send requests
    pub fn new(config: StreamableHttpConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint: config.url,
            session_id: Arc::new(RwLock::new(None)),
            headers: config.headers,
            timeout: Duration::from_millis(config.timeout_ms),
        }
    }

    /// Send request with session management.
    ///
    /// Automatically handles session ID storage and inclusion in requests.
    /// If there's no active session and the request is not `initialize`,
    /// automatically sends an `initialize` request first to establish the session.
    /// If a session error occurs (400/401), clears the session ID to force
    /// reinitialization on the next request.
    ///
    /// # Arguments
    ///
    /// * `request` - MCP JSON-RPC request to send
    ///
    /// # Returns
    ///
    /// * `Ok(McpResponse)` - Successful response from server
    /// * `Err(StreamableHttpError)` - Network, protocol, or parsing error
    pub async fn send_request(
        &self,
        request: McpRequest,
    ) -> Result<McpResponse, StreamableHttpError> {
        // 0. Check if we need to initialize first
        let needs_init = {
            let session = self.session_id.read().await;
            session.is_none() && request.method() != "initialize"
        };

        if needs_init {
            info!("No session ID, sending initialize request first");

            // Send initialize request to establish session
            let init_request = McpRequest::new(
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": {
                        "name": "Only1MCP",
                        "version": "0.2.0"
                    }
                }),
                Some(serde_json::json!(1)), // Use a simple numeric ID for init
            );

            // Send initialize and get session ID
            let _init_response = self.send_request_internal(init_request).await?;
            info!("Session initialized successfully");
        }

        // Now send the actual request with session
        self.send_request_internal(request).await
    }

    /// Internal method to send a request without automatic initialization.
    ///
    /// This is used by `send_request` after handling initialization logic.
    async fn send_request_internal(
        &self,
        request: McpRequest,
    ) -> Result<McpResponse, StreamableHttpError> {
        // 1. Build base request
        let mut req_builder = self
            .client
            .post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        // 2. Add custom headers from config
        for (key, value) in &self.headers {
            req_builder = req_builder.header(key, value);
        }

        // 3. Add session ID if we have one
        if let Some(session_id) = self.session_id.read().await.as_ref() {
            req_builder = req_builder.header("mcp-session-id", session_id);
            debug!("Using session ID: {}", session_id);
        } else {
            debug!("No session ID, expecting server to create new session");
        }

        // 4. Send request
        let response = req_builder
            .json(&request)
            .send()
            .await
            .map_err(StreamableHttpError::RequestFailed)?;

        // 5. Extract session ID from response (if new or updated)
        self.extract_session_id(&response).await;

        // 6. Check status code
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Handle session errors (may need to reinitialize)
            if status == 400 || status == 401 {
                warn!("Session error ({}): {}", status, body);
                // Clear session ID to force reinitialization
                *self.session_id.write().await = None;
            }

            return Err(StreamableHttpError::ProtocolError(format!(
                "Server returned {}: {}",
                status, body
            )));
        }

        // 7. Parse response (handles both JSON and SSE)
        self.parse_response(response).await
    }

    /// Extract and store session ID from response headers.
    ///
    /// Spawns a background task to update the session ID to avoid blocking
    /// the request path. The session ID is stored in an Arc<RwLock> for
    /// thread-safe access.
    ///
    /// # Arguments
    ///
    /// * `response` - HTTP response from server
    async fn extract_session_id(&self, response: &reqwest::Response) {
        if let Some(session_header) = response.headers().get("mcp-session-id") {
            if let Ok(session_str) = session_header.to_str() {
                let session_id = session_str.to_string();
                info!("Received session ID: {}", session_id);

                // Store session ID
                *self.session_id.write().await = Some(session_id);
            }
        }
    }

    /// Parse response, automatically detecting JSON vs SSE format.
    ///
    /// Checks the `Content-Type` header to determine format:
    /// - `application/json` → Parse as standard JSON-RPC response
    /// - `text/event-stream` → Parse SSE format (`data: <json>\n\n`)
    ///
    /// # Arguments
    ///
    /// * `response` - HTTP response from server
    ///
    /// # Returns
    ///
    /// * `Ok(McpResponse)` - Parsed response
    /// * `Err(StreamableHttpError)` - Parsing error
    async fn parse_response(
        &self,
        response: reqwest::Response,
    ) -> Result<McpResponse, StreamableHttpError> {
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Check if response is SSE format
        if content_type.contains("text/event-stream") {
            debug!("Parsing SSE response");
            self.parse_sse_response(response).await
        } else {
            debug!("Parsing JSON response");
            // Standard JSON response
            response
                .json::<McpResponse>()
                .await
                .map_err(|e| StreamableHttpError::ParseError(e.to_string()))
        }
    }

    /// Parse SSE format response.
    ///
    /// SSE format uses lines prefixed with `data:` followed by JSON:
    /// ```text
    /// data: {"jsonrpc":"2.0","id":1,"result":{...}}
    ///
    /// ```
    ///
    /// This function:
    /// 1. Extracts all lines starting with `data:`
    /// 2. Removes the `data:` prefix
    /// 3. Combines multi-line data fields
    /// 4. Parses as JSON-RPC response
    ///
    /// # Arguments
    ///
    /// * `response` - HTTP response with SSE content
    ///
    /// # Returns
    ///
    /// * `Ok(McpResponse)` - Parsed response from SSE data
    /// * `Err(StreamableHttpError)` - Parsing error
    async fn parse_sse_response(
        &self,
        response: reqwest::Response,
    ) -> Result<McpResponse, StreamableHttpError> {
        let body = response.text().await?;

        // Parse SSE format: "data: <json>\n\n"
        let mut data_lines = Vec::new();

        for line in body.lines() {
            if line.starts_with("data:") {
                // Extract JSON after "data: " prefix
                let json_str = line["data:".len()..].trim();
                data_lines.push(json_str);
            }
        }

        if data_lines.is_empty() {
            return Err(StreamableHttpError::ParseError(
                "No data lines found in SSE response".to_string(),
            ));
        }

        // Combine all data lines (handles multi-line events)
        let json_str = data_lines.join("");

        debug!("SSE data: {}", json_str);

        // Parse as JSON-RPC response
        serde_json::from_str(&json_str).map_err(|e| {
            StreamableHttpError::ParseError(format!("Failed to parse SSE data: {}", e))
        })
    }

    /// Get the current session ID (if any).
    ///
    /// Useful for debugging or monitoring session state.
    ///
    /// # Returns
    ///
    /// * `Some(String)` - Active session ID
    /// * `None` - No session established yet
    pub async fn get_session_id(&self) -> Option<String> {
        self.session_id.read().await.clone()
    }

    /// Clear the current session ID.
    ///
    /// Forces reinitialization on the next request. Useful for testing
    /// or manual session management.
    pub async fn clear_session(&self) {
        *self.session_id.write().await = None;
        info!("Cleared session ID");
    }
}

/// Connection pool for Streamable HTTP transports.
///
/// Manages a pool of transports to avoid recreating clients and sessions
/// for repeated requests to the same endpoint.
#[derive(Clone)]
pub struct StreamableHttpTransportPool {
    /// Pool of transports keyed by endpoint URL
    transports: Arc<dashmap::DashMap<String, Arc<StreamableHttpTransport>>>,
}

impl StreamableHttpTransportPool {
    /// Create a new transport pool
    pub fn new() -> Self {
        Self {
            transports: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Get or create a transport for the given configuration.
    ///
    /// Reuses existing transports for the same endpoint to preserve sessions.
    ///
    /// # Arguments
    ///
    /// * `config` - Transport configuration
    ///
    /// # Returns
    ///
    /// Shared reference to transport (maintains session across requests)
    pub fn get_or_create(&self, config: StreamableHttpConfig) -> Arc<StreamableHttpTransport> {
        let key = config.url.clone();

        self.transports
            .entry(key)
            .or_insert_with(|| Arc::new(StreamableHttpTransport::new(config)))
            .clone()
    }

    /// Get pool size (number of unique endpoints)
    pub fn size(&self) -> usize {
        self.transports.len()
    }

    /// Clear all transports (forces session reinitialization)
    pub fn clear(&self) {
        self.transports.clear();
    }
}

impl Default for StreamableHttpTransportPool {
    fn default() -> Self {
        Self::new()
    }
}

// Convert StreamableHttpError to our Error type
impl From<StreamableHttpError> for Error {
    fn from(err: StreamableHttpError) -> Self {
        Error::Transport(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = StreamableHttpConfig {
            url: "http://test".to_string(),
            headers: HashMap::new(),
            timeout_ms: default_timeout_ms(),
        };

        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_pool_creation() {
        let pool = StreamableHttpTransportPool::new();
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_pool_reuse() {
        let pool = StreamableHttpTransportPool::new();

        let config1 = StreamableHttpConfig {
            url: "http://test1".to_string(),
            headers: HashMap::new(),
            timeout_ms: 30000,
        };

        let config2 = StreamableHttpConfig {
            url: "http://test1".to_string(), // Same URL
            headers: HashMap::new(),
            timeout_ms: 30000,
        };

        let t1 = pool.get_or_create(config1);
        let t2 = pool.get_or_create(config2);

        // Should be the same transport (Arc equality)
        assert!(Arc::ptr_eq(&t1, &t2));
        assert_eq!(pool.size(), 1);
    }
}
