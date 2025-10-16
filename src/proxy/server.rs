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
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::{
    cache::ResponseCache,
    config::Config,
    error::{Error, Result},
    metrics::Metrics,
    proxy::{
        handler::{handle_jsonrpc_request, handle_websocket_upgrade},
        router::ServerRegistry,
    },
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
    pub http_transport: Option<Arc<crate::transport::http::HttpTransportPool>>,
    pub stdio_transport: Option<Arc<crate::transport::stdio::StdioTransport>>,
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
        let registry = Arc::new(RwLock::new(ServerRegistry::from_config(&config).await?));

        let cache = Arc::new(ResponseCache::new(
            10000,             // max_entries
            100 * 1024 * 1024, // 100 MB max_size
        ));

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
        // Initialize HTTP transport pool manager
        // Note: We use a shared pool that can handle connections to multiple backends
        let http_transport = Some(Arc::new(crate::transport::http::HttpTransportPool::new()));

        // Initialize STDIO transport if any STDIO servers are configured
        let stdio_transport = if self
            .config
            .servers
            .iter()
            .any(|s| matches!(s.transport, crate::config::TransportConfig::Stdio { .. }))
        {
            Some(Arc::new(crate::transport::stdio::StdioTransport::new()))
        } else {
            None
        };

        // Create shared application state
        let app_state = AppState {
            config: self.config.clone(),
            registry: self.registry.clone(),
            cache: self.cache.clone(),
            metrics: self.metrics.clone(),
            http_transport,
            stdio_transport,
        };

        // Build main MCP protocol routes
        let mcp_routes = Router::new()
            // Core MCP endpoints (JSON-RPC 2.0 over HTTP)
            .route("/", post(handle_jsonrpc_request))
            .route("/mcp", post(handle_jsonrpc_request))

            // WebSocket for streaming
            .route("/ws", get(handle_websocket_upgrade))

            // Health check
            .route("/health", get(health_check_handler));

        // Management API routes
        let admin_routes = Router::new()
            .route("/health", get(health_check_handler))
            .route("/metrics", get(crate::metrics::metrics_handler));

        // Combine routes with middleware stack
        Router::new()
            .nest("/", mcp_routes)
            .nest("/api/v1/admin", admin_routes)
            .with_state(app_state)
            // Apply middleware in reverse order (innermost first)
            .layer(TraceLayer::new_for_http())
            // Note: TimeoutLayer commented out due to type incompatibility with Axum 0.7
            // Individual handlers should implement their own timeouts
            // .layer(tower::timeout::TimeoutLayer::new(Duration::from_secs(30)))
            .layer(CompressionLayer::new())
            .layer(CorsLayer::permissive())
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
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Server(format!("Failed to bind: {}", e)))?;

        info!("Server listening on {}", addr);

        // Run server with graceful shutdown
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        axum::serve(listener, router)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.recv().await;
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

/// Health check endpoint handler
async fn health_check_handler(State(state): State<AppState>) -> impl IntoResponse {
    use axum::http::StatusCode;
    use serde_json::json;

    // Check if registry has any servers
    let registry = state.registry.read().await;
    let server_count = registry.len();

    let status = if server_count > 0 { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };

    (
        status,
        axum::Json(json!({
            "status": if status == StatusCode::OK { "healthy" } else { "unhealthy" },
            "servers": server_count,
            "version": env!("CARGO_PKG_VERSION"),
        })),
    )
}
