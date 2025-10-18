//! Request handler implementations for all MCP endpoints.
//!
//! Handles JSON-RPC requests, tool discovery, resource management,
//! and WebSocket upgrades for the MCP protocol.

use crate::error::{Error, ProxyError, Result};
use crate::proxy::router::RequestRouter;
use crate::proxy::server::AppState;
use crate::types::{McpRequest, McpResponse, Prompt, Resource, Tool};
use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    Json,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};

/// Handle generic JSON-RPC requests.
#[instrument(skip(state, payload))]
pub async fn handle_jsonrpc_request(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    // Parse request
    let request: McpRequest =
        serde_json::from_value(payload).map_err(|e| ProxyError::InvalidRequest(e.to_string()))?;

    // Route to appropriate handler based on method
    let response = match request.method().as_str() {
        "tools/list" => handle_tools_list_impl(state, request).await?,
        "tools/call" => handle_tools_call_impl(state, request).await?,
        "resources/list" => handle_resources_list_impl(state, request).await?,
        "resources/read" => handle_resources_read_impl(state, request).await?,
        "resources/subscribe" => handle_resources_subscribe_impl(state, request).await?,
        "prompts/list" => handle_prompts_list_impl(state, request).await?,
        "prompts/get" => handle_prompts_get_impl(state, request).await?,
        "sampling/createMessage" => handle_sampling_create_impl(state, request).await?,
        _ => {
            // Unknown method, try to route to a backend
            route_generic_request(state, request).await?
        },
    };

    Ok(Json(response))
}

/// Handle tools/list request with aggregation.
async fn handle_tools_list_impl(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    let start = Instant::now();

    // Check cache
    let cache_key = format!("tools:list:{}", state.config.server.port);
    if let Some(cached) = state.cache.get(&cache_key).await {
        state.metrics.cache_hits().inc();
        debug!("Cache hit for tools/list");
        return Ok(serde_json::from_slice(&cached)?);
    }

    // Get all healthy servers
    let registry = state.registry.read().await;
    let servers = registry.get_healthy_servers().await;

    if servers.is_empty() {
        return Err(ProxyError::NoBackendAvailable("No healthy servers".into()));
    }

    // Parallel fetch from all servers (with optional batching)
    let mut tasks = Vec::new();
    for server in servers {
        let state = state.clone();
        let request = request.clone();

        tasks.push(tokio::spawn(async move {
            // Check if batching is enabled for this method
            if state.config.context_optimization.batching.enabled
                && state.config.context_optimization.batching.methods.contains(&request.method)
            {
                // Route through BatchAggregator
                debug!(
                    "Routing tools/list through batch aggregator for server: {}",
                    server
                );
                state.batch_aggregator.submit_request(server.clone(), request).await.and_then(
                    |response| {
                        // Parse response and extract tools array
                        let result = response.result.ok_or_else(|| {
                            Error::Server("No result in tools/list response".into())
                        })?;

                        let tools_value = result
                            .get("tools")
                            .ok_or_else(|| Error::Server("No tools field in response".into()))?;

                        let tools: Vec<Tool> = serde_json::from_value(tools_value.clone())
                            .map_err(|e| {
                                Error::Serialization(format!("Failed to parse tools: {}", e))
                            })?;

                        Ok(tools)
                    },
                )
            } else {
                // Direct backend call (existing path)
                fetch_tools_from_server(state, server, request).await
            }
        }));
    }

    // Wait for all responses
    let results = futures::future::join_all(tasks).await;

    // Store count before consuming results
    let server_count = results.len();

    // Aggregate tools
    let mut all_tools = Vec::new();
    for result in results {
        match result {
            Ok(Ok(tools)) => all_tools.extend(tools),
            Ok(Err(e)) => warn!("Failed to fetch tools: {}", e),
            Err(e) => error!("Task panic: {}", e),
        }
    }

    // Deduplicate tools by name
    all_tools.sort_by(|a, b| a.name.cmp(&b.name));
    all_tools.dedup_by(|a, b| a.name == b.name);

    // Build response
    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id(),
        "result": {
            "tools": all_tools
        }
    });

    // Cache response (5 minute TTL)
    if let Ok(serialized) = serde_json::to_vec(&response) {
        state.cache.set(cache_key, serialized, "tools/list").await;
    }

    state.metrics.tools_list_duration().record(start.elapsed().as_secs_f64());
    info!(
        "Aggregated {} tools from {} servers",
        all_tools.len(),
        server_count
    );
    Ok(response)
}

