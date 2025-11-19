//! Protocol Adapter Plugin Example
//!
//! Demonstrates how to create a protocol adapter that converts between
//! different protocols (REST, GraphQL, gRPC) and MCP

use async_trait::async_trait;
use only1mcp::plugins::*;
use only1mcp::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Protocol types supported
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProtocolType {
    Rest,
    GraphQL,
    #[serde(rename = "grpc")]
    GRPC,
    Mcp,
}

/// REST request mapping
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RestMapping {
    pub method: String,      // HTTP method (GET, POST, etc.)
    pub path: String,        // URL path template
    pub mcp_method: String,  // Mapped MCP method
    pub param_mapping: HashMap<String, String>, // Path/query param to MCP param mapping
}

/// GraphQL query mapping
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GraphQLMapping {
    pub query_name: String,
    pub mcp_method: String,
    pub field_mapping: HashMap<String, String>,
}

/// Protocol adapter configuration
#[derive(Clone, Debug, Deserialize)]
pub struct AdapterConfig {
    pub source_protocol: ProtocolType,
    pub target_protocol: ProtocolType,
    pub rest_mappings: Option<Vec<RestMapping>>,
    pub graphql_mappings: Option<Vec<GraphQLMapping>>,
}

/// Protocol adapter plugin
pub struct ProtocolAdapterPlugin {
    metadata: PluginMetadata,
    state: PluginState,
    config: Arc<RwLock<Option<AdapterConfig>>>,
}

