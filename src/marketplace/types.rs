/// Plugin Marketplace Types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Plugin information stored in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: i64,
    pub name: String,
    pub latest_version: String,
    pub author: String,
    pub description: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: String,
    pub downloads: i64,
    pub rating: Option<f32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Plugin version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginVersion {
    pub id: i64,
    pub plugin_id: i64,
    pub version: String,
    pub tarball_url: String,
    pub checksum_sha256: String,
    pub manifest: PluginManifest,
    pub published_at: DateTime<Utc>,
    pub yanked: bool,
}

/// Plugin manifest (plugin.yaml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: String,

    /// Plugin capabilities
    pub capabilities: Vec<PluginCapability>,

    /// Runtime dependencies
    pub dependencies: HashMap<String, String>,

    /// Configuration schema
    pub config_schema: Option<serde_json::Value>,

    /// Search keywords
    pub keywords: Vec<String>,

    /// Compatibility information
    pub compatibility: PluginCompatibility,
}

/// Plugin capabilities enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    RequestTransform,
    ResponseTransform,
    Authentication,
    RateLimiting,
    Caching,
    MetricsCollector,
    LoadBalancer,
    ProtocolAdapter,
    Custom(String),
}

/// Plugin compatibility requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCompatibility {
    pub min_only1mcp_version: String,
    pub platforms: Vec<String>,
}

/// Plugin metadata for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub downloads: i64,
    pub rating: Option<f32>,
    pub capabilities: Vec<PluginCapability>,
}

/// Plugin search query
#[derive(Debug, Clone, Deserialize)]
pub struct PluginSearchQuery {
    pub query: Option<String>,
    pub capability: Option<PluginCapability>,
    pub sort_by: Option<SortBy>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

/// Sort options for plugin search
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    Downloads,
    Rating,
    RecentlyUpdated,
    Name,
}

/// Plugin rating submission
#[derive(Debug, Clone, Deserialize)]
pub struct PluginRating {
    pub rating: i32, // 1-5 stars
    pub review: Option<String>,
}

/// Plugin publish request
#[derive(Debug, Clone, Deserialize)]
pub struct PublishRequest {
    pub name: String,
    pub version: String,
    pub tarball_base64: String, // Base64-encoded tarball
    pub manifest: PluginManifest,
}

/// Plugin list response
#[derive(Debug, Clone, Serialize)]
pub struct PluginListResponse {
    pub plugins: Vec<PluginMetadata>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}
