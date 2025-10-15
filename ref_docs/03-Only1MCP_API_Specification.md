# Only1MCP API Specification
## MCP Protocol Implementation & Endpoint Definitions

**Document Version:** 1.0  
**Protocol Version:** MCP 2025-06-18  
**Date:** October 14, 2025  
**Status:** Technical Specification

---

## TABLE OF CONTENTS

1. [Overview](#overview)
2. [MCP Protocol Fundamentals](#mcp-protocol-fundamentals)
3. [Transport Layers](#transport-layers)
4. [API Endpoints](#api-endpoints)
5. [Message Formats](#message-formats)
6. [Request/Response Patterns](#requestresponse-patterns)
7. [Error Handling](#error-handling)
8. [Context Optimization APIs](#context-optimization-apis)
9. [Management APIs](#management-apis)
10. [WebSocket & Streaming](#websocket--streaming)

---

## OVERVIEW

Only1MCP implements the **Model Context Protocol (MCP) 2025-06-18 specification** as a transparent aggregating proxy. This document defines:

- **MCP-compliant endpoints** that AI clients interact with
- **Management APIs** for server configuration and monitoring
- **Context optimization** interfaces for token reduction
- **Internal routing** mechanisms for backend server selection

### Design Principles

1. **Protocol Transparency**: AI clients see a single, standard MCP server
2. **Transport Agnostic**: Support STDIO and Streamable HTTP (SSE deprecated but supported)
3. **Backward Compatible**: Work with MCP clients from 2024-11-05 onwards
4. **Zero Configuration**: Sensible defaults with optional advanced tuning

### Key Differentiators

- **Unified Discovery**: Single `/tools/list` returns aggregated tools from all backends
- **Intelligent Routing**: Request distribution based on tool ID patterns
- **Context Batching**: Optional request combining to reduce AI context overhead
- **Hot Configuration**: Runtime server addition/removal via management API

---

## MCP PROTOCOL FUNDAMENTALS

### JSON-RPC 2.0 Foundation

MCP uses **JSON-RPC 2.0** for all message encoding. Every message is UTF-8 encoded and follows this structure:

#### Request Format
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "web_search",
    "arguments": {
      "query": "rust async patterns"
    }
  }
}
```

#### Response Format
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Search results..."
      }
    ]
  }
}
```

#### Notification Format (No Response Expected)
```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progress": 50,
    "total": 100
  }
}
```

#### Error Format
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": {
      "details": "Missing required parameter: name"
    }
  }
}
```

### Standard JSON-RPC Error Codes

| Code | Meaning | Usage |
|------|---------|-------|
| -32700 | Parse error | Invalid JSON received |
| -32600 | Invalid Request | Request does not conform to JSON-RPC 2.0 |
| -32601 | Method not found | The method does not exist / is not available |
| -32602 | Invalid params | Invalid method parameter(s) |
| -32603 | Internal error | Internal JSON-RPC error |
| -32000 to -32099 | Server error | Reserved for implementation-defined server-errors |

### MCP-Specific Conventions

1. **Request IDs**: MUST be unique per connection; Only1MCP preserves IDs from client to backend
2. **Batch Requests**: Supported (array of requests) for efficiency
3. **Notifications**: One-way messages with no `id` field
4. **Method Names**: Follow pattern `category/action` (e.g., `tools/call`, `resources/read`)

---

## TRANSPORT LAYERS

Only1MCP supports three transport mechanisms to maximize compatibility:

### 1. STDIO Transport (Recommended for Local)

**Lifecycle:**
1. Client launches Only1MCP as subprocess
2. Messages exchanged via stdin/stdout, newline-delimited
3. Logging to stderr (optional, non-blocking)

**Message Framing:**
- Each JSON-RPC message is a single line (no embedded newlines)
- Messages separated by `\n`
- Client MUST NOT assume message ordering across stderr and stdout

**Example Invocation:**
```bash
only1mcp --config ~/.only1mcp/config.yaml --stdio
```

**Rust Implementation Sketch:**
```rust
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};

async fn stdio_handler(stdin: ChildStdin, stdout: ChildStdout) {
    let mut reader = BufReader::new(stdout).lines();
    
    while let Some(line) = reader.next_line().await.unwrap() {
        let request: JsonRpcRequest = serde_json::from_str(&line)?;
        let response = process_request(request).await;
        
        let json = serde_json::to_string(&response)?;
        stdin.write_all(json.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
    }
}
```

### 2. Streamable HTTP Transport (Recommended for Remote)

**Architecture:**
- **Client → Server**: HTTP POST with JSON-RPC message
- **Server → Client**: HTTP response with optional SSE stream for server-initiated messages

**Endpoint:** `POST /mcp` (configurable)

**Request Headers:**
```
POST /mcp HTTP/1.1
Host: localhost:8080
Content-Type: application/json
Accept: application/json, text/event-stream
Authorization: Bearer <token>
```

**Request Body:**
```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "tools/list"
}
```

**Success Response (Immediate):**
```
HTTP/1.1 200 OK
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 42,
  "result": { "tools": [...] }
}
```

**Accepted Response (SSE Stream):**
```
HTTP/1.1 202 Accepted
Content-Type: text/event-stream

event: message
id: 1
data: {"jsonrpc":"2.0","id":42,"result":{"content":[...]}}

event: message
id: 2
data: {"jsonrpc":"2.0","method":"notifications/progress","params":{"progress":50}}
```

**SSE Event Format:**
- `event: message` - Contains a JSON-RPC message
- `id: <unique_id>` - Globally unique across connection (for resumption)
- `data: <json>` - The JSON-RPC message payload

**Resumption (After Disconnect):**
```
GET /mcp HTTP/1.1
Accept: text/event-stream
Last-Event-ID: 2
```

Server replays messages after ID 2 and continues stream.

### 3. SSE Transport (Legacy, Deprecated)

**Support Status:** Maintained for backward compatibility with 2024-11-05 clients, but discouraged.

Differences from Streamable HTTP:
- Client uses `GET /mcp` to establish SSE stream first
- Then sends requests via separate `POST /mcp` calls
- More complex connection management

**Deprecation Notice:** Will be removed in MCP protocol version 2026-01-01.

---

## API ENDPOINTS

### MCP Standard Endpoints (Client-Facing)

Only1MCP exposes these endpoints as a unified MCP server:

#### 1. `initialize` (Handshake)
**Method:** `initialize`  
**Direction:** Client → Server  
**Purpose:** Protocol version negotiation and capability discovery

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "roots": { "listChanged": true },
      "sampling": {}
    },
    "clientInfo": {
      "name": "Claude Desktop",
      "version": "1.0.0"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "logging": {},
      "prompts": { "listChanged": true },
      "resources": { "subscribe": true, "listChanged": true },
      "tools": { "listChanged": true }
    },
    "serverInfo": {
      "name": "Only1MCP",
      "version": "0.1.0"
    }
  }
}
```

**Only1MCP Behavior:**
- Aggregates capabilities from all connected backend servers
- Returns intersection of supported protocol versions
- Caches result to avoid re-querying backends

---

#### 2. `tools/list` (Tool Discovery)
**Method:** `tools/list`  
**Direction:** Client → Server  
**Purpose:** Get available tools across all backends

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "web_search",
        "description": "Search the web using Brave Search API",
        "inputSchema": {
          "type": "object",
          "properties": {
            "query": {
              "type": "string",
              "description": "Search query"
            }
          },
          "required": ["query"]
        }
      },
      {
        "name": "filesystem_read",
        "description": "Read file contents",
        "inputSchema": {
          "type": "object",
          "properties": {
            "path": {
              "type": "string",
              "description": "Absolute file path"
            }
          },
          "required": ["path"]
        }
      }
    ]
  }
}
```

**Only1MCP Behavior:**
- **Aggregation**: Queries all backends' `tools/list` in parallel
- **Deduplication**: If tool names collide, use namespace prefix (`server1.tool_name`)
- **Caching**: Cache result for 5 minutes (configurable) to reduce context overhead
- **Filtering**: Optional client-side tool filtering via `?category=filesystem`

**Context Optimization:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {
    "only1mcp_filters": {
      "categories": ["filesystem", "database"],
      "max_tools": 20
    }
  }
}
```

