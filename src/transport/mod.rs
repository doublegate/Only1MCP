//! Transport layer implementations
//!
//! Supports multiple MCP transport protocols:
//! - STDIO (process-based)
//! - HTTP (request-response)
//! - SSE (server-sent events, deprecated)
//! - WebSocket (full-duplex)

pub mod stdio;
pub mod http;
pub mod sse;
pub mod websocket;

// TODO: Implement transport handlers
