//! Authentication and authorization module
//!
//! Provides comprehensive authentication and authorization:
//! - JWT token validation - IMPLEMENTED in jwt.rs
//! - OAuth2 flow - IMPLEMENTED in oauth.rs
//! - RBAC permission checking - IMPLEMENTED in rbac.rs
//! - Middleware integration - IMPLEMENTED in middleware.rs
//! - HTTP handlers for token management - IMPLEMENTED in handlers.rs
//! - API key management - Phase 3 feature (planned)

pub mod handlers;
pub mod jwt;
pub mod middleware;
pub mod oauth;
pub mod rbac;
