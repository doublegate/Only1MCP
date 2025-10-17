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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[derive(Default)]
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
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_max_batch_size")]
    pub max_batch_size: usize,
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
    50
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "json".to_string()
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

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: default_interval_seconds(),
            timeout_seconds: default_timeout_seconds(),
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
            max_batch_size: default_max_batch_size(),
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
        // Check standard locations
        let mut search_paths = vec![
            PathBuf::from("only1mcp.yaml"),
            PathBuf::from("only1mcp.toml"),
        ];

        if let Some(home) = dirs::home_dir() {
            search_paths.push(home.join(".only1mcp/config.yaml"));
        }

        search_paths.push(PathBuf::from("/etc/only1mcp/config.yaml"));

        for path in search_paths {
            if path.exists() {
                return Self::from_file(&path);
            }
        }

        // Return minimal default config
        Ok(Self::default())
    }

    /// Validate configuration file
    pub fn validate_file(path: &Path) -> Result<()> {
        let _config = Self::from_file(path)?;
        // Additional validation logic here
        Ok(())
    }
}

