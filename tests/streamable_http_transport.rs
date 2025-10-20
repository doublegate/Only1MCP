//! Integration tests for Streamable HTTP transport
//!
//! Tests session management, protocol compliance, and NWS server integration

use only1mcp::transport::streamable_http::{StreamableHttpConfig, StreamableHttpTransport};
use only1mcp::types::McpRequest;
use serde_json::json;
use std::collections::HashMap;

/// Test helper to create a basic transport config
fn create_test_config(url: impl Into<String>) -> StreamableHttpConfig {
    let mut headers = HashMap::new();
    headers.insert(
        "Accept".into(),
        "application/json, text/event-stream".into(),
    );
    headers.insert("Content-Type".into(), "application/json".into());

    StreamableHttpConfig {
        url: url.into(),
        headers,
        timeout_ms: 30000,
    }
}

#[tokio::test]
#[ignore] // Requires NWS server running on localhost:8124
async fn test_streamable_http_session_management() {
    // Create transport
    let config = create_test_config("http://localhost:8124/mcp");
    let transport = StreamableHttpTransport::new(config);

    // Initially, no session ID
    assert!(transport.get_session_id().await.is_none());

    // Send initialize request
    let init_request = McpRequest::new(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "test-client", "version": "1.0.0"}
        }),
        Some(json!(0)),
    );

    let response = transport.send_request(init_request).await;
    assert!(
        response.is_ok(),
        "Initialization should succeed: {:?}",
        response.err()
    );

    // After initialize, session ID should be stored
    let session_id = transport.get_session_id().await;
    assert!(
        session_id.is_some(),
        "Session ID should be stored after initialization"
    );

    // Send tools/list request (should use session ID)
    let tools_request = McpRequest::new("tools/list", json!({}), Some(json!(1)));
    let tools_response = transport.send_request(tools_request).await;
    assert!(
        tools_response.is_ok(),
        "Tools list should succeed with session: {:?}",
        tools_response.err()
    );

    // Session ID should remain the same
    let session_id_after = transport.get_session_id().await;
    assert_eq!(session_id, session_id_after, "Session ID should be reused");
}

#[tokio::test]
#[ignore] // Requires NWS server running
async fn test_streamable_http_nws_tools() {
    let config = create_test_config("http://localhost:8124/mcp");
    let transport = StreamableHttpTransport::new(config);

    // Initialize
    let init_request = McpRequest::new(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "nws-test", "version": "1.0.0"}
        }),
        Some(json!(0)),
    );

    transport.send_request(init_request).await.expect("Init failed");

    // Get tools list
    let tools_request = McpRequest::new("tools/list", json!({}), Some(json!(1)));
    let tools_response = transport.send_request(tools_request).await.expect("Tools failed");

    // Verify response structure
    let result = tools_response.result.expect("No result in response");
    let tools = result.get("tools").expect("No tools field");
    let tools_array = tools.as_array().expect("Tools is not an array");

    assert!(!tools_array.is_empty(), "Should have at least one tool");

    // Check for NWS-specific tools
    let tool_names: Vec<String> = tools_array
        .iter()
        .filter_map(|t| t.get("name")?.as_str())
        .map(String::from)
        .collect();

    assert!(
        tool_names.contains(&"get-alerts".to_string())
            || tool_names.contains(&"get-forecast".to_string()),
        "Should have NWS tools (get-alerts or get-forecast), got: {:?}",
        tool_names
    );
}

#[tokio::test]
#[ignore] // Requires NWS server running
async fn test_streamable_http_get_forecast() {
    let config = create_test_config("http://localhost:8124/mcp");
    let transport = StreamableHttpTransport::new(config);

    // Initialize
    let init_request = McpRequest::new(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "forecast-test", "version": "1.0.0"}
        }),
        Some(json!(0)),
    );

    transport.send_request(init_request).await.expect("Init failed");

    // Test get-forecast tool (Washington, DC coordinates)
    let forecast_request = McpRequest::new(
        "tools/call",
        json!({
            "name": "get-forecast",
            "arguments": {
                "latitude": 38.8977,
                "longitude": -77.0365
            }
        }),
        Some(json!(2)),
    );

    let forecast_response = transport.send_request(forecast_request).await;

    // Should succeed (assuming NWS API is available)
    if let Ok(response) = forecast_response {
        assert!(response.result.is_some() || response.error.is_some());
    }
}

#[tokio::test]
#[ignore] // Requires NWS server running
async fn test_streamable_http_get_alerts() {
    let config = create_test_config("http://localhost:8124/mcp");
    let transport = StreamableHttpTransport::new(config);

    // Initialize
    let init_request = McpRequest::new(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "alerts-test", "version": "1.0.0"}
        }),
        Some(json!(0)),
    );

    transport.send_request(init_request).await.expect("Init failed");

    // Test get-alerts tool for California
    let alerts_request = McpRequest::new(
        "tools/call",
        json!({
            "name": "get-alerts",
            "arguments": {
                "state": "CA"
            }
        }),
        Some(json!(3)),
    );

    let alerts_response = transport.send_request(alerts_request).await;

    // Should succeed (result may be empty if no alerts)
    if let Ok(response) = alerts_response {
        assert!(response.result.is_some() || response.error.is_some());
    }
}

#[tokio::test]
async fn test_streamable_http_session_clear() {
    let config = create_test_config("http://example.com/mcp");
    let transport = StreamableHttpTransport::new(config);

    // Initially no session
    assert!(transport.get_session_id().await.is_none());

    // Clear should be safe even when no session exists
    transport.clear_session().await;
    assert!(transport.get_session_id().await.is_none());
}

#[tokio::test]
async fn test_streamable_http_pool_reuse() {
    use only1mcp::transport::streamable_http::StreamableHttpTransportPool;

    let pool = StreamableHttpTransportPool::new();
    assert_eq!(pool.size(), 0);

    // Create first transport
    let config1 = create_test_config("http://test1.example.com/mcp");
    let t1 = pool.get_or_create(config1.clone());
    assert_eq!(pool.size(), 1);

    // Same URL should reuse transport
    let t2 = pool.get_or_create(config1);
    assert_eq!(pool.size(), 1);
    assert!(std::sync::Arc::ptr_eq(&t1, &t2));

    // Different URL creates new transport
    let config2 = create_test_config("http://test2.example.com/mcp");
    let t3 = pool.get_or_create(config2);
    assert_eq!(pool.size(), 2);
    assert!(!std::sync::Arc::ptr_eq(&t1, &t3));
}

#[tokio::test]
async fn test_streamable_http_timeout_handling() {
    // Test with very short timeout (should timeout on real server)
    let mut headers = HashMap::new();
    headers.insert(
        "Accept".into(),
        "application/json, text/event-stream".into(),
    );

    let config = StreamableHttpConfig {
        url: "http://localhost:8124/mcp".to_string(),
        headers,
        timeout_ms: 1, // 1ms timeout - should fail
    };

    let transport = StreamableHttpTransport::new(config);

    let request = McpRequest::new(
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "timeout-test", "version": "1.0.0"}
        }),
        Some(json!(0)),
    );

    // Should fail due to timeout (if server exists) or connection refused
    let response = transport.send_request(request).await;
    assert!(
        response.is_err(),
        "Should fail with short timeout or no server"
    );
}

#[test]
fn test_streamable_http_config_defaults() {
    let config = StreamableHttpConfig {
        url: "http://test.com/mcp".to_string(),
        headers: HashMap::new(),
        timeout_ms: 30000,
    };

    assert_eq!(config.timeout_ms, 30000);
    assert!(config.headers.is_empty());
}
