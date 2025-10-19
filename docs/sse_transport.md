# SSE Transport Implementation Guide

## Overview

The SSE (Server-Sent Events) transport enables Only1MCP to communicate with MCP servers that return responses in SSE format instead of plain JSON. This is particularly important for services like Context7 that use SSE for their MCP endpoints.

## What is SSE?

Server-Sent Events is a text-based protocol where:
- Each message consists of `event:` and `data:` lines
- Multiple `data:` lines can be concatenated
- Messages are separated by empty lines
- JSON payloads are embedded in the `data:` fields

### Example SSE Response

```
event: message
data: {"jsonrpc":"2.0","result":{"tools":[...]}}

```

## Architecture

### Core Components

1. **SseTransport** - Handles individual SSE endpoint connections
   - Sends JSON-RPC requests via POST
   - Receives and parses SSE responses
   - Extracts JSON from SSE data fields

2. **SseTransportPool** - Manages multiple SSE transports
   - Caches transports by endpoint + headers
   - Provides connection pooling
   - Handles concurrent requests

3. **Integration** - Seamlessly works with existing infrastructure
   - Integrated into ProxyServer
   - Supported in all fetch handlers
   - Works with batching and caching

## Configuration

### YAML Configuration

```yaml
servers:
  - id: "context7"
    name: "Context7 MCP Server"
    enabled: true
    transport:
      type: "sse"  # Use SSE transport
      url: "https://mcp.context7.com/mcp"
      headers:
        Accept: "application/json, text/event-stream"
        Content-Type: "application/json"
    health_check:
      enabled: false
    weight: 100
```

### Programmatic Usage

```rust
use only1mcp::transport::sse::{SseTransport, SseTransportConfig};
use std::collections::HashMap;

// Create configuration
let mut headers = HashMap::new();
headers.insert("Accept".to_string(), "application/json, text/event-stream".to_string());
headers.insert("Content-Type".to_string(), "application/json".to_string());

let config = SseTransportConfig {
    base_url: "https://mcp.context7.com/mcp".to_string(),
    request_timeout: std::time::Duration::from_secs(30),
    headers,
};

// Create transport
let transport = SseTransport::new(config).await?;

// Send request
let request = McpRequest::new("tools/list", json!({}), Some(json!(1)));
let response = transport.send_request(&endpoint, request).await?;
```

## Features

### SSE Parsing

- **Multi-line data support**: Automatically concatenates multiple `data:` lines
- **Event type handling**: Parses `event:` lines (stored for future use)
- **Robust parsing**: Ignores SSE metadata fields (id, retry, etc.)
- **Error handling**: Clear error messages for malformed SSE

### Connection Management

- **Endpoint caching**: One transport per unique endpoint+headers combination
- **Header-aware caching**: Different headers create separate transports
- **Lock-free access**: Uses DashMap for concurrent transport access
- **Automatic cleanup**: Arc reference counting ensures cleanup

### Request Handling

- **Standard headers**: Automatically includes SSE-specific Accept header
- **Custom headers**: Supports per-request header customization
- **Timeout support**: Configurable request timeouts
- **Error propagation**: Detailed error types for debugging

## Testing

### Unit Tests (9 tests)

Located in `src/transport/sse.rs`:

1. `test_parse_single_line_sse` - Basic SSE parsing
2. `test_parse_multiline_data_sse` - Multi-line data concatenation
3. `test_parse_no_event_type` - Optional event type handling
4. `test_parse_invalid_sse_no_data` - Error handling for missing data
5. `test_parse_invalid_json` - JSON parsing error handling
6. `test_parse_with_extra_fields` - Ignoring SSE metadata
7. `test_parse_context7_format` - Real Context7 format
8. `test_transport_pool_caching` - Pool caching verification
9. `test_transport_pool_different_headers` - Header-based caching

Run with:
```bash
cargo test --lib transport::sse
```

### Integration Tests (6 tests)

Located in `tests/sse_transport.rs`:

1. `test_context7_tools_list` - Real Context7 integration (requires network)
2. `test_sse_pool_caching` - Pool behavior verification
3. `test_sse_pool_different_headers` - Different header handling
4. `test_sse_pool_send_request` - Convenience method testing (requires network)
5. `test_sse_error_handling_invalid_endpoint` - Error handling
6. `test_sse_error_handling_timeout` - Timeout handling

