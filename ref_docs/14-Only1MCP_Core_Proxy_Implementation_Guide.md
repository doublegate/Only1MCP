# 14-Only1MCP Core Proxy Implementation Guide
## Detailed Axum Framework Setup, Request Routing Architecture, and Streaming Protocols

**Document Version:** 1.0  
**Implementation Focus:** Core Proxy Engine with Axum  
**Target Components:** Router, Transport Handlers, Streaming, Connection Management  
**Date:** October 14, 2025  
**Status:** Technical Implementation Specification

---

## TABLE OF CONTENTS

1. [Executive Summary](#executive-summary)
2. [Axum Server Architecture](#axum-server-architecture)
3. [Request Routing Engine](#request-routing-engine)
4. [Transport Protocol Handlers](#transport-protocol-handlers)
5. [Streaming Implementation](#streaming-implementation)
6. [Connection Pool Management](#connection-pool-management)
7. [Error Handling & Resilience](#error-handling--resilience)
8. [Performance Optimizations](#performance-optimizations)
9. [State Management](#state-management)
10. [Testing Strategy](#testing-strategy)
11. [Monitoring & Observability](#monitoring--observability)
12. [Security Middleware](#security-middleware)
13. [Hot-Reload Implementation](#hot-reload-implementation)
14. [Integration Examples](#integration-examples)
15. [Troubleshooting Guide](#troubleshooting-guide)

---

## EXECUTIVE SUMMARY

### Core Architecture Principles

The Only1MCP proxy engine leverages **Axum** for its proven performance advantages (30-60% faster than NGINX in benchmarks) and excellent async Rust integration. This implementation guide provides production-ready code for:

- **Sub-5ms latency overhead** through zero-copy streaming and connection pooling
- **10k+ req/s throughput** via Tokio multi-threaded runtime
- **Protocol-agnostic routing** supporting STDIO, HTTP, SSE, and WebSocket transports
- **Hot-swappable backends** with zero-downtime configuration updates
- **Intelligent load balancing** using consistent hashing with virtual nodes

### Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Web Framework** | Axum 0.7+ | Tower ecosystem, type safety, performance |
| **Async Runtime** | Tokio (multi-threaded) | Production-proven, excellent performance |
| **State Management** | Arc<RwLock<T>> + DashMap | Lock-free reads, concurrent writes |
| **Streaming** | Tokio streams + SSE | Backpressure support, memory efficiency |
| **Serialization** | Serde with simd-json | 30% faster JSON parsing |
| **Connection Pooling** | bb8 + custom pool | Automatic health checks, configurable limits |

---

## AXUM SERVER ARCHITECTURE

### Core Server Setup

```rust
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
//! 
//! # Example
//! 
//! ```rust
//! let server = ProxyServer::new(config).await?;
//! server.run().await?;
//! ```

use axum::{
    Router,
    Server,
    extract::{State, Path, Query, Json, WebSocketUpgrade},
    handler::Handler,
    http::{StatusCode, HeaderMap, Request, Response},
    middleware::{self, from_fn_with_state, Next},
    response::{IntoResponse, Sse, Html},
    body::{Body, Bytes},
    error_handling::HandleErrorLayer,
    BoxError,
};
use tower::{
    ServiceBuilder,
    buffer::BufferLayer,
    limit::{ConcurrencyLimitLayer, RateLimitLayer},
    timeout::TimeoutLayer,
    retry::RetryLayer,
    load_shed::LoadShedLayer,
};
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    trace::{TraceLayer, DefaultOnRequest, DefaultOnResponse},
    request_id::{RequestIdLayer, MakeRequestId},
    sensitive_headers::SetSensitiveRequestHeadersLayer,
    validate_request::ValidateRequestHeaderLayer,
};
use std::{
    sync::Arc,
    time::Duration,
    net::SocketAddr,
    collections::HashMap,
};
use tokio::sync::RwLock;
use dashmap::DashMap;
use tracing::{info, error, debug, warn, instrument};

/// Main proxy server structure containing all shared state and configuration.
pub struct ProxyServer {
    /// Server configuration loaded from YAML/TOML
    config: Arc<ServerConfig>,
    /// Registry of backend MCP servers
    registry: Arc<RwLock<ServerRegistry>>,
    /// LRU cache for response memoization
    cache: Arc<ResponseCache>,
    /// Connection pools for each backend
    pools: Arc<ConnectionPools>,
    /// Metrics collector (Prometheus)
    metrics: Arc<Metrics>,
    /// Hot-reload configuration watcher
    config_watcher: Arc<ConfigWatcher>,
    /// Graceful shutdown handle
    shutdown_tx: tokio::sync::broadcast::Sender<()>,
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
    pub async fn new(config: ServerConfig) -> Result<Self, Error> {
        // Initialize shared application state
        let registry = Arc::new(RwLock::new(
            ServerRegistry::from_config(&config.servers)?
        ));
        
        let cache = Arc::new(
            ResponseCache::new(
                config.context_optimization.cache.max_entries,
                config.context_optimization.cache.max_size_mb * 1024 * 1024,
            )
        );
        
        let pools = Arc::new(
            ConnectionPools::new(config.proxy.connection_pool.clone())
        );
        
        let metrics = Arc::new(Metrics::new());
        
        // Setup hot-reload configuration watcher
        let config_watcher = Arc::new(
            ConfigWatcher::new(
                config.config_path.clone(),
                config.proxy.hot_reload.clone(),
            ).await?
        );
        
        let (shutdown_tx, _) = tokio::sync::broadcast::channel(1);
        
        Ok(Self {
            config: Arc::new(config),
            registry,
            cache,
            pools,
            metrics,
            config_watcher,
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
            pools: self.pools.clone(),
            metrics: self.metrics.clone(),
        };
        
        // Build main MCP protocol routes
        let mcp_routes = Router::new()
            // Core MCP endpoints (JSON-RPC 2.0 over HTTP)
            .route("/", post(handle_jsonrpc_request))
            .route("/mcp", post(handle_jsonrpc_request))
            
            // Tool discovery and invocation
            .route("/tools/list", post(handle_tools_list))
            .route("/tools/call", post(handle_tools_call))
            
            // Resource management
            .route("/resources/list", post(handle_resources_list))
            .route("/resources/read", post(handle_resources_read))
            .route("/resources/subscribe", post(handle_resources_subscribe))
            
            // Prompts and sampling
            .route("/prompts/list", post(handle_prompts_list))
            .route("/prompts/get", post(handle_prompts_get))
            .route("/sampling/createMessage", post(handle_sampling_create))
            
            // WebSocket for streaming
            .route("/ws", get(handle_websocket_upgrade))
            
            // Server-Sent Events (legacy but still supported)
            .route("/sse", get(handle_sse_stream));
        
        // Management API routes (separate port in production)
        let admin_routes = Router::new()
            // Server management
            .route("/servers", get(list_servers).post(add_server))
            .route("/servers/:id", 
                get(get_server)
                .patch(update_server)
                .delete(remove_server)
            )
            
            // Health and metrics
            .route("/health", get(health_check))
            .route("/metrics", get(prometheus_metrics))
            
            // Cache management
            .route("/cache/stats", get(cache_stats))
            .route("/cache/clear", post(cache_clear))
            
            // Configuration
            .route("/config", get(get_config).post(update_config))
            .route("/config/reload", post(reload_config));
        
        // Combine routes with middleware stack
        Router::new()
            .nest("/", mcp_routes)
            .nest("/api/v1/admin", admin_routes)
            .layer(
                ServiceBuilder::new()
                    // Add request ID for tracing
                    .layer(RequestIdLayer::new(
                        UuidRequestId::default(),
                        HeaderName::from_static("x-request-id"),
                    ))
                    
                    // Security headers
                    .layer(SetSensitiveRequestHeadersLayer::new([
                        header::AUTHORIZATION,
                        header::COOKIE,
                    ]))
                    
                    // CORS for browser-based clients
                    .layer(
                        CorsLayer::new()
                            .allow_origin(tower_http::cors::Any)
                            .allow_methods([Method::GET, Method::POST])
                            .allow_headers([header::CONTENT_TYPE])
                    )
                    
                    // Compression for responses
                    .layer(CompressionLayer::new())
                    
                    // Rate limiting (60 req/min per client)
                    .layer(RateLimitLayer::new(
                        60,
                        Duration::from_secs(60)
                    ))
                    
                    // Concurrent request limit (1000 concurrent)
                    .layer(ConcurrencyLimitLayer::new(1000))
                    
                    // Request timeout (30s default)
                    .layer(TimeoutLayer::new(Duration::from_secs(30)))
                    
                    // Load shedding under pressure
                    .layer(LoadShedLayer::new())
                    
                    // Request/response tracing
                    .layer(
                        TraceLayer::new_for_http()
                            .make_span_with(|request: &Request<_>| {
                                let request_id = request
                                    .headers()
                                    .get("x-request-id")
                                    .and_then(|v| v.to_str().ok())
                                    .unwrap_or("unknown");
                                    
                                tracing::info_span!(
                                    "http_request",
                                    method = %request.method(),
                                    uri = %request.uri(),
                                    request_id = %request_id,
                                )
                            })
                            .on_response(|response: &Response<_>, latency: Duration, _span: &Span| {
                                tracing::info!(
                                    status = response.status().as_u16(),
                                    latency = ?latency,
                                    "response"
                                );
                            })
                    )
                    
                    // Custom middleware
                    .layer(from_fn_with_state(
                        app_state.clone(),
                        validate_auth,
                    ))
                    .layer(from_fn_with_state(
                        app_state.clone(),
                        collect_metrics,
                    ))
            )
            .with_state(app_state)
    }
    
    /// Start the proxy server and begin accepting connections.
    #[instrument(skip(self))]
    pub async fn run(self) -> Result<(), Error> {
        let router = self.build_router();
        
        // Bind to configured address
        let addr = SocketAddr::from((
            self.config.server.host.parse::<IpAddr>()?,
            self.config.server.port,
        ));
        
        info!("Starting Only1MCP proxy server on {}", addr);
        
        // Configure TLS if enabled
        let server = if self.config.server.tls.enabled {
            let tls_config = RustlsConfig::from_pem_file(
                &self.config.server.tls.cert_path,
                &self.config.server.tls.key_path,
            ).await?;
            
            axum_server::bind_rustls(addr, tls_config)
                .serve(router.into_make_service())
        } else {
            Server::bind(&addr)
                .serve(router.into_make_service())
        };
        
        // Start background tasks
        tokio::spawn(self.start_health_checker());
        tokio::spawn(self.start_config_watcher());
        tokio::spawn(self.start_metrics_collector());
        
        // Run server with graceful shutdown
        server
            .with_graceful_shutdown(async {
                let _ = self.shutdown_tx.subscribe().recv().await;
                info!("Shutting down proxy server gracefully...");
            })
            .await?;
        
        info!("Proxy server stopped");
        Ok(())
    }
}
```

---

## REQUEST ROUTING ENGINE

### Intelligent Request Router

```rust
//! Request routing engine for distributing MCP requests to backend servers.
//! 
//! Implements multiple routing algorithms:
//! - Consistent hashing with virtual nodes for session affinity
//! - Least connections for optimal load distribution
//! - Round-robin for simple fairness
//! - Health-aware routing with automatic failover
//! 
//! # Routing Decision Flow
//! 
//! 1. Extract tool/method from request
//! 2. Check cache for memoized response
//! 3. Find eligible backend servers
//! 4. Apply routing algorithm
//! 5. Handle failures with retry/failover
//! 6. Cache successful responses

use std::hash::{Hash, Hasher};
use xxhash_rust::xxh3::Xxh3;
use arc_swap::ArcSwap;

/// Main request router responsible for backend selection and load balancing.
pub struct RequestRouter {
    /// Consistent hash ring for session affinity
    hash_ring: Arc<ArcSwap<ConsistentHashRing>>,
    /// Active server health states
    health_states: Arc<DashMap<ServerId, HealthState>>,
    /// Per-server connection counts
    connection_counts: Arc<DashMap<ServerId, AtomicUsize>>,
    /// Routing configuration
    config: RoutingConfig,
    /// Circuit breakers per backend
    circuit_breakers: Arc<DashMap<ServerId, CircuitBreaker>>,
}

impl RequestRouter {
    /// Route an incoming MCP request to the appropriate backend server.
    /// 
    /// # Arguments
    /// 
    /// * `request` - The MCP JSON-RPC request to route
    /// * `registry` - Current registry of available servers
    /// * `cache` - Response cache for memoization
    /// 
    /// # Returns
    /// 
    /// * `Ok((ServerId, Duration))` - Selected server and expected latency
    /// * `Err(RoutingError)` - No available backend or routing failure
    #[instrument(skip(self, request, registry, cache))]
    pub async fn route_request(
        &self,
        request: &McpRequest,
        registry: &ServerRegistry,
        cache: &ResponseCache,
    ) -> Result<(ServerId, Duration), RoutingError> {
        let method = request.method();
        let tool_name = extract_tool_name(request)?;
        
        // Step 1: Check cache for memoized response
        let cache_key = self.compute_cache_key(request);
        if let Some(cached) = cache.get(&cache_key).await {
            self.metrics.cache_hits.inc();
            debug!("Cache hit for {}", tool_name);
            return Ok((cached.server_id, Duration::ZERO));
        }
        self.metrics.cache_misses.inc();
        
        // Step 2: Find servers that support this tool
        let eligible_servers = registry
            .find_servers_for_tool(&tool_name)
            .await?;
        
        if eligible_servers.is_empty() {
            error!("No servers available for tool: {}", tool_name);
            return Err(RoutingError::NoBackendAvailable(tool_name));
        }
        
        // Step 3: Filter by health status and circuit breaker state
        let healthy_servers: Vec<ServerId> = eligible_servers
            .into_iter()
            .filter(|&id| {
                // Check health state
                let is_healthy = self.health_states
                    .get(&id)
                    .map(|state| state.is_healthy())
                    .unwrap_or(false);
                    
                // Check circuit breaker
                let circuit_open = self.circuit_breakers
                    .get(&id)
                    .map(|cb| cb.is_open())
                    .unwrap_or(false);
                    
                is_healthy && !circuit_open
            })
            .collect();
        
        if healthy_servers.is_empty() {
            warn!("All backends unhealthy for tool: {}", tool_name);
            return Err(RoutingError::AllBackendsUnhealthy(tool_name));
        }
        
        // Step 4: Apply routing algorithm
        let selected_server = match self.config.algorithm {
            RoutingAlgorithm::ConsistentHash => {
                self.route_consistent_hash(&tool_name, &healthy_servers)
            },
            RoutingAlgorithm::LeastConnections => {
                self.route_least_connections(&healthy_servers)
            },
            RoutingAlgorithm::RoundRobin => {
                self.route_round_robin(&healthy_servers)
            },
            RoutingAlgorithm::Random => {
                self.route_random(&healthy_servers)
            },
            RoutingAlgorithm::WeightedRandom => {
                self.route_weighted_random(&healthy_servers, registry).await
            },
        }?;
        
        // Step 5: Update connection count
        self.connection_counts
            .entry(selected_server.clone())
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);
        
        // Step 6: Estimate latency based on historical data
        let estimated_latency = self.estimate_latency(&selected_server).await;
        
        info!(
            "Routed {} to server {} (estimated latency: {:?})",
            tool_name, selected_server, estimated_latency
        );
        
        Ok((selected_server, estimated_latency))
    }
    
    /// Consistent hashing implementation with virtual nodes.
    /// 
    /// Provides session affinity while maintaining good load distribution
    /// even when servers are added or removed from the pool.
    fn route_consistent_hash(
        &self,
        key: &str,
        servers: &[ServerId],
    ) -> Result<ServerId, RoutingError> {
        let hash_ring = self.hash_ring.load();
        
        // Hash the routing key
        let mut hasher = Xxh3::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();
        
        // Find the server in the ring
        let server = hash_ring
            .get_server(hash, servers)
            .ok_or_else(|| RoutingError::HashRingEmpty)?;
        
        debug!("Consistent hash selected: {} for key: {}", server, key);
        Ok(server.clone())
    }
    
    /// Least connections routing using Power of Two Choices.
    /// 
    /// Randomly selects two servers and routes to the one with fewer
    /// active connections. O(1) complexity with near-optimal distribution.
    fn route_least_connections(
        &self,
        servers: &[ServerId],
    ) -> Result<ServerId, RoutingError> {
        use rand::seq::SliceRandom;
        
        if servers.len() == 1 {
            return Ok(servers[0].clone());
        }
        
        // Power of Two Choices algorithm
        let mut rng = rand::thread_rng();
        let candidates: Vec<_> = servers
            .choose_multiple(&mut rng, 2.min(servers.len()))
            .collect();
        
        // Select server with minimum connections
        let selected = candidates
            .into_iter()
            .min_by_key(|&&id| {
                self.connection_counts
                    .get(id)
                    .map(|count| count.load(Ordering::Relaxed))
                    .unwrap_or(0)
            })
            .ok_or(RoutingError::NoServerSelected)?;
        
        debug!("Least connections selected: {}", selected);
        Ok(selected.clone())
    }
    
    /// Update health state based on request outcome.
    pub async fn update_health(
        &self,
        server_id: &ServerId,
        success: bool,
        latency: Duration,
    ) {
        let mut health = self.health_states
            .entry(server_id.clone())
            .or_insert_with(|| HealthState::new());
        
        if success {
            health.record_success(latency);
            
            // Update circuit breaker
            if let Some(mut cb) = self.circuit_breakers.get_mut(server_id) {
                cb.record_success();
            }
        } else {
            health.record_failure();
            
            // Update circuit breaker
            if let Some(mut cb) = self.circuit_breakers.get_mut(server_id) {
                cb.record_failure();
            }
        }
    }
}

/// Consistent hash ring with virtual nodes for better distribution.
pub struct ConsistentHashRing {
    /// Virtual nodes in the ring (server_id, virtual_node_id) -> hash
    ring: BTreeMap<u64, (ServerId, u32)>,
    /// Number of virtual nodes per physical server
    virtual_nodes: u32,
}

impl ConsistentHashRing {
    /// Create a new hash ring with the specified virtual node count.
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
        }
    }
    
    /// Add a server to the hash ring.
    pub fn add_server(&mut self, server_id: &ServerId) {
        for vnode in 0..self.virtual_nodes {
            let key = format!("{}:{}", server_id, vnode);
            let hash = xxhash_rust::xxh3::xxh3_64(key.as_bytes());
            self.ring.insert(hash, (server_id.clone(), vnode));
        }
        
        debug!(
            "Added {} to hash ring with {} virtual nodes",
            server_id, self.virtual_nodes
        );
    }
    
    /// Remove a server from the hash ring.
    pub fn remove_server(&mut self, server_id: &ServerId) {
        self.ring.retain(|_, (id, _)| id != server_id);
        debug!("Removed {} from hash ring", server_id);
    }
    
    /// Find the server responsible for a given hash.
    pub fn get_server(
        &self,
        hash: u64,
        eligible_servers: &[ServerId],
    ) -> Option<&ServerId> {
        // Find the first node with hash >= input hash
        let start_iter = self.ring.range(hash..).map(|(_, (id, _))| id);
        let wrap_iter = self.ring.iter().map(|(_, (id, _))| id);
        
        // Check servers in order starting from hash position
        start_iter
            .chain(wrap_iter)
            .find(|&id| eligible_servers.contains(id))
    }
}
```

---

## TRANSPORT PROTOCOL HANDLERS

### Multi-Transport Support

```rust
//! Transport protocol handlers for STDIO, HTTP, SSE, and WebSocket.
//! 
//! Each transport has unique characteristics:
//! - STDIO: Process-based, bidirectional pipes
//! - HTTP: Request-response, stateless
//! - SSE: Server-push, unidirectional
//! - WebSocket: Full-duplex, persistent connection

/// Unified transport handler supporting all MCP transport types.
pub struct TransportHandler {
    /// Active STDIO server processes
    stdio_processes: Arc<DashMap<ServerId, StdioProcess>>,
    /// HTTP client with connection pooling
    http_client: reqwest::Client,
    /// WebSocket connections
    ws_connections: Arc<DashMap<ServerId, WsConnection>>,
    /// Transport metrics
    metrics: Arc<TransportMetrics>,
}

impl TransportHandler {
    /// Send a request to a backend server using the appropriate transport.
    #[instrument(skip(self, request))]
    pub async fn send_request(
        &self,
        server: &ServerConfig,
        request: McpRequest,
    ) -> Result<McpResponse, TransportError> {
        match &server.transport {
            Transport::Stdio(config) => {
                self.send_stdio_request(server.id.clone(), config, request).await
            },
            Transport::Http(config) => {
                self.send_http_request(config, request).await
            },
            Transport::Sse(config) => {
                self.send_sse_request(config, request).await
            },
            Transport::WebSocket(config) => {
                self.send_ws_request(server.id.clone(), config, request).await
            },
        }
    }
    
    /// STDIO transport implementation with process management.
    async fn send_stdio_request(
        &self,
        server_id: ServerId,
        config: &StdioConfig,
        request: McpRequest,
    ) -> Result<McpResponse, TransportError> {
        // Get or create STDIO process
        let process = self.get_or_create_stdio_process(
            server_id.clone(),
            config
        ).await?;
        
        // Send request through stdin
        let request_bytes = serde_json::to_vec(&request)?;
        process.send(request_bytes).await?;
        
        // Read response from stdout with timeout
        let response_bytes = tokio::time::timeout(
            Duration::from_millis(config.timeout_ms),
            process.receive()
        ).await
            .map_err(|_| TransportError::Timeout)?
            ?;
        
        // Parse response
        let response: McpResponse = serde_json::from_slice(&response_bytes)?;
        
        self.metrics.stdio_requests.inc();
        Ok(response)
    }
    
    /// Get existing or spawn new STDIO process.
    async fn get_or_create_stdio_process(
        &self,
        server_id: ServerId,
        config: &StdioConfig,
    ) -> Result<Arc<StdioProcess>, TransportError> {
        // Check if process exists and is healthy
        if let Some(process) = self.stdio_processes.get(&server_id) {
            if process.is_healthy().await {
                return Ok(process.clone());
            }
            // Process unhealthy, remove it
            self.stdio_processes.remove(&server_id);
        }
        
        // Spawn new process with security restrictions
        let mut command = tokio::process::Command::new(&config.command);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        
        // Apply environment variables
        for (key, value) in &config.env {
            command.env(key, value);
        }
        
        // Security: Restrict process capabilities (Linux)
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::process::CommandExt;
            command.uid(1000);  // Run as non-root user
            command.gid(1000);
            
            // Set resource limits
            command.pre_exec(|| {
                // Limit CPU time (30 seconds)
                let rlimit = libc::rlimit {
                    rlim_cur: 30,
                    rlim_max: 30,
                };
                unsafe {
                    libc::setrlimit(libc::RLIMIT_CPU, &rlimit);
                }
                
                // Limit memory (2GB)
                let rlimit = libc::rlimit {
                    rlim_cur: 2 * 1024 * 1024 * 1024,
                    rlim_max: 2 * 1024 * 1024 * 1024,
                };
                unsafe {
                    libc::setrlimit(libc::RLIMIT_AS, &rlimit);
                }
                
                Ok(())
            });
        }
        
        let mut child = command.spawn()
            .map_err(|e| TransportError::ProcessSpawnFailed(e))?;
        
        let stdin = child.stdin.take()
            .ok_or(TransportError::NoStdin)?;
        let stdout = child.stdout.take()
            .ok_or(TransportError::NoStdout)?;
        let stderr = child.stderr.take()
            .ok_or(TransportError::NoStderr)?;
        
        let process = Arc::new(StdioProcess::new(
            child,
            stdin,
            stdout,
            stderr,
        ));
        
        // Store in process map
        self.stdio_processes.insert(server_id, process.clone());
        
        info!("Spawned STDIO process: {}", config.command);
        Ok(process)
    }
}

/// STDIO process wrapper with bidirectional communication.
pub struct StdioProcess {
    /// Child process handle
    child: Arc<Mutex<Child>>,
    /// Stdin writer (to server)
    stdin: Arc<Mutex<ChildStdin>>,
    /// Stdout reader (from server)
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    /// Stderr reader (for diagnostics)
    stderr: Arc<Mutex<BufReader<ChildStderr>>>,
    /// Process health status
    healthy: Arc<AtomicBool>,
}

impl StdioProcess {
    /// Send a message to the STDIO server.
    pub async fn send(&self, data: Vec<u8>) -> Result<(), TransportError> {
        use tokio::io::AsyncWriteExt;
        
        let mut stdin = self.stdin.lock().await;
        
        // Write length-prefixed message (4 bytes length + data)
        let len = data.len() as u32;
        stdin.write_u32(len).await?;
        stdin.write_all(&data).await?;
        stdin.flush().await?;
        
        Ok(())
    }
    
    /// Receive a message from the STDIO server.
    pub async fn receive(&self) -> Result<Vec<u8>, TransportError> {
        use tokio::io::AsyncReadExt;
        
        let mut stdout = self.stdout.lock().await;
        
        // Read length prefix
        let len = stdout.read_u32().await?;
        
        // Read message data
        let mut buffer = vec![0u8; len as usize];
        stdout.read_exact(&mut buffer).await?;
        
        Ok(buffer)
    }
    
    /// Check if the process is still running and responsive.
    pub async fn is_healthy(&self) -> bool {
        let mut child = self.child.lock().await;
        
        // Check if process is still running
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process has exited
                error!("STDIO process exited with status: {:?}", status);
                self.healthy.store(false, Ordering::Relaxed);
                false
            },
            Ok(None) => {
                // Process still running
                self.healthy.load(Ordering::Relaxed)
            },
            Err(e) => {
                error!("Failed to check process status: {}", e);
                false
            }
        }
    }
}
```

---

## STREAMING IMPLEMENTATION

### Advanced Streaming Protocols

```rust
//! Streaming implementation for Server-Sent Events (SSE) and chunked responses.
//! 
//! Supports:
//! - Backpressure handling to prevent memory exhaustion
//! - Automatic reconnection for SSE streams
//! - Chunked transfer encoding for large responses
//! - WebSocket message framing

use futures::{Stream, StreamExt, SinkExt};
use tokio::sync::mpsc;
use bytes::{Bytes, BytesMut, BufMut};

/// SSE stream handler for legacy MCP servers.
pub struct SseHandler {
    /// Event source client
    client: eventsource_client::Client,
    /// Response buffer
    buffer: BytesMut,
    /// Reconnection configuration
    reconnect_config: ReconnectConfig,
    /// Stream metrics
    metrics: Arc<StreamMetrics>,
}

impl SseHandler {
    /// Establish SSE connection and stream responses.
    pub async fn stream_response(
        &mut self,
        url: &str,
        request: McpRequest,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error> {
        // Send initial request via POST
        let response = self.client
            .post(url)
            .json(&request)
            .send()
            .await?;
        
        // Create SSE stream from response
        let event_stream = eventsource_client::stream(response);
        
        // Transform SSE events into response chunks
        let stream = event_stream
            .filter_map(|event| async move {
                match event {
                    Ok(Event::Message(msg)) => {
                        // Parse JSON-RPC response from event data
                        match serde_json::from_str::<McpResponse>(&msg.data) {
                            Ok(response) => {
                                let bytes = serde_json::to_vec(&response).ok()?;
                                Some(Ok(Bytes::from(bytes)))
                            },
                            Err(e) => Some(Err(Error::ParseError(e))),
                        }
                    },
                    Ok(Event::Error(e)) => {
                        Some(Err(Error::SseError(e)))
                    },
                    _ => None,  // Ignore other event types
                }
            })
            .take_while(|result| {
                // Stop on terminal error
                futures::future::ready(!matches!(result, Err(Error::Terminal(_))))
            });
        
        Ok(Box::pin(stream))
    }
}

/// WebSocket streaming handler with full-duplex communication.
pub struct WebSocketHandler {
    /// Active WebSocket connections
    connections: Arc<DashMap<ServerId, WsConnection>>,
    /// Pending response channels
    pending_responses: Arc<DashMap<RequestId, oneshot::Sender<McpResponse>>>,
    /// Connection pool configuration
    pool_config: WsPoolConfig,
}

impl WebSocketHandler {
    /// Send request over WebSocket and stream response.
    pub async fn send_streaming(
        &self,
        server_id: ServerId,
        url: &str,
        request: McpRequest,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error> {
        // Get or establish WebSocket connection
        let mut conn = self.get_or_connect(server_id, url).await?;
        
        // Create response channel
        let (tx, rx) = mpsc::channel::<Result<Bytes, Error>>(32);
        
        // Send request
        let request_id = request.id.clone();
        let message = tungstenite::Message::Text(
            serde_json::to_string(&request)?
        );
        
        conn.send(message).await?;
        
        // Spawn response handler
        let pending = self.pending_responses.clone();
        tokio::spawn(async move {
            while let Some(msg) = conn.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Parse response
                        if let Ok(response) = serde_json::from_str::<McpResponse>(&text) {
                            if response.id == request_id {
                                // Check if this is a streaming response
                                if response.is_streaming() {
                                    // Send chunk through channel
                                    let bytes = Bytes::from(text.into_bytes());
                                    if tx.send(Ok(bytes)).await.is_err() {
                                        break;  // Receiver dropped
                                    }
                                } else {
                                    // Final response
                                    let bytes = Bytes::from(text.into_bytes());
                                    let _ = tx.send(Ok(bytes)).await;
                                    break;
                                }
                            }
                        }
                    },
                    Ok(Message::Binary(data)) => {
                        // Binary frame
                        if tx.send(Ok(Bytes::from(data))).await.is_err() {
                            break;
                        }
                    },
                    Ok(Message::Close(_)) => {
                        // Connection closed
                        let _ = tx.send(Err(Error::ConnectionClosed)).await;
                        break;
                    },
                    Err(e) => {
                        // WebSocket error
                        let _ = tx.send(Err(Error::WebSocketError(e))).await;
                        break;
                    },
                    _ => {},  // Ignore ping/pong
                }
            }
        });
        
        // Return stream
        Ok(tokio_stream::wrappers::ReceiverStream::new(rx))
    }
}

/// Chunked HTTP response streaming.
pub struct ChunkedStreamHandler {
    /// HTTP client with streaming support
    client: reqwest::Client,
    /// Chunk size for transfer encoding
    chunk_size: usize,
    /// Decompression support
    decompress: bool,
}

impl ChunkedStreamHandler {
    /// Stream response using chunked transfer encoding.
    pub async fn stream_chunked(
        &self,
        url: &str,
        request: McpRequest,
    ) -> Result<impl Stream<Item = Result<Bytes, Error>>, Error> {
        // Send request with streaming response
        let response = self.client
            .post(url)
            .json(&request)
            .send()
            .await?;
        
        // Check for chunked encoding
        let is_chunked = response
            .headers()
            .get(header::TRANSFER_ENCODING)
            .and_then(|v| v.to_str().ok())
            .map(|v| v.contains("chunked"))
            .unwrap_or(false);
        
        if !is_chunked {
            // Not chunked, return single response
            let bytes = response.bytes().await?;
            return Ok(futures::stream::once(
                futures::future::ready(Ok(bytes))
            ).boxed());
        }
        
        // Stream chunks with backpressure control
        let stream = response
            .bytes_stream()
            .map(|result| {
                result.map_err(|e| Error::StreamError(e.to_string()))
            })
            // Apply backpressure if consumer is slow
            .ready_chunks(10)  // Buffer up to 10 chunks
            .map(|chunks| {
                // Combine chunks for efficiency
                let mut combined = BytesMut::new();
                for chunk in chunks {
                    match chunk {
                        Ok(bytes) => combined.put(bytes),
                        Err(e) => return Err(e),
                    }
                }
                Ok(combined.freeze())
            });
        
        Ok(Box::pin(stream))
    }
}

/// Backpressure-aware stream processor.
pub struct BackpressureHandler {
    /// Maximum buffer size before applying backpressure
    max_buffer_size: usize,
    /// Current buffer usage
    buffer_usage: Arc<AtomicUsize>,
    /// Pause threshold (percentage)
    pause_threshold: f32,
    /// Resume threshold (percentage)
    resume_threshold: f32,
}

impl BackpressureHandler {
    /// Process stream with automatic backpressure control.
    pub fn apply_backpressure<S>(
        &self,
        stream: S,
    ) -> impl Stream<Item = S::Item>
    where
        S: Stream + Unpin,
    {
        let buffer_usage = self.buffer_usage.clone();
        let max_size = self.max_buffer_size;
        let pause_threshold = self.pause_threshold;
        let resume_threshold = self.resume_threshold;
        
        stream
            .then(move |item| {
                let usage = buffer_usage.clone();
                async move {
                    let current = usage.load(Ordering::Relaxed);
                    let percentage = current as f32 / max_size as f32;
                    
                    if percentage > pause_threshold {
                        // Apply backpressure - slow down
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    } else if percentage < resume_threshold {
                        // Resume normal speed
                        // No delay needed
                    }
                    
                    item
                }
            })
    }
}
```

---

## CONNECTION POOL MANAGEMENT

### High-Performance Connection Pooling

```rust
//! Connection pool management for efficient backend communication.
//! 
//! Features:
//! - Per-backend connection limits
//! - Health-aware pool sizing
//! - Automatic connection recycling
//! - Idle connection pruning

use bb8::{Pool, PooledConnection, ManageConnection};
use std::sync::atomic::{AtomicU64, Ordering};

/// Connection pool manager for all backend servers.
pub struct ConnectionPools {
    /// HTTP connection pools (one per backend)
    http_pools: Arc<DashMap<ServerId, Pool<HttpConnectionManager>>>,
    /// WebSocket connection pools
    ws_pools: Arc<DashMap<ServerId, Pool<WsConnectionManager>>>,
    /// Pool configuration
    config: PoolConfig,
    /// Connection statistics
    stats: Arc<PoolStats>,
}

impl ConnectionPools {
    /// Get or create a connection pool for the given server.
    pub async fn get_pool<M>(
        &self,
        server_id: &ServerId,
        manager: M,
    ) -> Result<Pool<M>, Error>
    where
        M: ManageConnection + Send + Sync + 'static,
    {
        // Build pool with configuration
        let pool = Pool::builder()
            .max_size(self.config.max_per_backend)
            .min_idle(Some(self.config.min_idle))
            .max_lifetime(Some(Duration::from_millis(
                self.config.max_lifetime_ms
            )))
            .idle_timeout(Some(Duration::from_millis(
                self.config.max_idle_time_ms
            )))
            .connection_timeout(Duration::from_millis(
                self.config.connection_timeout_ms
            ))
            .test_on_check_out(true)
            .build(manager)
            .await?;
        
        Ok(pool)
    }
    
    /// Prune idle connections across all pools.
    pub async fn prune_idle_connections(&self) {
        // Prune HTTP pools
        for mut entry in self.http_pools.iter_mut() {
            let pool = entry.value_mut();
            let state = pool.state();
            
            if state.idle_connections > self.config.min_idle {
                // Too many idle connections
                debug!(
                    "Pruning idle connections for {}: {} idle",
                    entry.key(),
                    state.idle_connections
                );
                
                // bb8 automatically handles pruning on next check
            }
        }
        
        // Similar for WebSocket pools
        for mut entry in self.ws_pools.iter_mut() {
            let pool = entry.value_mut();
            let state = pool.state();
            
            if state.idle_connections > self.config.min_idle {
                debug!(
                    "Pruning idle WS connections for {}: {} idle",
                    entry.key(),
                    state.idle_connections
                );
            }
        }
    }
    
    /// Get connection statistics for monitoring.
    pub fn get_stats(&self) -> PoolStats {
        let mut stats = PoolStats::default();
        
        // Aggregate HTTP pool stats
        for entry in self.http_pools.iter() {
            let state = entry.value().state();
            stats.total_connections += state.connections;
            stats.idle_connections += state.idle_connections;
            stats.pending_connections += state.pending_connections;
        }
        
        // Aggregate WebSocket pool stats
        for entry in self.ws_pools.iter() {
            let state = entry.value().state();
            stats.total_connections += state.connections;
            stats.idle_connections += state.idle_connections;
            stats.pending_connections += state.pending_connections;
        }
        
        stats
    }
}

/// HTTP connection manager for bb8 pool.
pub struct HttpConnectionManager {
    /// Base URL for the backend
    base_url: String,
    /// HTTP client for connections
    client: reqwest::Client,
    /// Connection timeout
    timeout: Duration,
}

#[async_trait]
impl ManageConnection for HttpConnectionManager {
    type Connection = HttpConnection;
    type Error = Error;
    
    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        // Test connection with health check
        let health_url = format!("{}/health", self.base_url);
        
        let response = self.client
            .get(&health_url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::ConnectionFailed(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(Error::HealthCheckFailed(response.status()));
        }
        
        Ok(HttpConnection {
            base_url: self.base_url.clone(),
            client: self.client.clone(),
            created_at: Instant::now(),
            request_count: Arc::new(AtomicU64::new(0)),
        })
    }
    
    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Check if connection is still valid
        if conn.created_at.elapsed() > Duration::from_secs(300) {
            // Connection too old
            return Err(Error::ConnectionExpired);
        }
        
        if conn.request_count.load(Ordering::Relaxed) > 1000 {
            // Too many requests on this connection
            return Err(Error::ConnectionExhausted);
        }
        
        // Perform quick health check
        let response = conn.client
            .head(&format!("{}/health", conn.base_url))
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .map_err(|_| Error::HealthCheckFailed(StatusCode::REQUEST_TIMEOUT))?;
        
        if response.status().is_success() {
            Ok(())
        } else {
            Err(Error::HealthCheckFailed(response.status()))
        }
    }
    
    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        // Check if connection is broken
        conn.request_count.load(Ordering::Relaxed) > 10000 ||
        conn.created_at.elapsed() > Duration::from_secs(600)
    }
}

/// Pooled HTTP connection wrapper.
pub struct HttpConnection {
    /// Base URL for requests
    base_url: String,
    /// Reusable HTTP client
    client: reqwest::Client,
    /// Connection creation time
    created_at: Instant,
    /// Number of requests sent
    request_count: Arc<AtomicU64>,
}

impl HttpConnection {
    /// Send a request using this connection.
    pub async fn send(&self, request: McpRequest) -> Result<McpResponse, Error> {
        self.request_count.fetch_add(1, Ordering::Relaxed);
        
        let response = self.client
            .post(&format!("{}/mcp", self.base_url))
            .json(&request)
            .send()
            .await?;
        
        let mcp_response: McpResponse = response.json().await?;
        Ok(mcp_response)
    }
}
```

---

## ERROR HANDLING & RESILIENCE

### Comprehensive Error Handling

```rust
//! Error handling and resilience patterns for the proxy.
//! 
//! Implements:
//! - Retry logic with exponential backoff
//! - Circuit breakers for failing backends
//! - Graceful degradation
//! - Error categorization and recovery

use thiserror::Error;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Main error type for the proxy system.
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("No backend available for tool: {0}")]
    NoBackendAvailable(String),
    
    #[error("All backends unhealthy for tool: {0}")]
    AllBackendsUnhealthy(String),
    
    #[error("Backend timeout after {0}ms")]
    BackendTimeout(u64),
    
    #[error("Circuit breaker open for server: {0}")]
    CircuitBreakerOpen(String),
    
    #[error("Rate limit exceeded: {0} req/s")]
    RateLimitExceeded(u32),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Transport error: {0}")]
    TransportError(#[from] TransportError),
    
    #[error("Cache error: {0}")]
    CacheError(#[from] CacheError),
    
    #[error("Configuration error: {0}")]
    ConfigError(#[from] ConfigError),
    
    #[error("Internal server error: {0}")]
    InternalError(String),
}

impl ProxyError {
    /// Determine if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            ProxyError::BackendTimeout(_) |
            ProxyError::TransportError(_) |
            ProxyError::InternalError(_)
        )
    }
    
    /// Get the appropriate HTTP status code for this error.
    pub fn status_code(&self) -> StatusCode {
        match self {
            ProxyError::NoBackendAvailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ProxyError::AllBackendsUnhealthy(_) => StatusCode::SERVICE_UNAVAILABLE,
            ProxyError::BackendTimeout(_) => StatusCode::GATEWAY_TIMEOUT,
            ProxyError::CircuitBreakerOpen(_) => StatusCode::SERVICE_UNAVAILABLE,
            ProxyError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            ProxyError::AuthenticationFailed(_) => StatusCode::UNAUTHORIZED,
            ProxyError::ConfigError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_GATEWAY,
        }
    }
    
    /// Convert to MCP error response.
    pub fn to_mcp_error(&self, id: Option<Value>) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "data": self.error_data()
            }
        })
    }
    
    fn error_code(&self) -> i32 {
        match self {
            ProxyError::BackendTimeout(_) => -32001,
            ProxyError::NoBackendAvailable(_) => -32002,
            ProxyError::AuthenticationFailed(_) => -32003,
            ProxyError::RateLimitExceeded(_) => -32004,
            _ => -32000,  // Generic server error
        }
    }
    
    fn error_data(&self) -> Option<Value> {
        match self {
            ProxyError::BackendTimeout(ms) => Some(json!({
                "timeout_ms": ms
            })),
            ProxyError::RateLimitExceeded(rate) => Some(json!({
                "rate_limit": rate,
                "retry_after_seconds": 60
            })),
            _ => None,
        }
    }
}

/// Circuit breaker for backend failure detection.
pub struct CircuitBreaker {
    /// Current state
    state: Arc<RwLock<CircuitState>>,
    /// Failure count
    failure_count: Arc<AtomicU32>,
    /// Success count
    success_count: Arc<AtomicU32>,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Last state change time
    last_change: Arc<RwLock<Instant>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    Closed,     // Normal operation
    Open,       // Failing, rejecting requests
    HalfOpen,   // Testing if recovered
}

impl CircuitBreaker {
    /// Create a new circuit breaker with configuration.
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            config,
            last_change: Arc::new(RwLock::new(Instant::now())),
        }
    }
    
    /// Check if circuit allows request.
    pub async fn allow_request(&self) -> bool {
        let state = self.state.read().await;
        
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has elapsed
                let last_change = self.last_change.read().await;
                if last_change.elapsed() > self.config.timeout {
                    // Transition to half-open
                    drop(state);
                    let mut state = self.state.write().await;
                    *state = CircuitState::HalfOpen;
                    self.success_count.store(0, Ordering::Relaxed);
                    info!("Circuit breaker transitioning to half-open");
                    true
                } else {
                    false
                }
            },
            CircuitState::HalfOpen => {
                // Allow limited requests for testing
                self.success_count.load(Ordering::Relaxed) < 
                    self.config.half_open_requests
            }
        }
    }
    
    /// Record a successful request.
    pub async fn record_success(&self) {
        let state = self.state.read().await;
        
        match *state {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if count >= self.config.success_threshold {
                    // Transition to closed
                    drop(state);
                    let mut state = self.state.write().await;
                    *state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    let mut last_change = self.last_change.write().await;
                    *last_change = Instant::now();
                    info!("Circuit breaker closed (recovered)");
                }
            },
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            },
            _ => {}
        }
    }
    
    /// Record a failed request.
    pub async fn record_failure(&self) {
        let state = self.state.read().await;
        
        match *state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if count >= self.config.failure_threshold {
                    // Transition to open
                    drop(state);
                    let mut state = self.state.write().await;
                    *state = CircuitState::Open;
                    let mut last_change = self.last_change.write().await;
                    *last_change = Instant::now();
                    error!("Circuit breaker opened after {} failures", count);
                }
            },
            CircuitState::HalfOpen => {
                // Single failure in half-open returns to open
                drop(state);
                let mut state = self.state.write().await;
                *state = CircuitState::Open;
                self.failure_count.store(0, Ordering::Relaxed);
                let mut last_change = self.last_change.write().await;
                *last_change = Instant::now();
                warn!("Circuit breaker reopened after failure in half-open");
            },
            _ => {}
        }
    }
    
    /// Check if circuit is currently open.
    pub async fn is_open(&self) -> bool {
        let state = self.state.read().await;
        *state == CircuitState::Open
    }
}

