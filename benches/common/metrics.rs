//! Measurement helpers for benchmarking
//!
//! This module provides utilities for measuring performance characteristics
//! beyond what Criterion provides out-of-the-box.

use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Measures the latency of a synchronous function
///
/// # Arguments
/// * `f` - The function to measure
///
/// # Returns
/// Duration of the function execution
///
/// # Example
/// ```ignore
/// let latency = measure_latency(|| {
///     expensive_operation();
/// });
/// println!("Latency: {:?}", latency);
/// ```
pub fn measure_latency<F, R>(f: F) -> Duration
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let _ = f();
    start.elapsed()
}

/// Measures the latency of an async function
///
/// # Arguments
/// * `runtime` - Tokio runtime to use
/// * `f` - The async function to measure
///
/// # Returns
/// Duration of the async function execution
pub fn measure_async_latency<F, Fut, R>(runtime: &tokio::runtime::Runtime, f: F) -> Duration
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let start = Instant::now();
    runtime.block_on(f());
    start.elapsed()
}

/// Tracks throughput (operations per second)
///
/// # Example
/// ```ignore
/// let tracker = ThroughputTracker::new();
/// for _ in 0..1000 {
///     operation();
///     tracker.increment();
/// }
/// let ops_per_sec = tracker.ops_per_second();
/// ```
pub struct ThroughputTracker {
    start: Instant,
    count: Arc<AtomicU64>,
}

impl ThroughputTracker {
    /// Creates a new throughput tracker
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increments the operation count
    pub fn increment(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns operations per second
    pub fn ops_per_second(&self) -> f64 {
        let elapsed = self.start.elapsed().as_secs_f64();
        let count = self.count.load(Ordering::Relaxed);
        count as f64 / elapsed
    }

    /// Returns total operations
    pub fn total_ops(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

impl Default for ThroughputTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Tracks cache hit rate
///
/// # Example
/// ```ignore
/// let tracker = CacheHitTracker::new();
/// tracker.record_hit();
/// tracker.record_miss();
/// let hit_rate = tracker.hit_rate();
/// ```
pub struct CacheHitTracker {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl CacheHitTracker {
    /// Creates a new cache hit tracker
    pub fn new() -> Self {
        Self {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Records a cache hit
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Records a cache miss
    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Returns hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    /// Returns total hits
    pub fn total_hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Returns total misses
    pub fn total_misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }
}

impl Default for CacheHitTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Measures memory allocation size
///
/// Note: This is a placeholder. Actual memory profiling should use
/// external tools like valgrind or heaptrack for production benchmarks.
///
/// # Returns
/// Estimated heap size (simplified)
pub fn estimate_heap_size<T>(item: &T) -> usize {
    std::mem::size_of_val(item)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measure_latency() {
        let latency = measure_latency(|| {
            std::thread::sleep(Duration::from_millis(10));
        });
        assert!(latency >= Duration::from_millis(10));
    }

    #[test]
    fn test_throughput_tracker() {
        let tracker = ThroughputTracker::new();
        for _ in 0..100 {
            tracker.increment();
        }
        assert_eq!(tracker.total_ops(), 100);
    }

    #[test]
    fn test_cache_hit_tracker() {
        let tracker = CacheHitTracker::new();
        tracker.record_hit();
        tracker.record_hit();
        tracker.record_miss();

        assert_eq!(tracker.total_hits(), 2);
        assert_eq!(tracker.total_misses(), 1);
        assert!((tracker.hit_rate() - 0.666).abs() < 0.01);
    }
}
