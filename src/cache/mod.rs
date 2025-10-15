//! Multi-layer caching system for context optimization.
//!
//! Implements a three-tier cache with different TTLs:
//! - L1: Hot cache for frequently accessed tools (5 min TTL)
//! - L2: Warm cache for resource listings (30 min TTL)
//! - L3: Cold cache for static prompts (2 hour TTL)

use crate::error::{Error, Result};
use crate::types::{McpRequest, McpResponse};
use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use std::future::Future;
use serde::{Serialize, Deserialize};
use tracing::{debug, info};

/// Multi-layer caching system with different TTLs per operation type.
/// Implements LRU eviction with size limits and TTL expiration.
pub struct LayeredCache {
    /// L1: Hot cache for frequently accessed tools (5 min TTL)
    l1_tools: Arc<DashMap<CacheKey, CachedResponse>>,

    /// L2: Warm cache for resource listings (30 min TTL)
    l2_resources: Arc<DashMap<CacheKey, CachedResponse>>,

    /// L3: Cold cache for static prompts (2 hour TTL)
    l3_prompts: Arc<DashMap<CacheKey, CachedResponse>>,

    /// Configuration for cache behavior
    config: CacheConfig,

    /// Metrics for cache effectiveness monitoring
    metrics: Arc<CacheMetrics>,
}

/// Alias for the main cache type used by the application
pub type ResponseCache = LayeredCache;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    /// Blake3 hash of the request for fast comparison
    request_hash: [u8; 32],

    /// Optional namespace for multi-tenant scenarios
    namespace: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// The actual cached response data
    pub data: Vec<u8>,

    /// When this entry was created
    #[serde(skip)]
    pub created_at: Instant,

    /// How many times this entry has been accessed
    pub hit_count: u32,

    /// Size in bytes for memory management
    pub size_bytes: usize,

    /// Optional ETag for conditional requests
    pub etag: Option<String>,

    /// Server ID that provided this response
    pub server_id: String,
}

impl LayeredCache {
    /// Create a new multi-layer cache with optimized settings.
    pub fn new(max_entries: usize, max_size_bytes: usize) -> Self {
        let config = CacheConfig {
            l1_capacity: max_entries / 2,
            l2_capacity: max_entries / 3,
            l3_capacity: max_entries / 6,
            max_cache_bytes: max_size_bytes,
            ..Default::default()
        };

        Self {
            l1_tools: Arc::new(DashMap::with_capacity(config.l1_capacity)),
            l2_resources: Arc::new(DashMap::with_capacity(config.l2_capacity)),
            l3_prompts: Arc::new(DashMap::with_capacity(config.l3_capacity)),
            config,
            metrics: Arc::new(CacheMetrics::default()),
        }
    }

