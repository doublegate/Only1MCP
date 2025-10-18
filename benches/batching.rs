//! Batching Benchmarks
//!
//! This benchmark suite measures request batching performance and backend
//! call reduction efficiency.
//!
//! Benchmarks:
//! - No batching (baseline)
//! - Batching enabled (100ms window)
//! - Varying batch sizes (1, 5, 10, 20, 50)
//! - Concurrent submissions (10 clients)
//!
//! Total: 4 benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use only1mcp::batching::BatchAggregator;
use only1mcp::config::BatchingConfig;
use only1mcp::types::McpRequest;
use serde_json::json;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Create mock request
fn mock_request(id: i64) -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(id)),
        method: "tools/list".to_string(),
        params: Some(json!({})),
    }
}

/// Benchmark baseline (no batching)
fn bench_no_batching(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Disabled batching
    let aggregator = BatchAggregator::new(BatchingConfig {
        enabled: false,
        window_ms: 0,
        max_batch_size: 1,
        methods: vec!["tools/list".to_string()],
    });

    let mut group = c.benchmark_group("batching/disabled");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicI64::new(0);
    group.bench_function("baseline", |b| {
        b.to_async(&rt).iter(|| async {
            let count = counter.fetch_add(1, Ordering::Relaxed);
            let request = mock_request(count);
            // Note: This will error since there's no actual backend,
            // but we're benchmarking the batching overhead
            let _ = aggregator
                .submit_request(black_box("server1".to_string()), black_box(request))
                .await;
        });
    });

    group.finish();
}

/// Benchmark batching enabled
fn bench_batching_enabled(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let aggregator = BatchAggregator::new(BatchingConfig {
        enabled: true,
        window_ms: 100,
        max_batch_size: 10,
        methods: vec!["tools/list".to_string()],
    });

    let mut group = c.benchmark_group("batching/enabled");
    group.throughput(Throughput::Elements(1));

    let counter = AtomicI64::new(0);
    group.bench_function("100ms_window", |b| {
        b.to_async(&rt).iter(|| async {
            let count = counter.fetch_add(1, Ordering::Relaxed);
            let request = mock_request(count);
            let _ = aggregator
                .submit_request(black_box("server1".to_string()), black_box(request))
                .await;
        });
    });

    group.finish();
}

/// Benchmark varying batch sizes
fn bench_varying_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("batching/sizes");

    for size in [1, 5, 10, 20, 50] {
        let aggregator = BatchAggregator::new(BatchingConfig {
            enabled: true,
            window_ms: 100,
            max_batch_size: size,
            methods: vec!["tools/list".to_string()],
        });

        group.throughput(Throughput::Elements(1));

        let counter = AtomicI64::new(0);
        group.bench_with_input(
            criterion::BenchmarkId::from_parameter(size),
            &size,
            |b, _| {
                b.to_async(&rt).iter(|| async {
                    let count = counter.fetch_add(1, Ordering::Relaxed);
                    let request = mock_request(count);
                    let _ = aggregator
                        .submit_request(black_box("server1".to_string()), black_box(request))
                        .await;
                });
            },
        );
    }

    group.finish();
}

/// Benchmark concurrent submissions
fn bench_concurrent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let aggregator = Arc::new(BatchAggregator::new(BatchingConfig {
        enabled: true,
        window_ms: 100,
        max_batch_size: 50,
        methods: vec!["tools/list".to_string()],
    }));

    let mut group = c.benchmark_group("batching/concurrent");
    group.throughput(Throughput::Elements(10)); // 10 concurrent clients

    group.bench_function("10_clients", |b| {
        b.to_async(&rt).iter(|| async {
            let mut handles = vec![];

            // Spawn 10 concurrent requests
            for i in 0..10 {
                let agg = Arc::clone(&aggregator);
                let handle = tokio::spawn(async move {
                    let request = mock_request(i);
                    let _ = agg.submit_request("server1".to_string(), request).await;
                });
                handles.push(handle);
            }

            // Wait for all to complete
            for handle in handles {
                let _ = handle.await;
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_no_batching,
    bench_batching_enabled,
    bench_varying_sizes,
    bench_concurrent
);
criterion_main!(benches);
