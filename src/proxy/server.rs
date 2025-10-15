//! Main proxy server implementation using Axum web framework.
//!
//! This module initializes the core HTTP server with all required middleware,
//! routes, and shared application state. Supports both MCP protocol endpoints
//! and management APIs for hot configuration updates.
//!
//! # Features
//!
//! - High-performance async request handling via Tokio
//! - Zero-copy streaming for large payloads
//! - Graceful shutdown with connection draining
//! - TLS 1.3 support with Rustls
//! - Prometheus metrics and OpenTelemetry tracing

use axum::{
    Router,
    extract::{State, Path, Query, Json},
    http::{StatusCode, HeaderMap, Request, Response},
    middleware::{self, from_fn_with_state},
    response::IntoResponse,
    body::Body,
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::TraceLayer,
};
use std::{
    sync::Arc,
    time::Duration,
    net::SocketAddr,
};
use tokio::sync::RwLock;
use tracing::{info, error, debug};

use crate::{
    config::Config,
    error::{Result, Error},
    proxy::{
        registry::ServerRegistry,
        handler::{handle_jsonrpc_request, handle_websocket_upgrade, health_check},
    },
    cache::ResponseCache,
    metrics::Metrics,
};

/// Main proxy server structure containing all shared state and configuration.
pub struct ProxyServer {
    /// Server configuration loaded from YAML/TOML
    config: Arc<Config>,
    /// Registry of backend MCP servers
    registry: Arc<RwLock<ServerRegistry>>,
    /// LRU cache for response memoization
    cache: Arc<ResponseCache>,
    /// Metrics collector (Prometheus)
    metrics: Arc<Metrics>,
    /// Graceful shutdown handle
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
}

/// Shared application state passed to all handlers
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub registry: Arc<RwLock<ServerRegistry>>,
    pub cache: Arc<ResponseCache>,
    pub metrics: Arc<Metrics>,
}

impl ProxyServer {
    /// Initialize a new proxy server with the given configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration loaded from file or environment
    ///
    /// # Returns
    ///
    /// * `Ok(ProxyServer)` - Initialized server ready to run
    /// * `Err(Error)` - Configuration or initialization error
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing Only1MCP proxy server");

        // Initialize shared application state
        let registry = Arc::new(RwLock::new(
            ServerRegistry::from_config(&config).await?
        ));

        let cache = Arc::new(
            ResponseCache::new(
                10000,  // max_entries
                100 * 1024 * 1024,  // 100 MB max_size
            )
        );

        let metrics = Arc::new(Metrics::new());

        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);

        Ok(Self {
            config: Arc::new(config),
            registry,
            cache,
            metrics,
            shutdown_tx,
        })
    }

    /// Build the Axum router with all routes and middleware.
    fn build_router(&self) -> Router {
        // Create shared application state
        let app_state = AppState {
            config: self.config.clone(),
            registry: self.registry.clone(),
            cache: self.cache.clone(),
            metrics: self.metrics.clone(),
        };

        // Build main MCP protocol routes
        let mcp_routes = Router::new()
            // Core MCP endpoints (JSON-RPC 2.0 over HTTP)
            .route("/", post(handle_jsonrpc_request))
            .route("/mcp", post(handle_jsonrpc_request))

            // WebSocket for streaming
            .route("/ws", get(handle_websocket_upgrade))

            // Health check
            .route("/health", get(health_check));

        // Management API routes
        let admin_routes = Router::new()
            .route("/health", get(health_check))
            .route("/metrics", get(crate::metrics::prometheus_metrics));

        // Combine routes with middleware stack
        Router::new()
            .nest("/", mcp_routes)
            .nest("/api/v1/admin", admin_routes)
            .layer(
                ServiceBuilder::new()
                    // CORS for browser-based clients
                    .layer(CorsLayer::permissive())

                    // Compression for responses
                    .layer(CompressionLayer::new())

                    // Request timeout (30s default)
                    .layer(tower::timeout::TimeoutLayer::new(Duration::from_secs(30)))

                    // Request/response tracing
                    .layer(TraceLayer::new_for_http())
            )
            .with_state(app_state)
    }

    /// Start the proxy server and begin accepting connections.
    pub async fn run(self) -> Result<()> {
        let router = self.build_router();

        // Bind to configured address
        let addr = format!("{}:{}", self.config.server.host, self.config.server.port)
            .parse::<SocketAddr>()
            .map_err(|e| Error::Config(format!("Invalid address: {}", e)))?;

        info!("Starting Only1MCP proxy server on {}", addr);

        // Create TCP listener
        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| Error::Server(format!("Failed to bind: {}", e)))?;

        info!("Server listening on {}", addr);

        // Run server with graceful shutdown
        axum::serve(listener, router)
            .with_graceful_shutdown(async {
                let _ = self.shutdown_tx.subscribe().recv().await;
                info!("Shutting down proxy server gracefully...");
            })
            .await
            .map_err(|e| Error::Server(format!("Server error: {}", e)))?;

        info!("Proxy server stopped");
        Ok(())
    }

    /// Trigger graceful shutdown
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}