/// Handle tools/call with routing and retries.
pub async fn handle_tools_call(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    let request: McpRequest = serde_json::from_value(payload)?;
    handle_tools_call_impl(state, request).await.map(Json)
}

async fn handle_tools_call_impl(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    let start = Instant::now();

    // Extract tool name
    let tool_name = request
        .params()
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ProxyError::InvalidRequest("Missing tool name".into()))?;

    debug!("Calling tool: {}", tool_name);

    // Route request
    let router = RequestRouter::new(state.config.proxy.routing.clone());
    let (server_id, _) = router
        .route_request(&request, &*state.registry.read().await, &state.cache)
        .await?;

    // Get server configuration
    let registry = state.registry.read().await;
    let server = registry
        .get_server(&server_id)
        .ok_or_else(|| ProxyError::NoBackendAvailable(tool_name.to_string()))?;

    // Execute with retry
    let response = execute_with_retry(
        || send_request_to_backend(state.clone(), server.clone(), request.clone()),
        3,
    )
    .await?;

    state.metrics.tools_call_duration().record(start.elapsed().as_secs_f64());
    info!("Tool {} executed in {:?}", tool_name, start.elapsed());
    Ok(response)
}

/// Handle resources/list request.
pub async fn handle_resources_list(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    let request: McpRequest = serde_json::from_value(payload)?;
    handle_resources_list_impl(state, request).await.map(Json)
}

async fn handle_resources_list_impl(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    // Similar to tools/list but for resources
    let start = Instant::now();

    // Check cache
    let cache_key = format!("resources:list:{}", state.config.server.port);
    if let Some(cached) = state.cache.get(&cache_key).await {
        return Ok(serde_json::from_slice(&cached)?);
    }

    // Get all healthy servers and aggregate resources
    let registry = state.registry.read().await;
    let servers = registry.get_healthy_servers().await;

    let mut all_resources = Vec::new();
    for server in servers {
        // Check if batching is enabled for this method
        if state.config.context_optimization.batching.enabled
            && state.config.context_optimization.batching.methods.contains(&request.method)
        {
            // Route through BatchAggregator
            debug!(
                "Routing resources/list through batch aggregator for server: {}",
                server
            );
            match state
                .batch_aggregator
                .submit_request(server.clone(), request.clone())
                .await
                .and_then(|response| {
                    // Parse response and extract resources array
                    let result = response.result.ok_or_else(|| {
                        Error::Server("No result in resources/list response".into())
                    })?;

                    let resources_value = result
                        .get("resources")
                        .ok_or_else(|| Error::Server("No resources field in response".into()))?;

                    let resources: Vec<Resource> = serde_json::from_value(resources_value.clone())
                        .map_err(|e| {
                            Error::Serialization(format!("Failed to parse resources: {}", e))
                        })?;

                    Ok(resources)
                }) {
                Ok(resources) => all_resources.extend(resources),
                Err(e) => warn!("Failed to fetch resources: {}", e),
            }
        } else {
            // Direct backend call (existing path)
            match fetch_resources_from_server(&state, server, request.clone()).await {
                Ok(resources) => all_resources.extend(resources),
                Err(e) => warn!("Failed to fetch resources: {}", e),
            }
        }
    }

    // Deduplicate by URI
    all_resources.sort_by(|a, b| a.uri.cmp(&b.uri));
    all_resources.dedup_by(|a, b| a.uri == b.uri);

    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id(),
        "result": {
            "resources": all_resources
        }
    });

    state.metrics.resources_list_duration().record(start.elapsed().as_secs_f64());
    Ok(response)
}

/// Handle resources/read request.
pub async fn handle_resources_read(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    let request: McpRequest = serde_json::from_value(payload)?;
    handle_resources_read_impl(state, request).await.map(Json)
}

async fn handle_resources_read_impl(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    let uri = request
        .params()
        .get("uri")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ProxyError::InvalidRequest("Missing resource URI".into()))?;

    debug!("Reading resource: {}", uri);

    // Route to server that has this resource
    let router = RequestRouter::new(state.config.proxy.routing.clone());
    let (server_id, _) = router
        .route_request(&request, &*state.registry.read().await, &state.cache)
        .await?;

    let server = {
        let registry = state.registry.read().await;
        registry
            .get_server(&server_id)
            .ok_or_else(|| ProxyError::NoBackendAvailable(uri.to_string()))?
            .clone()
    };

    send_request_to_backend(state, server, request).await
}

/// Handle resources/subscribe for real-time updates.
pub async fn handle_resources_subscribe(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    let request: McpRequest = serde_json::from_value(payload)?;
    handle_resources_subscribe_impl(state, request).await.map(Json)
}

