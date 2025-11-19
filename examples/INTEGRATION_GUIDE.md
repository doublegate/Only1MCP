# Integration Guide

This guide shows how to integrate Only1MCP with popular AI frameworks, languages, and platforms.

## Table of Contents

- [JavaScript/TypeScript](#javascripttypescript)
- [Python](#python)
- [Rust](#rust)
- [Claude Desktop](#claude-desktop)
- [OpenAI API](#openai-api)
- [LangChain](#langchain)
- [Custom Integration](#custom-integration)

---

## JavaScript/TypeScript

### Using the MCP Client SDK

```typescript
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

// Connect to Only1MCP proxy instead of direct MCP server
const transport = new StdioClientTransport({
  command: "curl",
  args: ["-X", "POST", "http://localhost:8080/mcp"],
  // Or use WebSocket transport for better performance
});

const client = new Client({
  name: "my-app",
  version: "1.0.0",
}, {
  capabilities: {
    tools: {}
  }
});

await client.connect(transport);

// List available tools from all MCP servers
const tools = await client.listTools();
console.log("Available tools:", tools);

// Call a tool
const result = await client.callTool({
  name: "read_file",
  arguments: {
    path: "/path/to/file.txt"
  }
});
```

### Using HTTP Directly

```javascript
const fetch = require('node-fetch');

async function callMCP(method, params) {
  const response = await fetch('http://localhost:8080/mcp', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer YOUR_JWT_TOKEN' // If auth enabled
    },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: Date.now(),
      method: method,
      params: params
    })
  });

  return await response.json();
}

// Example usage
const tools = await callMCP('tools/list', {});
console.log(tools);
```

### Using WebSocket for Real-time Updates

```javascript
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:8080/ws');

ws.on('open', () => {
  // Subscribe to events
  ws.send(JSON.stringify({
    type: 'subscribe',
    event_types: ['server_health_changed', 'metric_updated']
  }));
});

ws.on('message', (data) => {
  const event = JSON.parse(data);
  console.log('Received event:', event);
});
```

---

## Python

### Using HTTP Requests

```python
import requests
import json

class Only1MCPClient:
    def __init__(self, base_url="http://localhost:8080", token=None):
        self.base_url = base_url
        self.token = token
        self.session = requests.Session()
        if token:
            self.session.headers.update({
                'Authorization': f'Bearer {token}'
            })

    def call_mcp(self, method, params=None):
        """Call MCP JSON-RPC method"""
        payload = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params or {}
        }

        response = self.session.post(
            f"{self.base_url}/mcp",
            json=payload
        )
        response.raise_for_status()
        return response.json()

    def list_tools(self):
        """List all available tools"""
        return self.call_mcp("tools/list")

    def call_tool(self, name, arguments):
        """Call a specific tool"""
        return self.call_mcp("tools/call", {
            "name": name,
            "arguments": arguments
        })

    def get_dashboard(self):
        """Get dashboard data from GUI API"""
        response = self.session.get(f"{self.base_url}/api/gui/dashboard")
        return response.json()

# Usage
client = Only1MCPClient()

# List tools
tools = client.list_tools()
print("Available tools:", tools)

# Call a tool
result = client.call_tool("read_file", {
    "path": "/etc/hosts"
})
print("Result:", result)

# Get dashboard data
dashboard = client.get_dashboard()
print(f"Proxy status: {dashboard['is_running']}")
print(f"Active connections: {dashboard['active_connections']}")
```

### Using asyncio

```python
import aiohttp
import asyncio

class AsyncMCPClient:
    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url

    async def call_mcp(self, method, params=None):
        async with aiohttp.ClientSession() as session:
            payload = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": method,
                "params": params or {}
            }

            async with session.post(
                f"{self.base_url}/mcp",
                json=payload
            ) as response:
                return await response.json()

    async def batch_calls(self, calls):
        """Execute multiple MCP calls concurrently"""
        tasks = [
            self.call_mcp(method, params)
            for method, params in calls
        ]
        return await asyncio.gather(*tasks)

# Usage
async def main():
    client = AsyncMCPClient()

    # Concurrent calls
    results = await client.batch_calls([
        ("tools/list", {}),
        ("resources/list", {}),
        ("prompts/list", {})
    ])

    print(results)

asyncio.run(main())
```

---

## Rust

### Using the Library Directly

```rust
use only1mcp::{Config, ProxyServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = Config::load_from_file("only1mcp.yaml")?;

    // Create and start proxy server
    let server = ProxyServer::new(config);
    server.run().await?;

    Ok(())
}
```

### Using the GUI Backend

```rust
use only1mcp::gui_simple::{GuiBackend, ServerInfo};

#[tokio::main]
async fn main() {
    let backend = GuiBackend::new();

    // Register servers
    backend.add_server(ServerInfo {
        id: "mcp-1".to_string(),
        name: "My MCP Server".to_string(),
        transport_type: "stdio".to_string(),
        healthy: true,
        enabled: true,
    }).await;

    // Set running status
    backend.set_running(true).await;

    // Record metrics
    backend.record_metric("request_count".to_string(), 42.0).await;

    // Get dashboard data
    let dashboard = backend.get_dashboard().await;
    println!("Dashboard: {:?}", dashboard);
}
```

### Making HTTP Requests to Only1MCP

```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    // Call MCP method
    let response = client
        .post("http://localhost:8080/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }))
        .send()
        .await?;

    let result: serde_json::Value = response.json().await?;
    println!("Tools: {:#?}", result);

    Ok(())
}
```

---

## Claude Desktop

Configure Claude Desktop to use Only1MCP as a proxy:

### macOS Configuration

Edit `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "only1mcp-proxy": {
      "command": "curl",
      "args": [
        "-X", "POST",
        "http://localhost:8080/mcp",
        "-H", "Content-Type: application/json",
        "-d", "@-"
      ]
    }
  }
}
```

### Windows Configuration

Edit `%APPDATA%\Claude\claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "only1mcp-proxy": {
      "command": "curl.exe",
      "args": [
        "-X", "POST",
        "http://localhost:8080/mcp",
        "-H", "Content-Type: application/json",
        "-d", "@-"
      ]
    }
  }
}
```

---

## OpenAI API

Wrap Only1MCP tools in OpenAI function calling:

```python
import openai
import requests

def get_mcp_tools():
    """Fetch tools from Only1MCP"""
    response = requests.post(
        "http://localhost:8080/mcp",
        json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list",
            "params": {}
        }
    )
    return response.json()["result"]["tools"]

def call_mcp_tool(name, arguments):
    """Call MCP tool through Only1MCP"""
    response = requests.post(
        "http://localhost:8080/mcp",
        json={
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments
            }
        }
    )
    return response.json()["result"]

# Convert MCP tools to OpenAI functions format
mcp_tools = get_mcp_tools()
openai_functions = [
    {
        "name": tool["name"],
        "description": tool["description"],
        "parameters": tool["inputSchema"]
    }
    for tool in mcp_tools
]

# Use with OpenAI
response = openai.ChatCompletion.create(
    model="gpt-4",
    messages=[{"role": "user", "content": "Read /etc/hosts"}],
    functions=openai_functions,
    function_call="auto"
)

# If function call requested, execute via Only1MCP
if response.choices[0].message.get("function_call"):
    func = response.choices[0].message["function_call"]
    result = call_mcp_tool(
        func["name"],
        json.loads(func["arguments"])
    )
    print("MCP Result:", result)
```

---

## LangChain

Integrate Only1MCP with LangChain:

```python
from langchain.tools import Tool
from langchain.agents import initialize_agent, AgentType
from langchain.llms import OpenAI
import requests

class MCPToolWrapper:
    """Wrapper for MCP tools to use with LangChain"""

    def __init__(self, base_url="http://localhost:8080"):
        self.base_url = base_url
        self.tools = self._fetch_tools()

    def _fetch_tools(self):
        """Fetch available MCP tools"""
        response = requests.post(
            f"{self.base_url}/mcp",
            json={
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list",
                "params": {}
            }
        )
        return response.json()["result"]["tools"]

    def _call_tool(self, name, arguments):
        """Call an MCP tool"""
        response = requests.post(
            f"{self.base_url}/mcp",
            json={
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": name,
                    "arguments": arguments
                }
            }
        )
        return response.json()["result"]

    def to_langchain_tools(self):
        """Convert MCP tools to LangChain Tool objects"""
        langchain_tools = []

        for mcp_tool in self.tools:
            def make_func(tool_name):
                def func(arguments_str):
                    import json
                    args = json.loads(arguments_str)
                    return self._call_tool(tool_name, args)
                return func

            tool = Tool(
                name=mcp_tool["name"],
                func=make_func(mcp_tool["name"]),
                description=mcp_tool["description"]
            )
            langchain_tools.append(tool)

        return langchain_tools

# Usage
mcp_wrapper = MCPToolWrapper()
langchain_tools = mcp_wrapper.to_langchain_tools()

# Create LangChain agent with MCP tools
llm = OpenAI(temperature=0)
agent = initialize_agent(
    langchain_tools,
    llm,
    agent=AgentType.ZERO_SHOT_REACT_DESCRIPTION,
    verbose=True
)

# Use the agent
result = agent.run("Read the file /etc/hosts and summarize it")
print(result)
```

---

## Custom Integration

### Building a Custom Client

```python
class CustomMCPClient:
    """Example custom MCP client with advanced features"""

    def __init__(self, base_url, token=None):
        self.base_url = base_url
        self.token = token
        self.session = requests.Session()

        if token:
            self.session.headers.update({
                'Authorization': f'Bearer {token}'
            })

    def authenticate(self, username, password):
        """Get JWT token"""
        response = self.session.post(
            f"{self.base_url}/auth/login",
            json={"username": username, "password": password}
        )
        data = response.json()
        self.token = data["access_token"]
        self.session.headers.update({
            'Authorization': f'Bearer {self.token}'
        })

    def call_with_retry(self, method, params, max_retries=3):
        """Call with automatic retry on failure"""
        for attempt in range(max_retries):
            try:
                return self.call_mcp(method, params)
            except requests.exceptions.RequestException as e:
                if attempt == max_retries - 1:
                    raise
                time.sleep(2 ** attempt)  # Exponential backoff

    def stream_events(self, event_types=None):
        """Stream real-time events via WebSocket"""
        import websocket

        def on_message(ws, message):
            event = json.loads(message)
            self.handle_event(event)

        ws = websocket.WebSocketApp(
            f"ws://{self.base_url.replace('http://', '')}/api/gui/ws",
            on_message=on_message
        )

        # Subscribe to specific events
        if event_types:
            ws.send(json.dumps({
                "type": "subscribe",
                "event_types": event_types
            }))

        ws.run_forever()

    def handle_event(self, event):
        """Override this to handle events"""
        print(f"Event: {event}")

    def get_metrics(self, metric_names):
        """Get metrics for visualization"""
        response = self.session.post(
            f"{self.base_url}/api/gui/metrics",
            json={"names": metric_names}
        )
        return response.json()

    def export_audit_log(self, limit=1000):
        """Export audit log"""
        response = self.session.get(
            f"{self.base_url}/api/gui/audit/export",
            params={"limit": limit}
        )
        return response.json()
```

---

## Best Practices

### 1. Connection Pooling

```python
# Reuse HTTP session for better performance
session = requests.Session()
session.headers.update({'Content-Type': 'application/json'})

# Configure connection pool
adapter = requests.adapters.HTTPAdapter(
    pool_connections=10,
    pool_maxsize=20
)
session.mount('http://', adapter)
```

### 2. Error Handling

```python
def safe_call_mcp(method, params):
    try:
        response = call_mcp(method, params)
        if "error" in response:
            raise Exception(f"MCP Error: {response['error']}")
        return response["result"]
    except requests.exceptions.Timeout:
        print("Request timed out, using fallback")
        return fallback_response()
    except requests.exceptions.ConnectionError:
        print("Connection failed, retrying...")
        return retry_with_backoff(method, params)
```

### 3. Authentication Token Refresh

```python
def ensure_authenticated(func):
    """Decorator to refresh token if needed"""
    def wrapper(self, *args, **kwargs):
        try:
            return func(self, *args, **kwargs)
        except requests.exceptions.HTTPError as e:
            if e.response.status_code == 401:
                self.refresh_token()
                return func(self, *args, **kwargs)
            raise
    return wrapper
```

### 4. Caching Responses

```python
from functools import lru_cache
from datetime import datetime, timedelta

class CachedMCPClient:
    def __init__(self):
        self.cache = {}
        self.cache_ttl = timedelta(minutes=5)

    def call_with_cache(self, method, params):
        cache_key = f"{method}:{json.dumps(params, sort_keys=True)}"

        if cache_key in self.cache:
            cached_time, cached_result = self.cache[cache_key]
            if datetime.now() - cached_time < self.cache_ttl:
                return cached_result

        result = self.call_mcp(method, params)
        self.cache[cache_key] = (datetime.now(), result)
        return result
```

---

## Troubleshooting

### Connection Issues

```python
# Test connection
try:
    response = requests.get('http://localhost:8080/health', timeout=5)
    print(f"Proxy healthy: {response.status_code == 200}")
except requests.exceptions.ConnectionError:
    print("Cannot connect to Only1MCP. Is it running?")
```

### Authentication Issues

```python
# Verify token
response = requests.get(
    'http://localhost:8080/auth/verify',
    headers={'Authorization': f'Bearer {token}'}
)
if response.status_code == 200:
    print("Token valid")
else:
    print("Token invalid or expired")
```

### Performance Issues

```python
# Enable request batching
# Only1MCP automatically batches requests within 100ms window

# Or manually batch:
async def batch_requests(requests_list):
    tasks = [make_request(r) for r in requests_list]
    return await asyncio.gather(*tasks)
```

---

## Next Steps

- Explore [performance benchmarks](../benches/) to optimize your integration
- Review [example applications](../examples/) for complete implementations
- Check [API documentation](../docs/API.md) for full endpoint reference
- Join [discussions](https://github.com/doublegate/Only1MCP/discussions) for community support
