//! Integration tests for proxy server startup and lifecycle

mod common;

use common::*;
use serde_json::json;

#[tokio::test]
async fn test_server_starts_and_binds() {
    // Given: A test configuration
    let config = test_config();

    // When: Server is started
    let server = start_test_server(config).await;

    // Then: Server is listening on the specified address
    let client = test_client();

    // Health endpoint should be accessible (503 is OK for no backends)
    let response = client.get(format!("{}/api/v1/admin/health", server.url())).send().await;

    assert!(response.is_ok(), "Failed to connect to server");
    let response = response.unwrap();
    // Accept both 200 (with backends) and 503 (without backends)
    assert!(
        response.status() == 200 || response.status() == 503,
        "Health check returned unexpected status: {}",
        response.status()
    );
}

#[tokio::test]
async fn test_health_endpoint_returns_status() {
    // Given: A running server with no backends
    let config = test_config();
    let server = start_test_server(config).await;

    // When: Health endpoint is queried
    let client = test_client();
    let response = client
        .get(format!("{}/api/v1/admin/health", server.url()))
        .send()
        .await
        .expect("Failed to send request");

    // Then: Response contains health status (unhealthy due to no backends)
    assert_eq!(response.status(), 503);

    let body: serde_json::Value = response.json().await.expect("Failed to parse JSON");
    assert!(body.get("status").is_some(), "Missing status field");
    assert_eq!(body["status"], "unhealthy");
    assert_eq!(body["servers"], 0);
}

#[tokio::test]
async fn test_metrics_endpoint_accessible() {
    // Given: A running server
    let config = test_config();
    let server = start_test_server(config).await;

    // When: Metrics endpoint is queried
    let client = test_client();
    let response = client
        .get(format!("{}/api/v1/admin/metrics", server.url()))
        .send()
        .await
        .expect("Failed to send request");

    // Then: Endpoint is accessible
    assert_eq!(response.status(), 200);

    // Response should have correct content type
    let content_type = response.headers().get("content-type").and_then(|v| v.to_str().ok());

    assert!(
        content_type.is_some_and(|ct| ct.contains("text/plain")),
        "Expected text/plain content type"
    );

    // Body should be valid (may be empty if no metrics recorded yet)
    let body = response.text().await.expect("Failed to read response");
    // Empty is OK for fresh server with no activity
    assert!(
        body.is_empty() || body.contains("# HELP") || body.contains("# TYPE"),
        "Expected empty or Prometheus format, got: {}",
        &body[..body.len().min(200)]
    );
}

#[tokio::test]
async fn test_server_handles_invalid_json() {
    // Given: A running server
    let config = test_config();
    let server = start_test_server(config).await;

    // When: Invalid JSON is sent
    let client = test_client();
    let response = client
        .post(format!("{}/", server.url()))
        .header("content-type", "application/json")
        .body("invalid json{")
        .send()
        .await
        .expect("Failed to send request");

    // Then: Server returns 400 Bad Request
    assert!(
        response.status().is_client_error(),
        "Should return 4xx error"
    );
}

#[tokio::test]
async fn test_server_handles_missing_method() {
    // Given: A running server
    let config = test_config();
    let server = start_test_server(config).await;

    // When: Request without method is sent
    let client = test_client();
    let response = client
        .post(format!("{}/", server.url()))
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "params": {}
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Then: Server returns error response
    // Server should return 200 with JSON-RPC error in body, or 400 for invalid format
    assert!(
        response.status().is_success() || response.status().is_client_error(),
        "Expected 2xx or 4xx status, got {}",
        response.status()
    );

    // If we get a JSON body, it should have an error field or be a JSON-RPC error
    if let Ok(body) = response.json::<serde_json::Value>().await {
        // JSON-RPC should have jsonrpc field and either error or result
        if body.get("jsonrpc").is_some() {
            assert!(
                body.get("error").is_some() || body.get("result").is_some(),
                "JSON-RPC response should have error or result field"
            );

            // If there's an error, it should have a code
            if let Some(error) = body.get("error") {
                assert!(
                    error.get("code").is_some(),
                    "JSON-RPC error should have a code field"
                );
            }
        }
    }
}

#[tokio::test]
async fn test_concurrent_requests() {
    // Given: A running server
    let config = test_config();
    let server = start_test_server(config).await;

    // When: Multiple concurrent requests are sent
    let client = test_client();
    let mut tasks = vec![];

    for i in 0..10 {
        let client = client.clone();
        let url = format!("{}/api/v1/admin/health", server.url());

        tasks.push(tokio::spawn(async move {
            client.get(&url).send().await.unwrap_or_else(|_| panic!("Request {} failed", i))
        }));
    }

    // Then: All requests succeed (503 is OK for no backends)
    for task in tasks {
        let response = task.await.expect("Task panicked");
        assert!(
            response.status() == 200 || response.status() == 503,
            "Health check returned unexpected status: {}",
            response.status()
        );
    }
}