async fn handle_resources_subscribe_impl(
    _state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    // For subscriptions, we need to establish a persistent connection
    // This would typically upgrade to WebSocket or SSE
    let uri = request
        .params()
        .get("uri")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ProxyError::InvalidRequest("Missing resource URI".into()))?;

    info!("Subscribing to resource updates: {}", uri);

    // For now, return a subscription ID
    // Full implementation would establish persistent connection
    let subscription_id = uuid::Uuid::new_v4().to_string();

    Ok(json!({
        "jsonrpc": "2.0",
        "id": request.id(),
        "result": {
            "subscriptionId": subscription_id
        }
    }))
}

/// Handle prompts/list request.
pub async fn handle_prompts_list(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    let request: McpRequest = serde_json::from_value(payload)?;
    handle_prompts_list_impl(state, request).await.map(Json)
}

async fn handle_prompts_list_impl(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    // Similar aggregation pattern as tools/list
    let cache_key = format!("prompts:list:{}", state.config.server.port);
    if let Some(cached) = state.cache.get(&cache_key).await {
        return Ok(serde_json::from_slice(&cached)?);
    }

    let registry = state.registry.read().await;
    let servers = registry.get_healthy_servers().await;

    let mut all_prompts = Vec::new();
    for server in servers {
        // Check if batching is enabled for this method
        if state.config.context_optimization.batching.enabled
            && state.config.context_optimization.batching.methods.contains(&request.method)
        {
            // Route through BatchAggregator
            debug!(
                "Routing prompts/list through batch aggregator for server: {}",
                server
            );
            match state
                .batch_aggregator
                .submit_request(server.clone(), request.clone())
                .await
                .and_then(|response| {
                    // Parse response and extract prompts array
                    let result = response.result.ok_or_else(|| {
                        Error::Server("No result in prompts/list response".into())
                    })?;

                    let prompts_value = result
                        .get("prompts")
                        .ok_or_else(|| Error::Server("No prompts field in response".into()))?;

                    let prompts: Vec<Prompt> = serde_json::from_value(prompts_value.clone())
                        .map_err(|e| {
                            Error::Serialization(format!("Failed to parse prompts: {}", e))
                        })?;

                    Ok(prompts)
                }) {
                Ok(prompts) => all_prompts.extend(prompts),
                Err(e) => warn!("Failed to fetch prompts: {}", e),
            }
        } else {
            // Direct backend call (existing path)
            match fetch_prompts_from_server(&state, server, request.clone()).await {
                Ok(prompts) => all_prompts.extend(prompts),
                Err(e) => warn!("Failed to fetch prompts: {}", e),
            }
        }
    }

    all_prompts.sort_by(|a, b| a.name.cmp(&b.name));
    all_prompts.dedup_by(|a, b| a.name == b.name);

    Ok(json!({
        "jsonrpc": "2.0",
        "id": request.id(),
        "result": {
            "prompts": all_prompts
        }
    }))
}

/// Handle prompts/get request.
pub async fn handle_prompts_get(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    let request: McpRequest = serde_json::from_value(payload)?;
    handle_prompts_get_impl(state, request).await.map(Json)
}

async fn handle_prompts_get_impl(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    let name = request
        .params()
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ProxyError::InvalidRequest("Missing prompt name".into()))?;

    debug!("Getting prompt: {}", name);

    // Route to appropriate server
    let router = RequestRouter::new(state.config.proxy.routing.clone());
    let (server_id, _) = router
        .route_request(&request, &*state.registry.read().await, &state.cache)
        .await?;

    let server = {
        let registry = state.registry.read().await;
        registry
            .get_server(&server_id)
            .ok_or_else(|| ProxyError::NoBackendAvailable(name.to_string()))?
            .clone()
    };

    send_request_to_backend(state, server, request).await
}

