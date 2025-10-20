//! SSE (Server-Sent Events) transport implementation
//!
//! Handles SSE-based MCP servers that return responses in Server-Sent Events format.
//! SSE is a text-based protocol where each message consists of:
//! - `event:` line (optional) specifying the event type
//! - `data:` line(s) containing the payload
//! - Empty line separating messages
//!
//! Context7 MCP server uses SSE format with JSON payloads embedded in data fields.

use reqwest::{Client, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

use crate::types::{McpRequest, McpResponse};

/// SSE transport errors
#[derive(Error, Debug)]
pub enum SseError {
    /// Connection to SSE endpoint failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// SSE response format is invalid or malformed
    #[error("Invalid SSE format: {0}")]
    InvalidFormat(String),

    /// HTTP request to SSE endpoint failed
    #[error("Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    /// JSON parsing of SSE data field failed
    #[error("Invalid JSON in SSE data: {0}")]
    InvalidJson(String),

    /// Request timed out
    #[error("Timeout after {0}ms")]
    Timeout(u64),

    /// Server returned non-success status code
    #[error("Server error {0}: {1}")]
    ServerError(StatusCode, String),
}

/// SSE transport configuration
#[derive(Debug, Clone)]
pub struct SseTransportConfig {
    /// Base URL of the SSE server
    pub base_url: String,

    /// Request timeout
    pub request_timeout: Duration,

    /// Custom HTTP headers (e.g., Accept, Content-Type, Authorization)
    pub headers: std::collections::HashMap<String, String>,
}

impl Default for SseTransportConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            request_timeout: Duration::from_secs(30),
            headers: std::collections::HashMap::new(),
        }
    }
}

/// SSE transport implementation
///
/// This transport sends JSON-RPC requests via POST and receives SSE-formatted responses.
/// Unlike HTTP transport, SSE can handle streaming responses, though Context7 currently
/// returns single-message responses.
pub struct SseTransport {
    /// Configuration for this transport
    config: SseTransportConfig,

    /// Reusable HTTP client
    client: Client,
}

impl SseTransport {
    /// Create a new SSE transport
    ///
    /// # Arguments
    ///
    /// * `config` - SSE transport configuration
    ///
    /// # Returns
    ///
    /// * `Ok(SseTransport)` - Successfully created transport
    /// * `Err(SseError)` - Failed to create HTTP client
    pub async fn new(config: SseTransportConfig) -> Result<Self, SseError> {
        let client = Client::builder()
            .timeout(config.request_timeout)
            .build()
            .map_err(|e| SseError::ConnectionFailed(e.to_string()))?;

        Ok(Self { config, client })
    }

    /// Send an MCP request to the SSE endpoint
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Full URL to send the request to
    /// * `request` - MCP JSON-RPC request
    ///
    /// # Returns
    ///
    /// * `Ok(McpResponse)` - Parsed response from server
    /// * `Err(SseError)` - Request or parsing failed
    pub async fn send_request(
        &self,
        endpoint: &str,
        request: McpRequest,
    ) -> Result<McpResponse, SseError> {
        // Build request with SSE headers
        let mut request_builder = self
            .client
            .post(endpoint)
            .json(&request)
            .timeout(self.config.request_timeout)
            .header("Accept", "application/json, text/event-stream")
            .header("Content-Type", "application/json");

        // Apply custom headers from config
        for (key, value) in &self.config.headers {
            request_builder = request_builder.header(key, value);
        }

        // Send request
        let response = request_builder.send().await?;

        // Check status code
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(SseError::ServerError(status, body));
        }

        // Get response body as text (SSE format)
        let body = response.text().await?;

