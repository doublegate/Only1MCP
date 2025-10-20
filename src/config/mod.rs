//! Configuration module for Only1MCP
//!
//! Handles loading, validation, and hot-reloading of configuration files.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod loader;
pub mod schema;
pub mod validation;

pub use loader::ConfigLoader;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    #[serde(default)]
    pub server: ServerConfig,
    pub servers: Vec<McpServerConfig>,
    #[serde(default)]
    pub proxy: ProxyConfig,
    #[serde(default)]
    pub context_optimization: ContextOptimizationConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub observability: ObservabilityConfig,
    #[serde(default)]
    pub tui: TuiConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub worker_threads: usize,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default)]
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct TlsConfig {
    #[serde(default)]
    pub enabled: bool,
    pub cert_path: Option<PathBuf>,
    pub key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpServerConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub transport: TransportConfig,
    #[serde(default)]
    pub health_check: HealthCheckConfig,
    #[serde(default)]
    pub routing: RoutingConfig,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum TransportConfig {
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
    },
    Sse {
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
    },
    #[serde(rename = "streamable_http")]
    StreamableHttp {
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
        #[serde(default = "default_timeout_ms")]
        timeout_ms: u64,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HealthCheckConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_interval_seconds")]
    pub interval_seconds: u64,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,
    #[serde(default = "default_health_path")]
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct RoutingConfig {
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default = "default_priority")]
    pub priority: u32,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoutingAlgorithmConfig {
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    #[serde(default = "default_virtual_nodes")]
    pub virtual_nodes: usize,
    #[serde(default)]
    pub sticky_sessions: bool,
}

impl Default for RoutingAlgorithmConfig {
    fn default() -> Self {
        Self {
            algorithm: default_algorithm(),
            virtual_nodes: default_virtual_nodes(),
            sticky_sessions: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ProxyConfig {
    #[serde(default)]
    pub load_balancer: LoadBalancerConfig,
    #[serde(default)]
    pub connection_pool: ConnectionPoolConfig,
    #[serde(default)]
    pub routing: RoutingAlgorithmConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoadBalancerConfig {
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
    #[serde(default = "default_virtual_nodes")]
    pub virtual_nodes: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectionPoolConfig {
    #[serde(default = "default_max_per_backend")]
    pub max_per_backend: usize,
    #[serde(default = "default_min_idle")]
    pub min_idle: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ContextOptimizationConfig {
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub batching: BatchingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_entries")]
    pub max_entries: usize,
    #[serde(default = "default_ttl_seconds")]
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchingConfig {
    /// Enable or disable request batching (default: false for backward compatibility)
    #[serde(default)]
    pub enabled: bool,

    /// Time window in milliseconds to collect requests (default: 100ms)
    #[serde(default = "default_batch_window_ms")]
    pub window_ms: u64,

    /// Maximum number of requests in a batch before forcing flush (default: 10)
    #[serde(default = "default_max_batch_size")]
    pub max_batch_size: usize,

    /// Whitelist of methods that support batching (default: list methods)
    #[serde(default = "default_batch_methods")]
    pub methods: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AuthConfig {
    // Auth configuration (placeholder)
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ObservabilityConfig {
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TuiConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_tui_default_tab")]
    pub default_tab: String,
    #[serde(default = "default_tui_refresh_ms")]
    pub refresh_ms: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

// Default functions
fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8080
}
fn default_max_connections() -> usize {
    10000
}
fn default_true() -> bool {
    true
}
fn default_interval_seconds() -> u64 {
    10
}
fn default_timeout_seconds() -> u64 {
    5
}
fn default_healthy_threshold() -> u32 {
    2
}
fn default_unhealthy_threshold() -> u32 {
    3
}
fn default_health_path() -> String {
    "/health".to_string()
}
fn default_priority() -> u32 {
    100
}
fn default_weight() -> u32 {
    1
}
fn default_algorithm() -> String {
    "round_robin".to_string()
}
fn default_virtual_nodes() -> usize {
    150
}
fn default_max_per_backend() -> usize {
    100
}
fn default_min_idle() -> usize {
    10
}
fn default_max_entries() -> usize {
    10000
}
fn default_ttl_seconds() -> u64 {
    300
}
fn default_max_batch_size() -> usize {
    10
}
fn default_batch_window_ms() -> u64 {
    100
}
fn default_batch_methods() -> Vec<String> {
    vec![
        "tools/list".to_string(),
        "resources/list".to_string(),
        "prompts/list".to_string(),
    ]
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
}
fn default_tui_default_tab() -> String {
    "overview".to_string()
}
fn default_tui_refresh_ms() -> u64 {
    1000
}
fn default_timeout_ms() -> u64 {
    30000
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            worker_threads: 0,
            max_connections: default_max_connections(),
            tls: TlsConfig::default(),
        }
    }
}

impl Default for TuiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            default_tab: default_tui_default_tab(),
            refresh_ms: default_tui_refresh_ms(),
        }
    }
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: default_interval_seconds(),
            timeout_seconds: default_timeout_seconds(),
            healthy_threshold: default_healthy_threshold(),
            unhealthy_threshold: default_unhealthy_threshold(),
            path: default_health_path(),
        }
    }
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            algorithm: default_algorithm(),
            virtual_nodes: default_virtual_nodes(),
        }
    }
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_per_backend: default_max_per_backend(),
            min_idle: default_min_idle(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_entries: default_max_entries(),
            ttl_seconds: default_ttl_seconds(),
        }
    }
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            window_ms: default_batch_window_ms(),
            max_batch_size: default_max_batch_size(),
            methods: default_batch_methods(),
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
        }
    }
}

