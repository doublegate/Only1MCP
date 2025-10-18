//! Only1MCP Library
//!
//! Core functionality for the MCP server aggregator.
//! This library can be embedded in other applications.

pub mod auth;
pub mod batching;
pub mod cache;
pub mod config;
pub mod error;
pub mod health;
pub mod metrics;
pub mod proxy;
pub mod routing;
pub mod transport;
pub mod tui;
pub mod types;

pub use config::Config;
pub use error::{Error, Result};
pub use proxy::ProxyServer;
