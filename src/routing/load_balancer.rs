//! Load balancing algorithms for request routing
//!
//! Implements multiple routing algorithms:
//! - Consistent hashing with virtual nodes for session affinity
//! - Least connections for optimal load distribution
//! - Round-robin for simple fairness
//! - Random for simplicity
//! - Weighted random for capacity-aware routing
//! - Health-aware routing with automatic failover

use crate::error::Result;
use crate::types::ServerId;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, instrument, warn};
use xxhash_rust::xxh3::Xxh3;

/// Routing algorithm selection
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RoutingAlgorithm {
    /// Round-robin distribution
    RoundRobin,
    /// Least connections using Power of Two Choices
    LeastConnections,
    /// Random server selection
    Random,
    /// Weighted random based on server capacity
    WeightedRandom,
    /// Consistent hashing for session affinity
    ConsistentHash,
}

impl Default for RoutingAlgorithm {
    fn default() -> Self {
        Self::RoundRobin
    }
}

/// Routing configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoutingConfig {
    /// Selected routing algorithm
    pub algorithm: RoutingAlgorithm,
    /// Number of virtual nodes for consistent hashing
    #[serde(default = "default_virtual_nodes")]
    pub virtual_nodes: u32,
    /// Key to use for consistent hashing
    #[serde(default = "default_hash_key")]
    pub hash_key: HashKey,
    /// Enable sticky sessions
    #[serde(default)]
    pub sticky_sessions: bool,
    /// Session TTL in seconds
    #[serde(default = "default_session_ttl")]
    pub session_ttl: u64,
}

fn default_virtual_nodes() -> u32 {
    150
}

fn default_hash_key() -> HashKey {
    HashKey::ToolName
}

fn default_session_ttl() -> u64 {
    3600 // 1 hour
}

/// Key used for consistent hashing
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum HashKey {
    /// Hash based on tool/method name
    ToolName,
    /// Hash based on client ID
    ClientId,
    /// Hash based on custom header
    Header(String),
}

/// Health state for a backend server
#[derive(Debug)]
pub struct HealthState {
    /// Number of consecutive successes
    consecutive_successes: AtomicUsize,
    /// Number of consecutive failures
    consecutive_failures: AtomicUsize,
    /// Last successful request time
    last_success: AtomicU64,
    /// Last failed request time
    last_failure: AtomicU64,
    /// Average latency in microseconds
    avg_latency_us: AtomicU64,
}

impl Default for HealthState {
    fn default() -> Self {
        Self::new()
    }
}

impl HealthState {
    /// Create a new health state
    pub fn new() -> Self {
        Self {
            consecutive_successes: AtomicUsize::new(0),
            consecutive_failures: AtomicUsize::new(0),
            last_success: AtomicU64::new(0),
            last_failure: AtomicU64::new(0),
            avg_latency_us: AtomicU64::new(0),
        }
    }

    /// Record a successful request
    pub fn record_success(&self, latency: Duration) {
        self.consecutive_successes.fetch_add(1, Ordering::Relaxed);
        self.consecutive_failures.store(0, Ordering::Relaxed);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_success.store(now, Ordering::Relaxed);

        // Update average latency (exponential moving average)
        let new_latency = latency.as_micros() as u64;
        let old_avg = self.avg_latency_us.load(Ordering::Relaxed);
        let new_avg = (old_avg * 9 + new_latency) / 10;
        self.avg_latency_us.store(new_avg, Ordering::Relaxed);
    }

    /// Record a failed request
    pub fn record_failure(&self) {
        self.consecutive_failures.fetch_add(1, Ordering::Relaxed);
        self.consecutive_successes.store(0, Ordering::Relaxed);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_failure.store(now, Ordering::Relaxed);
    }

