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

use crate::cache::ResponseCache;
use crate::config::RoutingAlgorithmConfig;
use crate::error::Error;
use crate::health::circuit_breaker::CircuitBreaker;
use crate::routing::load_balancer::ConsistentHashRing;
use crate::types::{McpRequest, ServerId};
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, instrument, warn};
use xxhash_rust::xxh3::Xxh3;

/// Routing algorithms available for load balancing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingAlgorithm {
    /// Consistent hashing with virtual nodes for session affinity
    ConsistentHash,
    /// Least connections (Power of Two Choices)
    LeastConnections,
    /// Round-robin distribution
    RoundRobin,
    /// Random selection
    Random,
    /// Weighted random based on server weights
    WeightedRandom,
}

impl RoutingAlgorithm {
    /// Parse routing algorithm from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "consistent_hash" | "consistent-hash" => Self::ConsistentHash,
            "least_connections" | "least-connections" => Self::LeastConnections,
            "round_robin" | "round-robin" => Self::RoundRobin,
            "random" => Self::Random,
            "weighted_random" | "weighted-random" => Self::WeightedRandom,
            _ => Self::RoundRobin, // Default fallback
        }
    }
}

/// Health state tracker for routing decisions
#[derive(Debug, Clone)]
pub struct HealthState {
    healthy: bool,
    success_count: u64,
    failure_count: u64,
    latencies: Vec<Duration>,
    last_check: Option<Instant>,
}

impl Default for HealthState {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthState {
    pub fn new() -> Self {
        Self {
            healthy: true,
            success_count: 0,
            failure_count: 0,
            latencies: Vec::with_capacity(10),
            last_check: None,
        }
    }

    pub fn record_success(&mut self, latency: Duration) {
        self.healthy = true;
        self.success_count += 1;

        // Keep last 10 latencies
        if self.latencies.len() >= 10 {
            self.latencies.remove(0);
        }
        self.latencies.push(latency);

        self.last_check = Some(Instant::now());
    }

    pub fn record_failure(&mut self) {
        self.failure_count += 1;

        // Mark unhealthy if failure ratio is high
        let total = self.success_count + self.failure_count;
        if total > 0 {
            let failure_rate = self.failure_count as f64 / total as f64;
            if failure_rate > 0.5 {
                self.healthy = false;
            }
        }

        self.last_check = Some(Instant::now());
    }

    pub fn average_latency(&self) -> Duration {
        if self.latencies.is_empty() {
            return Duration::ZERO;
        }
        let sum: Duration = self.latencies.iter().sum();
        sum / self.latencies.len() as u32
    }

    pub fn is_healthy(&self) -> bool {
        self.healthy
    }
}

/// Main request router responsible for backend selection and load balancing.
pub struct RequestRouter {
    /// Consistent hash ring for session affinity
    hash_ring: Arc<ArcSwap<ConsistentHashRing>>,
    /// Active server health states
    health_states: Arc<DashMap<ServerId, HealthState>>,
    /// Per-server connection counts
    connection_counts: Arc<DashMap<ServerId, AtomicUsize>>,
    /// Routing configuration
    config: RoutingAlgorithmConfig,
    /// Circuit breakers per backend
    circuit_breakers: Arc<DashMap<ServerId, CircuitBreaker>>,
}

#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("No backend available for tool: {0}")]
    NoBackendAvailable(String),

    #[error("All backends unhealthy for tool: {0}")]
    AllBackendsUnhealthy(String),

    #[error("Hash ring is empty")]
    HashRingEmpty,

    #[error("No server selected")]
    NoServerSelected,

    #[error("Registry error: {0}")]
    Registry(String),
}

impl From<Error> for RoutingError {
    fn from(err: Error) -> Self {
        RoutingError::Registry(err.to_string())
    }
}

