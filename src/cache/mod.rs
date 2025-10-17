//! Response caching system with TTL and LRU eviction using moka.
//!
//! Implements a three-tier cache with different TTLs:
//! - L1: Hot cache for frequently accessed tools (5 min TTL)
//! - L2: Warm cache for resource listings (30 min TTL)
//! - L3: Cold cache for static prompts (2 hour TTL)
//!
//! Uses the moka crate for production-grade caching with:
//! - Automatic TTL expiration
//! - Automatic LRU eviction when capacity is reached
//! - Lock-free concurrent access
//! - Async API compatible with Tokio

use crate::error::{Error, Result};
use crate::types::{McpRequest, McpResponse};
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

/// Multi-layer caching system with different TTLs per operation type.
/// Implements automatic TTL expiration and LRU eviction using moka.
pub struct LayeredCache {
    /// L1: Hot cache for frequently accessed tools (5 min TTL)
    l1_tools: Arc<Cache<String, Vec<u8>>>,

    /// L2: Warm cache for resource listings (30 min TTL)
    l2_resources: Arc<Cache<String, Vec<u8>>>,

    /// L3: Cold cache for static prompts (2 hour TTL)
    l3_prompts: Arc<Cache<String, Vec<u8>>>,

    /// Configuration for cache behavior
    config: CacheConfig,

    /// Metrics for cache effectiveness monitoring
    metrics: Arc<CacheMetrics>,
}

/// Alias for the main cache type used by the application
pub type ResponseCache = LayeredCache;

impl LayeredCache {
    /// Create a new multi-layer cache with moka-based TTL and LRU.
    pub fn new(config: CacheConfig) -> Self {
        // Create L1 cache (tools) with 5-minute TTL
        let l1_tools = Cache::builder()
            .max_capacity(config.l1_capacity)
            .time_to_live(config.l1_ttl)
            .eviction_listener(|_key, _value: Vec<u8>, _cause| {
                crate::metrics::CACHE_EVICTIONS_TOTAL.inc();
            })
            .build();

        // Create L2 cache (resources) with 30-minute TTL
        let l2_resources = Cache::builder()
            .max_capacity(config.l2_capacity)
            .time_to_live(config.l2_ttl)
            .eviction_listener(|_key, _value: Vec<u8>, _cause| {
                crate::metrics::CACHE_EVICTIONS_TOTAL.inc();
            })
            .build();

        // Create L3 cache (prompts) with 2-hour TTL
        let l3_prompts = Cache::builder()
            .max_capacity(config.l3_capacity)
            .time_to_live(config.l3_ttl)
            .eviction_listener(|_key, _value: Vec<u8>, _cause| {
                crate::metrics::CACHE_EVICTIONS_TOTAL.inc();
            })
            .build();

        Self {
            l1_tools: Arc::new(l1_tools),
            l2_resources: Arc::new(l2_resources),
            l3_prompts: Arc::new(l3_prompts),
            config,
            metrics: Arc::new(CacheMetrics::default()),
        }
    }

    /// Get a cached response if available (moka handles TTL automatically).
    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.config.enabled {
            return None;
        }

        // Try all cache layers in order
        for cache in [&self.l1_tools, &self.l2_resources, &self.l3_prompts] {
            if let Some(value) = cache.get(key).await {
                self.metrics.hits.fetch_add(1, Ordering::Relaxed);
                crate::metrics::CACHE_HITS_TOTAL.inc();
                crate::metrics::CACHE_SIZE_ENTRIES.set(self.total_size() as i64);
                return Some(value);
            }
        }