        // Parse SSE format and extract JSON
        self.parse_sse_response(&body)
    }

    /// Parse SSE-formatted response and extract JSON-RPC payload
    ///
    /// SSE format example:
    /// ```text
    /// event: message
    /// data: {"jsonrpc":"2.0","result":{"tools":[...]}}
    /// ```
    ///
    /// Multiple data lines are concatenated together.
    ///
    /// # Arguments
    ///
    /// * `sse_text` - Raw SSE response text
    ///
    /// # Returns
    ///
    /// * `Ok(McpResponse)` - Parsed JSON-RPC response
    /// * `Err(SseError)` - Invalid SSE format or JSON parsing failed
    fn parse_sse_response(&self, sse_text: &str) -> Result<McpResponse, SseError> {
        let mut _event_type: Option<String> = None;
        let mut data_lines = Vec::new();

        for line in sse_text.lines() {
            let trimmed = line.trim();

            // Skip empty lines (message separators)
            if trimmed.is_empty() {
                continue;
            }

            // Parse event type (stored but not currently used - may be needed for future multi-event support)
            if let Some(event) = trimmed.strip_prefix("event:") {
                _event_type = Some(event.trim().to_string());
            }
            // Parse data line
            else if let Some(data) = trimmed.strip_prefix("data:") {
                data_lines.push(data.trim().to_string());
            }
            // Ignore other SSE fields (id, retry, etc.)
        }

        // SSE spec allows multiple data lines that should be concatenated
        let json_str = data_lines.join("\n");

        if json_str.is_empty() {
            return Err(SseError::InvalidFormat(
                "No data found in SSE response".to_string(),
            ));
        }

        // Parse JSON-RPC response
        serde_json::from_str(&json_str)
            .map_err(|e| SseError::InvalidJson(format!("{}: {}", e, json_str)))
    }
}

/// Multi-endpoint SSE transport pool manager
///
/// Maintains a pool of SSE transports, one per unique endpoint. Supports
/// per-request header customization for authentication and content negotiation.
pub struct SseTransportPool {
    /// Transports per endpoint (lazy initialization)
    transports: dashmap::DashMap<String, Arc<SseTransport>>,

    /// Default configuration for new transports
    default_config: SseTransportConfig,
}

impl Default for SseTransportPool {
    fn default() -> Self {
        Self::new(SseTransportConfig::default())
    }
}

impl SseTransportPool {
    /// Create a new SSE transport pool
    ///
    /// # Arguments
    ///
    /// * `config` - Default configuration for new transports
    pub fn new(config: SseTransportConfig) -> Self {
        Self {
            transports: dashmap::DashMap::new(),
            default_config: config,
        }
    }

