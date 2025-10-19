//! Transport layer implementations
//!
//! Supports multiple MCP transport protocols:
//! - STDIO (process-based)
//! - HTTP (request-response)
//! - SSE (server-sent events, legacy)
//! - Streamable HTTP (modern MCP 2025-03-26 specification)
//! - WebSocket (full-duplex)

pub mod http;
pub mod sse;
pub mod stdio;
pub mod streamable_http;
pub mod websocket;

// Re-export commonly used types
pub use streamable_http::{StreamableHttpConfig, StreamableHttpTransport, StreamableHttpTransportPool};
