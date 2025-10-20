//! Integration tests for SSE transport
//!
//! Tests the SSE transport implementation with real Context7 MCP server.

use only1mcp::transport::sse::{SseTransport, SseTransportConfig, SseTransportPool};
use only1mcp::types::McpRequest;
use serde_json::json;
use std::collections::HashMap;

/// Test direct SSE transport with Context7
///
/// This test makes a real network request to Context7's public MCP endpoint.
/// It verifies that:
/// 1. SSE transport can connect to Context7
/// 2. SSE response parsing works correctly
/// 3. Context7 returns the expected tools (resolve-library-id, get-library-docs)
#[tokio::test]
#[ignore] // Requires network access - run with --ignored flag
async fn test_context7_tools_list() {
    // Create SSE transport config for Context7
    let mut headers = HashMap::new();
    headers.insert(
        "Accept".to_string(),
        "application/json, text/event-stream".to_string(),
    );
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let config = SseTransportConfig {
        base_url: "https://mcp.context7.com/mcp".to_string(),
        request_timeout: std::time::Duration::from_secs(30),
        headers,
    };

    // Create transport
    let transport = SseTransport::new(config).await.expect("Failed to create SSE transport");

    // Create tools/list request
    let request = McpRequest::new("tools/list", json!({}), Some(json!(1)));

    // Send request
    let response = transport
        .send_request("https://mcp.context7.com/mcp", request)
        .await
        .expect("Failed to send request to Context7");

    // Verify response structure
    assert!(
        response.result.is_some(),
        "Response should have result field"
    );

    let result = response.result.unwrap();
    assert!(
        result.get("tools").is_some(),
        "Result should have tools field"
    );

    let tools = result.get("tools").unwrap();
    assert!(tools.is_array(), "Tools should be an array");

    // Verify Context7's expected tools are present
    let tools_array = tools.as_array().unwrap();
    assert!(!tools_array.is_empty(), "Tools array should not be empty");

    // Check for expected Context7 tools
    let tool_names: Vec<String> = tools_array
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();

    assert!(
        tool_names.contains(&"resolve-library-id".to_string()),
        "Context7 should provide resolve-library-id tool"
    );
    assert!(
        tool_names.contains(&"get-library-docs".to_string()),
        "Context7 should provide get-library-docs tool"
    );

    println!("✅ Context7 SSE integration test passed!");
    println!("   Found {} tools: {:?}", tool_names.len(), tool_names);
}

/// Test SSE transport pool with caching
///
/// Verifies that the SseTransportPool correctly caches transports
/// for the same endpoint+headers combination.
#[tokio::test]
async fn test_sse_pool_caching() {
    let pool = SseTransportPool::default();

    let endpoint = "https://mcp.context7.com/mcp";
    let mut headers = HashMap::new();
    headers.insert(
        "Accept".to_string(),
        "application/json, text/event-stream".to_string(),
    );

    // First request creates transport
    let transport1 = pool.get_or_create(endpoint, headers.clone()).await.unwrap();

    // Second request should return cached transport
    let transport2 = pool.get_or_create(endpoint, headers).await.unwrap();

    // Should be the same Arc instance
    assert!(
        std::sync::Arc::ptr_eq(&transport1, &transport2),
        "Transport pool should cache by endpoint+headers"
    );
}

/// Test SSE pool with different headers creates different transports
#[tokio::test]
async fn test_sse_pool_different_headers() {
    let pool = SseTransportPool::default();

    let endpoint = "https://mcp.context7.com/mcp";

    let mut headers1 = HashMap::new();
    headers1.insert("Authorization".to_string(), "Bearer token1".to_string());

    let mut headers2 = HashMap::new();
    headers2.insert("X-API-Key".to_string(), "key123".to_string());

    let transport1 = pool.get_or_create(endpoint, headers1).await.unwrap();
    let transport2 = pool.get_or_create(endpoint, headers2).await.unwrap();

    // Should be different Arc instances (different header values)
    assert!(
        !std::sync::Arc::ptr_eq(&transport1, &transport2),
        "Different headers should create different transports"
    );
}

/// Test SSE pool convenience method (no headers)
#[tokio::test]
#[ignore] // Requires network access
async fn test_sse_pool_send_request() {
    let pool = SseTransportPool::default();

    let endpoint = "https://mcp.context7.com/mcp";
    let request = McpRequest::new("tools/list", json!({}), Some(json!(1)));

    // This will fail without headers (Context7 requires Accept header)
    // but verifies the API works
    let result = pool.send_request(endpoint, request).await;

    // We expect this to fail (no Accept header) but the pool should work
    assert!(result.is_err(), "Request without headers should fail");
}

/// Test error handling for invalid SSE endpoint
#[tokio::test]
async fn test_sse_error_handling_invalid_endpoint() {
    let config = SseTransportConfig {
        base_url: "https://invalid.example.com".to_string(),
        request_timeout: std::time::Duration::from_secs(5),
        headers: HashMap::new(),
    };

    let transport = SseTransport::new(config).await.unwrap();

    let request = McpRequest::new("tools/list", json!({}), Some(json!(1)));

    let result = transport.send_request("https://invalid.example.com", request).await;

    assert!(result.is_err(), "Invalid endpoint should return error");
}

/// Test error handling for malformed SSE response
#[tokio::test]
async fn test_sse_error_handling_timeout() {
    let config = SseTransportConfig {
        base_url: "https://httpbin.org/delay/10".to_string(),
        request_timeout: std::time::Duration::from_millis(100), // Very short timeout
        headers: HashMap::new(),
    };

    let transport = SseTransport::new(config).await.unwrap();

    let request = McpRequest::new("tools/list", json!({}), Some(json!(1)));

    let result = transport.send_request("https://httpbin.org/delay/10", request).await;

    assert!(result.is_err(), "Request should timeout with short timeout");
}
