//! Plugin system for Only1MCP
//!
//! Provides a flexible plugin architecture for extending proxy functionality:
//! - Request/response transformers
//! - Custom protocol handlers
//! - Metrics collectors
//! - Cache policies
//! - Load balancing strategies

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::Result;
use crate::types::{McpRequest, McpResponse};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,

    /// Plugin version (semver)
    pub version: String,

    /// Plugin author
    pub author: String,

    /// Plugin description
    pub description: String,

    /// Plugin capabilities
    pub capabilities: Vec<PluginCapability>,

    /// Dependencies on other plugins
    pub dependencies: Vec<String>,
}

/// Plugin capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Transform requests before routing
    RequestTransform,

    /// Transform responses before returning
    ResponseTransform,

    /// Custom protocol handler
    ProtocolHandler,

    /// Metrics collection
    MetricsCollector,

    /// Custom cache policy
    CachePolicy,

    /// Custom load balancing strategy
    LoadBalancer,

    /// Authentication provider
    AuthProvider,

    /// Custom middleware
    Middleware,
}

/// Plugin lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// Plugin loaded but not initialized
    Loaded,

    /// Plugin initialized and ready
    Ready,

    /// Plugin running
    Running,

    /// Plugin paused
    Paused,

    /// Plugin stopped
    Stopped,

    /// Plugin failed
    Failed,
}

impl fmt::Display for PluginState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginState::Loaded => write!(f, "loaded"),
            PluginState::Ready => write!(f, "ready"),
            PluginState::Running => write!(f, "running"),
            PluginState::Paused => write!(f, "paused"),
            PluginState::Stopped => write!(f, "stopped"),
            PluginState::Failed => write!(f, "failed"),
        }
    }
}

/// Plugin context passed to plugins
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Plugin configuration
    pub config: HashMap<String, serde_json::Value>,

    /// Server ID (if applicable)
    pub server_id: Option<String>,

    /// Request ID
    pub request_id: String,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl PluginContext {
    pub fn new(request_id: String) -> Self {
        Self {
            config: HashMap::new(),
            server_id: None,
            request_id,
            metadata: HashMap::new(),
        }
    }

    pub fn with_config(mut self, config: HashMap<String, serde_json::Value>) -> Self {
        self.config = config;
        self
    }

    pub fn with_server(mut self, server_id: String) -> Self {
        self.server_id = Some(server_id);
        self
    }
}

/// Main plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize plugin
    async fn initialize(&mut self, config: HashMap<String, serde_json::Value>) -> Result<()>;

    /// Start plugin
    async fn start(&mut self) -> Result<()>;

    /// Stop plugin
    async fn stop(&mut self) -> Result<()>;

    /// Get plugin state
    fn state(&self) -> PluginState;

    /// Health check
    async fn health_check(&self) -> Result<PluginHealth>;
}

/// Plugin health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginHealth {
    pub healthy: bool,
    pub message: Option<String>,
    pub metrics: HashMap<String, f64>,
}

impl PluginHealth {
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            message: None,
            metrics: HashMap::new(),
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            healthy: false,
            message: Some(message.into()),
            metrics: HashMap::new(),
        }
    }
}

/// Request transformer plugin trait
#[async_trait]
pub trait RequestTransformer: Plugin {
    /// Transform request before routing
    async fn transform_request(
        &self,
        request: McpRequest,
        context: &PluginContext,
    ) -> Result<McpRequest>;
}

/// Response transformer plugin trait
#[async_trait]
pub trait ResponseTransformer: Plugin {
    /// Transform response before returning
    async fn transform_response(
        &self,
        response: McpResponse,
        context: &PluginContext,
    ) -> Result<McpResponse>;
}

