//! Load Balancing Benchmarks
//!
//! This benchmark suite measures the performance of different load balancing
//! algorithms across various registry sizes (5, 50, 500 servers).
//!
//! Benchmarks:
//! - Round-Robin (3 sizes)
//! - Least Connections (3 sizes)
//! - Consistent Hash (3 sizes)
//! - Random (3 sizes)
//! - Weighted Random (3 sizes)
//!
//! Total: 15 benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use only1mcp::routing::load_balancer::{HashKey, LoadBalancer, RoutingAlgorithm, RoutingConfig};
use tokio::runtime::Runtime;

/// Helper to create routing config for a specific algorithm
fn routing_config(algorithm: RoutingAlgorithm) -> RoutingConfig {
    RoutingConfig {
        algorithm,
        virtual_nodes: 150,
        hash_key: HashKey::ToolName,
        sticky_sessions: false,
        session_ttl: 3600,
    }
}

/// Generate mock server IDs
fn mock_servers(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("server-{}", i)).collect()
}

/// Benchmark round-robin algorithm across different registry sizes
fn bench_round_robin(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("load_balancing/round_robin");

    for size in [5, 50, 500] {
        let servers = mock_servers(size);
        let lb = LoadBalancer::new(routing_config(RoutingAlgorithm::RoundRobin));

        // Add servers to load balancer
        for server_id in &servers {
            lb.add_server(server_id);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                // Benchmark the routing decision
                let _ = lb.select_server(black_box("tools/list"), black_box(&servers), None).await;
            });
        });
    }

    group.finish();
}

/// Benchmark least connections algorithm
fn bench_least_connections(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("load_balancing/least_connections");

    for size in [5, 50, 500] {
        let servers = mock_servers(size);
        let lb = LoadBalancer::new(routing_config(RoutingAlgorithm::LeastConnections));

        // Add servers to load balancer
        for server_id in &servers {
            lb.add_server(server_id);
            // Simulate some connection activity for realism
            if server_id.ends_with('0') || server_id.ends_with('2') {
                lb.update_health(server_id, true, std::time::Duration::from_millis(50));
            }
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let _ = lb.select_server(black_box("tools/list"), black_box(&servers), None).await;
            });
        });
    }

    group.finish();
}

/// Benchmark consistent hash algorithm
fn bench_consistent_hash(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("load_balancing/consistent_hash");

    for size in [5, 50, 500] {
        let servers = mock_servers(size);
        let lb = LoadBalancer::new(routing_config(RoutingAlgorithm::ConsistentHash));

        // Add servers to hash ring
        for server_id in &servers {
            lb.add_server(server_id);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                // Use varying keys to test hash distribution
                let key = format!("request-{}", black_box(12345));
                let _ = lb.select_server(black_box(&key), black_box(&servers), None).await;
            });
        });
    }

    group.finish();
}

/// Benchmark random algorithm
fn bench_random(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("load_balancing/random");

    for size in [5, 50, 500] {
        let servers = mock_servers(size);
        let lb = LoadBalancer::new(routing_config(RoutingAlgorithm::Random));

        for server_id in &servers {
            lb.add_server(server_id);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let _ = lb.select_server(black_box("tools/list"), black_box(&servers), None).await;
            });
        });
    }

    group.finish();
}

/// Benchmark weighted random algorithm
fn bench_weighted_random(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("load_balancing/weighted_random");

    for size in [5, 50, 500] {
        let servers = mock_servers(size);
        let lb = LoadBalancer::new(routing_config(RoutingAlgorithm::WeightedRandom));

        // Add servers with implicit equal weights (actual weighted routing would need
        // server configuration with weights, which requires registry setup)
        for server_id in &servers {
            lb.add_server(server_id);
        }

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.to_async(&rt).iter(|| async {
                let _ = lb.select_server(black_box("tools/list"), black_box(&servers), None).await;
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_round_robin,
    bench_least_connections,
    bench_consistent_hash,
    bench_random,
    bench_weighted_random
);
criterion_main!(benches);