/// Handle sampling/createMessage request.
pub async fn handle_sampling_create(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> std::result::Result<Json<Value>, ProxyError> {
    let request: McpRequest = serde_json::from_value(payload)?;
    handle_sampling_create_impl(state, request).await.map(Json)
}

async fn handle_sampling_create_impl(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    // Route to a capable server
    let router = RequestRouter::new(state.config.proxy.routing.clone());
    let (server_id, _) = router
        .route_request(&request, &*state.registry.read().await, &state.cache)
        .await?;

    let server = {
        let registry = state.registry.read().await;
        registry
            .get_server(&server_id)
            .ok_or_else(|| ProxyError::NoBackendAvailable("sampling".to_string()))?
            .clone()
    };

    send_request_to_backend(state, server, request).await
}

/// Handle WebSocket upgrade for streaming.
pub async fn handle_websocket_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(_socket: axum::extract::ws::WebSocket, _state: AppState) {
    // TODO: Implement WebSocket handling for bidirectional streaming
    info!("WebSocket connection established");
}

/// Handle Server-Sent Events stream.
pub async fn handle_sse_stream(
    State(_state): State<AppState>,
) -> std::result::Result<Response, ProxyError> {
    // TODO: Implement SSE for server push
    Ok(Response::new("SSE endpoint".into()))
}

/// Route generic/unknown requests to appropriate backend.
async fn route_generic_request(
    state: AppState,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    let router = RequestRouter::new(state.config.proxy.routing.clone());
    let (server_id, _) = router
        .route_request(&request, &*state.registry.read().await, &state.cache)
        .await?;

    let server = {
        let registry = state.registry.read().await;
        registry
            .get_server(&server_id)
            .ok_or_else(|| ProxyError::NoBackendAvailable(request.method()))?
            .clone()
    };

    send_request_to_backend(state, server, request).await
}

// Helper functions

async fn fetch_tools_from_server(
    state: AppState,
    server_id: String,
    request: McpRequest,
) -> Result<Vec<Tool>> {
    // Get server config from the config (not registry, as registry only has ServerInfo)
    let server_config = state
        .config
        .servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| Error::ServerNotFound(server_id.clone()))?;

    // Create tools/list JSON-RPC request
    let tools_request = McpRequest::new("tools/list", serde_json::json!({}), request.id());

    // Send via appropriate transport
    let response = match &server_config.transport {
        crate::config::TransportConfig::Http { url, .. } => {
            let http_transport = state
                .http_transport
                .as_ref()
                .ok_or_else(|| Error::Transport("HTTP transport not initialized".into()))?;

            http_transport
                .send_request(url, tools_request)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?
        },
        crate::config::TransportConfig::Stdio { command, args, env } => {
            let stdio_transport = state
                .stdio_transport
                .as_ref()
                .ok_or_else(|| Error::Transport("STDIO transport not initialized".into()))?;

            // Create STDIO config
            let stdio_config = crate::transport::stdio::StdioConfig {
                command: command.clone(),
                args: args.clone(),
                env: env.clone(),
                cwd: None,
                timeout_ms: 30000,
                max_memory_mb: Some(512),
                max_cpu_percent: Some(50),
                sandbox: true,
            };

            stdio_transport
                .send_request_with_config(server_id.clone(), &stdio_config, tools_request)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?
        },
        crate::config::TransportConfig::Sse { .. } => {
            return Err(Error::Transport("SSE not yet implemented".into()));
        },
    };

    // Parse response and extract tools array
    let result = response
        .result()
        .ok_or_else(|| Error::Server("No result in tools/list response".into()))?;

    let tools_value = result
        .get("tools")
        .ok_or_else(|| Error::Server("No tools field in response".into()))?;

    let tools: Vec<Tool> = serde_json::from_value(tools_value.clone())
        .map_err(|e| Error::Serialization(format!("Failed to parse tools: {}", e)))?;

    Ok(tools)
}

async fn fetch_resources_from_server(
    state: &AppState,
    server_id: String,
    request: McpRequest,
) -> Result<Vec<Resource>> {
    // Get server config
    let server_config = state
        .config
        .servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| Error::ServerNotFound(server_id.clone()))?;

    // Create resources/list JSON-RPC request
    let resources_request = McpRequest::new("resources/list", serde_json::json!({}), request.id());

    // Send via appropriate transport
    let response = match &server_config.transport {
        crate::config::TransportConfig::Http { url, .. } => {
            let http_transport = state
                .http_transport
                .as_ref()
                .ok_or_else(|| Error::Transport("HTTP transport not initialized".into()))?;

            http_transport
                .send_request(url, resources_request)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?
        },
        crate::config::TransportConfig::Stdio { command, args, env } => {
            let stdio_transport = state
                .stdio_transport
                .as_ref()
                .ok_or_else(|| Error::Transport("STDIO transport not initialized".into()))?;

            let stdio_config = crate::transport::stdio::StdioConfig {
                command: command.clone(),
                args: args.clone(),
                env: env.clone(),
                cwd: None,
                timeout_ms: 30000,
                max_memory_mb: Some(512),
                max_cpu_percent: Some(50),
                sandbox: true,
            };

            stdio_transport
                .send_request_with_config(server_id.clone(), &stdio_config, resources_request)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?
        },
        crate::config::TransportConfig::Sse { .. } => {
            return Err(Error::Transport("SSE not yet implemented".into()));
        },
    };

    // Parse response and extract resources array
    let result = response
        .result()
        .ok_or_else(|| Error::Server("No result in resources/list response".into()))?;

    let resources_value = result
        .get("resources")
        .ok_or_else(|| Error::Server("No resources field in response".into()))?;

    let resources: Vec<Resource> = serde_json::from_value(resources_value.clone())
        .map_err(|e| Error::Serialization(format!("Failed to parse resources: {}", e)))?;

    Ok(resources)
}