/// Retry middleware with exponential backoff.
pub struct RetryHandler {
    /// Maximum retry attempts
    max_attempts: u32,
    /// Initial delay between retries
    initial_delay: Duration,
    /// Maximum delay between retries
    max_delay: Duration,
    /// Backoff multiplier
    multiplier: f32,
    /// Add jitter to prevent thundering herd
    jitter: bool,
}

impl RetryHandler {
    /// Execute a request with retry logic.
    pub async fn execute_with_retry<F, T, E>(
        &self,
        mut f: F,
    ) -> Result<T, E>
    where
        F: FnMut() -> BoxFuture<'static, Result<T, E>>,
        E: std::fmt::Debug + RetryableError,
    {
        let mut attempt = 0;
        let mut delay = self.initial_delay;
        
        loop {
            attempt += 1;
            
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) if !e.is_retryable() => {
                    // Non-retryable error, fail immediately
                    return Err(e);
                },
                Err(e) if attempt >= self.max_attempts => {
                    // Max attempts reached
                    warn!("Max retry attempts ({}) reached: {:?}", self.max_attempts, e);
                    return Err(e);
                },
                Err(e) => {
                    // Retryable error, wait and retry
                    debug!("Attempt {} failed: {:?}, retrying in {:?}", attempt, e, delay);
                    
                    // Apply jitter if configured
                    let actual_delay = if self.jitter {
                        let jitter = rand::random::<f32>() * 0.3;  // ±30% jitter
                        delay.mul_f32(1.0 + jitter - 0.15)
                    } else {
                        delay
                    };
                    
                    tokio::time::sleep(actual_delay).await;
                    
                    // Calculate next delay with exponential backoff
                    delay = (delay.mul_f32(self.multiplier)).min(self.max_delay);
                }
            }
        }
    }
}

