//! Integration test for STDIO MCP protocol initialization

use only1mcp::transport::stdio::{StdioConfig, StdioTransport};
use only1mcp::types::McpRequest;
use serde_json::json;
use std::collections::HashMap;

#[tokio::test]
async fn test_stdio_sequential_thinking_initialization() {
    let transport = StdioTransport::new();

    let config = StdioConfig {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "@modelcontextprotocol/server-sequential-thinking".to_string()],
        env: HashMap::new(),
        cwd: None,
        timeout_ms: 30000,
        max_memory_mb: Some(512),
        max_cpu_percent: Some(50),
        sandbox: false, // Disable sandbox for test
    };

    // Create a tools/list request
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: Some(json!({})),
        id: Some(json!(1)),
    };

    // Send request (should trigger initialization automatically)
    let result = transport.send_request_with_config(
        "test-sequential-thinking".to_string(),
        &config,
        request,
    ).await;

    // Check result
    assert!(result.is_ok(), "Failed to get tools list: {:?}", result.err());

    let response = result.unwrap();
    println!("Response: {:?}", response);

    // Verify we got a response
    assert!(response.result.is_some());

    let tools = response.result.unwrap();
    println!("Tools: {}", serde_json::to_string_pretty(&tools).unwrap());

    // Should have sequentialthinking tool
    let tools_array = tools.get("tools").expect("Missing tools field");
    assert!(tools_array.is_array());
    assert!(tools_array.as_array().unwrap().len() > 0, "No tools returned");

    // Check for sequentialthinking tool
    let has_seq_thinking = tools_array.as_array().unwrap().iter().any(|tool| {
        tool.get("name").and_then(|n| n.as_str()) == Some("sequentialthinking")
    });
    assert!(has_seq_thinking, "sequentialthinking tool not found");
}

#[tokio::test]
async fn test_stdio_memory_initialization() {
    let transport = StdioTransport::new();

    let config = StdioConfig {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "@modelcontextprotocol/server-memory".to_string()],
        env: HashMap::new(),
        cwd: None,
        timeout_ms: 30000,
        max_memory_mb: Some(512),
        max_cpu_percent: Some(50),
        sandbox: false,
    };

    // Create a tools/list request
    let request = McpRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: Some(json!({})),
        id: Some(json!(1)),
    };

    // Send request (should trigger initialization automatically)
    let result = transport.send_request_with_config(
        "test-memory".to_string(),
        &config,
        request,
    ).await;

    // Check result
    assert!(result.is_ok(), "Failed to get tools list: {:?}", result.err());

    let response = result.unwrap();

    // Verify we got a response
    assert!(response.result.is_some());

    let tools = response.result.unwrap();
    println!("Memory tools: {}", serde_json::to_string_pretty(&tools).unwrap());

    // Should have memory tools
    let tools_array = tools.get("tools").expect("Missing tools field");
    assert!(tools_array.is_array());
    let tools_list = tools_array.as_array().unwrap();
    assert!(tools_list.len() > 0, "No tools returned");

    // Check for memory-specific tools
    let tool_names: Vec<String> = tools_list.iter()
        .filter_map(|tool| tool.get("name").and_then(|n| n.as_str()))
        .map(|s| s.to_string())
        .collect();

    println!("Memory tool names: {:?}", tool_names);
    assert!(tool_names.contains(&"create_entities".to_string()) ||
            tool_names.contains(&"read_graph".to_string()) ||
            tool_names.len() > 3,
            "Expected memory tools not found");
}
