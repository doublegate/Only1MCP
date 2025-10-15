//! Plugin system architecture combining native Rust plugins (high performance)
//! with WASM modules (security/portability). Plugins extend Only1MCP functionality
//! without modifying core code, enabling custom transforms, protocol adapters,
//! authentication providers, and monitoring integrations.

use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use thiserror::Error;

/// Plugin error types
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Unsupported plugin format")]
    UnsupportedFormat,

    #[error("Incompatible plugin version: required {required}, got {found}")]
    IncompatibleVersion { required: String, found: String },

    #[error("Missing capability: {0}")]
    MissingCapability(String),

    #[error("Plugin initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Plugin execution error: {0}")]
    ExecutionError(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Plugin communication error: {0}")]
    Communication(String),
}

/// Plugin format
#[derive(Debug, Clone, PartialEq)]
pub enum PluginFormat {
    Native,
    Wasm,
}

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// Unique plugin identifier
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin author
    pub author: String,

    /// Plugin description
    pub description: String,

    /// Required Only1MCP version
    pub min_version: String,

    /// Plugin type
    pub plugin_type: PluginType,

    /// Required capabilities
    pub capabilities: Vec<Capability>,
}

/// Plugin type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginType {
    RequestTransformer,
    ResponseTransformer,
    Authentication,
    Authorization,
    RateLimit,
    Monitoring,
    Caching,
    Custom,
}

/// Plugin capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Capability {
    NetworkAccess,
    FileSystemRead,
    FileSystemWrite,
    ProcessSpawn,
    EnvironmentVariables,
    SystemInfo,
    MetricsAccess,
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin directory paths
    pub plugin_dirs: Vec<PathBuf>,

    /// Enable hot-reload
    pub hot_reload: bool,

    /// Maximum plugin execution time
    pub max_execution_time: std::time::Duration,

    /// Memory limit for WASM plugins (in bytes)
    pub wasm_memory_limit: usize,

    /// Enable plugin marketplace
    pub marketplace_enabled: bool,

    /// Registry URL
    pub registry_url: Option<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugin_dirs: vec![PathBuf::from("./plugins")],
            hot_reload: false,
            max_execution_time: std::time::Duration::from_secs(30),
            wasm_memory_limit: 100 * 1024 * 1024, // 100MB
            marketplace_enabled: false,
            registry_url: None,
        }
    }
}

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn initialize(&mut self, config: serde_json::Value) -> Result<(), PluginError>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<(), PluginError>;

    /// Execute plugin logic
    async fn execute(&self, context: PluginContext) -> Result<PluginResponse, PluginError>;

    /// Health check
    async fn health_check(&self) -> Result<(), PluginError> {
        Ok(())
    }
}

/// Plugin execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginContext {
    /// Request ID
    pub request_id: String,

    /// Request method
    pub method: String,

    /// Request headers
    pub headers: HashMap<String, String>,

    /// Request body
    pub body: Option<serde_json::Value>,

    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Plugin response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// Modified headers
    pub headers: Option<HashMap<String, String>>,

    /// Modified body
    pub body: Option<serde_json::Value>,

    /// Action to take
    pub action: PluginAction,

    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Plugin action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PluginAction {
    Continue,
    Modify,
    Reject(u16, String), // Status code and message
    Redirect(String),     // URL
}

/// Plugin lifecycle manager handles loading, initialization, and unloading
pub struct PluginManager {
    /// Registry of loaded plugins
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,

    /// Configuration
    config: PluginConfig,

    /// Metrics collector
    metrics: Arc<PluginMetrics>,

    /// Plugin paths
    plugin_paths: Arc<RwLock<HashMap<String, PathBuf>>>,
}

impl PluginManager {
    /// Create new plugin manager
    pub fn new(config: PluginConfig) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(PluginMetrics::new()),
            plugin_paths: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Load a plugin from file path
    pub async fn load_plugin(&self, path: &Path) -> Result<String, PluginError> {
        // 1. Determine plugin type by extension
        let plugin_format = match path.extension().and_then(|s| s.to_str()) {
            Some("so") | Some("dll") | Some("dylib") => PluginFormat::Native,
            Some("wasm") => PluginFormat::Wasm,
            _ => return Err(PluginError::UnsupportedFormat),
        };

        // 2. Load based on type
        let plugin = match plugin_format {
            PluginFormat::Native => {
                // Native plugin loading would go here
                // For now, return error as implementation requires unsafe code
                return Err(PluginError::InitializationFailed(
                    "Native plugin loading not yet implemented".to_string()
                ));
            }
            PluginFormat::Wasm => {
                // WASM plugin loading would go here
                return Err(PluginError::InitializationFailed(
                    "WASM plugin loading not yet implemented".to_string()
                ));
            }
        };

        // The rest would be implemented when actual plugin loading is added
        /*
        // 3. Validate metadata
        let metadata = plugin.metadata();
        self.validate_compatibility(&metadata)?;

        // 4. Check capabilities
        self.validate_capabilities(&metadata.capabilities)?;

        // 5. Initialize plugin
        let config = self.config.for_plugin(&metadata.id);
        plugin.initialize(config).await?;

        // 6. Register in manager
        let plugin_id = metadata.id.clone();
        self.plugins.write().await.insert(plugin_id.clone(), plugin);
        self.plugin_paths.write().await.insert(plugin_id.clone(), path.to_path_buf());

        // 7. Update metrics
        self.metrics.plugin_loaded(&plugin_id);

        Ok(plugin_id)
        */
    }

