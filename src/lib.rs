//! Only1MCP Library
//!
//! Core functionality for the MCP server aggregator.
//! This library can be embedded in other applications.

pub mod admin;
pub mod ai;
pub mod analytics;
pub mod audit;
pub mod auth;
pub mod batching;
pub mod cache;
pub mod config;
pub mod daemon;
pub mod error;
pub mod health;
pub mod metrics;
pub mod multiregion;
pub mod plugins;
pub mod proxy;
pub mod ratelimit;
pub mod routing;
pub mod security;
pub mod tenancy;
pub mod transport;
pub mod tui;
pub mod types;

pub use config::Config;
pub use error::{Error, Result};
pub use proxy::ProxyServer;