    /// Get or create an SSE transport for a specific endpoint (for testing)
    ///
    /// This is a test utility method that allows direct access to the transport cache.
    /// In production, use `send_request` or `send_request_with_headers` instead.
    ///
    /// # Arguments
    ///
    /// * `endpoint` - SSE endpoint URL
    /// * `headers` - Custom headers for this transport
    ///
    /// # Returns
    ///
    /// * `Ok(Arc<SseTransport>)` - Cached or newly created transport
    /// * `Err(SseError)` - Transport creation failed
    #[doc(hidden)]
    pub async fn get_or_create(
        &self,
        endpoint: &str,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<Arc<SseTransport>, SseError> {
        self.get_or_create_internal(endpoint, headers).await
    }

    /// Internal get_or_create implementation
    async fn get_or_create_internal(
        &self,
        endpoint: &str,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<Arc<SseTransport>, SseError> {
        // Create cache key that includes headers (for authentication scenarios)
        let cache_key = if headers.is_empty() {
            endpoint.to_string()
        } else {
            // Include sorted headers in key for caching
            let mut header_keys: Vec<String> = headers.keys().cloned().collect();
            header_keys.sort();
            format!("{}:{}", endpoint, header_keys.join(","))
        };

        // Check if we already have a transport for this endpoint+headers combo
        if let Some(transport) = self.transports.get(&cache_key) {
            return Ok(transport.clone());
        }

        // Create new transport
        let config = SseTransportConfig {
            base_url: endpoint.to_string(),
            request_timeout: self.default_config.request_timeout,
            headers,
        };

        let transport = Arc::new(SseTransport::new(config).await?);

        // Store for reuse
        self.transports.insert(cache_key, transport.clone());

        Ok(transport)
    }

    /// Send request to a specific endpoint with custom headers
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Full SSE endpoint URL
    /// * `request` - MCP JSON-RPC request
    /// * `headers` - Custom HTTP headers
    ///
    /// # Returns
    ///
    /// * `Ok(McpResponse)` - Parsed response
    /// * `Err(SseError)` - Request or parsing failed
    pub async fn send_request_with_headers(
        &self,
        endpoint: &str,
        request: McpRequest,
        headers: std::collections::HashMap<String, String>,
    ) -> Result<McpResponse, SseError> {
        // Get or create transport
        let transport = self.get_or_create_internal(endpoint, headers).await?;

        // Send request
        transport.send_request(endpoint, request).await
    }

    /// Send request to a specific endpoint (no custom headers)
    ///
    /// # Arguments
    ///
    /// * `endpoint` - Full SSE endpoint URL
    /// * `request` - MCP JSON-RPC request
    ///
    /// # Returns
    ///
    /// * `Ok(McpResponse)` - Parsed response
    /// * `Err(SseError)` - Request or parsing failed
    pub async fn send_request(
        &self,
        endpoint: &str,
        request: McpRequest,
    ) -> Result<McpResponse, SseError> {
        self.send_request_with_headers(endpoint, request, std::collections::HashMap::new())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_single_line_sse() {
        let config = SseTransportConfig::default();
        let transport = SseTransport {
            config,
            client: Client::new(),
        };

        let sse_text =
            "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}\n\n";
        let response = transport.parse_sse_response(sse_text).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
        assert!(response.result.is_some());
    }

    #[test]
    fn test_parse_multiline_data_sse() {
        let config = SseTransportConfig::default();
        let transport = SseTransport {
            config,
            client: Client::new(),
        };

        // SSE spec allows splitting data across multiple lines
        let sse_text =
            "event: message\ndata: {\"jsonrpc\":\"2.0\",\ndata: \"id\":1,\"result\":{}}\n\n";
        let response = transport.parse_sse_response(sse_text).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
    }

    #[test]
    fn test_parse_no_event_type() {
        let config = SseTransportConfig::default();
        let transport = SseTransport {
            config,
            client: Client::new(),
        };

        // Event type is optional in SSE
        let sse_text = "data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n";
        let response = transport.parse_sse_response(sse_text).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
    }

    #[test]
    fn test_parse_invalid_sse_no_data() {
        let config = SseTransportConfig::default();
        let transport = SseTransport {
            config,
            client: Client::new(),
        };

        let sse_text = "event: message\n\n";
        let result = transport.parse_sse_response(sse_text);

        assert!(result.is_err());
        match result {
            Err(SseError::InvalidFormat(msg)) => {
                assert!(msg.contains("No data found"));
            },
            _ => panic!("Expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_parse_invalid_json() {
        let config = SseTransportConfig::default();
        let transport = SseTransport {
            config,
            client: Client::new(),
        };

        let sse_text = "event: message\ndata: {invalid json}\n\n";
        let result = transport.parse_sse_response(sse_text);

        assert!(result.is_err());
        match result {
            Err(SseError::InvalidJson(_)) => {},
            _ => panic!("Expected InvalidJson error"),
        }
    }

    #[test]
    fn test_parse_with_extra_fields() {
        let config = SseTransportConfig::default();
        let transport = SseTransport {
            config,
            client: Client::new(),
        };

        // SSE can include id, retry, and other fields - should be ignored
        let sse_text = "id: 123\nevent: message\nretry: 10000\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n";
        let response = transport.parse_sse_response(sse_text).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
    }

    #[test]
    fn test_parse_context7_format() {
        let config = SseTransportConfig::default();
        let transport = SseTransport {
            config,
            client: Client::new(),
        };

        // Context7-style response (with jsonrpc added for valid JSON-RPC)
        let sse_text = r#"event: message
data: {"jsonrpc":"2.0","result":{"tools":[{"name":"resolve-library-id","description":"Test"}]}}

"#;
        let response = transport.parse_sse_response(sse_text).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        let tools = result.get("tools").unwrap();
        assert!(tools.is_array());
    }

    #[tokio::test]
    async fn test_transport_pool_caching() {
        let pool = SseTransportPool::default();

        // Create transport for endpoint
        let endpoint = "https://example.com/sse";
        let headers1 = std::collections::HashMap::new();

        let transport1 = pool.get_or_create(endpoint, headers1.clone()).await.unwrap();
        let transport2 = pool.get_or_create(endpoint, headers1).await.unwrap();

        // Should be the same Arc (cached)
        assert!(Arc::ptr_eq(&transport1, &transport2));
    }

    #[tokio::test]
    async fn test_transport_pool_different_headers() {
        let pool = SseTransportPool::default();

        let endpoint = "https://example.com/sse";
        let mut headers1 = std::collections::HashMap::new();
        headers1.insert("Authorization".to_string(), "Bearer token1".to_string());

        let mut headers2 = std::collections::HashMap::new();
        headers2.insert("X-API-Key".to_string(), "key123".to_string());

        let transport1 = pool.get_or_create(endpoint, headers1).await.unwrap();
        let transport2 = pool.get_or_create(endpoint, headers2).await.unwrap();

        // Should be different Arc instances (different header keys)
        assert!(!Arc::ptr_eq(&transport1, &transport2));
    }
}
