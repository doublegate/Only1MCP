//! End-to-End Benchmarks
//!
//! This benchmark suite measures full request-response cycle performance
//! with various feature combinations. It simulates realistic proxy usage
//! patterns to measure overall system performance.
//!
//! Benchmarks:
//! - Proxy latency overhead (request through full stack)
//! - Cache hit vs miss performance
//! - Concurrent request handling
//! -Full feature stack with caching
//!
//! Total: 4 benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use only1mcp::cache::{CacheConfig, LayeredCache};
use only1mcp::types::{McpRequest, McpResponse, Tool};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

/// Create mock MCP request
fn mock_request(method: &str, id: i64) -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(id)),
        method: method.to_string(),
        params: Some(json!({})),
    }
}

/// Create mock MCP response with tools
fn mock_tools_response(count: usize) -> McpResponse {
    let tools: Vec<Tool> = (0..count)
        .map(|i| Tool {
            name: format!("tool_{}", i),
            description: Some(format!("Test tool {}", i)),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        })
        .collect();

    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        result: Some(json!({ "tools": tools })),
        error: None,
    }
}

/// Benchmark basic proxy request handling overhead
fn bench_proxy_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end/proxy_overhead");
    group.throughput(Throughput::Elements(1));

    group.bench_function("request_parsing", |b| {
        b.to_async(&rt).iter(|| async {
            let request = mock_request("tools/list", 1);
            // Simulate minimal proxy processing
            let json = serde_json::to_string(&request).unwrap();
            let _parsed: McpRequest = serde_json::from_str(black_box(&json)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark cache hit vs miss performance
fn bench_cache_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end/cache");

    // Create cache with reasonable defaults
    let cache = Arc::new(LayeredCache::new(CacheConfig {
        enabled: true,
        l1_capacity: 1000,
        l1_ttl: Duration::from_secs(300),
        l2_capacity: 500,
        l2_ttl: Duration::from_secs(1800),
        l3_capacity: 200,
        l3_ttl: Duration::from_secs(7200),
    }));

    // Pre-populate cache
    rt.block_on(async {
        let response = mock_tools_response(50);
        let value = serde_json::to_vec(&response).unwrap();
        cache.set("cache_key_hit".to_string(), value, "tools/list").await;
        cache.sync().await;
    });

    group.bench_function("cache_hit", |b| {
        b.to_async(&rt).iter(|| async {
            let _response = cache.get(black_box("cache_key_hit")).await;
        });
    });

    group.bench_function("cache_miss", |b| {
        b.to_async(&rt).iter(|| async {
            let _response = cache.get(black_box("nonexistent_key")).await;
        });
    });

    group.finish();
}

/// Benchmark concurrent request handling
fn bench_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end/concurrent");

    for concurrency in [1, 10, 50, 100] {
        group.throughput(Throughput::Elements(concurrency));
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrency),
            &concurrency,
            |b, &conc| {
                b.to_async(&rt).iter(|| async {
                    let mut handles = vec![];

                    for i in 0..conc {
                        let handle = tokio::spawn(async move {
                            let request = mock_request("tools/list", i as i64);
                            let json = serde_json::to_string(&request).unwrap();
                            let _parsed: McpRequest = serde_json::from_str(&json).unwrap();
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        let _ = handle.await;
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark full feature stack (cache + serialization)
fn bench_full_stack(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("end_to_end/full_stack");
    group.throughput(Throughput::Elements(1));

    // Setup cache
    let cache = Arc::new(LayeredCache::new(CacheConfig {
        enabled: true,
        l1_capacity: 1000,
        l1_ttl: Duration::from_secs(300),
        l2_capacity: 500,
        l2_ttl: Duration::from_secs(1800),
        l3_capacity: 200,
        l3_ttl: Duration::from_secs(7200),
    }));

    // Pre-populate cache
    rt.block_on(async {
        let response = mock_tools_response(20);
        let value = serde_json::to_vec(&response).unwrap();
        cache.set("full_stack_key".to_string(), value, "tools/list").await;
        cache.sync().await;
    });

    group.bench_function("cached_request", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate full request flow with cache hit
            let request = mock_request("tools/list", 1);

            // 1. Parse request
            let json = serde_json::to_string(&request).unwrap();
            let _parsed: McpRequest = serde_json::from_str(&json).unwrap();

            // 2. Check cache
            let _cached = cache.get(black_box("full_stack_key")).await;
        });
    });

    group.bench_function("uncached_request", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate full request flow with cache miss
            let request = mock_request("tools/list", 1);

            // 1. Parse request
            let json = serde_json::to_string(&request).unwrap();
            let _parsed: McpRequest = serde_json::from_str(&json).unwrap();

            // 2. Check cache (miss)
            let _cached = cache.get(black_box("nonexistent")).await;
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_proxy_overhead,
    bench_cache_performance,
    bench_concurrent_requests,
    bench_full_stack
);
criterion_main!(benches);
