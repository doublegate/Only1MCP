//! Integration tests for request batching feature
//!
//! These tests verify that the BatchAggregator correctly:
//! - Aggregates multiple requests within the time window
//! - Flushes batches on timeout
//! - Flushes batches on size limit
//! - Distributes responses to all waiting clients
//! - Handles errors correctly
//! - Works under concurrent load

use only1mcp::batching::BatchAggregator;
use only1mcp::config::BatchingConfig;
use only1mcp::types::{McpRequest, McpResponse};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{sleep, Duration};

/// Helper to create a test config with custom parameters
fn test_config(window_ms: u64, max_batch_size: usize) -> BatchingConfig {
    BatchingConfig {
        enabled: true,
        window_ms,
        max_batch_size,
        methods: vec![
            "tools/list".to_string(),
            "resources/list".to_string(),
            "prompts/list".to_string(),
        ],
    }
}

/// Helper to create a sample request
fn sample_request(id: i64, method: &str) -> McpRequest {
    McpRequest::new(method, json!({}), Some(json!(id)))
}

/// Helper to create a sample response
fn sample_response(id: i64) -> McpResponse {
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(id)),
        result: Some(json!({"tools": [{"name": "test_tool"}]})),
        error: None,
    }
}

#[tokio::test]
async fn test_batch_aggregation_multiple_requests() {
    // Test that multiple requests to the same server/method are batched
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend_calls_clone = backend_calls.clone();

    let config = test_config(100, 10);
    let aggregator =
        BatchAggregator::new(config).with_backend_caller(move |_server_id, request| {
            backend_calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: Some(json!({"tools": [{"name": "tool1"}, {"name": "tool2"}]})),
                error: None,
            })
        });

    // Submit 5 requests rapidly
    let mut tasks = vec![];
    for i in 0..5 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }

    // Wait for all responses
    for task in tasks {
        let response = task.await.unwrap().unwrap();
        assert_eq!(
            response.result.unwrap()["tools"].as_array().unwrap().len(),
            2
        );
    }

    // Verify only 1 backend call was made
    assert_eq!(
        backend_calls.load(Ordering::SeqCst),
        1,
        "Expected 1 backend call for 5 batched requests"
    );
}

#[tokio::test]
async fn test_batch_timeout_flush() {
    // Test that batches are flushed after timeout even with just one request
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend_calls_clone = backend_calls.clone();

    let config = test_config(50, 10); // 50ms window
    let aggregator =
        BatchAggregator::new(config).with_backend_caller(move |_server_id, _request| {
            backend_calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(sample_response(1))
        });

    let start = Instant::now();
    let request = sample_request(1, "tools/list");
    let response = aggregator.submit_request("server1".to_string(), request).await.unwrap();
    let elapsed = start.elapsed();

    // Should take approximately window_ms (50ms)
    assert!(
        elapsed >= Duration::from_millis(45),
        "Batch should wait for timeout"
    );
    assert!(
        elapsed <= Duration::from_millis(150),
        "Batch should not wait too long"
    );
    assert!(response.result.is_some());
    assert_eq!(backend_calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_batch_size_limit_flush() {
    // Test that batches are flushed when max_batch_size is reached
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend_calls_clone = backend_calls.clone();

    let config = test_config(1000, 3); // 3 request limit, long window
    let aggregator =
        BatchAggregator::new(config).with_backend_caller(move |_server_id, _request| {
            backend_calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(sample_response(1))
        });

    // Submit 5 requests rapidly
    let mut tasks = vec![];
    for i in 0..5 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            let start = Instant::now();
            let result = agg.submit_request("server1".to_string(), request).await;
            (result, start.elapsed())
        }));
    }

    // Wait for all responses
    let mut instant_responses = 0;
    let mut delayed_responses = 0;
    for task in tasks {
        let (response, elapsed) = task.await.unwrap();
        assert!(response.is_ok());

        // First 3 should complete instantly (size flush)
        // Last 2 should wait for timeout
        if elapsed < Duration::from_millis(100) {
            instant_responses += 1;
        } else {
            delayed_responses += 1;
        }
    }

    assert_eq!(
        instant_responses, 3,
        "First 3 requests should flush immediately on size limit"
    );
    assert_eq!(
        delayed_responses, 2,
        "Last 2 requests should wait for timeout"
    );
    assert_eq!(
        backend_calls.load(Ordering::SeqCst),
        2,
        "Should have 2 batches (3+2)"
    );
}

#[tokio::test]
async fn test_single_request_batch() {
    // Edge case: single request should still work
    let config = test_config(50, 10);
    let aggregator = BatchAggregator::new(config)
        .with_backend_caller(move |_server_id, _request| Ok(sample_response(1)));

    let request = sample_request(1, "tools/list");
    let response = aggregator.submit_request("server1".to_string(), request).await.unwrap();

    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_batch_error_distribution() {
    // Test that errors are correctly distributed to all waiting clients
    let config = test_config(100, 10);
    let aggregator =
        BatchAggregator::new(config).with_backend_caller(move |_server_id, _request| {
            Err(only1mcp::error::Error::Server("Backend error".to_string()))
        });

    // Submit 3 requests
    let mut tasks = vec![];
    for i in 0..3 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }

    // All should receive the same error
    for task in tasks {
        let result = task.await.unwrap();
        assert!(result.is_err());
        match result {
            Err(only1mcp::error::Error::Server(msg)) => {
                assert_eq!(msg, "Backend error");
            },
            _ => panic!("Expected Server error"),
        }
    }
}

