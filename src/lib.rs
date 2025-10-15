//! Only1MCP Library
//!
//! Core functionality for the MCP server aggregator.
//! This library can be embedded in other applications.

pub mod config;
pub mod proxy;
pub mod transport;
pub mod auth;
pub mod cache;
pub mod metrics;
pub mod health;
pub mod error;
pub mod types;
pub mod routing;

pub use error::{Error, Result};
pub use config::Config;
pub use proxy::ProxyServer;
