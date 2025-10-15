# Only1MCP API Reference

## Overview

Only1MCP exposes both MCP protocol endpoints for AI clients and administrative endpoints for managing the proxy server. All endpoints follow JSON-RPC 2.0 specification unless otherwise noted.

## Base URLs

- **MCP Endpoints:** `http://localhost:8080/`
- **Admin API:** `http://localhost:8080/api/v1/admin/`
- **WebSocket:** `ws://localhost:8080/ws`
- **SSE Stream:** `http://localhost:8080/sse`

## Authentication

### API Key Authentication
```http
Authorization: Bearer <api_key>
```

### JWT Authentication
```http
Authorization: Bearer <jwt_token>
```

---

## MCP Protocol Endpoints

### Core JSON-RPC Endpoint

**POST** `/` or `/mcp`

Main endpoint for all MCP protocol requests using JSON-RPC 2.0.

#### Request Format
```json
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 1
}
```

#### Response Format
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [...]
  },
  "id": 1
}
```

### Tool Operations

#### List Available Tools
**POST** `/tools/list`

Returns all tools available across aggregated MCP servers.

```json
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "id": 1
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "github_search",
        "description": "Search GitHub repositories",
        "inputSchema": {
          "type": "object",
          "properties": {
            "query": {"type": "string"}
          }
        }
      }
    ]
  },
  "id": 1
}
```

#### Call a Tool
**POST** `/tools/call`

Execute a tool on one of the backend servers.

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "github_search",
    "arguments": {
      "query": "rust mcp"
    }
  },
  "id": 2
}
```

### Resource Operations

#### List Resources
**POST** `/resources/list`

List available resources across all servers.

```json
{
  "jsonrpc": "2.0",
  "method": "resources/list",
  "id": 3
}
```

#### Read Resource
**POST** `/resources/read`

Read content from a specific resource.

```json
{
  "jsonrpc": "2.0",
  "method": "resources/read",
  "params": {
    "uri": "file:///path/to/resource"
  },
  "id": 4
}
```

#### Subscribe to Resource Updates
**POST** `/resources/subscribe`

Subscribe to real-time updates for a resource.

```json
{
  "jsonrpc": "2.0",
  "method": "resources/subscribe",
  "params": {
    "uri": "file:///path/to/resource"
  },
  "id": 5
}
```

### Prompt Operations

#### List Prompts
**POST** `/prompts/list`

Get all available prompt templates.

```json
{
  "jsonrpc": "2.0",
  "method": "prompts/list",
  "id": 6
}
```

#### Get Prompt
**POST** `/prompts/get`

Retrieve a specific prompt template.

```json
{
  "jsonrpc": "2.0",
  "method": "prompts/get",
  "params": {
    "name": "code_review"
  },
  "id": 7
}
```

### Sampling Operations

#### Create Message
**POST** `/sampling/createMessage`

Create a message using the sampling API.

```json
{
  "jsonrpc": "2.0",
  "method": "sampling/createMessage",
  "params": {
    "messages": [
      {
        "role": "user",
        "content": "Hello"
      }
    ],
    "modelPreferences": {
      "hints": ["fast", "creative"]
    }
  },
  "id": 8
}
```

---

## WebSocket API

### Connection
**GET** `/ws`

Upgrade to WebSocket for bidirectional streaming communication.

#### Client Message Format
```json
{
  "type": "request",
  "id": "req_123",
  "method": "tools/call",
  "params": {...}
}
```

#### Server Message Format
```json
{
  "type": "response",
  "id": "req_123",
  "result": {...}
}
```

#### Server Push Notifications
```json
{
  "type": "notification",
  "event": "server_added",
  "data": {
    "server_id": "github",
    "status": "healthy"
  }
}
```

---

## Admin API Endpoints

### Server Management

#### List Servers
**GET** `/api/v1/admin/servers`

Get all configured MCP servers.

Response:
```json
{
  "servers": [
    {
      "id": "github",
      "name": "GitHub MCP",
      "transport": "stdio",
      "status": "healthy",
      "uptime": 3600
    }
  ]
}
```

