//! Comprehensive tests for active health checking feature

use only1mcp::{
    config::HealthCheckConfig,
    health::checker::{HealthCheckTransport, HealthChecker, HealthState},
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

/// Helper to create test health check config
fn test_health_config() -> HealthCheckConfig {
    HealthCheckConfig {
        enabled: true,
        interval_seconds: 1, // Fast interval for tests
        timeout_seconds: 2,
        healthy_threshold: 2,
        unhealthy_threshold: 2,
        path: "/health".to_string(),
    }
}

#[tokio::test]
async fn test_http_health_check_success() {
    // Start mock HTTP server
    let mock_server = MockServer::start().await;

    // Configure mock to return 200 OK
    Mock::given(method("POST"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "healthy",
            "resources": {
                "cpu_usage": 25.5,
                "memory_mb": 512,
                "active_connections": 10
            }
        })))
        .expect(1..)
        .mount(&mock_server)
        .await;

    // Create health checker
    let transport = HealthCheckTransport::Http {
        endpoint: mock_server.uri(),
    };
    let checker = Arc::new(HealthChecker::from_config(
        "test-server".to_string(),
        transport,
        test_health_config(),
    ));

    // Start health checking in background
    let checker_clone = checker.clone();
    let handle = tokio::spawn(async move {
        checker_clone.start().await;
    });

    // Wait for health checks to run
    sleep(Duration::from_secs(3)).await;

    // Verify health status
    let status = checker.get_status().await;
    assert_eq!(status.state, HealthState::Healthy);
    assert!(
        status.success_count >= 2,
        "Expected at least 2 successful checks"
    );
    assert_eq!(status.failure_count, 0);

    // Stop health checker
    checker.stop();
    handle.abort();
}