Run with:
```bash
cargo test --test sse_transport
cargo test --test sse_transport -- --ignored  # Include network tests
```

## Use Cases

### Context7 Integration

Context7 provides up-to-date library documentation via MCP:

```yaml
servers:
  - id: "context7"
    transport:
      type: "sse"
      url: "https://mcp.context7.com/mcp"
      headers:
        Accept: "application/json, text/event-stream"
```

Request example:
```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

Response:
```json
{
  "id": 1,
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {"name": "resolve-library-id", ...},
      {"name": "get-library-docs", ...}
    ]
  }
}
```

### Other SSE Servers

Any MCP server returning SSE format can be used:

```yaml
servers:
  - id: "custom-sse-server"
    transport:
      type: "sse"
      url: "https://your-server.com/mcp"
      headers:
        Authorization: "Bearer YOUR_TOKEN"
```

## Performance Characteristics

### Latency
- **Parsing overhead**: < 1ms for typical responses
- **Connection reuse**: Cached transports avoid connection overhead
- **No streaming overhead**: Single-response SSE is as fast as HTTP

### Memory
- **Transport cache**: ~1KB per unique endpoint+headers
- **No persistent connections**: SSE connections are request-scoped
- **Minimal overhead**: Only stores Arc pointers in DashMap

### Throughput
- **Concurrent requests**: Lock-free transport access
- **Connection pooling**: reqwest client handles internal pooling
- **No bottlenecks**: Linear scaling with backend capacity

## Error Handling

### Error Types

```rust
pub enum SseError {
    ConnectionFailed(String),      // Network or connection errors
    InvalidFormat(String),          // Malformed SSE response
    RequestFailed(reqwest::Error),  // HTTP request errors
    InvalidJson(String),            // JSON parsing errors
    Timeout(u64),                   // Request timeout
    ServerError(StatusCode, String),// HTTP error responses
}
```

### Common Errors

**No data in SSE response**:
```
Error: Invalid SSE format: No data found in SSE response
```
Solution: Verify server returns `data:` lines

**JSON parsing error**:
```
Error: Invalid JSON in SSE data: expected value at line 1 column 1
```
Solution: Check SSE data contains valid JSON

**Connection timeout**:
```
Error: Timeout after 30000ms
```
Solution: Increase `request_timeout` in config

## Troubleshooting

### SSE vs HTTP Detection

Only1MCP automatically routes requests based on transport type:

```yaml
# SSE transport
transport:
  type: "sse"

# HTTP transport
transport:
  type: "http"
```

### Headers Not Working

Ensure headers are in the correct format:

```yaml
headers:
  Accept: "application/json, text/event-stream"  # Correct
  # NOT: Accept: ["application/json", "text/event-stream"]
```

### Debugging

Enable debug logging:
```bash
RUST_LOG=only1mcp::transport::sse=debug cargo run
```

View raw SSE responses:
```rust
tracing::debug!("SSE response: {}", sse_text);
```

## Future Enhancements

### Streaming Support

Currently handles single-message SSE. Future versions could support:
- Multi-message streaming responses
- Real-time event streams
- Long-polling connections

### Advanced Features

Planned improvements:
- Automatic SSE detection (Content-Type header inspection)
- SSE connection pooling (persistent connections)
- Compression support (gzip/brotli for SSE)
- Retry logic with exponential backoff

## References

- [SSE Specification](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- [MCP Protocol](https://modelcontextprotocol.io/)
- [Context7 Documentation](https://context7.com/)
- [Only1MCP Architecture](./ARCHITECTURE.md)

## Contributing

When adding SSE transport features:

1. **Write tests first**: Add unit tests in `src/transport/sse.rs`
2. **Test with Context7**: Verify against real endpoint
3. **Update documentation**: Keep this guide current
4. **Consider edge cases**: Handle malformed SSE gracefully

## Support

For issues or questions:
- GitHub Issues: https://github.com/doublegate/Only1MCP/issues
- Documentation: https://only1mcp.dev
- MCP Protocol: https://modelcontextprotocol.io/

---

**Last Updated**: October 19, 2025
**Version**: 0.2.0
