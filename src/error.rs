//! Error types for Only1MCP

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::io;
use thiserror::Error;

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

    #[error("Server error: {0}")]
    Server(String),

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

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::BackendTimeout(_) | Error::Transport(_) | Error::Internal(_)
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

/// Proxy-specific errors for HTTP handlers
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("No backend available: {0}")]
    NoBackendAvailable(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Core error: {0}")]
    Core(Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

impl ProxyError {
    pub fn is_retryable(&self) -> bool {
        matches!(self, ProxyError::BackendError(_) | ProxyError::Timeout(_))
    }
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ProxyError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ProxyError::NoBackendAvailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg.clone()),
            ProxyError::BackendError(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            ProxyError::Timeout(msg) => (StatusCode::GATEWAY_TIMEOUT, msg.clone()),
            ProxyError::Transport(msg) => (StatusCode::BAD_GATEWAY, msg.clone()),
            ProxyError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg.clone()),
            ProxyError::Json(err) => (StatusCode::BAD_REQUEST, err.to_string()),
            ProxyError::Serialization(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ProxyError::Deserialization(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ProxyError::Core(err) => (
                StatusCode::from_u16(err.status_code())
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
                err.to_string(),
            ),
        };

        let body = Json(json!({
            "jsonrpc": "2.0",
            "error": {
                "code": status.as_u16(),
                "message": error_message,
            },
            "id": null
        }));

        (status, body).into_response()
    }
}

// From trait implementations for error conversions
impl From<crate::proxy::router::RoutingError> for ProxyError {
    fn from(err: crate::proxy::router::RoutingError) -> Self {
        match err {
            crate::proxy::router::RoutingError::NoBackendAvailable(msg) => {
                ProxyError::NoBackendAvailable(msg)
            },
            crate::proxy::router::RoutingError::AllBackendsUnhealthy(msg) => {
                ProxyError::NoBackendAvailable(format!("All backends unhealthy: {}", msg))
            },
            crate::proxy::router::RoutingError::HashRingEmpty => {
                ProxyError::Internal("Hash ring empty".to_string())
            },
            crate::proxy::router::RoutingError::NoServerSelected => {
                ProxyError::Internal("No server selected".to_string())
            },
            crate::proxy::router::RoutingError::Registry(msg) => {
                ProxyError::Internal(format!("Registry error: {}", msg))
            },
        }
    }
}

impl From<Error> for ProxyError {
    fn from(err: Error) -> Self {
        ProxyError::Core(err)
    }
}