impl ProtocolAdapterPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                name: "protocol-adapter".to_string(),
                version: "1.0.0".to_string(),
                author: "Only1MCP Team".to_string(),
                description: "Multi-protocol adapter for converting between REST/GraphQL/gRPC and MCP".to_string(),
                capabilities: vec![
                    PluginCapability::RequestTransform,
                    PluginCapability::ResponseTransform,
                ],
                dependencies: vec![],
            },
            state: PluginState::Loaded,
            config: Arc::new(RwLock::new(None)),
        }
    }

    /// Convert REST request to MCP
    async fn rest_to_mcp(&self, rest_request: RestRequest) -> Result<McpRequest, String> {
        let config = self.config.read().await;
        let cfg = config
            .as_ref()
            .ok_or_else(|| "Adapter not configured".to_string())?;

        let mappings = cfg
            .rest_mappings
            .as_ref()
            .ok_or_else(|| "No REST mappings configured".to_string())?;

        // Find matching mapping
        let mapping = mappings
            .iter()
            .find(|m| {
                m.method == rest_request.method
                    && Self::path_matches(&m.path, &rest_request.path)
            })
            .ok_or_else(|| format!("No mapping found for {} {}", rest_request.method, rest_request.path))?;

        // Extract path parameters
        let path_params = Self::extract_path_params(&mapping.path, &rest_request.path);

        // Build MCP params
        let mut mcp_params = serde_json::Map::new();

        // Map path parameters
        for (rest_param, mcp_param) in &mapping.param_mapping {
            if let Some(value) = path_params.get(rest_param) {
                mcp_params.insert(mcp_param.clone(), serde_json::Value::String(value.clone()));
            }
        }

        // Map query parameters
        for (key, value) in rest_request.query_params {
            if let Some(mcp_param) = mapping.param_mapping.get(&key) {
                mcp_params.insert(mcp_param.clone(), serde_json::Value::String(value));
            }
        }

        // Map body if present
        if let Some(body) = rest_request.body {
            if let Ok(body_obj) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&body) {
                for (key, value) in body_obj {
                    if let Some(mcp_param) = mapping.param_mapping.get(&key) {
                        mcp_params.insert(mcp_param.clone(), value);
                    } else {
                        mcp_params.insert(key, value);
                    }
                }
            }
        }

        Ok(McpRequest {
            method: mapping.mcp_method.clone(),
            params: serde_json::Value::Object(mcp_params),
            headers: rest_request.headers,
        })
    }

    /// Convert GraphQL query to MCP
    async fn graphql_to_mcp(&self, graphql_query: GraphQLQuery) -> Result<McpRequest, String> {
        let config = self.config.read().await;
        let cfg = config
            .as_ref()
            .ok_or_else(|| "Adapter not configured".to_string())?;

        let mappings = cfg
            .graphql_mappings
            .as_ref()
            .ok_or_else(|| "No GraphQL mappings configured".to_string())?;

        // Parse query to find operation name
        let query_name = Self::extract_graphql_operation(&graphql_query.query)?;

        // Find matching mapping
        let mapping = mappings
            .iter()
            .find(|m| m.query_name == query_name)
            .ok_or_else(|| format!("No mapping found for query: {}", query_name))?;

        // Build MCP params from GraphQL variables
        let mut mcp_params = serde_json::Map::new();

        if let Some(variables) = graphql_query.variables {
            if let serde_json::Value::Object(var_map) = variables {
                for (key, value) in var_map {
                    if let Some(mcp_field) = mapping.field_mapping.get(&key) {
                        mcp_params.insert(mcp_field.clone(), value);
                    } else {
                        mcp_params.insert(key, value);
                    }
                }
            }
        }

        Ok(McpRequest {
            method: mapping.mcp_method.clone(),
            params: serde_json::Value::Object(mcp_params),
            headers: HashMap::new(),
        })
    }

    /// Convert MCP response to REST
    fn mcp_to_rest(&self, mcp_response: McpResponse) -> RestResponse {
        if let Some(error) = mcp_response.error {
            return RestResponse {
                status_code: 500,
                headers: HashMap::new(),
                body: serde_json::json!({
                    "error": error.message,
                    "code": error.code
                })
                .to_string(),
            };
        }

        RestResponse {
            status_code: 200,
            headers: {
                let mut headers = HashMap::new();
                headers.insert("Content-Type".to_string(), "application/json".to_string());
                headers
            },
            body: mcp_response.result.to_string(),
        }
    }

    /// Convert MCP response to GraphQL
    fn mcp_to_graphql(&self, mcp_response: McpResponse) -> GraphQLResponse {
        if let Some(error) = mcp_response.error {
            return GraphQLResponse {
                data: None,
                errors: Some(vec![GraphQLError {
                    message: error.message,
                    locations: None,
                    path: None,
                }]),
            };
        }

        GraphQLResponse {
            data: Some(mcp_response.result),
            errors: None,
        }
    }

    /// Check if path matches pattern
    fn path_matches(pattern: &str, path: &str) -> bool {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return false;
        }

        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if pattern_part.starts_with(':') {
                // This is a parameter, matches any value
                continue;
            }
            if pattern_part != path_part {
                return false;
            }
        }

        true
    }

    /// Extract path parameters
    fn extract_path_params(pattern: &str, path: &str) -> HashMap<String, String> {
        let pattern_parts: Vec<&str> = pattern.split('/').collect();
        let path_parts: Vec<&str> = path.split('/').collect();

        let mut params = HashMap::new();

        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if let Some(param_name) = pattern_part.strip_prefix(':') {
                params.insert(param_name.to_string(), path_part.to_string());
            }
        }

        params
    }

    /// Extract GraphQL operation name from query
    fn extract_graphql_operation(query: &str) -> Result<String, String> {
        // Simplified parser - in production use a proper GraphQL parser
        let query = query.trim();

        // Look for query/mutation name
        if let Some(start) = query.find('{') {
            let before_brace = &query[..start].trim();
            let words: Vec<&str> = before_brace.split_whitespace().collect();

            // Format: "query QueryName" or just "QueryName"
            if words.len() >= 2 {
                return Ok(words[1].to_string());
            } else if !words.is_empty() && (words[0] == "query" || words[0] == "mutation") {
                // Anonymous query
                return Ok("anonymous".to_string());
            }
        }

        Err("Could not extract operation name from query".to_string())
    }
}