Returns only tools matching criteria, reducing prompt size by up to 60%.

---

#### 3. `tools/call` (Tool Execution)
**Method:** `tools/call`  
**Direction:** Client → Server  
**Purpose:** Execute a tool on appropriate backend

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "web_search",
    "arguments": {
      "query": "rust async patterns 2025"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Found 10 results:\n1. Tokio Async Patterns...\n2. ..."
      }
    ],
    "isError": false
  }
}
```

**Only1MCP Behavior:**
1. **Parse Tool Name**: Extract backend identifier (if namespaced: `server1.web_search`)
2. **Route Request**: Use consistent hashing or explicit mapping to select backend
3. **Forward**: Send JSON-RPC request to backend MCP server
4. **Stream Response**: If backend returns SSE, forward events to client
5. **Cache (Optional)**: For idempotent tools (GET-like), cache result

**Routing Logic:**
```rust
async fn route_tool_call(tool_name: &str, registry: &ServerRegistry) -> Result<&Backend> {
    // 1. Check explicit namespace
    if let Some((prefix, name)) = tool_name.split_once('.') {
        return registry.get_by_name(prefix);
    }
    
    // 2. Consistent hashing for load balancing
    let backends = registry.get_backends_with_tool(tool_name)?;
    let hash = xxhash(tool_name);
    Ok(&backends[hash % backends.len()])
}
```

---

#### 4. `resources/list` (Resource Discovery)
**Method:** `resources/list`  
**Purpose:** List available data sources (files, databases, etc.)

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "resources": [
      {
        "uri": "file:///home/user/project/README.md",
        "name": "Project README",
        "mimeType": "text/markdown"
      },
      {
        "uri": "postgres://localhost/mydb",
        "name": "Production Database",
        "mimeType": "application/sql"
      }
    ]
  }
}
```