#### Get Server Details
**GET** `/api/v1/admin/servers/:id`

Get detailed information about a specific server.

#### Add Server
**POST** `/api/v1/admin/servers`

Add a new MCP server to the proxy.

Request:
```json
{
  "id": "new_server",
  "name": "New MCP Server",
  "transport": {
    "type": "http",
    "url": "http://localhost:3000"
  },
  "health_check": {
    "enabled": true,
    "interval": 30
  }
}
```

#### Update Server
**PATCH** `/api/v1/admin/servers/:id`

Update server configuration.

#### Remove Server
**DELETE** `/api/v1/admin/servers/:id`

Remove a server from the proxy.

### Health and Metrics

#### Health Check
**GET** `/api/v1/admin/health`

Get proxy health status.

Response:
```json
{
  "status": "healthy",
  "uptime": 3600,
  "servers": {
    "total": 5,
    "healthy": 4,
    "unhealthy": 1
  }
}
```

#### Prometheus Metrics
**GET** `/api/v1/admin/metrics`

Export Prometheus-formatted metrics.

```
# HELP only1mcp_requests_total Total number of requests
# TYPE only1mcp_requests_total counter
only1mcp_requests_total{method="tools/call"} 1234

# HELP only1mcp_cache_hit_ratio Cache hit ratio
# TYPE only1mcp_cache_hit_ratio gauge
only1mcp_cache_hit_ratio 0.75
```

### Cache Management

#### Cache Statistics
**GET** `/api/v1/admin/cache/stats`

Get cache performance statistics.

Response:
```json
{
  "l1_entries": 523,
  "l2_entries": 187,
  "l3_entries": 42,
  "total_hits": 8934,
  "total_misses": 2341,
  "hit_rate": 79.2,
  "memory_used_mb": 23.5
}
```

#### Clear Cache
**POST** `/api/v1/admin/cache/clear`

Clear all cache entries.

### Configuration

#### Get Configuration
**GET** `/api/v1/admin/config`

Get current proxy configuration.

#### Update Configuration
**POST** `/api/v1/admin/config`

Update proxy configuration (requires restart for some changes).

#### Reload Configuration
**POST** `/api/v1/admin/config/reload`

Hot-reload configuration without restart.

---

## Error Responses

All errors follow JSON-RPC 2.0 error format:

```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32601,
    "message": "Method not found",
    "data": {
      "method": "unknown/method"
    }
  },
  "id": 1
}
```

### Error Codes

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Invalid JSON-RPC request |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid method parameters |
| -32603 | Internal error | Internal server error |
| -32000 | Server error | Generic server error |
| -32001 | No backend available | No healthy backend for request |
| -32002 | Timeout | Request timeout |
| -32003 | Rate limited | Too many requests |
| -32004 | Unauthorized | Authentication required |
| -32005 | Forbidden | Insufficient permissions |

---

## Rate Limiting

Default rate limits:
- **Global:** 10,000 requests/minute
- **Per Client:** 1,000 requests/minute
- **Per Method:** Varies (tools/call: 100/min, resources/list: 500/min)

Rate limit headers:
```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1634567890
```

---

## Streaming Responses

For long-running operations, responses can be streamed:

### Server-Sent Events (SSE)
```http
GET /sse?method=tools/call&params=...

data: {"partial": true, "content": "Processing..."}
data: {"partial": true, "content": "Still working..."}
data: {"partial": false, "result": {...}}
```

### Chunked Transfer Encoding
Large responses automatically use chunked encoding:
```http
Transfer-Encoding: chunked
```

---

## Examples

### Complete Tool Call Example

```bash
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "calculator",
      "arguments": {
        "expression": "2 + 2"
      }
    },
    "id": 1
  }'
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "4"
      }
    ]
  },
  "id": 1
}
```

### WebSocket Streaming Example

```javascript
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'request',
    id: 'req_1',
    method: 'tools/list'
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log('Received:', msg);
};
```