//! Authentication and authorization module
//!
//! Provides comprehensive authentication and authorization:
//! - JWT token validation - IMPLEMENTED in jwt.rs
//! - OAuth2 flow - IMPLEMENTED in oauth.rs
//! - RBAC permission checking - IMPLEMENTED in rbac.rs
//! - API key management - Phase 3 feature (planned)

pub mod jwt;
pub mod oauth;
pub mod rbac;