impl Config {
    /// Load configuration from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("yaml");

        match extension {
            "yaml" | "yml" => serde_yaml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse YAML: {}", e))),
            "toml" => toml::from_str(&content)
                .map_err(|e| Error::Config(format!("Failed to parse TOML: {}", e))),
            _ => Err(Error::Config(format!(
                "Unsupported config format: {}",
                extension
            ))),
        }
    }

    /// Discover and load configuration from standard locations
    pub fn discover_and_load() -> Result<Self> {
        Self::discover_and_load_with_path(None)
    }

    /// Discover and load configuration from standard locations with optional CLI override
    pub fn discover_and_load_with_path(cli_path: Option<PathBuf>) -> Result<Self> {
        use tracing::{info, warn};

        // 1. CLI flag (highest priority)
        if let Some(path) = cli_path {
            info!("Using config from CLI path: {:?}", path);
            return Self::from_file(&path);
        }

        // 2. XDG_CONFIG_HOME (new default)
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config).join("only1mcp")
        } else {
            dirs::home_dir()
                .ok_or_else(|| Error::Config("Cannot determine home directory".into()))?
                .join(".config")
                .join("only1mcp")
        };

        let config_path = config_dir.join("only1mcp.yaml");

        if config_path.exists() {
            info!("Using config from: {:?}", config_path);
            return Self::from_file(&config_path);
        }

        // 3. Legacy paths (for backwards compatibility)
        let mut legacy_paths = vec![
            PathBuf::from("only1mcp.yaml"), // Current directory
            PathBuf::from("only1mcp.toml"), // Current directory
        ];

        // Add home directory path if available
        if let Some(home) = dirs::home_dir() {
            legacy_paths.push(home.join(".only1mcp/config.yaml"));
        }

        // Add system-wide path
        legacy_paths.push(PathBuf::from("/etc/only1mcp/config.yaml"));

        for legacy_path in legacy_paths {
            if legacy_path.exists() {
                warn!("Using legacy config path: {:?}", legacy_path);
                warn!("Consider migrating to: {:?}", config_path);
                return Self::from_file(&legacy_path);
            }
        }

        // 4. Create default config from template
        info!("No config found, creating default at: {:?}", config_path);
        Self::create_default_config(&config_path)?;
        Self::from_file(&config_path)
    }

    /// Create default configuration file from embedded template
    fn create_default_config(path: &Path) -> Result<()> {
        use std::fs;

        // Create parent directory
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| Error::Config(format!("Failed to create config directory: {}", e)))?;
        }

        // Embedded default config (solo.yaml template)
        let default_config = include_str!("../../config/templates/solo.yaml");

        fs::write(path, default_config)
            .map_err(|e| Error::Config(format!("Failed to write default config: {}", e)))?;

        Ok(())
    }

    /// Validate configuration file
    pub fn validate_file(path: &Path) -> Result<()> {
        let _config = Self::from_file(path)?;
        // Additional validation logic here
        Ok(())
    }
}
