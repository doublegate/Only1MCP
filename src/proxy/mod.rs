//! Core proxy server implementation
//!
//! This module contains the main proxy server logic that aggregates
//! multiple MCP servers behind a unified interface.

use crate::{config::Config, error::Result};
use std::sync::Arc;

pub mod handler;
pub mod registry;
pub mod router;
pub mod server;

pub use server::ProxyServer;

/// Initialize the proxy server with configuration
pub async fn init(config: Config) -> Result<ProxyServer> {
    ProxyServer::new(config).await
}