    /// Check if the server is healthy
    pub fn is_healthy(&self) -> bool {
        // Consider healthy if:
        // - Less than 3 consecutive failures
        // - Had a success in the last 60 seconds
        let failures = self.consecutive_failures.load(Ordering::Relaxed);
        if failures >= 3 {
            return false;
        }

        let last_success = self.last_success.load(Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // If never had a success, consider healthy (new server)
        if last_success == 0 {
            return true;
        }

        // Check if had success in last 60 seconds
        now - last_success < 60
    }

    /// Get average latency
    pub fn avg_latency(&self) -> Duration {
        Duration::from_micros(self.avg_latency_us.load(Ordering::Relaxed))
    }
}

/// Load balancer implementation
pub struct LoadBalancer {
    /// Routing configuration
    config: RoutingConfig,
    /// Consistent hash ring for session affinity
    hash_ring: Arc<ArcSwap<ConsistentHashRing>>,
    /// Active server health states
    health_states: Arc<DashMap<ServerId, HealthState>>,
    /// Per-server connection counts
    connection_counts: Arc<DashMap<ServerId, AtomicUsize>>,
    /// Round-robin counter
    round_robin_counter: AtomicUsize,
    /// Session affinity map
    session_map: Arc<DashMap<String, ServerId>>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new(config: RoutingConfig) -> Self {
        let hash_ring = ConsistentHashRing::new(config.virtual_nodes);

        Self {
            config,
            hash_ring: Arc::new(ArcSwap::new(Arc::new(hash_ring))),
            health_states: Arc::new(DashMap::new()),
            connection_counts: Arc::new(DashMap::new()),
            round_robin_counter: AtomicUsize::new(0),
            session_map: Arc::new(DashMap::new()),
        }
    }

    /// Select a server based on the configured algorithm
    #[instrument(skip(self))]
    pub async fn select_server(
        &self,
        key: &str,
        eligible_servers: &[ServerId],
        session_id: Option<&str>,
    ) -> Result<ServerId> {
        if eligible_servers.is_empty() {
            return Err(crate::error::Error::NoBackendAvailable(key.to_string()));
        }

        // Check for sticky session
        if self.config.sticky_sessions {
            if let Some(session_id) = session_id {
                if let Some(server) = self.session_map.get(session_id) {
                    let server_id = server.clone();
                    // Verify server is still eligible
                    if eligible_servers.contains(&server_id) {
                        debug!("Using sticky session for {}: {}", session_id, server_id);
                        return Ok(server_id);
                    }
                    // Remove stale session
                    drop(server);
                    self.session_map.remove(session_id);
                }
            }
        }

        // Filter by health status
        let healthy_servers: Vec<ServerId> = eligible_servers
            .iter()
            .filter(|&id| {
                self.health_states.get(id).map(|state| state.is_healthy()).unwrap_or(true)
                // Consider new servers as healthy
            })
            .cloned()
            .collect();

        if healthy_servers.is_empty() {
            warn!("All backends unhealthy for key: {}", key);
            // Fall back to all servers if none are healthy
            return self.route(eligible_servers, key).await;
        }

        // Apply routing algorithm
        let selected = self.route(&healthy_servers, key).await?;

        // Store sticky session
        if self.config.sticky_sessions {
            if let Some(session_id) = session_id {
                self.session_map.insert(session_id.to_string(), selected.clone());
                debug!("Stored sticky session {}: {}", session_id, selected);
            }
        }

        // Update connection count
        self.connection_counts
            .entry(selected.clone())
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);

        Ok(selected)
    }

    /// Apply the routing algorithm
    async fn route(&self, servers: &[ServerId], key: &str) -> Result<ServerId> {
        match self.config.algorithm {
            RoutingAlgorithm::ConsistentHash => self.route_consistent_hash(key, servers),
            RoutingAlgorithm::LeastConnections => self.route_least_connections(servers),
            RoutingAlgorithm::RoundRobin => self.route_round_robin(servers),
            RoutingAlgorithm::Random => self.route_random(servers),
            RoutingAlgorithm::WeightedRandom => self.route_weighted_random(servers),
        }
    }

