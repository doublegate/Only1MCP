//! Advanced routing algorithms

pub mod load_balancer;

// Re-export commonly used types
pub use load_balancer::{
    ConsistentHashRing, LoadBalancer, RoutingAlgorithm, RoutingConfig, ServerStats,
};
