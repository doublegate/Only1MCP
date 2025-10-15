//! Error types for Only1MCP

use thiserror::Error;
use std::io;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid template: {0}")]
    InvalidTemplate(String),

    #[error("Server not found: {0}")]
    ServerNotFound(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Backend timeout after {0}ms")]
    BackendTimeout(u64),

    #[error("No backend available for tool: {0}")]
    NoBackendAvailable(String),

    #[error("All backends unhealthy for tool: {0}")]
    AllBackendsUnhealthy(String),

    #[error("Circuit breaker open for server: {0}")]
    CircuitBreakerOpen(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            Error::BackendTimeout(_) |
            Error::Transport(_) |
            Error::Internal(_)
        )
    }

    pub fn status_code(&self) -> u16 {
        match self {
            Error::NoBackendAvailable(_) => 503,
            Error::AllBackendsUnhealthy(_) => 503,
            Error::BackendTimeout(_) => 504,
            Error::CircuitBreakerOpen(_) => 503,
            Error::RateLimitExceeded => 429,
            Error::AuthFailed(_) => 401,
            _ => 500,
        }
    }
}
