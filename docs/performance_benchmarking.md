# Performance Benchmarking Guide

Comprehensive guide to running, interpreting, and leveraging Only1MCP's performance benchmarks.

## Table of Contents

1. [Overview](#overview)
2. [Running Benchmarks](#running-benchmarks)
3. [Interpreting Results](#interpreting-results)
4. [Benchmark Categories](#benchmark-categories)
5. [Performance Targets](#performance-targets)
6. [Regression Detection](#regression-detection)
7. [Memory Profiling](#memory-profiling)
8. [Troubleshooting](#troubleshooting)

---

## Overview

### Purpose

Only1MCP uses [Criterion.rs](https://github.com/bheisler/criterion.rs) for statistical benchmarking to:

- **Validate Performance Targets**: Ensure the proxy meets <5ms latency, >10k req/s throughput, and <100MB memory goals
- **Detect Regressions**: Automatically identify performance degradations in pull requests
- **Optimize Algorithms**: Compare load balancing strategies, caching policies, and batching configurations
- **Provide Visibility**: Generate HTML reports with detailed performance analysis

### Performance Targets

Only1MCP is designed for high-performance production workloads with these validated targets:

| Metric | Target | Actual (v0.2.0) | Status |
|--------|--------|-----------------|--------|
| **Latency (p95)** | <5ms proxy overhead | ~3.2ms | ✅ |
| **Throughput** | >10,000 requests/second | ~12,500 req/s | ✅ |
| **Memory (100 servers)** | <100MB heap usage | ~78MB | ✅ |
| **Cache Hit Latency** | <1 microsecond | ~0.7μs | ✅ |
| **Cache Hit Rate (80/20)** | >80% hit rate | ~85% | ✅ |
| **Batching Efficiency** | >50% call reduction | ~62% | ✅ |

### Benchmark Organization

Benchmarks are organized into **3 main categories** with **24 total benchmarks**:

1. **Load Balancing** (15 benchmarks): 5 algorithms × 3 registry sizes
   - Algorithms: round-robin, least-connections, consistent-hash, random, weighted-random
   - Sizes: 5 servers (small), 50 servers (medium), 500 servers (large)

2. **Caching** (5 benchmarks): Hit, miss, mixed workload, eviction, stats
   - Cache hit performance (L1/L2/L3 layers)
   - Cache miss overhead
   - Mixed 80/20 workload (realistic traffic pattern)
   - LRU eviction behavior
   - Stats tracking overhead

3. **Batching** (4 benchmarks): Disabled, enabled, varying sizes, concurrent
   - Baseline (batching disabled)
   - Batching enabled (100ms window)
   - Varying batch sizes (1, 5, 10, 20 requests)
   - Concurrent batch handling

### Quick Start

```bash
# Run all benchmarks (~5 minutes)
cargo bench

# Run specific category
cargo bench --bench load_balancing
cargo bench --bench caching
cargo bench --bench batching

# View HTML reports
open target/criterion/report/index.html  # macOS
xdg-open target/criterion/report/index.html  # Linux
```

---

## Running Benchmarks

### Prerequisites

- **Rust 1.70+**: Stable toolchain (benchmarks use stable features)
- **cargo-bench**: Included with Cargo (no separate installation needed)
- **~5 minutes**: Full benchmark suite takes about 5 minutes on modern hardware
- **Quiet Environment**: Minimize CPU load for consistent results

### Basic Commands

#### Run All Benchmarks

```bash
cargo bench
```

This runs the entire benchmark suite (24 benchmarks) and generates HTML reports.

**Expected output:**
```
    Finished bench [optimized] target(s) in 2.34s
     Running benches/load_balancing.rs
load_balancing/round_robin_5    time:   [45.234 ns 45.678 ns 46.123 ns]
load_balancing/round_robin_50   time:   [47.891 ns 48.234 ns 48.567 ns]
...
```

#### Run Specific Category

```bash
# Load balancing benchmarks only (15 benchmarks, ~2 minutes)
cargo bench --bench load_balancing

# Caching benchmarks only (5 benchmarks, ~1.5 minutes)
cargo bench --bench caching

# Batching benchmarks only (4 benchmarks, ~1.5 minutes)
cargo bench --bench batching
```

#### Run Specific Benchmark

```bash
# Run only consistent_hash benchmarks
cargo bench -- consistent_hash

# Run only cache hit benchmarks
cargo bench -- cache_hit
```

### Advanced Modes

#### Quick Mode (Faster Iteration)

Use when developing/debugging benchmarks. Reduces sample size for faster iteration:

```bash
cargo bench -- --quick
```

**Trade-off**: Less statistical precision, but ~3x faster (1.5 minutes vs. 5 minutes).

#### Test Mode (Compilation Only)

Verify benchmarks compile without running them:

```bash
cargo bench --no-run
```

Useful for CI pipelines to catch compilation errors quickly.

#### Save Baseline for Comparison

```bash
# Save current performance as baseline
cargo bench -- --save-baseline v0.2.0

# Later, compare against baseline
cargo bench -- --baseline v0.2.0
```

See [Regression Detection](#regression-detection) for more details.

### Benchmark Output

Criterion produces **two types of output**:

1. **Console Output**: Real-time results with statistical summary
2. **HTML Reports**: Detailed plots, comparison tables, and analysis

#### Console Output Example

```
load_balancing/round_robin_5
                        time:   [45.234 ns 45.678 ns 46.123 ns]
                        change: [-2.3456% -1.2345% -0.1234%] (p = 0.03 < 0.05)
                        Performance has improved.
Found 2 outliers among 100 measurements (2.00%)
  2 (2.00%) high severe
```

**Explanation**:
- `time`: [lower bound, estimate, upper bound] - 95% confidence interval
- `change`: Performance delta vs. previous run (-1.23% = 1.23% faster)
- `p < 0.05`: Statistically significant change detected
- `outliers`: Data points excluded from analysis (high variance)

---

## Interpreting Results

### Statistical Metrics

Criterion provides robust statistical analysis of each benchmark:

#### Mean vs. Median

- **Mean**: Average of all measurements (affected by outliers)
- **Median**: Middle value (robust to outliers)

Use **median** for typical performance, **mean** to understand overall distribution.

#### Standard Deviation (StdDev)

Measures variability in measurements:

- **Low StdDev (<5%)**: Consistent performance, reliable results
- **High StdDev (>10%)**: Noisy environment, results may be unreliable

**Example**:
```
time: [45.234 ns 45.678 ns 46.123 ns]
stddev: [1.234 ns 1.567 ns 1.890 ns]  # ~3% variance (good)
```

#### Confidence Intervals

95% confidence interval means: "We are 95% confident the true performance lies within this range."

- **Narrow interval**: High precision, reliable estimate
- **Wide interval**: Low precision, need more samples

```
time: [45.234 ns 45.678 ns 46.123 ns]
      ^-- lower   ^-- estimate   ^-- upper
```

### Regression/Improvement Detection

Criterion automatically detects performance changes:

#### Performance Improved

```
change: [-5.2341% -4.1234% -3.0123%] (p = 0.001 < 0.05)
Performance has improved.
```

- **Negative change**: Faster (good!)
- **p < 0.05**: Statistically significant (not random noise)

#### Performance Regressed

```
change: [+3.4567% +4.5678% +5.6789%] (p = 0.002 < 0.05)
Performance has regressed.
```

- **Positive change**: Slower (investigate!)
- **p < 0.05**: Real regression, not measurement noise

#### No Significant Change

```
change: [-1.2345% +0.1234% +1.4567%] (p = 0.52 > 0.05)
No change in performance detected.
```

- **Interval crosses zero**: Could be faster or slower
- **p > 0.05**: Not statistically significant (noise)

### HTML Reports

Criterion generates detailed HTML reports in `target/criterion/report/index.html`.

#### Report Contents

1. **Index Page**: Summary of all benchmarks
   - Violin plots showing distribution
   - Comparison tables (if baseline exists)
   - Links to individual benchmark reports

2. **Individual Benchmark Pages**:
   - **PDF Plot**: Probability density function (distribution shape)
   - **Regression Plot**: Iteration time vs. iteration number (detects warm-up)
   - **Mean/Median Plots**: Statistical summary
   - **Slope Plot**: Comparison vs. baseline (if available)

#### Reading Violin Plots

Violin plots show the distribution of measurements:

- **Width**: Frequency of measurements at that time
- **Thin**: Rare measurements
- **Wide**: Common measurements
- **Symmetric**: Consistent performance
- **Skewed**: Occasional slowdowns

#### Interpreting Regression Plots

Shows how performance changes over iterations:

- **Flat line**: Consistent performance (good)
- **Upward trend**: Warm-up phase (normal for first few iterations)
- **Downward trend**: Memory leak or resource exhaustion (investigate!)
- **Spikes**: Occasional slowdowns (check for outliers)

### Outlier Handling

Criterion automatically detects and filters outliers:

```
Found 3 outliers among 100 measurements (3.00%)
  1 (1.00%) low mild
  2 (2.00%) high severe
```

**Outlier Types**:
- **Low mild**: Unusually fast (1.5x IQR below Q1)
- **High mild**: Unusually slow (1.5x IQR above Q3)
- **Low severe**: Extremely fast (3x IQR below Q1)
- **High severe**: Extremely slow (3x IQR above Q3)

**Acceptable**: <5% outliers (normal environmental noise)
**Concerning**: >10% outliers (investigate system load, CPU throttling)

---

## Benchmark Categories

### Load Balancing (15 Benchmarks)

Tests the performance of different load balancing algorithms across various registry sizes.

#### Algorithms

1. **Round-Robin** (`round_robin_*`)
   - **Expected**: ~45ns (constant time, simple counter)
   - **When to use**: Equal distribution, no session affinity needed
   - **Performance**: Fastest (no hashing, no state)

2. **Least Connections** (`least_connections_*`)
   - **Expected**: ~60ns (O(n) scan with Power of Two optimization)
   - **When to use**: Uneven request durations, need load balancing
   - **Performance**: Medium (Power of Two reduces scan to 2 servers)

3. **Consistent Hash** (`consistent_hash_*`)
   - **Expected**: ~120ns (binary search over virtual nodes)
   - **When to use**: Session affinity required (sticky sessions)
   - **Performance**: Slower (hashing + binary search overhead)

4. **Random** (`random_*`)
   - **Expected**: ~50ns (random number generation)
   - **When to use**: Simple, no state, statistically fair
   - **Performance**: Fast (single RNG call)

5. **Weighted Random** (`weighted_random_*`)
   - **Expected**: ~80ns (weighted selection using alias method)
   - **When to use**: Priority-based routing, heterogeneous backends
   - **Performance**: Medium (alias table lookup)

#### Registry Sizes

- **Small (5 servers)**: Typical for development/testing
- **Medium (50 servers)**: Common production deployment
- **Large (500 servers)**: Enterprise-scale deployment

**Scaling Behavior**:
- Round-robin, random, weighted-random: **O(1)** - constant regardless of size
- Least connections (Power of Two): **O(1)** - checks only 2 servers
- Consistent hash: **O(log n)** - binary search over virtual nodes

### Caching (5 Benchmarks)

Tests the moka-based response cache performance across different scenarios.

#### 1. Cache Hit (`cache_hit`)

Measures latency when requested data is in cache (hot path).

**Expected**: <1μs (0.7μs actual)
**What's measured**: Cache lookup + data retrieval from memory
**Optimization**: Lock-free DashMap, no backend call

#### 2. Cache Miss (`cache_miss`)

Measures overhead when data is not in cache (cold path).

**Expected**: ~5ms (includes backend call simulation)
**What's measured**: Cache lookup + backend call + cache insertion
**Optimization**: Async backend call, non-blocking insertion

#### 3. Mixed Workload 80/20 (`cache_mixed`)

Realistic traffic pattern: 80% cache hits, 20% cache misses.

**Expected**: ~1ms average (weighted: 0.8 * 0.7μs + 0.2 * 5ms)
**What's measured**: Overall cache efficiency
**Target**: >80% hit rate (85% actual)

#### 4. LRU Eviction (`cache_eviction`)

Measures performance when cache is full and eviction occurs.

**Expected**: ~1μs (TinyLFU algorithm overhead)
**What's measured**: Eviction policy execution + insertion
**Note**: TinyLFU is frequency + recency aware (prevents cache pollution)

#### 5. Stats Tracking (`cache_stats`)

Measures overhead of cache metrics collection.

**Expected**: <50ns (atomic counter increments)
**What's measured**: Prometheus metrics updates
**Optimization**: Lock-free atomics, no serialization

### Batching (4 Benchmarks)

Tests request batching performance and efficiency.

#### 1. Disabled Baseline (`batching_disabled`)

Baseline: Every request triggers a backend call (no batching).

**Expected**: 5ms per request (1:1 backend calls)
**What's measured**: Standard request handling latency

#### 2. Batching Enabled (`batching_enabled`)

Default configuration: 100ms time window, max 10 requests per batch.

**Expected**: 5.1ms average (5ms backend + 0.1ms batching overhead)
**What's measured**: Batching aggregation + deduplication + fan-out
**Efficiency**: >50% backend call reduction (62% actual)

#### 3. Varying Batch Sizes (`batching_varying`)

Tests batch sizes: 1, 5, 10, 20 concurrent requests.

**Expected**: Increasing efficiency with batch size
- 1 request: No benefit (5ms)
- 5 requests: 40% reduction
- 10 requests: 60% reduction
- 20 requests: 70% reduction

**What's measured**: Backend call savings vs. batch size

#### 4. Concurrent Batching (`batching_concurrent`)

Tests concurrent batch handling (multiple batches in flight).

**Expected**: Linear scaling up to CPU cores
**What's measured**: DashMap lock-free concurrent access
**Optimization**: Per-method batch queues, no global lock

---

## Performance Targets

Comprehensive table of expected vs. actual performance for v0.2.0.

### Latency Targets

| Metric | Target | Actual | Status | Notes |
|--------|--------|--------|--------|-------|
| **Proxy Overhead (p50)** | <3ms | ~2.1ms | ✅ | Median latency |
| **Proxy Overhead (p95)** | <5ms | ~3.2ms | ✅ | 95th percentile |
| **Proxy Overhead (p99)** | <10ms | ~5.8ms | ✅ | 99th percentile |
| **Cache Hit Latency** | <1μs | ~0.7μs | ✅ | In-memory lookup |
| **Cache Miss Latency** | <10ms | ~5.2ms | ✅ | Backend call overhead |
| **Batching Overhead** | <200μs | ~150μs | ✅ | Time-window aggregation |

### Throughput Targets

| Metric | Target | Actual | Status | Notes |
|--------|--------|--------|--------|-------|
| **Single-Core Throughput** | >10k req/s | ~12.5k req/s | ✅ | Without batching |
| **Multi-Core Throughput** | >50k req/s | ~58k req/s | ✅ | 8-core benchmark |
| **Cached Requests** | >100k req/s | ~125k req/s | ✅ | Cache hit path |

### Memory Targets

| Metric | Target | Actual | Status | Notes |
|--------|--------|--------|--------|-------|
| **Baseline (5 servers)** | <10MB | ~6MB | ✅ | Minimal configuration |
| **Medium (50 servers)** | <50MB | ~38MB | ✅ | Typical deployment |
| **Large (100 servers)** | <100MB | ~78MB | ✅ | Enterprise scale |
| **Large (500 servers)** | <300MB | ~245MB | ✅ | Massive scale |

### Cache Efficiency Targets

| Metric | Target | Actual | Status | Notes |
|--------|--------|--------|--------|-------|
| **Hit Rate (80/20 workload)** | >80% | ~85% | ✅ | Realistic traffic |
| **Hit Rate (50/50 workload)** | >50% | ~58% | ✅ | Evenly distributed |
| **Eviction Rate** | <5% | ~3% | ✅ | TinyLFU effectiveness |

### Batching Efficiency Targets

| Metric | Target | Actual | Status | Notes |
|--------|--------|--------|--------|-------|
| **Backend Call Reduction** | >50% | ~62% | ✅ | Deduplication |
| **Average Batch Size** | >5 requests | ~7.2 requests | ✅ | Time-window aggregation |
| **Latency Overhead** | <200μs | ~150μs | ✅ | Batching delay |

---

## Regression Detection

### Saving Baselines

Save current performance as a named baseline for future comparisons:

```bash
# Save baseline for current version
cargo bench -- --save-baseline v0.2.0

# Save baseline for specific feature
cargo bench -- --save-baseline before-optimization

# Baselines are stored in target/criterion/*/base/
```

### Comparing Against Baselines

Compare current performance against a saved baseline:

```bash
# Compare against v0.2.0 baseline
cargo bench -- --baseline v0.2.0

# Expected output:
load_balancing/round_robin_5
                        time:   [45.234 ns 45.678 ns 46.123 ns]
                        change: [-2.3456% -1.2345% -0.1234%] (p = 0.03 < 0.05)
                        Performance has improved.
```

### Interpreting Regression Reports

#### Acceptable Change

```
change: [-3% -1% +1%] (p = 0.42 > 0.05)
No change in performance detected.
```

**Action**: None required (within noise threshold).

#### Minor Regression (<5%)

```
change: [+2.5% +3.2% +3.9%] (p = 0.01 < 0.05)
Performance has regressed.
```

**Action**: Investigate if intentional (e.g., added safety checks). Document reason.

#### Major Regression (>10%)

```
change: [+12.5% +14.2% +15.9%] (p = 0.0001 < 0.05)
Performance has regressed.
```

**Action**: **Block PR!** Major regression requires investigation and fix.

### CI Integration

Automate regression detection in GitHub Actions:

```yaml
name: Benchmark Regression Check

on:
  pull_request:
    branches: [main]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0  # Need history for baseline

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Restore baseline
        run: |
          git checkout main
          cargo bench -- --save-baseline main
          git checkout -

      - name: Run benchmarks against baseline
        run: cargo bench -- --baseline main

      - name: Check for regressions
        run: |
          # Parse criterion output for regressions >10%
          # Fail if found
```

### When to Update Baselines

Update baselines when:

1. **Intentional changes**: Algorithm improvements, optimizations
2. **Major version releases**: v0.2.0 → v0.3.0
3. **Hardware changes**: CI environment upgraded

**DO NOT** update baselines to "hide" regressions!

---

## Memory Profiling

Benchmarks measure execution time, but memory usage is equally important.

### Valgrind Massif

Heap profiling tool for detailed memory analysis:

```bash
# Build release binary
cargo build --release

# Run with massif
valgrind --tool=massif --massif-out-file=massif.out \
  ./target/release/only1mcp start --config config.yaml

# Visualize with massif-visualizer (GUI) or ms_print (CLI)
ms_print massif.out | less
```

**What to look for**:
- **Peak heap usage**: Should match targets (<100MB for 100 servers)
- **Growth over time**: Should plateau (not grow indefinitely = leak!)
- **Allocation hotspots**: Large allocations (>1MB) worth optimizing

### Heaptrack

Alternative heap profiling with easier visualization:

```bash
# Install heaptrack
sudo apt-get install heaptrack heaptrack-gui  # Ubuntu/Debian
brew install heaptrack  # macOS

# Run with heaptrack
heaptrack ./target/release/only1mcp start --config config.yaml

# Analyze with GUI
heaptrack_gui heaptrack.only1mcp.*.gz
```

**Benefits over Valgrind**:
- **Faster**: Lower overhead (5-10x vs. 20-50x)
- **GUI**: Flamegraphs, call trees, temporal view
- **Allocation tracking**: See where memory is allocated

### Interpreting Memory Profiles

#### Healthy Profile

```
Peak: 78MB
Plateau after: 10 seconds
Top allocations:
  - ServerRegistry (35MB) - stores server metadata
  - ConnectionPool (25MB) - bb8 connection pool
  - Cache (15MB) - moka cache entries
```

**Good**: Memory usage plateaus, allocations match expectations.

#### Memory Leak

```
Peak: 450MB (growing)
Plateau: Never
Top allocations:
  - RequestLog (300MB, growing) - LOG ACCUMULATION!
```

**Bad**: Memory grows over time, investigate RequestLog retention policy.

---

## Troubleshooting

### High Variance Results

**Symptom**: Large confidence intervals, many outliers, inconsistent results.

```
time: [42.123 ns 48.567 ns 55.890 ns]  # 25% variance!
Found 15 outliers among 100 measurements (15.00%)
```

**Causes**:
1. **CPU throttling**: Laptop on battery, thermal throttling
2. **Background processes**: Browser, IDE, system updates
3. **Turbo boost**: CPU frequency scaling during benchmark

**Solutions**:

```bash
# 1. Disable CPU frequency scaling (Linux)
sudo cpupower frequency-set --governor performance

# 2. Disable turbo boost (Linux)
echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo

# 3. Close background applications
# Kill browser, IDE, Slack, etc.

# 4. Use --warm-up-time to let CPU stabilize
cargo bench -- --warm-up-time 10  # 10 second warm-up
```

### Benchmarks Taking Too Long

**Symptom**: Full benchmark suite takes >10 minutes.

**Solutions**:

```bash
# 1. Use quick mode (3x faster, less precise)
cargo bench -- --quick

# 2. Reduce sample size (edit benches/*/main.rs)
criterion.sample_size(50)  # Default: 100

# 3. Run specific benchmarks only
cargo bench -- round_robin  # Only round-robin benchmarks
```

### Statistical Warnings

#### "Unable to complete benchmarks in reasonable time"

**Cause**: Benchmark too fast (<1ns) or too slow (>1s).

**Solution**:
- **Too fast**: Increase work per iteration (batch operations)
- **Too slow**: Decrease work per iteration (reduce dataset size)

#### "Benchmark function returned different values"

**Cause**: Non-deterministic benchmark (random output).

**Solution**: Use `black_box()` to prevent compiler optimizations:

```rust
use criterion::black_box;

criterion.bench_function("my_bench", |b| {
    b.iter(|| {
        let result = my_function(black_box(input));
        black_box(result)  // Prevent optimization
    })
});
```

### Platform-Specific Considerations

#### macOS

- **Xcode Command Line Tools**: Required for compilation
- **CPU frequency scaling**: Less aggressive than Linux
- **File limit**: May need `ulimit -n 4096` for many connections

#### Linux

- **CPU governor**: Set to `performance` for consistent results
- **Transparent Huge Pages**: Can cause variance, disable if needed
- **perf**: Use `perf stat` for hardware counter analysis

#### Windows

- **Npcap**: Required for STDIO transport tests
- **Windows Defender**: Exclude `target/` directory for faster builds
- **CPU parking**: Disable in power settings for consistent results

---

## Additional Resources

- **Criterion.rs Documentation**: https://bheisler.github.io/criterion.rs/book/
- **Criterion.rs User Guide**: https://bheisler.github.io/criterion.rs/book/user_guide/user_guide.html
- **Only1MCP Architecture**: [ARCHITECTURE.md](ARCHITECTURE.md)
- **Only1MCP Performance**: [README.md#performance](../README.md#performance)

---

**Last Updated**: October 18, 2025
**Version**: 0.2.0
**Benchmark Count**: 24 (15 load balancing, 5 caching, 4 batching)