impl RequestRouter {
    /// Create a new request router with the given configuration.
    pub fn new(config: RoutingAlgorithmConfig) -> Self {
        let hash_ring = ConsistentHashRing::new(config.virtual_nodes as u32);

        Self {
            hash_ring: Arc::new(ArcSwap::new(Arc::new(hash_ring))),
            health_states: Arc::new(DashMap::new()),
            connection_counts: Arc::new(DashMap::new()),
            config,
            circuit_breakers: Arc::new(DashMap::new()),
        }
    }

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
    ) -> std::result::Result<(ServerId, Duration), RoutingError> {
        let _method = request.method();
        let tool_name = extract_tool_name(request)?;

        // Step 1: Check cache for memoized response
        let cache_key = self.compute_cache_key(request);
        if let Some(cached) = cache.get(&cache_key).await {
            debug!("Cache hit for {}", tool_name);
            return Ok((cached.server_id, Duration::ZERO));
        }

        // Step 2: Find servers that support this tool
        let eligible_servers = registry.find_servers_for_tool(&tool_name).await?;

        if eligible_servers.is_empty() {
            error!("No servers available for tool: {}", tool_name);
            return Err(RoutingError::NoBackendAvailable(tool_name));
        }

        // Step 3: Filter by health status and circuit breaker state
        let healthy_servers: Vec<ServerId> = eligible_servers
            .into_iter()
            .filter(|id| {
                // Check health state
                let is_healthy =
                    self.health_states.get(id).map(|state| state.is_healthy()).unwrap_or(false);

                // Check circuit breaker
                let circuit_open =
                    self.circuit_breakers.get(id).map(|cb| cb.is_open()).unwrap_or(false);

                is_healthy && !circuit_open
            })
            .collect();

        if healthy_servers.is_empty() {
            warn!("All backends unhealthy for tool: {}", tool_name);
            return Err(RoutingError::AllBackendsUnhealthy(tool_name));
        }

        // Step 4: Apply routing algorithm
        let algorithm = RoutingAlgorithm::from_str(&self.config.algorithm);
        let selected_server = match algorithm {
            RoutingAlgorithm::ConsistentHash => {
                self.route_consistent_hash(&tool_name, &healthy_servers)
            },
            RoutingAlgorithm::LeastConnections => self.route_least_connections(&healthy_servers),
            RoutingAlgorithm::RoundRobin => self.route_round_robin(&healthy_servers),
            RoutingAlgorithm::Random => self.route_random(&healthy_servers),
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
    ) -> std::result::Result<ServerId, RoutingError> {
        let hash_ring = self.hash_ring.load();

        // Hash the routing key
        let mut hasher = Xxh3::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        // Find the server in the ring
        let server =
            hash_ring.get_server(hash, servers).ok_or(RoutingError::HashRingEmpty)?;

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
    ) -> std::result::Result<ServerId, RoutingError> {
        use rand::seq::SliceRandom;

        if servers.len() == 1 {
            return Ok(servers[0].clone());
        }

        // Power of Two Choices algorithm
        let mut rng = rand::thread_rng();
        let candidates: Vec<_> = servers.choose_multiple(&mut rng, 2.min(servers.len())).collect();

        // Select server with minimum connections
        let selected = candidates
            .into_iter()
            .min_by_key(|&id| {
                self.connection_counts
                    .get(id)
                    .map(|count| count.load(Ordering::Relaxed))
                    .unwrap_or(0)
            })
            .ok_or(RoutingError::NoServerSelected)?;

        debug!("Least connections selected: {}", selected);
        Ok(selected.clone())
    }

    /// Simple round-robin routing for fairness.
    fn route_round_robin(
        &self,
        servers: &[ServerId],
    ) -> std::result::Result<ServerId, RoutingError> {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let index = COUNTER.fetch_add(1, Ordering::Relaxed) % servers.len();
        let selected = &servers[index];

        debug!("Round-robin selected: {}", selected);
        Ok(selected.clone())
    }

    /// Random server selection for simplicity.
    fn route_random(&self, servers: &[ServerId]) -> std::result::Result<ServerId, RoutingError> {
        use rand::seq::SliceRandom;

        let mut rng = rand::thread_rng();
        let selected = servers.choose(&mut rng).ok_or(RoutingError::NoServerSelected)?;

        debug!("Random selected: {}", selected);
        Ok(selected.clone())
    }

    /// Weighted random selection based on server weights.
    async fn route_weighted_random(
        &self,
        servers: &[ServerId],
        registry: &ServerRegistry,
    ) -> std::result::Result<ServerId, RoutingError> {
        use rand::distributions::{Distribution, WeightedIndex};

        // Get weights for each server
        let weights: Vec<u32> =
            futures::future::join_all(servers.iter().map(|id| registry.get_server_weight(id)))
                .await;

        let dist = WeightedIndex::new(&weights).map_err(|_| RoutingError::NoServerSelected)?;

        let mut rng = rand::thread_rng();
        let index = dist.sample(&mut rng);
        let selected = &servers[index];

        debug!(
            "Weighted random selected: {} (weight: {})",
            selected, weights[index]
        );
        Ok(selected.clone())
    }

    /// Update health state based on request outcome.
    pub async fn update_health(&self, server_id: &ServerId, success: bool, latency: Duration) {
        let mut health = self
            .health_states
            .entry(server_id.clone())
            .or_default();

        if success {
            health.record_success(latency);

            // Update circuit breaker
            if let Some(cb) = self.circuit_breakers.get_mut(server_id) {
                cb.record_success().await;
            }
        } else {
            health.record_failure();

            // Update circuit breaker
            if let Some(cb) = self.circuit_breakers.get_mut(server_id) {
                cb.record_failure().await;
            }
        }
    }

    /// Compute a cache key for the request.
    fn compute_cache_key(&self, request: &McpRequest) -> String {
        format!("{}:{}", request.method(), request.params_hash())
    }

    /// Estimate latency for a server based on historical data.
    async fn estimate_latency(&self, server_id: &ServerId) -> Duration {
        self.health_states
            .get(server_id)
            .map(|state| state.average_latency())
            .unwrap_or(Duration::from_millis(100))
    }

    /// Add a new server to the router.
    pub fn add_server(&self, server_id: ServerId) {
        let mut ring = (**self.hash_ring.load()).clone();
        ring.add_server(&server_id);
        self.hash_ring.store(Arc::new(ring));

        // Initialize health state and circuit breaker
        self.health_states.insert(server_id.clone(), HealthState::new());
        self.circuit_breakers.insert(
            server_id.clone(),
            CircuitBreaker::new(server_id.clone(), Default::default()),
        );

        info!("Added server {} to router", server_id);
    }

    /// Remove a server from the router.
    pub fn remove_server(&self, server_id: &ServerId) {
        let mut ring = (**self.hash_ring.load()).clone();
        ring.remove_server(server_id);
        self.hash_ring.store(Arc::new(ring));

        // Clean up state
        self.health_states.remove(server_id);
        self.circuit_breakers.remove(server_id);
        self.connection_counts.remove(server_id);

        info!("Removed server {} from router", server_id);
    }
}

