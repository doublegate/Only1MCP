/// Plugin Marketplace Module
///
/// Provides a centralized registry for Only1MCP plugins with:
/// - Plugin discovery and search
/// - Version management
/// - Security validation
/// - Dependency resolution
/// - Automated testing

pub mod api;
pub mod db;
pub mod storage;
pub mod types;
pub mod validator;

pub use api::PluginRegistryApi;
pub use types::{Plugin, PluginManifest, PluginVersion, PluginMetadata};
pub use validator::PluginValidator;

use crate::error::Result;

/// Initialize the plugin marketplace system
pub async fn init_marketplace(db_url: &str, storage_config: storage::StorageConfig) -> Result<()> {
    // Initialize database connection pool
    db::init_db_pool(db_url).await?;

    // Initialize storage backend (S3/GCS)
    storage::init_storage(storage_config).await?;

    tracing::info!("Plugin marketplace initialized successfully");
    Ok(())
}
