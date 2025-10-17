//! Comprehensive tests for response caching with TTL and LRU eviction.

use only1mcp::cache::{CacheConfig, LayeredCache};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_cache_hit_and_miss() {
    let config = CacheConfig::default();
    let cache = LayeredCache::new(config);

    // Initial get should be a miss
    let result = cache.get("nonexistent_key").await;
    assert_eq!(result, None);

    // Set a value
    let key = "test_key".to_string();
    let value = vec![1, 2, 3, 4, 5];
    cache.set(key.clone(), value.clone(), "tools/list").await;

    // Get should now be a hit
    let result = cache.get(&key).await;
    assert_eq!(result, Some(value));

    // Stats should reflect 1 hit and 1 miss
    let stats = cache.stats().await;
    assert_eq!(stats.total_hits, 1);
    assert_eq!(stats.total_misses, 1);
    assert_eq!(stats.hit_rate, 50.0); // 1 hit out of 2 total accesses
}

#[tokio::test]
async fn test_ttl_expiry() {
    let config = CacheConfig {
        enabled: true,
        l1_capacity: 100,
        l1_ttl: Duration::from_millis(100), // Very short TTL for testing
        l2_capacity: 50,
        l2_ttl: Duration::from_secs(1800),
        l3_capacity: 20,
        l3_ttl: Duration::from_secs(7200),
    };
    let cache = LayeredCache::new(config);

    // Set a value in L1 (tools)
    let key = "expiring_key".to_string();
    let value = vec![9, 8, 7, 6];
    cache.set(key.clone(), value.clone(), "tools/list").await;

    // Should be retrievable immediately
    let result = cache.get(&key).await;
    assert_eq!(result, Some(value));

    // Wait for TTL to expire
    sleep(Duration::from_millis(150)).await;

    // Should now be expired and return None
    let result_after_expiry = cache.get(&key).await;
    assert_eq!(result_after_expiry, None);
}

#[tokio::test]
async fn test_lru_eviction() {
    let config = CacheConfig {
        enabled: true,
        l1_capacity: 3, // Very small capacity
        l1_ttl: Duration::from_secs(300),
        l2_capacity: 50,
        l2_ttl: Duration::from_secs(1800),
        l3_capacity: 20,
        l3_ttl: Duration::from_secs(7200),
    };
    let cache = LayeredCache::new(config);

    // Fill cache to capacity
    cache.set("key1".to_string(), vec![1], "tools/list").await;
    cache.set("key2".to_string(), vec![2], "tools/list").await;
    cache.set("key3".to_string(), vec![3], "tools/list").await;

    // Access key1 and key2 to make them recently used
    let _ = cache.get("key1").await;
    let _ = cache.get("key2").await;

    // Insert key4, which should evict key3 (least recently used)
    cache.set("key4".to_string(), vec![4], "tools/list").await;

    // Wait a moment for eviction to complete
    sleep(Duration::from_millis(50)).await;

    // key3 should be evicted, key1, key2, key4 should exist
    assert_eq!(cache.get("key3").await, None);
    assert_eq!(cache.get("key1").await, Some(vec![1]));
    assert_eq!(cache.get("key2").await, Some(vec![2]));
    assert_eq!(cache.get("key4").await, Some(vec![4]));
}

#[tokio::test]
async fn test_cache_invalidation() {
    let config = CacheConfig::default();
    let cache = LayeredCache::new(config);

    // Set multiple values
    cache.set("key1".to_string(), vec![1], "tools/list").await;
    cache.set("key2".to_string(), vec![2], "resources/list").await;
    cache.set("key3".to_string(), vec![3], "prompts/list").await;

    // Verify they exist
    assert!(cache.get("key1").await.is_some());
    assert!(cache.get("key2").await.is_some());
    assert!(cache.get("key3").await.is_some());

    // Invalidate key2
    cache.invalidate("key2").await;

    // key2 should be gone, others remain
    assert!(cache.get("key1").await.is_some());
    assert_eq!(cache.get("key2").await, None);
    assert!(cache.get("key3").await.is_some());
}

#[tokio::test]
async fn test_cache_clear_all() {
    let config = CacheConfig::default();
    let cache = LayeredCache::new(config);

    // Add entries to all layers
    cache.set("tools_key".to_string(), vec![1], "tools/list").await;
    cache.set("resources_key".to_string(), vec![2], "resources/list").await;
    cache.set("prompts_key".to_string(), vec![3], "prompts/list").await;

    // Wait for insertions to complete
    sleep(Duration::from_millis(50)).await;

    // Verify they exist
    let stats_before = cache.stats().await;
    assert_eq!(stats_before.l1_entries, 1);
    assert_eq!(stats_before.l2_entries, 1);
    assert_eq!(stats_before.l3_entries, 1);

    // Clear all
    cache.clear().await;

    // All layers should be empty
    let stats_after = cache.stats().await;
    assert_eq!(stats_after.l1_entries, 0);
    assert_eq!(stats_after.l2_entries, 0);
    assert_eq!(stats_after.l3_entries, 0);

    // All keys should be gone
    assert_eq!(cache.get("tools_key").await, None);
    assert_eq!(cache.get("resources_key").await, None);
    assert_eq!(cache.get("prompts_key").await, None);
}