    /// Unload a plugin safely
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        // 1. Get plugin
        let mut plugins = self.plugins.write().await;
        let mut plugin = plugins.remove(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        // 2. Graceful shutdown
        plugin.shutdown().await?;

        // 3. Remove path
        self.plugin_paths.write().await.remove(plugin_id);

        // 4. Update metrics
        self.metrics.plugin_unloaded(plugin_id);

        Ok(())
    }

    /// Hot-reload a plugin (unload + load)
    pub async fn reload_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        // Store path before unloading
        let path = self.get_plugin_path(plugin_id).await?;

        // Unload existing
        self.unload_plugin(plugin_id).await?;

        // Load new version
        self.load_plugin(&path).await?;

        Ok(())
    }

    /// Get plugin path
    async fn get_plugin_path(&self, plugin_id: &str) -> Result<PathBuf, PluginError> {
        self.plugin_paths
            .read()
            .await
            .get(plugin_id)
            .cloned()
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))
    }

    /// Execute plugin
    pub async fn execute_plugin(
        &self,
        plugin_id: &str,
        context: PluginContext,
    ) -> Result<PluginResponse, PluginError> {
        let plugins = self.plugins.read().await;
        let plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;

        // Record execution start
        let start = std::time::Instant::now();

        // Execute with timeout
        let result = tokio::time::timeout(
            self.config.max_execution_time,
            plugin.execute(context)
        ).await
            .map_err(|_| PluginError::ExecutionError("Plugin execution timed out".to_string()))?;

        // Record metrics
        self.metrics.record_execution(plugin_id, start.elapsed());

        result
    }

    /// List all loaded plugins
    pub async fn list_plugins(&self) -> Vec<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.values()
            .map(|p| p.metadata().clone())
            .collect()
    }

    /// Get plugin by ID
    pub async fn get_plugin(&self, plugin_id: &str) -> Option<PluginMetadata> {
        let plugins = self.plugins.read().await;
        plugins.get(plugin_id)
            .map(|p| p.metadata().clone())
    }
}

/// Plugin metrics collector
pub struct PluginMetrics {
    /// Plugin load counter
    loads: Arc<std::sync::atomic::AtomicU64>,

    /// Plugin unload counter
    unloads: Arc<std::sync::atomic::AtomicU64>,

    /// Execution counter
    executions: Arc<std::sync::atomic::AtomicU64>,

    /// Total execution time
    total_execution_time: Arc<std::sync::Mutex<std::time::Duration>>,
}

impl PluginMetrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            loads: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            unloads: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            executions: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_execution_time: Arc::new(std::sync::Mutex::new(std::time::Duration::ZERO)),
        }
    }

    /// Record plugin loaded
    pub fn plugin_loaded(&self, _plugin_id: &str) {
        self.loads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record plugin unloaded
    pub fn plugin_unloaded(&self, _plugin_id: &str) {
        self.unloads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record execution
    pub fn record_execution(&self, _plugin_id: &str, duration: std::time::Duration) {
        self.executions.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if let Ok(mut total) = self.total_execution_time.lock() {
            *total += duration;
        }
    }

    /// Get metrics summary
    pub fn get_summary(&self) -> PluginMetricsSummary {
        PluginMetricsSummary {
            total_loads: self.loads.load(std::sync::atomic::Ordering::Relaxed),
            total_unloads: self.unloads.load(std::sync::atomic::Ordering::Relaxed),
            total_executions: self.executions.load(std::sync::atomic::Ordering::Relaxed),
            total_execution_time: self.total_execution_time.lock()
                .unwrap_or_else(|_| std::time::Duration::ZERO.into())
                .clone(),
        }
    }
}

/// Plugin metrics summary
#[derive(Debug, Clone, Serialize)]
pub struct PluginMetricsSummary {
    pub total_loads: u64,
    pub total_unloads: u64,
    pub total_executions: u64,
    pub total_execution_time: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_creation() {
        let config = PluginConfig::default();
        let manager = PluginManager::new(config);

        let plugins = manager.list_plugins().await;
        assert!(plugins.is_empty());
    }

    #[tokio::test]
    async fn test_plugin_metrics() {
        let metrics = PluginMetrics::new();
        metrics.plugin_loaded("test");
        metrics.plugin_unloaded("test");
        metrics.record_execution("test", std::time::Duration::from_millis(100));

        let summary = metrics.get_summary();
        assert_eq!(summary.total_loads, 1);
        assert_eq!(summary.total_unloads, 1);
        assert_eq!(summary.total_executions, 1);
    }
}