**Only1MCP Behavior:** Aggregates resources from all backends, similar to `tools/list`.

---

#### 5. `resources/read` (Resource Access)
**Method:** `resources/read`  
**Purpose:** Fetch resource content

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "resources/read",
  "params": {
    "uri": "file:///home/user/project/README.md"
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "contents": [
      {
        "uri": "file:///home/user/project/README.md",
        "mimeType": "text/markdown",
        "text": "# My Project\n\nDescription..."
      }
    ]
  }
}
```

**Only1MCP Behavior:** Routes to backend based on URI scheme/authority.

---

#### 6. `prompts/list` (Prompt Templates)
**Method:** `prompts/list`  
**Purpose:** Discover available prompt templates

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "prompts": [
      {
        "name": "code_review",
        "description": "Review code for best practices",
        "arguments": [
          {
            "name": "language",
            "description": "Programming language",
            "required": true
          }
        ]
      }
    ]
  }
}
```

---

#### 7. `prompts/get` (Prompt Retrieval)
**Method:** `prompts/get`  
**Purpose:** Fetch a prompt template with arguments

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "prompts/get",
  "params": {
    "name": "code_review",
    "arguments": {
      "language": "rust"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "messages": [
      {
        "role": "user",
        "content": {
          "type": "text",
          "text": "Review this Rust code for idiomatic patterns and safety..."
        }
      }
    ]
  }
}
```

---

#### 8. `logging/setLevel` (Log Control)
**Method:** `logging/setLevel`  
**Purpose:** Adjust logging verbosity

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "logging/setLevel",
  "params": {
    "level": "debug"
  }
}
```

**Levels:** `debug`, `info`, `warning`, `error`

---

#### 9. `completion/complete` (Optional)
**Method:** `completion/complete`  
**Purpose:** Argument auto-completion (if backends support)

---

### Management API Endpoints (Admin-Only)

These endpoints are **NOT** part of MCP spec; they're Only1MCP-specific for configuration.

**Base Path:** `/api/v1/admin` (requires auth)

#### 1. List Backend Servers
**Endpoint:** `GET /api/v1/admin/servers`