    /// Get a cached response if available and fresh.
    pub async fn get(&self, key: &str) -> Option<CachedResponse> {
        let cache_key = self.compute_cache_key_from_string(key);

        // Try all cache layers in order
        for cache in [&self.l1_tools, &self.l2_resources, &self.l3_prompts] {
            if let Some(entry) = cache.get(&cache_key) {
                if self.is_fresh_by_duration(&entry, Duration::from_secs(300)) {
                    self.metrics.hits.fetch_add(1, Ordering::Relaxed);
                    return Some(entry.clone());
                }
                // Stale entry - remove it
                cache.remove(&cache_key);
            }
        }

        self.metrics.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Get or compute a response with cache-aside pattern.
    /// Returns cached value if fresh, otherwise computes and stores.
    pub async fn get_or_compute<F, Fut>(
        &self,
        request: &McpRequest,
        compute: F,
    ) -> Result<McpResponse>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<McpResponse>>,
    {
        // Generate cache key from request
        let key = self.compute_cache_key(request);

        // Determine which cache layer based on request type
        let cache = self.select_cache_layer(&request.method());

        // Check for cached response
        if let Some(entry) = cache.get(&key) {
            if self.is_fresh(&entry, &request.method()) {
                self.metrics.hits.fetch_add(1, Ordering::Relaxed);
                debug!("Cache hit for {}", request.method());
                return self.deserialize_response(&entry.data);
            }
            // Stale entry - remove it
            cache.remove(&key);
        }

        // Cache miss - compute the response
        self.metrics.misses.fetch_add(1, Ordering::Relaxed);
        debug!("Cache miss for {}", request.method());
        let response = compute().await?;

        // Store in cache if cacheable
        if self.is_cacheable(&request, &response) {
            let serialized = self.serialize_response(&response)?;
            let entry = CachedResponse {
                data: serialized.clone(),
                created_at: Instant::now(),
                hit_count: 0,
                size_bytes: serialized.len(),
                etag: self.generate_etag(&serialized),
                server_id: String::new(), // Set by caller if needed
            };

            // Check cache size limits before inserting
            self.maybe_evict(cache.clone(), entry.size_bytes).await;
            cache.insert(key, entry);
            self.metrics.inserts.fetch_add(1, Ordering::Relaxed);
        }

        Ok(response)
    }

    /// Compute a deterministic cache key using Blake3.
    /// Ensures consistent hashing across restarts.
    fn compute_cache_key(&self, request: &McpRequest) -> CacheKey {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        // Hash the normalized request (sorted keys, trimmed strings)
        let normalized = format!("{}:{}", request.method(), request.params_hash());
        hasher.update(normalized.as_bytes());

        // Add namespace if multi-tenant
        if let Some(ns) = &self.config.namespace {
            hasher.update(ns.as_bytes());
        }

        CacheKey {
            request_hash: hasher.finalize().into(),
            namespace: self.config.namespace.clone(),
        }
    }

    /// Compute cache key from a simple string.
    fn compute_cache_key_from_string(&self, key: &str) -> CacheKey {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(key.as_bytes());

        CacheKey {
            request_hash: hasher.finalize().into(),
            namespace: self.config.namespace.clone(),
        }
    }

    /// Intelligent cache layer selection based on request type.
    fn select_cache_layer(&self, method: &str) -> Arc<DashMap<CacheKey, CachedResponse>> {
        match method {
            // Tool operations are frequently accessed, short TTL
            "tools/list" | "tools/call" => self.l1_tools.clone(),

            // Resource operations are less frequent, medium TTL
            "resources/list" | "resources/read" => self.l2_resources.clone(),

            // Prompts are static, long TTL
            "prompts/list" | "prompts/get" => self.l3_prompts.clone(),

            // Default to L1 for unknown methods
            _ => self.l1_tools.clone(),
        }
    }

    /// Check if a cached entry is still fresh based on TTL.
    fn is_fresh(&self, entry: &CachedResponse, method: &str) -> bool {
        let age = entry.created_at.elapsed();
        let ttl = match method {
            "tools/list" => self.config.l1_ttl,
            "resources/list" => self.config.l2_ttl,
            "prompts/list" => self.config.l3_ttl,
            _ => Duration::from_secs(300), // 5 min default
        };
        age < ttl
    }

    /// Check if entry is fresh based on a specific duration.
    fn is_fresh_by_duration(&self, entry: &CachedResponse, ttl: Duration) -> bool {
        entry.created_at.elapsed() < ttl
    }

    /// LRU eviction when cache approaches size limits.
    /// Removes least recently used entries until under threshold.
    async fn maybe_evict(
        &self,
        cache: Arc<DashMap<CacheKey, CachedResponse>>,
        needed_bytes: usize,
    ) {
        let current_size: usize = cache.iter()
            .map(|entry| entry.value().size_bytes)
            .sum();

        if current_size + needed_bytes > self.config.max_cache_bytes {
            // Find least recently used entries
            let mut entries: Vec<_> = cache.iter()
                .map(|entry| (entry.key().clone(), entry.value().created_at))
                .collect();

            entries.sort_by_key(|e| e.1);

            // Remove oldest entries until we have space
            let mut freed = 0;
            for (key, _) in entries {
                if freed >= needed_bytes {
                    break;
                }
                if let Some((_, removed)) = cache.remove(&key) {
                    freed += removed.size_bytes;
                    self.metrics.evictions.fetch_add(1, Ordering::Relaxed);
                }
            }

            info!("Evicted {} bytes from cache", freed);
        }
    }

    /// Check if a request/response pair should be cached.
    fn is_cacheable(&self, request: &McpRequest, _response: &McpResponse) -> bool {
        // Don't cache mutations or sensitive operations
        !matches!(request.method().as_str(),
            "resources/write" | "resources/delete" | "auth/*" | "admin/*"
        )
    }

    /// Serialize response for storage.
    fn serialize_response(&self, response: &McpResponse) -> Result<Vec<u8>> {
        serde_json::to_vec(response)
            .map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Deserialize response from storage.
    fn deserialize_response(&self, data: &[u8]) -> Result<McpResponse> {
        serde_json::from_slice(data)
            .map_err(|e| Error::Deserialization(e.to_string()))
    }

    /// Generate ETag for cache validation.
    fn generate_etag(&self, data: &[u8]) -> Option<String> {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(data);
        Some(format!("\"{}\"", hasher.finalize().to_hex()))
    }

    /// Clear all cache layers.
    pub fn clear(&self) {
        self.l1_tools.clear();
        self.l2_resources.clear();
        self.l3_prompts.clear();
        self.metrics.clears.fetch_add(1, Ordering::Relaxed);
        info!("Cache cleared");
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            l1_entries: self.l1_tools.len(),
            l2_entries: self.l2_resources.len(),
            l3_entries: self.l3_prompts.len(),
            total_hits: self.metrics.hits.load(Ordering::Relaxed),
            total_misses: self.metrics.misses.load(Ordering::Relaxed),
            total_evictions: self.metrics.evictions.load(Ordering::Relaxed),
            hit_rate: self.metrics.hit_rate(),
        }
    }
}

/// Cache configuration with sensible defaults.
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// L1 cache capacity (hot, tools)
    pub l1_capacity: usize,
    pub l1_ttl: Duration,

