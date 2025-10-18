//! Mock data generators for benchmarking
//!
//! This module provides utilities for generating realistic mock data
//! for performance benchmarks. All mocks are deterministic to ensure
//! reproducible results across benchmark runs.

use only1mcp::proxy::registry::AtomicRegistry;
use only1mcp::types::{McpRequest, McpResponse, Tool, Resource, Prompt};
use std::sync::Arc;
use serde_json::{json, Value};

/// Creates a mock server registry with the specified number of servers
///
/// # Arguments
/// * `n` - Number of mock servers to create
///
/// # Returns
/// Arc-wrapped AtomicRegistry populated with mock servers
///
/// # Example
/// ```ignore
/// let registry = mock_registry(50);  // 50 servers for benchmarking
/// ```
pub fn mock_registry(n: usize) -> Arc<AtomicRegistry> {
    let registry = AtomicRegistry::new();

    for i in 0..n {
        let server_id = format!("server-{}", i);
        let endpoint = format!("http://localhost:{}", 9000 + i);
        let weight = ((i % 10) + 1) as u32;  // Weights 1-10

        registry.register_server(
            server_id,
            endpoint,
            weight,
            vec![format!("capability-{}", i % 5)],  // 5 different capabilities
        );
    }

    Arc::new(registry)
}

/// Creates a mock MCP request
///
/// # Arguments
/// * `method` - The JSON-RPC method name
///
/// # Returns
/// McpRequest with deterministic ID and params
///
/// # Example
/// ```ignore
/// let request = mock_request("tools/list");
/// ```
pub fn mock_request(method: &str) -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        method: method.to_string(),
        params: Some(json!({
            "key": "value",
            "timestamp": 1234567890
        })),
    }
}

/// Creates a mock MCP response with specified data size
///
/// # Arguments
/// * `size` - Approximate size in bytes (actual size may vary)
///
/// # Returns
/// McpResponse with payload of approximately the specified size
///
/// # Example
/// ```ignore
/// let response = mock_response(10_000);  // ~10KB response
/// ```
pub fn mock_response(size: usize) -> McpResponse {
    // Generate a string of approximately size/2 (since each char ~2 bytes in JSON)
    let data_len = size / 2;
    let data = "x".repeat(data_len);

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({
            "data": data,
            "timestamp": 1234567890
        })),
        error: None,
    }
}

/// Creates a mock tool list response
///
/// # Arguments
/// * `count` - Number of tools to include
///
/// # Returns
/// McpResponse containing a list of mock tools
pub fn mock_tools_response(count: usize) -> McpResponse {
    let tools: Vec<Tool> = (0..count)
        .map(|i| Tool {
            name: format!("tool-{}", i),
            description: Some(format!("Description for tool {}", i)),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "param1": { "type": "string" },
                    "param2": { "type": "number" }
                }
            }),
        })
        .collect();

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({ "tools": tools })),
        error: None,
    }
}

/// Creates a mock resource list response
///
/// # Arguments
/// * `count` - Number of resources to include
///
/// # Returns
/// McpResponse containing a list of mock resources
pub fn mock_resources_response(count: usize) -> McpResponse {
    let resources: Vec<Resource> = (0..count)
        .map(|i| Resource {
            uri: format!("resource://{}", i),
            name: format!("Resource {}", i),
            description: Some(format!("Description for resource {}", i)),
            mime_type: Some("application/json".to_string()),
        })
        .collect();

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({ "resources": resources })),
        error: None,
    }
}

/// Creates a mock prompt list response
///
/// # Arguments
/// * `count` - Number of prompts to include
///
/// # Returns
/// McpResponse containing a list of mock prompts
pub fn mock_prompts_response(count: usize) -> McpResponse {
    let prompts: Vec<Prompt> = (0..count)
        .map(|i| Prompt {
            name: format!("prompt-{}", i),
            description: Some(format!("Description for prompt {}", i)),
            arguments: vec![
                json!({
                    "name": "arg1",
                    "description": "First argument",
                    "required": true
                })
            ],
        })
        .collect();

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: json!(1),
        result: Some(json!({ "prompts": prompts })),
        error: None,
    }
}

/// Simulates concurrent clients making requests
///
/// # Arguments
/// * `n` - Number of concurrent clients
/// * `task` - The task function to execute for each client
///
/// # Example
/// ```ignore
/// concurrent_clients(10, || {
///     // Each client's task
///     lb.select_server(&registry);
/// });
/// ```
pub fn concurrent_clients<F>(n: usize, task: F)
where
    F: Fn() + Send + Sync + 'static,
{
    use std::thread;

    let task = Arc::new(task);
    let mut handles = vec![];

    for _ in 0..n {
        let task_clone = Arc::clone(&task);
        let handle = thread::spawn(move || {
            task_clone();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_registry() {
        let registry = mock_registry(5);
        assert_eq!(registry.server_count(), 5);
    }

    #[test]
    fn test_mock_request() {
        let req = mock_request("tools/list");
        assert_eq!(req.method, "tools/list");
        assert_eq!(req.jsonrpc, "2.0");
    }

    #[test]
    fn test_mock_response() {
        let resp = mock_response(1000);
        assert!(resp.result.is_some());
    }
}