impl Default for ProtocolAdapterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

// Supporting types
#[derive(Clone, Debug)]
pub struct RestRequest {
    pub method: String,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RestResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct GraphQLQuery {
    pub query: String,
    pub variables: Option<serde_json::Value>,
    pub operation_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphQLResponse {
    pub data: Option<serde_json::Value>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphQLError {
    pub message: String,
    pub locations: Option<Vec<GraphQLLocation>>,
    pub path: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize)]
pub struct GraphQLLocation {
    pub line: u32,
    pub column: u32,
}

#[async_trait]
impl Plugin for ProtocolAdapterPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(
        &mut self,
        config: HashMap<String, serde_json::Value>,
    ) -> Result<(), String> {
        let adapter_config: AdapterConfig = serde_json::from_value(
            config
                .get("adapter")
                .ok_or_else(|| "Missing 'adapter' configuration".to_string())?
                .clone(),
        )
        .map_err(|e| format!("Invalid adapter config: {}", e))?;

        let mut cfg = self.config.write().await;
        *cfg = Some(adapter_config);

        self.state = PluginState::Ready;
        Ok(())
    }

    async fn start(&mut self) -> Result<(), String> {
        self.state = PluginState::Running;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        self.state = PluginState::Stopped;
        Ok(())
    }

    fn state(&self) -> PluginState {
        self.state
    }

    async fn health_check(&self) -> Result<PluginHealth, String> {
        let config = self.config.read().await;
        Ok(PluginHealth {
            healthy: self.state == PluginState::Running && config.is_some(),
            message: if config.is_some() {
                "Adapter configured and running".to_string()
            } else {
                "Adapter not configured".to_string()
            },
        })
    }
}

#[async_trait]
impl RequestTransformer for ProtocolAdapterPlugin {
    async fn transform_request(
        &self,
        request: McpRequest,
        _context: &PluginContext,
    ) -> Result<McpRequest, String> {
        // Could implement protocol conversion here based on context
        Ok(request)
    }
}

// Example usage
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_matching() {
        assert!(ProtocolAdapterPlugin::path_matches(
            "/api/users/:id",
            "/api/users/123"
        ));
        assert!(!ProtocolAdapterPlugin::path_matches(
            "/api/users/:id",
            "/api/posts/123"
        ));
        assert!(!ProtocolAdapterPlugin::path_matches(
            "/api/users/:id",
            "/api/users/123/posts"
        ));
    }

    #[test]
    fn test_extract_path_params() {
        let params = ProtocolAdapterPlugin::extract_path_params(
            "/api/users/:id/posts/:postId",
            "/api/users/123/posts/456",
        );

        assert_eq!(params.get("id"), Some(&"123".to_string()));
        assert_eq!(params.get("postId"), Some(&"456".to_string()));
    }

    #[test]
    fn test_graphql_operation_extraction() {
        assert_eq!(
            ProtocolAdapterPlugin::extract_graphql_operation("query GetUser { user(id: 1) { name } }").unwrap(),
            "GetUser"
        );

        assert_eq!(
            ProtocolAdapterPlugin::extract_graphql_operation("mutation CreateUser { ... }").unwrap(),
            "CreateUser"
        );
    }

    #[tokio::test]
    async fn test_adapter_initialization() {
        let mut plugin = ProtocolAdapterPlugin::new();

        let mut config = HashMap::new();
        config.insert(
            "adapter".to_string(),
            serde_json::json!({
                "source_protocol": "rest",
                "target_protocol": "mcp",
                "rest_mappings": [{
                    "method": "GET",
                    "path": "/api/users/:id",
                    "mcp_method": "users.get",
                    "param_mapping": {
                        "id": "user_id"
                    }
                }]
            }),
        );

        plugin.initialize(config).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Ready);
    }
}