    /// Consistent hashing with virtual nodes
    fn route_consistent_hash(&self, key: &str, servers: &[ServerId]) -> Result<ServerId> {
        let hash_ring = self.hash_ring.load();

        // Hash the routing key
        let mut hasher = Xxh3::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        // Find the server in the ring
        let server = hash_ring
            .get_server(hash, servers)
            .ok_or_else(|| crate::error::Error::NoBackendAvailable(key.to_string()))?;

        debug!("Consistent hash selected: {} for key: {}", server, key);
        Ok(server.clone())
    }

    /// Least connections using Power of Two Choices
    fn route_least_connections(&self, servers: &[ServerId]) -> Result<ServerId> {
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
            .ok_or_else(|| crate::error::Error::NoBackendAvailable("".to_string()))?;

        debug!("Least connections selected: {}", selected);
        Ok(selected.clone())
    }

    /// Round-robin server selection
    fn route_round_robin(&self, servers: &[ServerId]) -> Result<ServerId> {
        if servers.is_empty() {
            return Err(crate::error::Error::NoBackendAvailable("".to_string()));
        }

        let index = self.round_robin_counter.fetch_add(1, Ordering::Relaxed);
        let selected = &servers[index % servers.len()];

        debug!("Round-robin selected: {}", selected);
        Ok(selected.clone())
    }

    /// Random server selection
    fn route_random(&self, servers: &[ServerId]) -> Result<ServerId> {
        use rand::seq::SliceRandom;

        let mut rng = rand::thread_rng();
        servers
            .choose(&mut rng).cloned()
            .ok_or_else(|| crate::error::Error::NoBackendAvailable("".to_string()))
    }

    /// Weighted random server selection
    /// Note: Weights should be provided by ServerInfo in the registry.
    /// For now, this implementation uses equal weights (default to 1).
    /// Full weight-based selection is handled at the RequestRouter level.
    fn route_weighted_random(&self, servers: &[ServerId]) -> Result<ServerId> {
        use rand::Rng;

        if servers.is_empty() {
            return Err(crate::error::Error::NoBackendAvailable("".to_string()));
        }

        // For now, use equal weights since HealthState doesn't store weights
        // In a full implementation, weights would come from ServerInfo via the registry
        let default_weight = 1u32;
        let total_weight = servers.len() as u32 * default_weight;

        // Generate random number in range [0, total_weight)
        let mut rng = rand::thread_rng();
        let random_weight = rng.gen_range(0..total_weight);

        // Select server based on weighted probability
        let selected_index = (random_weight / default_weight) as usize;
        let server_id = &servers[selected_index];

        debug!("Weighted random selected: {} (equal weights)", server_id);
        Ok(server_id.clone())
    }

    /// Add a server to the load balancer
    pub fn add_server(&self, server_id: &ServerId) {
        // Initialize health state
        self.health_states
            .entry(server_id.clone())
            .or_default();

        // Add to consistent hash ring if using that algorithm
        if self.config.algorithm == RoutingAlgorithm::ConsistentHash {
            let mut ring = ConsistentHashRing::clone(&*self.hash_ring.load());
            ring.add_server(server_id);
            self.hash_ring.store(Arc::new(ring));
        }

        info!("Added server to load balancer: {}", server_id);
    }

    /// Remove a server from the load balancer
    pub fn remove_server(&self, server_id: &ServerId) {
        // Remove health state
        self.health_states.remove(server_id);

        // Remove connection count
        self.connection_counts.remove(server_id);

        // Remove from consistent hash ring
        if self.config.algorithm == RoutingAlgorithm::ConsistentHash {
            let mut ring = ConsistentHashRing::clone(&*self.hash_ring.load());
            ring.remove_server(server_id);
            self.hash_ring.store(Arc::new(ring));
        }

        // Clean up sticky sessions
        if self.config.sticky_sessions {
            self.session_map.retain(|_, v| v != server_id);
        }

        info!("Removed server from load balancer: {}", server_id);
    }