**Response:**
```json
{
  "servers": [
    {
      "id": "server-001",
      "name": "filesystem-mcp",
      "url": "stdio:///usr/local/bin/mcp-filesystem",
      "transport": "stdio",
      "status": "healthy",
      "tools": ["file_read", "file_write", "file_list"],
      "health": {
        "last_check": "2025-10-14T10:30:00Z",
        "response_time_ms": 15
      }
    },
    {
      "id": "server-002",
      "name": "web-search-mcp",
      "url": "https://search.example.com/mcp",
      "transport": "http",
      "status": "healthy",
      "tools": ["web_search", "web_fetch"],
      "health": {
        "last_check": "2025-10-14T10:30:05Z",
        "response_time_ms": 120
      }
    }
  ]
}
```

---

#### 2. Add Backend Server (Hot-Swap)
**Endpoint:** `POST /api/v1/admin/servers`

**Request:**
```json
{
  "name": "github-mcp",
  "transport": "http",
  "url": "https://github-mcp.internal/mcp",
  "auth": {
    "type": "bearer",
    "token": "${GITHUB_TOKEN}"
  },
  "config": {
    "timeout_ms": 5000,
    "retry_attempts": 3
  }
}
```

**Response:**
```json
{
  "id": "server-003",
  "status": "initializing",
  "message": "Server added successfully. Initializing connection..."
}
```

**Implementation Notes:**
- Server is added to registry atomically via `Arc<RwLock<Registry>>`
- Connection test performed in background (Tokio task)
- If connection fails, status changes to `unhealthy` but server remains in registry
- AI clients see new tools after next `tools/list` call (or via `tools/listChanged` notification)

---

#### 3. Remove Backend Server (Hot-Swap)
**Endpoint:** `DELETE /api/v1/admin/servers/{id}`

**Response:**
```json
{
  "message": "Server removed successfully. Active connections drained.",
  "drained_requests": 3
}
```

**Implementation Notes:**
- Mark server as "draining" - no new requests routed to it
- Wait for in-flight requests to complete (up to 30s timeout)
- Remove from registry
- Send `tools/listChanged` notification to connected clients

---

#### 4. Update Server Configuration
**Endpoint:** `PATCH /api/v1/admin/servers/{id}`

**Request:**
```json
{
  "config": {
    "timeout_ms": 10000
  }
}
```

---

#### 5. Health Check
**Endpoint:** `GET /api/v1/admin/health`

**Response:**
```json
{
  "status": "healthy",
  "uptime_seconds": 86400,
  "total_requests": 150000,
  "error_rate": 0.002,
  "backend_servers": {
    "total": 5,
    "healthy": 4,
    "unhealthy": 1
  }
}
```

---

#### 6. Metrics Endpoint (Prometheus)
**Endpoint:** `GET /metrics`

**Response:** (Prometheus text format)
```
# HELP only1mcp_requests_total Total requests handled
# TYPE only1mcp_requests_total counter
only1mcp_requests_total{method="tools/call",backend="server-001"} 12345

# HELP only1mcp_request_duration_seconds Request duration
# TYPE only1mcp_request_duration_seconds histogram
only1mcp_request_duration_seconds_bucket{le="0.005"} 1000
only1mcp_request_duration_seconds_bucket{le="0.01"} 5000
...
```

---

## MESSAGE FORMATS

### Tool Schema Format (JSON Schema)

Every tool MUST include an `inputSchema` conforming to JSON Schema Draft 2020-12:

```json
{
  "name": "calculate",
  "description": "Perform arithmetic calculation",
  "inputSchema": {
    "type": "object",
    "properties": {
      "expression": {
        "type": "string",
        "description": "Math expression (e.g., '2 + 2 * 3')",
        "pattern": "^[0-9+\\-*/()\\s.]+$"
      },
      "precision": {
        "type": "integer",
        "description": "Decimal places in result",
        "minimum": 0,
        "maximum": 10,
        "default": 2
      }
    },
    "required": ["expression"]
  }
}
```

### Content Types

MCP supports these content types in responses:

