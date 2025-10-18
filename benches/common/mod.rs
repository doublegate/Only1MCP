//! Common utilities for benchmarking
//!
//! This module provides shared utilities used across all benchmark suites:
//! - Mock data generators for creating realistic test data
//! - Metrics helpers for measuring performance characteristics
//!
//! # Usage
//!
//! ```ignore
//! use common::mock::mock_registry;
//! use common::metrics::measure_latency;
//!
//! let registry = mock_registry(50);
//! let latency = measure_latency(|| {
//!     // operation to benchmark
//! });
//! ```

pub mod mock;
pub mod metrics;
