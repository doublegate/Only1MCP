// Stress test benchmark for Only1MCP
// Tests proxy performance under high load

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use only1mcp::gui_simple::GuiBackend;
use std::time::Duration;
use tokio::runtime::Runtime;

// Benchmark dashboard operations
fn bench_dashboard_ops(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let backend = rt.block_on(async { GuiBackend::new() });

    let mut group = c.benchmark_group("dashboard_operations");

    // Benchmark get_dashboard
    group.bench_function("get_dashboard", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(backend.get_dashboard().await)
        })
    });

    // Benchmark get_servers
    group.bench_function("get_servers", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(backend.get_servers().await)
        })
    });

    // Benchmark get_stats
    group.bench_function("get_stats", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(backend.get_stats().await)
        })
    });

    group.finish();
}

// Benchmark metric operations
fn bench_metric_ops(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let backend = rt.block_on(async { GuiBackend::new() });

    let mut group = c.benchmark_group("metric_operations");

    // Benchmark single metric recording
    group.bench_function("record_single_metric", |b| {
        b.to_async(&rt).iter(|| async {
            backend.record_metric(black_box("test_metric".to_string()), black_box(42.0)).await;
        })
    });

    // Benchmark metric retrieval
    rt.block_on(async {
        backend.record_metric("test_metric".to_string(), 42.0).await;
    });

    group.bench_function("get_metric", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(backend.get_metric("test_metric").await)
        })
    });

    // Benchmark listing metrics
    group.bench_function("list_metrics", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(backend.list_metrics().await)
        })
    });

    group.finish();
}

// Benchmark concurrent operations
fn bench_concurrent_ops(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let backend = rt.block_on(async { GuiBackend::new() });

    let mut group = c.benchmark_group("concurrent_operations");

    for concurrent_tasks in [10, 50, 100, 500].iter() {
        group.throughput(Throughput::Elements(*concurrent_tasks as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrent_tasks),
            concurrent_tasks,
            |b, &size| {
                b.to_async(&rt).iter(|| async {
                    let tasks: Vec<_> = (0..size)
                        .map(|i| {
                            let backend = &backend;
                            async move {
                                backend
                                    .record_metric(format!("metric_{}", i), i as f64)
                                    .await;
                            }
                        })
                        .collect();

                    futures::future::join_all(tasks).await;
                });
            },
        );
    }

    group.finish();
}

// Benchmark server operations
fn bench_server_ops(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let backend = rt.block_on(async { GuiBackend::new() });

    let mut group = c.benchmark_group("server_operations");

    // Benchmark adding servers
    group.bench_function("add_server", |b| {
        b.to_async(&rt).iter(|| async {
            use only1mcp::gui_simple::ServerInfo;
            backend
                .add_server(black_box(ServerInfo {
                    id: "test-server".to_string(),
                    name: "Test Server".to_string(),
                    transport_type: "stdio".to_string(),
                    healthy: true,
                    enabled: true,
                }))
                .await;
        })
    });

    // Add some servers for testing
    rt.block_on(async {
        use only1mcp::gui_simple::ServerInfo;
        for i in 0..10 {
            backend
                .add_server(ServerInfo {
                    id: format!("server-{}", i),
                    name: format!("Server {}", i),
                    transport_type: "stdio".to_string(),
                    healthy: true,
                    enabled: true,
                })
                .await;
        }
    });

    // Benchmark updating server health
    group.bench_function("update_server_health", |b| {
        b.to_async(&rt).iter(|| async {
            backend
                .update_server_health(black_box("server-0"), black_box(true))
                .await;
        })
    });

    group.finish();
}

// Benchmark event operations
fn bench_event_ops(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let backend = rt.block_on(async { GuiBackend::new() });

    let mut group = c.benchmark_group("event_operations");

    // Benchmark event logging
    group.bench_function("log_event", |b| {
        b.to_async(&rt).iter(|| async {
            use chrono::Utc;
            use only1mcp::gui_simple::{EventSeverity, SystemEvent};
            backend
                .log_event(black_box(SystemEvent {
                    timestamp: Utc::now(),
                    event_type: "test_event".to_string(),
                    message: "Test message".to_string(),
                    severity: EventSeverity::Info,
                }))
                .await;
        })
    });

    // Add some events for testing
    rt.block_on(async {
        use chrono::Utc;
        use only1mcp::gui_simple::{EventSeverity, SystemEvent};
        for i in 0..100 {
            backend
                .log_event(SystemEvent {
                    timestamp: Utc::now(),
                    event_type: format!("event_{}", i),
                    message: format!("Message {}", i),
                    severity: EventSeverity::Info,
                })
                .await;
        }
    });

    // Benchmark getting events
    group.bench_function("get_events_50", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(backend.get_events(50).await)
        })
    });

    group.finish();
}

// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let backend = rt.block_on(async { GuiBackend::new() });

    let mut group = c.benchmark_group("memory_patterns");
    group.sample_size(50); // Reduce sample size for memory-intensive tests

    // Benchmark growing metric storage
    group.bench_function("fill_metric_to_1000_points", |b| {
        b.to_async(&rt).iter(|| async {
            for i in 0..1000 {
                backend.record_metric("test_metric".to_string(), i as f64).await;
            }
        })
    });

    // Benchmark growing event log
    group.bench_function("fill_events_to_500", |b| {
        b.to_async(&rt).iter(|| async {
            use chrono::Utc;
            use only1mcp::gui_simple::{EventSeverity, SystemEvent};
            for i in 0..500 {
                backend
                    .log_event(SystemEvent {
                        timestamp: Utc::now(),
                        event_type: format!("event_{}", i),
                        message: format!("Message {}", i),
                        severity: EventSeverity::Info,
                    })
                    .await;
            }
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);
    targets = bench_dashboard_ops,
              bench_metric_ops,
              bench_concurrent_ops,
              bench_server_ops,
              bench_event_ops,
              bench_memory_patterns
}
criterion_main!(benches);