/// Plugin registry for managing loaded plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    request_transformers: Arc<RwLock<Vec<String>>>,
    response_transformers: Arc<RwLock<Vec<String>>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            request_transformers: Arc::new(RwLock::new(Vec::new())),
            response_transformers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a plugin
    pub async fn register(&self, plugin: Box<dyn Plugin>) -> Result<()> {
        let name = plugin.metadata().name.clone();
        let capabilities = plugin.metadata().capabilities.clone();

        let mut plugins = self.plugins.write().await;
        plugins.insert(name.clone(), plugin);

        // Register in capability lists
        if capabilities.contains(&PluginCapability::RequestTransform) {
            let mut transformers = self.request_transformers.write().await;
            transformers.push(name.clone());
        }

        if capabilities.contains(&PluginCapability::ResponseTransform) {
            let mut transformers = self.response_transformers.write().await;
            transformers.push(name);
        }

        Ok(())
    }

    /// Get plugin by name
    pub async fn get(&self, name: &str) -> Option<()> {
        let plugins = self.plugins.read().await;
        plugins.get(name).map(|_| ())
    }

    /// List all plugins
    pub async fn list(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.metadata().clone()).collect()
    }

    /// Start all plugins
    pub async fn start_all(&self) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        for plugin in plugins.values_mut() {
            plugin.start().await?;
        }
        Ok(())
    }

    /// Stop all plugins
    pub async fn stop_all(&self) -> Result<()> {
        let mut plugins = self.plugins.write().await;
        for plugin in plugins.values_mut() {
            plugin.stop().await?;
        }
        Ok(())
    }

    /// Get health status of all plugins
    pub async fn health_status(&self) -> HashMap<String, PluginHealth> {
        let plugins = self.plugins.read().await;
        let mut statuses = HashMap::new();

        for (name, plugin) in plugins.iter() {
            if let Ok(health) = plugin.health_check().await {
                statuses.insert(name.clone(), health);
            }
        }

        statuses
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin errors
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    NotFound(String),

    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin already registered: {0}")]
    AlreadyRegistered(String),

    #[error("Dependency not satisfied: {0}")]
    DependencyNotSatisfied(String),

    #[error("Invalid plugin state: expected {expected}, got {actual}")]
    InvalidState {
        expected: PluginState,
        actual: PluginState,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin {
        metadata: PluginMetadata,
        state: PluginState,
    }

    impl TestPlugin {
        fn new() -> Self {
            Self {
                metadata: PluginMetadata {
                    name: "test-plugin".to_string(),
                    version: "1.0.0".to_string(),
                    author: "Test Author".to_string(),
                    description: "A test plugin".to_string(),
                    capabilities: vec![PluginCapability::RequestTransform],
                    dependencies: vec![],
                },
                state: PluginState::Loaded,
            }
        }
    }

    #[async_trait]
    impl Plugin for TestPlugin {
        fn metadata(&self) -> &PluginMetadata {
            &self.metadata
        }

        async fn initialize(&mut self, _config: HashMap<String, serde_json::Value>) -> Result<()> {
            self.state = PluginState::Ready;
            Ok(())
        }

        async fn start(&mut self) -> Result<()> {
            self.state = PluginState::Running;
            Ok(())
        }

        async fn stop(&mut self) -> Result<()> {
            self.state = PluginState::Stopped;
            Ok(())
        }

        fn state(&self) -> PluginState {
            self.state
        }

        async fn health_check(&self) -> Result<PluginHealth> {
            Ok(PluginHealth::healthy())
        }
    }

    #[tokio::test]
    async fn test_plugin_registry() {
        let registry = PluginRegistry::new();
        let plugin = Box::new(TestPlugin::new());

        registry.register(plugin).await.unwrap();

        let plugins = registry.list().await;
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "test-plugin");
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() {
        let mut plugin = TestPlugin::new();

        assert_eq!(plugin.state(), PluginState::Loaded);

        plugin.initialize(HashMap::new()).await.unwrap();
        assert_eq!(plugin.state(), PluginState::Ready);

        plugin.start().await.unwrap();
        assert_eq!(plugin.state(), PluginState::Running);

        plugin.stop().await.unwrap();
        assert_eq!(plugin.state(), PluginState::Stopped);
    }
}