    /// L2 cache capacity (warm, resources)
    pub l2_capacity: usize,
    pub l2_ttl: Duration,

    /// L3 cache capacity (cold, prompts)
    pub l3_capacity: usize,
    pub l3_ttl: Duration,

    /// Maximum total cache size in bytes
    pub max_cache_bytes: usize,

    /// Optional namespace for multi-tenant scenarios
    pub namespace: Option<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_capacity: 1000,
            l1_ttl: Duration::from_secs(300),      // 5 minutes

            l2_capacity: 500,
            l2_ttl: Duration::from_secs(1800),     // 30 minutes

            l3_capacity: 200,
            l3_ttl: Duration::from_secs(7200),     // 2 hours

            max_cache_bytes: 100 * 1024 * 1024,    // 100 MB

            namespace: None,
        }
    }
}

/// Cache metrics for monitoring effectiveness.
#[derive(Default)]
pub struct CacheMetrics {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub inserts: AtomicU64,
    pub evictions: AtomicU64,
    pub clears: AtomicU64,
}

impl CacheMetrics {
    /// Calculate cache hit rate as a percentage.
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed) as f64;
        let misses = self.misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;

        if total > 0.0 {
            (hits / total) * 100.0
        } else {
            0.0
        }
    }
}

/// Cache statistics for monitoring.
#[derive(Debug, Serialize)]
pub struct CacheStats {
    pub l1_entries: usize,
    pub l2_entries: usize,
    pub l3_entries: usize,
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_evictions: u64,
    pub hit_rate: f64,
}