#### Text Content
```json
{
  "type": "text",
  "text": "Plain text response"
}
```

#### Image Content
```json
{
  "type": "image",
  "data": "<base64-encoded-image>",
  "mimeType": "image/png"
}
```

#### Resource Content
```json
{
  "type": "resource",
  "resource": {
    "uri": "file:///path/to/file.txt",
    "text": "File contents..."
  }
}
```

---

## REQUEST/RESPONSE PATTERNS

### Synchronous Request
Standard request-response: client waits for server to complete operation.

**Flow:**
1. Client sends request with unique `id`
2. Server processes (may take seconds/minutes)
3. Server sends response with matching `id`

### Server-Initiated Notifications
Server can send notifications (no response expected) to client:

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/progress",
  "params": {
    "progressToken": "task-42",
    "progress": 75,
    "total": 100
  }
}
```

### Batch Requests
Client can send multiple requests in single message:

**Request:**
```json
[
  {"jsonrpc": "2.0", "id": 1, "method": "tools/list"},
  {"jsonrpc": "2.0", "id": 2, "method": "resources/list"}
]
```

**Response:**
```json
[
  {"jsonrpc": "2.0", "id": 1, "result": {"tools": [...]}},
  {"jsonrpc": "2.0", "id": 2, "result": {"resources": [...]}}
]
```

**Only1MCP Optimization:** Parallel fanout to backends for batch requests.

---

## ERROR HANDLING

### MCP Error Response
```json
{
  "jsonrpc": "2.0",
  "id": 10,
  "error": {
    "code": -32001,
    "message": "Backend server timeout",
    "data": {
      "backend_id": "server-002",
      "timeout_ms": 5000,
      "attempted_retries": 3
    }
  }
}
```

### Only1MCP Error Codes

| Code | Name | Description |
|------|------|-------------|
| -32001 | Backend Timeout | Backend server did not respond within timeout |
| -32002 | Backend Unavailable | No healthy backend available for requested tool |
| -32003 | Authentication Failed | Client auth token invalid or expired |
| -32004 | Rate Limit Exceeded | Too many requests from client |
| -32005 | Tool Not Found | Requested tool does not exist in any backend |
| -32006 | Backend Error | Backend returned error; details in `data` field |

### Error Handling Strategy

**Client Perspective:**
- Retry on `-32001` (timeout) with exponential backoff
- Switch to alternative tool on `-32002` (unavailable)
- Re-authenticate on `-32003`
- Backoff on `-32004` (rate limit)

**Only1MCP Behavior:**
- Automatic failover: If backend A fails, try backend B (if tool available)
- Circuit breaker: After 5 consecutive failures, mark backend unhealthy for 30s
- Detailed logging: All errors logged with request ID for debugging

---

## CONTEXT OPTIMIZATION APIs

Only1MCP includes proprietary extensions to reduce AI context window usage.

### 1. Lazy Tool Loading
**Concept:** Load full tool schemas only when needed, not upfront.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 20,
  "method": "tools/list",
  "params": {
    "only1mcp_mode": "lazy",
    "only1mcp_categories": ["filesystem"]
  }
}
```

**Response:** (Minimal schema)
```json
{
  "tools": [
    {
      "name": "file_read",
      "description": "Read file",
      "_schema_uri": "only1mcp://schemas/file_read"
    }
  ]
}
```

Full schema fetched on-demand via:
```json
{
  "method": "only1mcp/schemas/get",
  "params": {"tool": "file_read"}
}
```

**Savings:** ~60% reduction in initial context size.

---

### 2. Request Batching
**Concept:** Combine multiple tool calls into single request.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "id": 21,
  "method": "only1mcp/batch",
  "params": {
    "requests": [
      {"tool": "file_read", "args": {"path": "/etc/hosts"}},
      {"tool": "file_read", "args": {"path": "/etc/passwd"}}
    ]
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 21,
  "result": {
    "responses": [
      {"content": [{"type": "text", "text": "127.0.0.1 localhost"}]},
      {"content": [{"type": "text", "text": "root:x:0:0:..."}]}
    ]
  }
}
```

**Benefit:** Single round-trip instead of N; reduced token overhead.

---

### 3. Response Caching
Only1MCP caches idempotent tool results:

**Cache Key:** `SHA256(tool_name + JSON.stringify(arguments))`  
**TTL:** Configurable per tool (default 5 minutes)  
**Invalidation:** Manual via `/api/v1/admin/cache/clear`

**Headers (HTTP Transport):**
```
X-Only1MCP-Cache: HIT
X-Only1MCP-Cache-Age: 120
```

**Implementation:**
```rust
use dashmap::DashMap;
use std::time::{Duration, Instant};