async fn fetch_prompts_from_server(
    state: &AppState,
    server_id: String,
    request: McpRequest,
) -> Result<Vec<Prompt>> {
    // Get server config
    let server_config = state
        .config
        .servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or_else(|| Error::ServerNotFound(server_id.clone()))?;

    // Create prompts/list JSON-RPC request
    let prompts_request = McpRequest::new("prompts/list", serde_json::json!({}), request.id());

    // Send via appropriate transport
    let response = match &server_config.transport {
        crate::config::TransportConfig::Http { url, .. } => {
            let http_transport = state
                .http_transport
                .as_ref()
                .ok_or_else(|| Error::Transport("HTTP transport not initialized".into()))?;

            http_transport
                .send_request(url, prompts_request)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?
        },
        crate::config::TransportConfig::Stdio { command, args, env } => {
            let stdio_transport = state
                .stdio_transport
                .as_ref()
                .ok_or_else(|| Error::Transport("STDIO transport not initialized".into()))?;

            let stdio_config = crate::transport::stdio::StdioConfig {
                command: command.clone(),
                args: args.clone(),
                env: env.clone(),
                cwd: None,
                timeout_ms: 30000,
                max_memory_mb: Some(512),
                max_cpu_percent: Some(50),
                sandbox: true,
            };

            stdio_transport
                .send_request_with_config(server_id.clone(), &stdio_config, prompts_request)
                .await
                .map_err(|e| Error::Transport(e.to_string()))?
        },
        crate::config::TransportConfig::Sse { .. } => {
            return Err(Error::Transport("SSE not yet implemented".into()));
        },
    };

    // Parse response and extract prompts array
    let result = response
        .result()
        .ok_or_else(|| Error::Server("No result in prompts/list response".into()))?;

    let prompts_value = result
        .get("prompts")
        .ok_or_else(|| Error::Server("No prompts field in response".into()))?;

    let prompts: Vec<Prompt> = serde_json::from_value(prompts_value.clone())
        .map_err(|e| Error::Serialization(format!("Failed to parse prompts: {}", e)))?;

    Ok(prompts)
}

async fn send_request_to_backend(
    state: AppState,
    server: crate::proxy::registry::ServerConfig,
    request: McpRequest,
) -> std::result::Result<Value, ProxyError> {
    use crate::proxy::registry::TransportType;

    let start = Instant::now();

    // Route based on transport type
    let response = match server.transport {
        TransportType::Http => {
            let http_transport = state
                .http_transport
                .as_ref()
                .ok_or_else(|| ProxyError::Transport("HTTP transport not available".into()))?;
            http_transport
                .send_request(&server.endpoint, request)
                .await
                .map_err(|e| ProxyError::Transport(e.to_string()))?
        },
        TransportType::Stdio => {
            let stdio_transport = state
                .stdio_transport
                .as_ref()
                .ok_or_else(|| ProxyError::Transport("STDIO transport not available".into()))?;
            stdio_transport
                .send_request(&server.id, request)
                .await
                .map_err(|e| ProxyError::Transport(e.to_string()))?
        },
        TransportType::WebSocket => {
            return Err(ProxyError::Transport(
                "WebSocket not yet implemented".into(),
            ));
        },
        TransportType::Sse => {
            return Err(ProxyError::Transport("SSE not yet implemented".into()));
        },
    };

    // Record metrics
    let duration = start.elapsed();
    info!(
        "Backend request to {} completed in {:?}",
        server.id, duration
    );

    // Convert response to JSON Value
    Ok(serde_json::to_value(response)?)
}

async fn execute_with_retry<F, Fut>(
    f: F,
    max_retries: u32,
) -> std::result::Result<Value, ProxyError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<Value, ProxyError>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if e.is_retryable() && attempts < max_retries => {
                attempts += 1;
                warn!("Retry attempt {} after error: {}", attempts, e);
                tokio::time::sleep(Duration::from_millis(100 * attempts as u64)).await;
            },
            Err(e) => return Err(e),
        }
    }
}

// Type definitions now imported from crate::types
// ServerConfig and TransportType are now imported from crate::proxy::registry