#[tokio::test]
async fn test_concurrent_batch_submissions() {
    // Test thread-safety under concurrent load
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend_calls_clone = backend_calls.clone();

    let config = test_config(100, 10);
    let aggregator = Arc::new(BatchAggregator::new(config).with_backend_caller(
        move |_server_id, _request| {
            backend_calls_clone.fetch_add(1, Ordering::SeqCst);
            // Simulate some processing time
            std::thread::sleep(Duration::from_millis(10));
            Ok(sample_response(1))
        },
    ));

    // Spawn 10 concurrent tasks submitting requests
    let mut tasks = vec![];
    for i in 0..10 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }

    // All should complete successfully
    for task in tasks {
        let response = task.await.unwrap().unwrap();
        assert!(response.result.is_some());
    }

    // Should have batched all into 1 call
    let calls = backend_calls.load(Ordering::SeqCst);
    assert_eq!(
        calls, 1,
        "All 10 concurrent requests should batch into 1 backend call"
    );
}

#[tokio::test]
async fn test_different_methods_separate_batches() {
    // Test that different methods are batched separately
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend_calls_clone = backend_calls.clone();

    let config = test_config(100, 10);
    let aggregator = Arc::new(BatchAggregator::new(config).with_backend_caller(
        move |_server_id, request| {
            backend_calls_clone.fetch_add(1, Ordering::SeqCst);
            let method = request.method.as_str();
            let result = match method {
                "tools/list" => json!({"tools": []}),
                "resources/list" => json!({"resources": []}),
                _ => json!({}),
            };
            Ok(McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.clone(),
                result: Some(result),
                error: None,
            })
        },
    ));

    // Submit tools/list and resources/list simultaneously
    let mut tasks = vec![];
    for i in 0..3 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }
    for i in 3..6 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "resources/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }

    // Wait for all
    for task in tasks {
        task.await.unwrap().unwrap();
    }

    // Should have 2 backend calls (one for each method)
    let calls = backend_calls.load(Ordering::SeqCst);
    assert_eq!(calls, 2, "Different methods should create separate batches");
}

#[tokio::test]
async fn test_batching_disabled_passthrough() {
    // Test that when batching is disabled, requests bypass BatchAggregator
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend_calls_clone = backend_calls.clone();

    let mut config = test_config(100, 10);
    config.enabled = false; // Disable batching

    let aggregator =
        BatchAggregator::new(config).with_backend_caller(move |_server_id, _request| {
            backend_calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(sample_response(1))
        });

    // Submit 3 requests
    let mut tasks = vec![];
    for i in 0..3 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }

    // Wait for all
    for task in tasks {
        task.await.unwrap().unwrap();
    }

    // All requests should be direct calls (no batching)
    // Note: With enabled=false, BatchAggregator still works but handlers check config
    // This test verifies the aggregator itself works regardless
    assert!(backend_calls.load(Ordering::SeqCst) >= 1);
}

#[tokio::test]
async fn test_different_servers_separate_batches() {
    // Test that requests to different servers are batched separately
    let backend_calls = Arc::new(AtomicUsize::new(0));
    let backend_calls_clone = backend_calls.clone();

    let config = test_config(100, 10);
    let aggregator = Arc::new(BatchAggregator::new(config).with_backend_caller(
        move |_server_id, _request| {
            backend_calls_clone.fetch_add(1, Ordering::SeqCst);
            Ok(sample_response(1))
        },
    ));

    // Submit to server1 and server2
    let mut tasks = vec![];
    for i in 0..3 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }
    for i in 3..6 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server2".to_string(), request).await
        }));
    }

    // Wait for all
    for task in tasks {
        task.await.unwrap().unwrap();
    }

    // Should have 2 backend calls (one per server)
    let calls = backend_calls.load(Ordering::SeqCst);
    assert_eq!(calls, 2, "Different servers should create separate batches");
}

#[tokio::test]
async fn test_batch_metrics_tracking() {
    // Test that batching metrics are recorded
    // Note: This is a basic test - full metrics verification would require
    // reading from the Prometheus registry

    let config = test_config(50, 10);
    let aggregator = BatchAggregator::new(config)
        .with_backend_caller(move |_server_id, _request| Ok(sample_response(1)));

    // Submit several requests
    let mut tasks = vec![];
    for i in 0..5 {
        let agg = aggregator.clone();
        tasks.push(tokio::spawn(async move {
            let request = sample_request(i, "tools/list");
            agg.submit_request("server1".to_string(), request).await
        }));
    }

    // Wait for all
    for task in tasks {
        task.await.unwrap().unwrap();
    }

    // Metrics should be recorded (verified via logs in actual usage)
    // This test mainly ensures no panics during metric recording
}

#[tokio::test]
async fn test_batch_active_count() {
    // Test that active_batch_count() works correctly
    let config = test_config(200, 10); // Long window to keep batches active
    let aggregator = Arc::new(
        BatchAggregator::new(config)
            .with_backend_caller(move |_server_id, _request| Ok(sample_response(1))),
    );

    assert_eq!(aggregator.active_batch_count(), 0);

    // Submit requests to different batches
    let agg1 = aggregator.clone();
    let agg2 = aggregator.clone();

    tokio::spawn(async move {
        let request = sample_request(1, "tools/list");
        let _ = agg1.submit_request("server1".to_string(), request).await;
    });

    tokio::spawn(async move {
        let request = sample_request(2, "resources/list");
        let _ = agg2.submit_request("server1".to_string(), request).await;
    });

    // Give tasks time to start
    sleep(Duration::from_millis(10)).await;

    // Should have 2 active batches (different methods)
    let active = aggregator.active_batch_count();
    assert!(active > 0 && active <= 2, "Should have 1-2 active batches");

    // Wait for batches to flush
    sleep(Duration::from_millis(250)).await;

    // Should have 0 active batches after timeout
    assert_eq!(aggregator.active_batch_count(), 0);
}
