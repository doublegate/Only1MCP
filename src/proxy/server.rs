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
    batching::BatchAggregator,
    cache::ResponseCache,
    config::Config,
    error::{Error, Result},
    metrics::Metrics,
    proxy::{
        handler::{handle_jsonrpc_request, handle_websocket_upgrade},
        router::ServerRegistry,
    },
    types::McpRequest,
};

/// Main proxy server structure containing all shared state and configuration.
#[derive(Clone)]
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
    pub sse_transport: Option<Arc<crate::transport::sse::SseTransportPool>>,
    pub streamable_http_transport: Option<Arc<crate::transport::streamable_http::StreamableHttpTransportPool>>,
    pub batch_aggregator: Arc<BatchAggregator>,
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

        let cache = Arc::new(ResponseCache::new(crate::cache::CacheConfig::default()));

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

        // Initialize SSE transport if any SSE servers are configured
        let sse_transport = if self
            .config
            .servers
            .iter()
            .any(|s| matches!(s.transport, crate::config::TransportConfig::Sse { .. }))
        {
            let sse_config = crate::transport::sse::SseTransportConfig::default();
            Some(Arc::new(crate::transport::sse::SseTransportPool::new(sse_config)))
        } else {
            None
        };

        // Initialize Streamable HTTP transport if any streamable_http servers are configured
        let streamable_http_transport = if self
            .config
            .servers
            .iter()
            .any(|s| matches!(s.transport, crate::config::TransportConfig::StreamableHttp { .. }))
        {
            Some(Arc::new(crate::transport::streamable_http::StreamableHttpTransportPool::new()))
        } else {
            None
        };

        // Initialize BatchAggregator with backend caller
        let batch_config = self.config.context_optimization.batching.clone();
        let batch_aggregator = {
            let http_transport_clone = http_transport.clone();
            let stdio_transport_clone = stdio_transport.clone();
            let sse_transport_clone = sse_transport.clone();
            let streamable_http_transport_clone = streamable_http_transport.clone();
            let config_clone = self.config.clone();

            Arc::new(BatchAggregator::new(batch_config).with_backend_caller(
                move |server_id: String, request: McpRequest| {
                    // Find server config
                    let server_config = config_clone
                        .servers
                        .iter()
                        .find(|s| s.id == server_id)
                        .ok_or_else(|| Error::ServerNotFound(server_id.clone()))?;

                    // Send via appropriate transport (synchronous wrapper around async)
                    let response = match &server_config.transport {
                        crate::config::TransportConfig::Http { url, headers } => {
                            // Nesting required for: transport extraction → error handling
                            #[allow(clippy::excessive_nesting)]
                            let http_transport =
                                http_transport_clone.as_ref().ok_or_else(|| {
                                    Error::Transport("HTTP transport not initialized".into())
                                })?;

                            // Use tokio runtime to block on async operation
                            // Nesting required for: block_in_place → block_on async runtime bridge
                            #[allow(clippy::excessive_nesting)]
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    http_transport
                                        .send_request_with_headers(url, request.clone(), headers.clone())
                                        .await
                                        .map_err(|e| Error::Transport(e.to_string()))
                                })
                            })?
                        },
                        crate::config::TransportConfig::Stdio { command, args, env } => {
                            // Nesting required for: transport extraction → error handling
                            #[allow(clippy::excessive_nesting)]
                            let stdio_transport =
                                stdio_transport_clone.as_ref().ok_or_else(|| {
                                    Error::Transport("STDIO transport not initialized".into())
                                })?;

                            let stdio_config = crate::transport::stdio::StdioConfig {
                                command: command.clone(),
                                args: args.clone(),
                                env: env.clone(),
                                cwd: None,
                                timeout_ms: 30000,
                                max_memory_mb: Some(512),
                                max_cpu_percent: Some(50),
                                sandbox: true,
                            };

                            // Nesting required for: block_in_place → block_on async runtime bridge
                            #[allow(clippy::excessive_nesting)]
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    stdio_transport
                                        .send_request_with_config(
                                            server_id.clone(),
                                            &stdio_config,
                                            request.clone(),
                                        )
                                        .await
                                        .map_err(|e| Error::Transport(e.to_string()))
                                })
                            })?
                        },
                        crate::config::TransportConfig::Sse { url, headers } => {
                            // Nesting required for: transport extraction → error handling
                            #[allow(clippy::excessive_nesting)]
                            let sse_transport =
                                sse_transport_clone.as_ref().ok_or_else(|| {
                                    Error::Transport("SSE transport not initialized".into())
                                })?;

                            // Nesting required for: block_in_place → block_on async runtime bridge
                            #[allow(clippy::excessive_nesting)]
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    sse_transport
                                        .send_request_with_headers(url, request.clone(), headers.clone())
                                        .await
                                        .map_err(|e| Error::Transport(e.to_string()))
                                })
                            })?
                        },
                        crate::config::TransportConfig::StreamableHttp { url, headers, timeout_ms } => {
                            // Nesting required for: transport extraction → error handling
                            #[allow(clippy::excessive_nesting)]
                            let streamable_http_transport =
                                streamable_http_transport_clone.as_ref().ok_or_else(|| {
                                    Error::Transport("Streamable HTTP transport not initialized".into())
                                })?;

                            // Create transport config for this server
                            let transport_config = crate::transport::streamable_http::StreamableHttpConfig {
                                url: url.clone(),
                                headers: headers.clone(),
                                timeout_ms: *timeout_ms,
                            };

                            // Get or create transport (maintains session)
                            let transport = streamable_http_transport.get_or_create(transport_config);

                            // Nesting required for: block_in_place → block_on async runtime bridge
                            #[allow(clippy::excessive_nesting)]
                            tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    transport
                                        .send_request(request.clone())
                                        .await
                                        .map_err(|e| Error::Transport(e.to_string()))
                                })
                            })?
                        },
                    };

                    Ok(response)
                },
            ))
        };

        // Create shared application state
        let app_state = AppState {
            config: self.config.clone(),
            registry: self.registry.clone(),
            cache: self.cache.clone(),
            metrics: self.metrics.clone(),
            http_transport,
            stdio_transport,
            sse_transport,
            streamable_http_transport,
            batch_aggregator,
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

    /// Run server with configuration hot-reload support
    ///
    /// This method creates a ConfigLoader that watches the configuration file
    /// for changes and automatically applies them without server restart.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file to watch
    ///
    /// # Example
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use only1mcp::proxy::server::ProxyServer;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     ProxyServer::run_with_hot_reload(PathBuf::from("config.yaml")).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn run_with_hot_reload(config_path: std::path::PathBuf) -> Result<()> {
        use crate::config::ConfigLoader;

        // Create config loader with hot-reload
        info!(
            "Enabling configuration hot-reload for: {}",
            config_path.display()
        );
        let loader = ConfigLoader::new(config_path)?.watch()?;

        let config = loader.get_config();
        let mut reload_rx = loader.subscribe();

        // Create server
        let server = Arc::new(Self::new(config.as_ref().clone()).await?);

        // Spawn reload handler
        let server_clone = server.clone();
        tokio::spawn(async move {
            while reload_rx.changed().await.is_ok() {
                let new_config = reload_rx.borrow().clone();
                info!("Configuration change detected, applying new configuration...");

                // Update server with new config
                if let Err(e) = server_clone.update_config(new_config.as_ref()).await {
                    tracing::error!("Failed to apply new config: {}", e);
                } else {
                    info!("Configuration successfully updated");
                }
            }
        });

        // Run server (we need to extract it from Arc)
        let server_owned = Arc::try_unwrap(server).unwrap_or_else(|arc| (*arc).clone());
        server_owned.run().await
    }

    /// Update server configuration during hot-reload
    ///
    /// This method is called when a configuration change is detected.
    /// It updates the server registry with new backend servers.
    ///
    /// Note: Some configuration changes (like server host/port, TLS settings)
    /// require a server restart and cannot be hot-reloaded.
    async fn update_config(&self, new_config: &Config) -> Result<()> {
        info!("Updating server configuration...");

        // Update registry with new backends
        let mut registry = self.registry.write().await;

        // Clear existing servers
        registry.clear();

        // Add new servers from updated config
        for server_config in &new_config.servers {
            if server_config.enabled {
                registry.add_server(server_config.clone()).await?;
            }
        }

        info!(
            "Configuration updated: {} backend servers registered",
            new_config.servers.iter().filter(|s| s.enabled).count()
        );

        Ok(())
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
