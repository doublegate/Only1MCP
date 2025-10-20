//! Common test utilities for integration tests

use only1mcp::config::{
    Config, HealthCheckConfig, McpServerConfig, ProxyConfig, RoutingConfig, ServerConfig,
    TransportConfig,
};
use only1mcp::proxy::server::ProxyServer;
use reqwest::Client;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;
use wiremock::matchers::{body_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Create a test configuration with sensible defaults
pub fn test_config() -> Config {
    test_config_with_port(0)
}

/// Create a test configuration with a specific port
pub fn test_config_with_port(port: u16) -> Config {
    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port,
            worker_threads: 2,
            max_connections: 100,
            tls: Default::default(),
        },
        servers: vec![],
        proxy: ProxyConfig::default(),
        context_optimization: Default::default(),
        auth: Default::default(),
        observability: Default::default(),
        tui: Default::default(),
    }
}

/// Create a test configuration with mock backends for full functionality
#[allow(dead_code)]
pub fn test_config_with_backends(port: u16, backend_urls: Vec<String>) -> Config {
    let mut servers = Vec::new();

    for (i, url) in backend_urls.iter().enumerate() {
        servers.push(McpServerConfig {
            id: format!("test-backend-{}", i),
            name: format!("Test Backend {}", i),
            enabled: true,
            transport: TransportConfig::Http {
                url: url.clone(),
                headers: Default::default(),
            },
            health_check: HealthCheckConfig {
                enabled: false, // Disable health checks for tests
                interval_seconds: 30,
                timeout_seconds: 5,
                healthy_threshold: 2,
                unhealthy_threshold: 3,
                path: "/health".to_string(),
            },
            routing: RoutingConfig {
                tools: vec!["*".to_string()], // Accept all tools
                priority: 100,
                weight: 1,
            },
            weight: 1,
        });
    }

    Config {
        server: ServerConfig {
            host: "127.0.0.1".to_string(),
            port,
            worker_threads: 2,
            max_connections: 100,
            tls: Default::default(),
        },
        servers,
        proxy: ProxyConfig::default(),
        context_optimization: Default::default(),
        auth: Default::default(),
        observability: Default::default(),
        tui: Default::default(),
    }
}

/// Create a mock MCP server configuration
#[allow(dead_code)]
pub fn mock_server_config(id: &str, url: &str) -> McpServerConfig {
    McpServerConfig {
        id: id.to_string(),
        name: format!("Test Server {}", id),
        enabled: true,
        transport: TransportConfig::Http {
            url: url.to_string(),
            headers: Default::default(),
        },
        health_check: HealthCheckConfig {
            enabled: false, // Disable for tests
            interval_seconds: 30,
            timeout_seconds: 5,
            healthy_threshold: 2,
            unhealthy_threshold: 3,
            path: "/health".to_string(),
        },
        routing: RoutingConfig::default(),
        weight: 1,
    }
}

/// Start a test proxy server with the given config
pub async fn start_test_server(mut config: Config) -> TestServer {
    // Use a fixed test port if not specified
    if config.server.port == 0 {
        config.server.port = find_free_port().await;
    }

    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .expect("Invalid address");

    let server = ProxyServer::new(config, std::path::PathBuf::from("test-config.yaml"))
        .await
        .expect("Failed to create server");

    // Spawn server in background
    let handle = tokio::spawn(async move {
        server.run().await.expect("Server failed");
    });

    // Wait for server to be ready
    tokio::time::sleep(Duration::from_millis(200)).await;

    TestServer { addr, handle }
}

/// Find a free port for testing
async fn find_free_port() -> u16 {
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("Failed to bind to port");
    let port = listener.local_addr().expect("Failed to get local addr").port();
    drop(listener);
    port
}

/// Test server handle
pub struct TestServer {
    addr: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    #[allow(dead_code)]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.addr)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Create a test HTTP client
pub fn test_client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to create client")
}

/// Assert that a JSON-RPC response is successful
#[allow(dead_code)]
pub fn assert_jsonrpc_success(response: &Value) {
    assert_eq!(response["jsonrpc"], "2.0", "Invalid JSON-RPC version");
    assert!(
        response.get("result").is_some(),
        "Missing result field: {:?}",
        response
    );
    assert!(
        response.get("error").is_none(),
        "Response has error: {:?}",
        response["error"]
    );
}

/// Assert that a JSON-RPC response has an error
#[allow(dead_code)]
pub fn assert_jsonrpc_error(response: &Value, expected_code: i32) {
    assert_eq!(response["jsonrpc"], "2.0", "Invalid JSON-RPC version");
    assert!(response.get("error").is_some(), "Missing error field");
    assert_eq!(
        response["error"]["code"].as_i64().unwrap(),
        expected_code as i64,
        "Wrong error code"
    );
}

/// Mount a mock tools/list endpoint
#[allow(dead_code)]
pub async fn mount_tools_list(mock: &MockServer, tools: Vec<Value>) {
    Mock::given(method("POST"))
        .and(body_json(json!({"method": "tools/list"})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": null,
            "result": {
                "tools": tools
            }
        })))
        .mount(mock)
        .await;
}

/// Mount a mock tools/call endpoint
#[allow(dead_code)]
pub async fn mount_tools_call(mock: &MockServer, tool_name: &str, response: Value) {
    Mock::given(method("POST"))
        .and(body_json(json!({
            "method": "tools/call",
            "params": {"name": tool_name}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "jsonrpc": "2.0",
            "id": null,
            "result": response
        })))
        .mount(mock)
        .await;
}

/// Create a sample tool JSON object
#[allow(dead_code)]
pub fn sample_tool(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": {},
            "required": []
        }
    })
}

/// Wait for a condition with timeout
#[allow(dead_code)]
pub async fn wait_for<F>(mut condition: F, timeout: Duration) -> bool
where
    F: FnMut() -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if condition() {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    false
}