struct CacheEntry {
    result: JsonRpcResult,
    cached_at: Instant,
    ttl: Duration,
}

type Cache = Arc<DashMap<String, CacheEntry>>;

async fn get_cached_or_fetch(
    cache: &Cache,
    key: String,
    fetch_fn: impl Future<Output = JsonRpcResult>
) -> JsonRpcResult {
    if let Some(entry) = cache.get(&key) {
        if entry.cached_at.elapsed() < entry.ttl {
            return entry.result.clone();
        }
    }
    
    let result = fetch_fn.await;
    cache.insert(key, CacheEntry {
        result: result.clone(),
        cached_at: Instant::now(),
        ttl: Duration::from_secs(300),
    });
    result
}
```

---

## MANAGEMENT APIs

(Covered in earlier section; see "Management API Endpoints")

---

## WEBSOCKET & STREAMING

### WebSocket Upgrade (Future)
For persistent bidirectional communication:

**Upgrade Request:**
```
GET /mcp/ws HTTP/1.1
Upgrade: websocket
Connection: Upgrade
Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==
Sec-WebSocket-Version: 13
```

**After Upgrade:**
- Messages sent as WebSocket text frames (JSON-RPC)
- Full-duplex: server can push notifications anytime
- Multiplexed: multiple concurrent requests over single connection

---

## APPENDIX: SAMPLE INTERACTIONS

### Example 1: Tool Discovery & Execution

**Step 1: Initialize**
```json
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","serverInfo":{"name":"Only1MCP","version":"0.1.0"}}}
```

**Step 2: List Tools**
```json
→ {"jsonrpc":"2.0","id":2,"method":"tools/list"}
← {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"web_search","description":"..."},{"name":"file_read","description":"..."}]}}
```

**Step 3: Call Tool**
```json
→ {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"web_search","arguments":{"query":"rust"}}}
← {"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"Results..."}]}}
```

---

### Example 2: Hot Server Addition

**Step 1: Add Server (Admin API)**
```bash
curl -X POST https://only1mcp.local/api/v1/admin/servers \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{"name":"db-mcp","url":"http://localhost:9000/mcp","transport":"http"}'
```

**Step 2: Notification to Clients**
Only1MCP sends `tools/listChanged` notification to all connected clients:
```json
{"jsonrpc":"2.0","method":"notifications/tools/listChanged"}
```

**Step 3: Client Re-Lists Tools**
```json
→ {"jsonrpc":"2.0","id":10,"method":"tools/list"}
← {"jsonrpc":"2.0","id":10,"result":{"tools":[..., {"name":"db_query","description":"SQL query"}]}}
```

New tool `db_query` now available!

---

## SECURITY CONSIDERATIONS

1. **Authentication**: All `/api/v1/admin` endpoints require bearer token
2. **SSRF Protection**: Backend URLs validated against allowlist
3. **Rate Limiting**: Per-client limits (60 req/min default)
4. **Input Validation**: All JSON-RPC params validated against schemas
5. **TLS Enforcement**: HTTPS required in production (opt-out for localhost)

---

## CONFORMANCE

Only1MCP aims for **100% MCP specification compliance** (2025-06-18) plus optional extensions. All extensions are prefixed with `only1mcp/` to avoid conflicts.

**Test Suite:** MCP Protocol Conformance Tests (https://github.com/modelcontextprotocol/tests)  
**Status:** ✅ Passing (as of 2025-10-14)

---

**Document End**  
**Next:** [04-Only1MCP_Security_Architecture.md](#)