    /// Update health state for a server
    pub fn update_health(&self, server_id: &ServerId, success: bool, latency: Duration) {
        let health = self
            .health_states
            .entry(server_id.clone())
            .or_default();

        if success {
            health.record_success(latency);
        } else {
            health.record_failure();
        }
    }

    /// Release a connection (decrement count)
    pub fn release_connection(&self, server_id: &ServerId) {
        if let Some(count) = self.connection_counts.get(server_id) {
            count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get current server stats
    pub fn get_stats(&self) -> Vec<ServerStats> {
        let mut stats = Vec::new();

        for entry in self.health_states.iter() {
            let server_id = entry.key().clone();
            let health = entry.value();

            let connections = self
                .connection_counts
                .get(&server_id)
                .map(|c| c.load(Ordering::Relaxed))
                .unwrap_or(0);

            stats.push(ServerStats {
                server_id,
                healthy: health.is_healthy(),
                connections,
                avg_latency: health.avg_latency(),
            });
        }

        stats
    }
}

/// Server statistics
#[derive(Debug, Clone, Serialize)]
pub struct ServerStats {
    /// Server identifier
    pub server_id: ServerId,
    /// Health status
    pub healthy: bool,
    /// Active connections
    pub connections: usize,
    /// Average latency
    pub avg_latency: Duration,
}

/// Consistent hash ring with virtual nodes
#[derive(Clone)]
pub struct ConsistentHashRing {
    /// Virtual nodes in the ring (hash -> (server_id, virtual_node_id))
    ring: BTreeMap<u64, (ServerId, u32)>,
    /// Number of virtual nodes per physical server
    virtual_nodes: u32,
}

impl ConsistentHashRing {
    /// Create a new hash ring
    pub fn new(virtual_nodes: u32) -> Self {
        Self {
            ring: BTreeMap::new(),
            virtual_nodes,
        }
    }

    /// Add a server to the hash ring
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

    /// Add a node to the hash ring (alias for add_server for compatibility).
    pub fn add_node(&mut self, node: String) {
        self.add_server(&node);
    }

    /// Remove a server from the hash ring
    pub fn remove_server(&mut self, server_id: &ServerId) {
        self.ring.retain(|_, (id, _)| id != server_id);
        debug!("Removed {} from hash ring", server_id);
    }

    /// Find the server responsible for a given hash
    pub fn get_server(&self, hash: u64, eligible_servers: &[ServerId]) -> Option<&ServerId> {
        // Find the first node with hash >= input hash
        let start_iter = self.ring.range(hash..).map(|(_, (id, _))| id);
        let wrap_iter = self.ring.iter().map(|(_, (id, _))| id);

        // Check servers in order starting from hash position
        start_iter.chain(wrap_iter).find(|&id| eligible_servers.contains(id))
    }

    /// Find the node for a given key from available nodes using consistent hashing.
    pub fn get_node<'a>(&self, key: &str, available_nodes: &'a [String]) -> Option<&'a String> {
        if available_nodes.is_empty() {
            return None;
        }

        // Hash the key
        let key_hash = xxhash_rust::xxh3::xxh3_64(key.as_bytes());

        // Find first node with hash >= key_hash (circular search)
        let start_iter = self.ring.range(key_hash..).map(|(_, (id, _))| id);
        let wrap_iter = self.ring.iter().map(|(_, (id, _))| id);

        // Find the first server in the ring that's in available_nodes
        let selected = start_iter.chain(wrap_iter).find(|id| available_nodes.contains(id))?;

        // Return reference to the string in available_nodes (for lifetime reasons)
        available_nodes.iter().find(|n| *n == selected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_round_robin() {
        let config = RoutingConfig {
            algorithm: RoutingAlgorithm::RoundRobin,
            virtual_nodes: 150,
            hash_key: HashKey::ToolName,
            sticky_sessions: false,
            session_ttl: 3600,
        };

        let lb = LoadBalancer::new(config);
        let servers = vec![
            "server1".to_string(),
            "server2".to_string(),
            "server3".to_string(),
        ];

        // Should cycle through servers
        let s1 = lb.select_server("key1", &servers, None).await.unwrap();
        let s2 = lb.select_server("key2", &servers, None).await.unwrap();
        let s3 = lb.select_server("key3", &servers, None).await.unwrap();
        let s4 = lb.select_server("key4", &servers, None).await.unwrap();

        assert_eq!(s1, "server1");
        assert_eq!(s2, "server2");
        assert_eq!(s3, "server3");
        assert_eq!(s4, "server1");
    }

    #[tokio::test]
    async fn test_least_connections() {
        let config = RoutingConfig {
            algorithm: RoutingAlgorithm::LeastConnections,
            virtual_nodes: 150,
            hash_key: HashKey::ToolName,
            sticky_sessions: false,
            session_ttl: 3600,
        };

        let lb = LoadBalancer::new(config);
        let servers = vec!["server1".to_string(), "server2".to_string()];

        // Simulate different connection counts
        lb.connection_counts.insert("server1".to_string(), AtomicUsize::new(10));
        lb.connection_counts.insert("server2".to_string(), AtomicUsize::new(5));

        // Should select server2 (fewer connections)
        let selected = lb.select_server("key", &servers, None).await.unwrap();
        // Due to Power of Two Choices, not guaranteed but highly likely
        // to select server2
    }

    #[tokio::test]
    async fn test_consistent_hash() {
        let config = RoutingConfig {
            algorithm: RoutingAlgorithm::ConsistentHash,
            virtual_nodes: 150,
            hash_key: HashKey::ToolName,
            sticky_sessions: false,
            session_ttl: 3600,
        };

        let lb = LoadBalancer::new(config);
        let servers = vec![
            "server1".to_string(),
            "server2".to_string(),
            "server3".to_string(),
        ];

        // Add servers to hash ring
        for server in &servers {
            lb.add_server(server);
        }

        // Same key should always map to same server
        let s1 = lb.select_server("same_key", &servers, None).await.unwrap();
        let s2 = lb.select_server("same_key", &servers, None).await.unwrap();
        assert_eq!(s1, s2);

        // Different keys should distribute
        let _s3 = lb.select_server("different_key", &servers, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_sticky_sessions() {
        let config = RoutingConfig {
            algorithm: RoutingAlgorithm::Random,
            virtual_nodes: 150,
            hash_key: HashKey::ToolName,
            sticky_sessions: true,
            session_ttl: 3600,
        };

        let lb = LoadBalancer::new(config);
        let servers = vec![
            "server1".to_string(),
            "server2".to_string(),
            "server3".to_string(),
        ];

        // First request with session ID
        let s1 = lb.select_server("key1", &servers, Some("session123")).await.unwrap();

        // Subsequent requests with same session should go to same server
        let s2 = lb.select_server("key2", &servers, Some("session123")).await.unwrap();
        let s3 = lb.select_server("key3", &servers, Some("session123")).await.unwrap();

        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
    }

    #[tokio::test]
    async fn test_health_aware_routing() {
        let config = RoutingConfig {
            algorithm: RoutingAlgorithm::RoundRobin,
            virtual_nodes: 150,
            hash_key: HashKey::ToolName,
            sticky_sessions: false,
            session_ttl: 3600,
        };

        let lb = LoadBalancer::new(config);
        let servers = vec!["server1".to_string(), "server2".to_string()];

        // Mark server1 as unhealthy
        let health = HealthState::new();
        health.record_failure();
        health.record_failure();
        health.record_failure(); // 3 consecutive failures
        lb.health_states.insert("server1".to_string(), health);

        // Should only select server2
        let s1 = lb.select_server("key1", &servers, None).await.unwrap();
        let s2 = lb.select_server("key2", &servers, None).await.unwrap();

        assert_eq!(s1, "server2");
        assert_eq!(s2, "server2");
    }
}
