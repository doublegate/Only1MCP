//! Transport Benchmarks
//!
//! This benchmark suite measures serialization/deserialization performance
//! for different payload sizes, which is the core overhead of any transport layer.
//!
//! Benchmarks:
//! - Request serialization (small, medium, large)
//! - Response deserialization (small, medium, large)
//! - Round-trip serialization
//!
//! Total: 5 benchmarks

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use only1mcp::types::{McpRequest, McpResponse};
use serde_json::json;

/// Create mock request with specified payload size
fn mock_request_with_payload(size: usize) -> McpRequest {
    let large_string = "x".repeat(size);
    McpRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": "test_tool",
            "arguments": {
                "data": large_string
            }
        })),
    }
}

/// Create mock response with specified payload size
fn mock_response_with_payload(size: usize) -> McpResponse {
    let large_string = "x".repeat(size);
    McpResponse {
        jsonrpc: "2.0".to_string(),
        id: Some(json!(1)),
        result: Some(json!({
            "data": large_string
        })),
        error: None,
    }
}

/// Benchmark request serialization
fn bench_request_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/request_serialize");

    for size in [100, 1000, 10000, 50000] {
        let request = mock_request_with_payload(size);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let json = serde_json::to_string(black_box(&request)).unwrap();
                black_box(json);
            });
        });
    }

    group.finish();
}

/// Benchmark response deserialization
fn bench_response_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/response_deserialize");

    for size in [100, 1000, 10000, 50000] {
        let response = mock_response_with_payload(size);
        let json_str = serde_json::to_string(&response).unwrap();
        group.throughput(Throughput::Bytes(json_str.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let _resp: McpResponse =
                    serde_json::from_str(black_box(&json_str)).unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark round-trip (serialize + deserialize)
fn bench_round_trip(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/round_trip");

    for size in [100, 1000, 10000] {
        let request = mock_request_with_payload(size);
        group.throughput(Throughput::Bytes((size * 2) as u64)); // 2x for both directions

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                // Serialize request
                let json = serde_json::to_string(black_box(&request)).unwrap();
                // Deserialize
                let _parsed: McpRequest = serde_json::from_str(&json).unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark JSON to Vec<u8> conversion (for caching)
fn bench_json_to_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/json_to_bytes");

    for size in [100, 1000, 10000] {
        let response = mock_response_with_payload(size);
        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let bytes = serde_json::to_vec(black_box(&response)).unwrap();
                black_box(bytes);
            });
        });
    }

    group.finish();
}

/// Benchmark Vec<u8> to JSON conversion (from cache)
fn bench_bytes_to_json(c: &mut Criterion) {
    let mut group = c.benchmark_group("transport/bytes_to_json");

    for size in [100, 1000, 10000] {
        let response = mock_response_with_payload(size);
        let bytes = serde_json::to_vec(&response).unwrap();
        group.throughput(Throughput::Bytes(bytes.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let _resp: McpResponse = serde_json::from_slice(black_box(&bytes)).unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_request_serialization,
    bench_response_deserialization,
    bench_round_trip,
    bench_json_to_bytes,
    bench_bytes_to_json
);
criterion_main!(benches);