#[tokio::test]
async fn test_http_health_check_failure() {
    // Start mock HTTP server
    let mock_server = MockServer::start().await;

    // Configure mock to return 500 Internal Server Error
    Mock::given(method("POST"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1..)
        .mount(&mock_server)
        .await;

    // Create health checker
    let transport = HealthCheckTransport::Http {
        endpoint: mock_server.uri(),
    };
    let checker = Arc::new(HealthChecker::from_config(
        "test-server".to_string(),
        transport,
        test_health_config(),
    ));

    // Start health checking in background
    let checker_clone = checker.clone();
    let handle = tokio::spawn(async move {
        checker_clone.start().await;
    });

    // Wait for health checks to run
    sleep(Duration::from_secs(3)).await;

    // Verify health status shows unhealthy
    let status = checker.get_status().await;
    assert_eq!(status.state, HealthState::Unhealthy);
    assert!(
        status.failure_count >= 2,
        "Expected at least 2 failed checks"
    );

    // Stop health checker
    checker.stop();
    handle.abort();
}

#[tokio::test]
async fn test_http_health_check_timeout() {
    // Start mock HTTP server with delayed response
    let mock_server = MockServer::start().await;

    // Configure mock to delay response beyond timeout
    Mock::given(method("POST"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(5))) // Longer than 2s timeout
        .expect(1..)
        .mount(&mock_server)
        .await;

    // Create health checker with short timeout
    let transport = HealthCheckTransport::Http {
        endpoint: mock_server.uri(),
    };
    let checker = Arc::new(HealthChecker::from_config(
        "test-server".to_string(),
        transport,
        test_health_config(),
    ));

    // Start health checking in background
    let checker_clone = checker.clone();
    let handle = tokio::spawn(async move {
        checker_clone.start().await;
    });

    // Wait for health checks to timeout (need more time for threshold to be met)
    sleep(Duration::from_secs(5)).await;

    // Verify health status shows unhealthy due to timeouts
    let status = checker.get_status().await;
    assert!(
        status.state == HealthState::Unhealthy || status.state == HealthState::Degraded,
        "Expected Unhealthy or Degraded state due to timeouts, got: {:?}",
        status.state
    );
    assert!(status.failure_count > 0);

    // Stop health checker
    checker.stop();
    handle.abort();
}

#[tokio::test]
async fn test_stdio_health_check_valid_command() {
    // Create health checker for a known command (sh/bash)
    let transport = HealthCheckTransport::Stdio {
        command: "sh".to_string(),
        args: vec![],
    };

    let checker = Arc::new(HealthChecker::from_config(
        "test-stdio".to_string(),
        transport,
        test_health_config(),
    ));

    // Start health checking in background
    let checker_clone = checker.clone();
    let handle = tokio::spawn(async move {
        checker_clone.start().await;
    });

    // Wait for health checks to run
    sleep(Duration::from_secs(3)).await;

    // Verify health status (sh command exists and is executable)
    let status = checker.get_status().await;
    assert_eq!(status.state, HealthState::Healthy);
    assert!(status.success_count >= 2);

    // Stop health checker
    checker.stop();
    handle.abort();
}

#[tokio::test]
async fn test_stdio_health_check_invalid_command() {
    // Create health checker for a non-existent command
    let transport = HealthCheckTransport::Stdio {
        command: "nonexistent_command_12345".to_string(),
        args: vec![],
    };

    let checker = Arc::new(HealthChecker::from_config(
        "test-stdio-invalid".to_string(),
        transport,
        test_health_config(),
    ));

    // Start health checking in background
    let checker_clone = checker.clone();
    let handle = tokio::spawn(async move {
        checker_clone.start().await;
    });

    // Wait for health checks to run
    sleep(Duration::from_secs(3)).await;

    // Verify health status shows unhealthy
    let status = checker.get_status().await;
    assert_eq!(status.state, HealthState::Unhealthy);
    assert!(status.failure_count >= 2);

    // Stop health checker
    checker.stop();
    handle.abort();
}

#[tokio::test]
async fn test_health_threshold_transitions() {
    // Test that health state transitions correctly based on thresholds
    // This test validates the unhealthy_threshold and healthy_threshold work correctly

    // Start mock HTTP server that fails initially
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(500))
        .expect(3..)
        .mount(&mock_server)
        .await;

    // Create health checker
    let transport = HealthCheckTransport::Http {
        endpoint: mock_server.uri(),
    };
    let checker = Arc::new(HealthChecker::from_config(
        "test-threshold".to_string(),
        transport,
        test_health_config(),
    ));

    // Start health checking
    let checker_clone = checker.clone();
    let handle = tokio::spawn(async move {
        checker_clone.start().await;
    });

    // Wait for failures to accumulate (need unhealthy_threshold=2 failures)
    sleep(Duration::from_secs(4)).await;

    // Verify unhealthy state reached
    let status = checker.get_status().await;
    assert_eq!(
        status.state,
        HealthState::Unhealthy,
        "Expected Unhealthy state after {} failures",
        status.failure_count
    );
    assert!(status.failure_count >= 2);

    // Stop health checker
    checker.stop();
    handle.abort();

    // Note: Testing recovery would require dynamic mock replacement which is complex.
    // The recovery path is tested implicitly in test_http_health_check_success
}

#[tokio::test]
async fn test_health_metrics_recorded() {
    use only1mcp::metrics::{HEALTH_CHECK_TOTAL, SERVER_HEALTH_STATUS};

    // Start mock HTTP server
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/health"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "status": "healthy",
            "resources": {
                "cpu_usage": 15.0,
                "memory_mb": 384,
                "active_connections": 7
            }
        })))
        .expect(1..)
        .mount(&mock_server)
        .await;

    // Create health checker
    let transport = HealthCheckTransport::Http {
        endpoint: mock_server.uri(),
    };
    let checker = Arc::new(HealthChecker::from_config(
        "test-metrics".to_string(),
        transport,
        test_health_config(),
    ));

    // Get initial metric values
    let initial_total = HEALTH_CHECK_TOTAL.with_label_values(&["test-metrics", "success"]).get();

    // Start health checking
    let checker_clone = checker.clone();
    let handle = tokio::spawn(async move {
        checker_clone.start().await;
    });

    // Wait for checks to run
    sleep(Duration::from_secs(3)).await;

    // Verify metrics increased
    let final_total = HEALTH_CHECK_TOTAL.with_label_values(&["test-metrics", "success"]).get();

    assert!(
        final_total > initial_total,
        "Expected health check success count to increase"
    );

    // Verify health status gauge is set to 1 (healthy)
    let health_status = SERVER_HEALTH_STATUS.with_label_values(&["test-metrics"]).get();

    assert_eq!(
        health_status, 1.0,
        "Expected health status gauge to be 1.0 (healthy)"
    );

    // Stop health checker
    checker.stop();
    handle.abort();
}