        self.metrics.misses.fetch_add(1, Ordering::Relaxed);
        crate::metrics::CACHE_MISSES_TOTAL.inc();
        None
    }

    /// Store response in cache (moka handles eviction automatically).
    pub async fn set(&self, key: String, value: Vec<u8>, method: &str) {
        if !self.config.enabled {
            return;
        }

        // Select cache layer based on method
        let cache = self.select_cache_layer(method);
        cache.insert(key, value).await;

        self.metrics.inserts.fetch_add(1, Ordering::Relaxed);
        crate::metrics::CACHE_SIZE_ENTRIES.set(self.total_size() as i64);
    }

    /// Run pending maintenance tasks to ensure immediate visibility (for testing).
    #[cfg(test)]
    pub async fn sync(&self) {
        self.l1_tools.run_pending_tasks().await;
        self.l2_resources.run_pending_tasks().await;
        self.l3_prompts.run_pending_tasks().await;
    }

    /// Invalidate specific key from all layers.
    pub async fn invalidate(&self, key: &str) {
        self.l1_tools.invalidate(key).await;
        self.l2_resources.invalidate(key).await;
        self.l3_prompts.invalidate(key).await;
        crate::metrics::CACHE_SIZE_ENTRIES.set(self.total_size() as i64);
    }

    /// Clear all cache entries across all layers.
    pub async fn clear(&self) {
        self.l1_tools.invalidate_all();
        self.l2_resources.invalidate_all();
        self.l3_prompts.invalidate_all();

        // Run pending tasks to ensure invalidation completes
        self.l1_tools.run_pending_tasks().await;
        self.l2_resources.run_pending_tasks().await;
        self.l3_prompts.run_pending_tasks().await;

        self.metrics.clears.fetch_add(1, Ordering::Relaxed);
        crate::metrics::CACHE_SIZE_ENTRIES.set(0);
        info!("Cache cleared");
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> CacheStats {
        CacheStats {
            l1_entries: self.l1_tools.entry_count(),
            l2_entries: self.l2_resources.entry_count(),
            l3_entries: self.l3_prompts.entry_count(),
            total_hits: self.metrics.hits.load(Ordering::Relaxed),
            total_misses: self.metrics.misses.load(Ordering::Relaxed),
            total_evictions: self.metrics.evictions.load(Ordering::Relaxed),
            hit_rate: self.metrics.hit_rate(),
        }
    }

    /// Compute a deterministic cache key using Blake3.
    pub fn cache_key(method: &str, params: &serde_json::Value) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();

        // Hash the method and params
        let key_data = format!("{}{}", method, params);
        hasher.update(key_data.as_bytes());

        hasher.finalize().to_hex().to_string()
    }

    /// Intelligent cache layer selection based on request type.
    fn select_cache_layer(&self, method: &str) -> Arc<Cache<String, Vec<u8>>> {
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

    /// Get total cache size across all layers.
    fn total_size(&self) -> u64 {
        self.l1_tools.entry_count() + self.l2_resources.entry_count() + self.l3_prompts.entry_count()
    }

    /// Check if a request should be cached.
    pub fn is_cacheable(&self, request: &McpRequest, _response: &McpResponse) -> bool {
        // Don't cache mutations or sensitive operations
        !matches!(
            request.method().as_str(),
            "resources/write" | "resources/delete" | "auth/*" | "admin/*"
        )
    }

    /// Serialize response for storage.
    pub fn serialize_response(&self, response: &McpResponse) -> Result<Vec<u8>> {
        serde_json::to_vec(response).map_err(|e| Error::Serialization(e.to_string()))
    }

    /// Deserialize response from storage.
    pub fn deserialize_response(&self, data: &[u8]) -> Result<McpResponse> {
        serde_json::from_slice(data).map_err(|e| Error::Deserialization(e.to_string()))
    }
}

/// Cache configuration with sensible defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,

    /// L1 cache capacity (hot, tools)
    pub l1_capacity: u64,
    pub l1_ttl: Duration,

    /// L2 cache capacity (warm, resources)
    pub l2_capacity: u64,
    pub l2_ttl: Duration,

    /// L3 cache capacity (cold, prompts)
    pub l3_capacity: u64,
    pub l3_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            l1_capacity: 1000,
            l1_ttl: Duration::from_secs(300), // 5 minutes

            l2_capacity: 500,
            l2_ttl: Duration::from_secs(1800), // 30 minutes

            l3_capacity: 200,
            l3_ttl: Duration::from_secs(7200), // 2 hours
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
    pub l1_entries: u64,
    pub l2_entries: u64,
    pub l3_entries: u64,
    pub total_hits: u64,
    pub total_misses: u64,
    pub total_evictions: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = CacheConfig::default();
        let cache = LayeredCache::new(config);

        // Test set and get
        let key = "test_key".to_string();
        let value = vec![1, 2, 3, 4];
        cache.set(key.clone(), value.clone(), "tools/list").await;

        let retrieved = cache.get(&key).await;
        assert_eq!(retrieved, Some(value));

        // Test invalidate
        cache.invalidate(&key).await;
        let after_invalidate = cache.get(&key).await;
        assert_eq!(after_invalidate, None);
    }

    #[tokio::test]
    async fn test_cache_layer_selection() {
        let config = CacheConfig::default();
        let cache = LayeredCache::new(config);

        // Tools should go to L1
        cache.set("tools_key".to_string(), vec![1], "tools/list").await;
        cache.sync().await;
        assert_eq!(cache.l1_tools.entry_count(), 1);

        // Resources should go to L2
        cache.set("resources_key".to_string(), vec![2], "resources/list").await;
        cache.sync().await;
        assert_eq!(cache.l2_resources.entry_count(), 1);

        // Prompts should go to L3
        cache.set("prompts_key".to_string(), vec![3], "prompts/list").await;
        cache.sync().await;
        assert_eq!(cache.l3_prompts.entry_count(), 1);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let config = CacheConfig::default();
        let cache = LayeredCache::new(config);

        // Add entries to all layers
        cache.set("key1".to_string(), vec![1], "tools/list").await;
        cache.set("key2".to_string(), vec![2], "resources/list").await;
        cache.set("key3".to_string(), vec![3], "prompts/list").await;

        // Clear all
        cache.clear().await;

        // Verify all layers are empty
        assert_eq!(cache.l1_tools.entry_count(), 0);
        assert_eq!(cache.l2_resources.entry_count(), 0);
        assert_eq!(cache.l3_prompts.entry_count(), 0);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let config = CacheConfig::default();
        let cache = LayeredCache::new(config);

        cache.set("key1".to_string(), vec![1], "tools/list").await;
        let _ = cache.get("key1").await; // hit
        let _ = cache.get("nonexistent").await; // miss

        let stats = cache.stats().await;
        assert_eq!(stats.total_hits, 1);
        assert_eq!(stats.total_misses, 1);
        assert!(stats.hit_rate > 0.0);
    }
}
