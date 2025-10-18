//! Caching Benchmarks
//!
//! This benchmark suite measures cache performance including hit/miss rates,
//! LRU eviction, and TTL expiration.
//!
//! Benchmarks:
//! - Cache hit (warm cache)
//! - Cache miss (cold cache)
//! - Mixed workload (80/20 hit/miss)
//! - LRU eviction
//! - Stats tracking
//!
//! Total: 5 benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use only1mcp::cache::{CacheConfig, LayeredCache};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Create a cache with test configuration
fn test_cache() -> LayeredCache {
    LayeredCache::new(CacheConfig {
        enabled: true,
        l1_capacity: 1000,
        l1_ttl: Duration::from_secs(300),
        l2_capacity: 500,
        l2_ttl: Duration::from_secs(1800),
        l3_capacity: 200,
        l3_ttl: Duration::from_secs(7200),
    })
}

/// Benchmark cache hit performance (warm cache)
fn bench_cache_hit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = test_cache();

    // Pre-populate cache
    rt.block_on(async {
        for i in 0..100 {
            let key = format!("key-{}", i);
            let value = format!("value-{}", i).into_bytes();
            cache.set(key, value, "test/method").await;
        }
        cache.sync().await; // Ensure all entries are committed
    });

    let mut group = c.benchmark_group("caching/hit");
    group.throughput(Throughput::Elements(1));

    group.bench_function("warm_cache", |b| {
        b.to_async(&rt).iter(|| async {
            // Hit the cache with existing keys
            let key = format!("key-{}", black_box(42));
            let _ = cache.get(black_box(&key)).await;
        });
    });

    group.finish();
}

/// Benchmark cache miss performance (cold cache)
fn bench_cache_miss(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = test_cache();

    let mut group = c.benchmark_group("caching/miss");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicU64::new(0);
    group.bench_function("cold_cache", |b| {
        b.to_async(&rt).iter(|| async {
            // Always miss with unique keys
            let count = counter.fetch_add(1, Ordering::Relaxed);
            let key = format!("nonexistent-{}", count);
            let _ = cache.get(black_box(&key)).await;
        });
    });

    group.finish();
}

/// Benchmark mixed workload (80% hits, 20% misses)
fn bench_mixed_workload(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = test_cache();

    // Pre-populate cache with 80 entries
    rt.block_on(async {
        for i in 0..80 {
            let key = format!("key-{}", i);
            let value = format!("value-{}", i).into_bytes();
            cache.set(key, value, "test/method").await;
        }
        cache.sync().await;
    });

    let mut group = c.benchmark_group("caching/mixed");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicU64::new(0);
    group.bench_function("80_20_hit_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let count = counter.fetch_add(1, Ordering::Relaxed);
            let key = if count % 5 == 0 {
                // 20% misses
                format!("nonexistent-{}", count)
            } else {
                // 80% hits
                format!("key-{}", count % 80)
            };
            let _ = cache.get(black_box(&key)).await;
        });
    });

    group.finish();
}

/// Benchmark LRU eviction performance
fn bench_lru_eviction(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Small cache to force evictions
    let cache = LayeredCache::new(CacheConfig {
        enabled: true,
        l1_capacity: 10, // Very small to trigger evictions
        l1_ttl: Duration::from_secs(300),
        l2_capacity: 5,
        l2_ttl: Duration::from_secs(1800),
        l3_capacity: 2,
        l3_ttl: Duration::from_secs(7200),
    });

    let mut group = c.benchmark_group("caching/eviction");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicU64::new(0);
    group.bench_function("lru_eviction", |b| {
        b.to_async(&rt).iter(|| async {
            let count = counter.fetch_add(1, Ordering::Relaxed);
            // Insert new entries, forcing evictions
            let key = format!("key-{}", count);
            let value = vec![0u8; 100]; // Small value
            cache.set(black_box(key), black_box(value), "test/method").await;
        });
    });

    group.finish();
}

/// Benchmark cache statistics tracking
fn bench_stats_tracking(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let cache = test_cache();

    // Pre-populate and create some hit/miss history
    rt.block_on(async {
        for i in 0..50 {
            let key = format!("key-{}", i);
            let value = format!("value-{}", i).into_bytes();
            cache.set(key, value, "test/method").await;
        }
        cache.sync().await;

        // Generate some cache activity
        for i in 0..100 {
            let key = if i % 2 == 0 {
                format!("key-{}", i % 50) // Hit
            } else {
                format!("miss-{}", i) // Miss
            };
            let _ = cache.get(&key).await;
        }
    });

    let mut group = c.benchmark_group("caching/stats");
    group.throughput(Throughput::Elements(1));

    group.bench_function("stats_retrieval", |b| {
        b.to_async(&rt).iter(|| async {
            let _ = cache.stats().await;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_hit,
    bench_cache_miss,
    bench_mixed_workload,
    bench_lru_eviction,
    bench_stats_tracking
);
criterion_main!(benches);
