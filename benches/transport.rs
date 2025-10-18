//! Transport Benchmarks
//!
//! This benchmark suite measures HTTP vs STDIO transport performance
//! with varying payload sizes.

use criterion::{criterion_group, criterion_main, Criterion};

fn bench_transport(_c: &mut Criterion) {
    // Implementation coming in Phase 4
}

criterion_group!(benches, bench_transport);
criterion_main!(benches);