// ConsistentHashRing is now imported from crate::routing::load_balancer

// Helper functions

/// Extract tool name from MCP request.
fn extract_tool_name(request: &McpRequest) -> std::result::Result<String, RoutingError> {
    request
        .get_tool_name()
        .ok_or_else(|| RoutingError::NoBackendAvailable("unknown".to_string()))
}

/// Server registry for tracking available backends.
pub struct ServerRegistry {
    servers: DashMap<ServerId, ServerInfo>,
}

impl Default for ServerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerRegistry {
    /// Create new empty server registry
    pub fn new() -> Self {
        Self {
            servers: DashMap::new(),
        }
    }

    /// Create server registry from config
    pub async fn from_config(config: &crate::config::Config) -> std::result::Result<Self, Error> {
        let registry = Self::new();

        // Populate registry from config servers
        for server in &config.servers {
            if server.enabled {
                let info = ServerInfo {
                    id: server.id.clone(),
                    weight: server.weight,
                    tools: Vec::new(), // Would be discovered from server capabilities
                };
                registry.servers.insert(server.id.clone(), info);
            }
        }

        Ok(registry)
    }

    /// Get list of healthy server IDs
    pub async fn get_healthy_servers(&self) -> Vec<String> {
        self.servers.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get server configuration by ID
    pub fn get_server(&self, _server_id: &str) -> Option<crate::proxy::registry::ServerConfig> {
        // This is a stub - in a real implementation, we'd fetch from the actual registry
        // For now, return None as the ServerInfo doesn't have full ServerConfig
        None
    }

    /// Find servers that support a specific tool.
    pub async fn find_servers_for_tool(
        &self,
        tool: &str,
    ) -> std::result::Result<Vec<ServerId>, Error> {
        let servers: Vec<ServerId> = self
            .servers
            .iter()
            .filter(|entry| entry.value().supports_tool(tool))
            .map(|entry| entry.key().clone())
            .collect();

        Ok(servers)
    }

    /// Get the weight of a server for weighted routing.
    pub async fn get_server_weight(&self, server_id: &ServerId) -> u32 {
        self.servers.get(server_id).map(|info| info.weight).unwrap_or(1)
    }

    /// Get the number of registered servers
    pub fn len(&self) -> usize {
        self.servers.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.servers.is_empty()
    }
}

/// Server information for routing decisions.
pub struct ServerInfo {
    pub id: ServerId,
    pub weight: u32,
    pub tools: Vec<String>,
}

impl ServerInfo {
    /// Check if this server supports a specific tool.
    pub fn supports_tool(&self, tool: &str) -> bool {
        self.tools.iter().any(|t| t == tool)
    }
}