/// Trait for retryable errors.
pub trait RetryableError {
    fn is_retryable(&self) -> bool;
}

impl RetryableError for ProxyError {
    fn is_retryable(&self) -> bool {
        self.is_retryable()
    }
}
```

---

## APPENDIX A: Complete Handler Functions

```rust
//! Request handler implementations for all MCP endpoints.

/// Handle generic JSON-RPC requests.
#[instrument(skip(state, payload))]
async fn handle_jsonrpc_request(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, ProxyError> {
    // Parse request
    let request: McpRequest = serde_json::from_value(payload)
        .map_err(|e| ProxyError::InvalidRequest(e.to_string()))?;
    
    // Route to appropriate handler based on method
    let response = match request.method.as_str() {
        "tools/list" => handle_tools_list_impl(state, request).await?,
        "tools/call" => handle_tools_call_impl(state, request).await?,
        "resources/list" => handle_resources_list_impl(state, request).await?,
        "resources/read" => handle_resources_read_impl(state, request).await?,
        "prompts/list" => handle_prompts_list_impl(state, request).await?,
        "prompts/get" => handle_prompts_get_impl(state, request).await?,
        _ => {
            // Unknown method, try to route to a backend
            route_generic_request(state, request).await?
        }
    };
    
    Ok(Json(response))
}

/// Handle tools/list request with aggregation.
async fn handle_tools_list_impl(
    state: AppState,
    request: McpRequest,
) -> Result<Value, ProxyError> {
    let start = Instant::now();
    
    // Check cache
    let cache_key = format!("tools:list:{}", state.config.version);
    if let Some(cached) = state.cache.get(&cache_key).await {
        state.metrics.cache_hits.inc();
        return Ok(cached);
    }
    
    // Get all healthy servers
    let registry = state.registry.read().await;
    let servers = registry.get_healthy_servers();
    
    // Parallel fetch from all servers
    let mut tasks = Vec::new();
    for server in servers {
        let state = state.clone();
        let request = request.clone();
        
        tasks.push(tokio::spawn(async move {
            fetch_tools_from_server(state, server, request).await
        }));
    }
    
    // Wait for all responses
    let results = futures::future::join_all(tasks).await;
    
    // Aggregate tools
    let mut all_tools = Vec::new();
    for result in results {
        match result {
            Ok(Ok(tools)) => all_tools.extend(tools),
            Ok(Err(e)) => warn!("Failed to fetch tools: {}", e),
            Err(e) => error!("Task panic: {}", e),
        }
    }
    
    // Deduplicate tools by name
    all_tools.sort_by(|a, b| a.name.cmp(&b.name));
    all_tools.dedup_by(|a, b| a.name == b.name);
    
    // Build response
    let response = json!({
        "jsonrpc": "2.0",
        "id": request.id,
        "result": {
            "tools": all_tools
        }
    });
    
    // Cache response
    state.cache.insert(
        cache_key,
        response.clone(),
        Duration::from_secs(300),  // 5 minute TTL
    ).await;
    
    state.metrics.tools_list_duration.record(start.elapsed());
    Ok(response)
}

/// Handle tools/call with routing and retries.
async fn handle_tools_call_impl(
    state: AppState,
    request: McpRequest,
) -> Result<Value, ProxyError> {
    let start = Instant::now();
    
    // Extract tool name
    let tool_name = request.params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ProxyError::InvalidRequest("Missing tool name".into()))?;
    
    // Route request
    let router = RequestRouter::from_state(&state);
    let (server_id, _) = router.route_request(&request, &*state.registry.read().await, &state.cache).await?;
    
    // Get server configuration
    let registry = state.registry.read().await;
    let server = registry.get_server(&server_id)
        .ok_or_else(|| ProxyError::NoBackendAvailable(tool_name.to_string()))?;
    
    // Execute with retry
    let retry_handler = RetryHandler::default();
    let response = retry_handler.execute_with_retry(|| {
        Box::pin(send_request_to_backend(
            state.clone(),
            server.clone(),
            request.clone(),
        ))
    }).await?;
    
    state.metrics.tools_call_duration.record(start.elapsed());
    Ok(response)
}
```

---

*End of Document - Only1MCP Core Proxy Implementation Guide v1.0*