#[tokio::test]
async fn test_cache_stats_tracking() {
    let config = CacheConfig::default();
    let cache = LayeredCache::new(config);

    // Perform some operations
    cache.set("key1".to_string(), vec![1], "tools/list").await;
    cache.set("key2".to_string(), vec![2], "resources/list").await;

    // Wait for insertions
    sleep(Duration::from_millis(50)).await;

    // Create hits and misses
    let _ = cache.get("key1").await; // hit
    let _ = cache.get("key1").await; // hit
    let _ = cache.get("nonexistent1").await; // miss
    let _ = cache.get("nonexistent2").await; // miss
    let _ = cache.get("key2").await; // hit

    // Check stats
    let stats = cache.stats().await;
    assert_eq!(stats.total_hits, 3);
    assert_eq!(stats.total_misses, 2);
    assert_eq!(stats.l1_entries, 1); // key1 in tools
    assert_eq!(stats.l2_entries, 1); // key2 in resources
    assert_eq!(stats.l3_entries, 0); // no prompts

    // Hit rate should be 60% (3 hits out of 5 total)
    assert!((stats.hit_rate - 60.0).abs() < 0.01);
}

#[tokio::test]
async fn test_cache_layer_routing() {
    let config = CacheConfig::default();
    let cache = LayeredCache::new(config);

    // Test that different methods route to different layers
    cache.set("tools1".to_string(), vec![1], "tools/list").await;
    cache.set("tools2".to_string(), vec![2], "tools/call").await;
    cache.set("res1".to_string(), vec![3], "resources/list").await;
    cache.set("res2".to_string(), vec![4], "resources/read").await;
    cache.set("prompt1".to_string(), vec![5], "prompts/list").await;
    cache.set("prompt2".to_string(), vec![6], "prompts/get").await;

    // Wait for insertions
    sleep(Duration::from_millis(100)).await;

    let stats = cache.stats().await;

    // L1 should have 2 entries (tools)
    assert_eq!(stats.l1_entries, 2);

    // L2 should have 2 entries (resources)
    assert_eq!(stats.l2_entries, 2);

    // L3 should have 2 entries (prompts)
    assert_eq!(stats.l3_entries, 2);
}

#[tokio::test]
async fn test_cache_key_generation() {
    use serde_json::json;

    // Test that cache key generation is deterministic
    let params1 = json!({"param": "value", "num": 42});
    let key1 = LayeredCache::cache_key("tools/list", &params1);
    let key2 = LayeredCache::cache_key("tools/list", &params1);

    // Same params should produce same key
    assert_eq!(key1, key2);

    // Different params should produce different key
    let params2 = json!({"param": "value", "num": 43});
    let key3 = LayeredCache::cache_key("tools/list", &params2);
    assert_ne!(key1, key3);

    // Different method should produce different key
    let key4 = LayeredCache::cache_key("resources/list", &params1);
    assert_ne!(key1, key4);
}

#[tokio::test]
async fn test_cache_disabled() {
    let config = CacheConfig {
        enabled: false, // Cache disabled
        ..Default::default()
    };
    let cache = LayeredCache::new(config);

    // Set should not store anything
    cache.set("key1".to_string(), vec![1, 2, 3], "tools/list").await;

    // Get should return None
    let result = cache.get("key1").await;
    assert_eq!(result, None);

    // Stats should show no entries
    let stats = cache.stats().await;
    assert_eq!(stats.l1_entries, 0);
    assert_eq!(stats.l2_entries, 0);
    assert_eq!(stats.l3_entries, 0);
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    let config = CacheConfig::default();
    let cache = std::sync::Arc::new(LayeredCache::new(config));

    // Spawn multiple concurrent tasks
    let mut handles = vec![];

    for i in 0..10 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i);
            let value = vec![i as u8];

            // Set
            cache_clone.set(key.clone(), value.clone(), "tools/list").await;

            // Small delay to ensure set completes
            tokio::time::sleep(Duration::from_millis(10)).await;

            // Get
            let result = cache_clone.get(&key).await;
            assert_eq!(result, Some(value));
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Wait for all insertions to settle
    sleep(Duration::from_millis(100)).await;

    // Verify all entries are in cache
    let stats = cache.stats().await;
    assert_eq!(stats.l1_entries, 10);
}

#[tokio::test]
async fn test_cache_eviction_metrics() {
    let config = CacheConfig {
        enabled: true,
        l1_capacity: 2, // Very small to force evictions
        l1_ttl: Duration::from_secs(300),
        l2_capacity: 50,
        l2_ttl: Duration::from_secs(1800),
        l3_capacity: 20,
        l3_ttl: Duration::from_secs(7200),
    };
    let cache = LayeredCache::new(config);

    // Fill beyond capacity to trigger evictions
    cache.set("key1".to_string(), vec![1], "tools/list").await;
    sleep(Duration::from_millis(20)).await;
    cache.set("key2".to_string(), vec![2], "tools/list").await;
    sleep(Duration::from_millis(20)).await;
    cache.set("key3".to_string(), vec![3], "tools/list").await;
    sleep(Duration::from_millis(20)).await;
    cache.set("key4".to_string(), vec![4], "tools/list").await;

    // Wait for evictions to complete
    sleep(Duration::from_millis(200)).await;

    // Evictions should have occurred
    let stats = cache.stats().await;
    assert_eq!(stats.l1_entries, 2); // Only 2 should remain due to capacity

    // Note: Eviction count tracking depends on eviction_listener being called
    // The actual count may vary based on moka's internal timing
}
