//! Health checking for backend servers
//!
//! This module provides comprehensive health checking functionality:
//! - Active health checks (periodic pings) - IMPLEMENTED in checker.rs
//! - Passive health checks (error rate monitoring) - IMPLEMENTED in checker.rs
//! - Circuit breaker pattern - IMPLEMENTED in circuit_breaker.rs
//! - Failure threshold configuration - IMPLEMENTED
//! - Automatic failover - IMPLEMENTED

pub mod checker;
pub mod circuit_breaker;
