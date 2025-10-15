# Only1MCP: The Ultimate MCP Server Aggregator

## Overall Project Overview

Only1MCP (short for **“Only1: The Ultimate MCP Server Aggregator / Context Switcher”**) is a stand-alone application that serves as a unified proxy and management layer for multiple **Model Context Protocol (MCP)** servers. It provides a single front-end endpoint through which AI applications and agents can interact with numerous external tool servers (APIs, databases, browsers, etc.), all while significantly reducing prompt **context overhead**. By consolidating what would otherwise be multiple MCP connections into **one unified interface**, Only1MCP minimizes token usage and latency. Instead of listing many tool endpoints in an AI prompt (which can bloat context by hundreds of tokens【25†L227-L236】), an AI agent only needs to reference the Only1MCP proxy. This consolidation can yield an estimated **50–70% reduction** in prompt size via intelligent request batching and caching (meaning fewer tokens consumed and faster responses).

**Key Differentiators:** Only1MCP is designed to be lightweight, cross-platform, and **not dependent on Docker or container runtimes** for core functionality. This is a deliberate contrast to Docker’s MCP Toolkit (which, while offering a catalog of 200+ servers【6†L97-L104】, requires Docker Desktop and encloses each server in a container). By eliminating the need for Docker Desktop, Only1MCP avoids the overhead of running containerized tools when it’s not necessary, and it simplifies the setup for users on any OS (Windows, macOS, Linux). Only1MCP can run as a simple compiled binary with minimal resource usage, thanks to being implemented in **Rust**. (A Docker image for Only1MCP will be optionally provided for those who prefer containerization, but it’s entirely optional for local deployment.) The focus is on **efficiency** and **security**: using Rust’s strong memory safety guarantees and performance to handle high-throughput request routing, while providing robust isolation between the AI model and external tools. Crucially, Only1MCP supports **“hot-swapping”** of underlying MCP servers – meaning you can **dynamically add, remove, or swap** out tool servers at runtime without restarting the aggregator or disrupting the AI client. This ensures zero-downtime updates to the toolset, a feature beyond the capabilities of current basic proxies.

**Context & Motivation:** The rise of MCP (a standard introduced by Anthropic in late 2024【24†L55-L63】) has led to an explosion of available tool servers – by late 2025, **over 200** MCP servers are readily accessible via catalogs like Docker’s MCP Catalog【6†L97-L104】 and thousands more in community directories like PulseMCP. AI agents often need multiple tools simultaneously (e.g. a browser, a database, a calculator), but connecting an AI to numerous MCP endpoints directly leads to prompt clutter and maintenance hassle. A Reddit user aptly described the pain point: *“I had to connect my Cursor (AI IDE) to 5 different MCP servers and suddenly there was an explosion of URLs and tools.”*【11†L503-L510】 In other words, managing each tool as a separate endpoint inflates the system prompt and complicates configuration. Only1MCP addresses this by acting as a **single hub**: AI clients talk to Only1MCP, which in turn brokers requests to the appropriate backend MCP server and relays the result. This design dramatically **simplifies prompt design** and centralizes tool management. Security is also improved: Only1MCP can enforce authentication and audit logging uniformly for all tool calls, rather than relying on each disparate MCP server to implement security on its own.

**Comparison with Existing Solutions:** While a few nascent projects attempt similar aggregation, they have notable drawbacks that Only1MCP overcomes. For example, **TBXark’s mcp-proxy** (an open-source MCP proxy server) also funnels multiple MCP servers behind a single HTTP entrypoint【2†L307-L315】, but it relies on running MCP servers via Node/Python and is often used in Docker; it lacks advanced context optimizations like caching and dynamic UI. **VeriTeknik’s plugged.in MCP Proxy** provides a unified web interface and some monitoring, but being Node/TypeScript-based, it can be heavy on memory and doesn’t prioritize minimal token footprint. **Veallym0n’s MCPEz** is a Python proxy aggregator with a web UI【17†L29-L34】, but it doesn’t emphasize hot-swapping or cross-platform binaries. Only1MCP’s unique proposition is a **Rust-native**, high-performance aggregator that **integrates the best ideas** from these tools while adding innovations (detailed below) in context efficiency, security, and user experience. In short, Only1MCP aims to be the “one proxy to rule them all” – providing a **unified, efficient, and secure gateway** for AI-to-tool interactions.

## Features List

Only1MCP’s feature set can be divided into **core features** (the essential capabilities to be delivered in the initial release) and **extended features** (additional enhancements to be rolled out in phases). Each feature is mapped to the development phase in which it will be implemented, aligning with an agile roadmap.

### Core Features (MVP – Phase 1)

- **Unified MCP Proxy:** Serve as a single HTTP(S) endpoint that aggregates multiple MCP servers. Only1MCP will expose the standard MCP API (e.g., `/tools/list`, `/tools/call`) to AI clients, but under the hood it will route each request to the appropriate registered MCP server. This allows **multiple tool servers to appear as one** to the AI【11†L503-L510】. The core proxy supports both **STDIO** and **HTTP transports** for flexibility (using Rust’s async runtime for concurrency).
- **Hot-Swappable Server Registry:** Provide the ability to **add or remove MCP servers on the fly**. For example, a user can run `only1mcp --add-server http://weather.mcp` and the aggregator will begin proxying to that server immediately, without restarting. Internally, this is handled via an async-aware configuration watcher that updates the in-memory registry of backend servers (using channels or an `RwLock` protected structure). Hot-swapping ensures **zero-downtime reconfiguration**, a significant improvement over existing proxies which typically require editing config files and restarting.
- **CLI Management Interface:** In Phase 1, Only1MCP is controlled via a **command-line interface (CLI)** for maximum stability and scriptability. Users can invoke subcommands like `only1mcp add <server_url>`, `only1mcp remove <name>` or `only1mcp list` to manage connections. The CLI will also allow loading a YAML/JSON config file specifying multiple servers and settings. This provides a quick and user-friendly way to get started (leveraging the `clap` crate for robust argument parsing).
- **Context Footprint Optimizations:** Even in the MVP, Only1MCP will implement strategies to minimize the prompt/context impact on connected AI models. This includes intelligent **request routing** (so that only the relevant tool’s schema/response is relayed to the model) and possibly **on-demand tool listing** (only presenting the AI with the tools from servers it actually needs, rather than the full union of all tools up front). By doing so, Only1MCP avoids the “full spec for every tool in every prompt” problem【25†L227-L236】, reducing token usage. Basic **response caching** (for frequently called read-only tools) will also be introduced to accelerate repeated queries.
- **Security and Access Control:** The core aggregator will support optional **authentication** on incoming requests (e.g., an API key or token that the AI client must provide). It will also enforce **TLS/SSL** for external connections where needed (leveraging Rust’s `rustls` for a secure TLS implementation). All traffic between Only1MCP and the backend MCP servers can be configured to use HTTPS as well, preventing man-in-the-middle risk. Additionally, an **audit log** will record all tool invocations (timestamp, tool name, calling client, outcome) to support debugging and compliance.

### Extended Features (Phase 2 and Beyond)

- **Interactive TUI (Phase 2):** Develop a **Text-based User Interface (TUI)** that runs in the terminal, providing real-time monitoring and control of Only1MCP. This TUI (built with `tui-rs` and `crossterm` crates) will display active connections, recent tool calls, server health status, and allow interactive commands. For instance, an operator could see a dashboard of tool usage and press keys to enable/disable certain servers. This makes managing the aggregator more intuitive, especially on headless or SSH servers.
- **Web Dashboard (Phase 3):** Offer a **web-based interface** for remote management and analytics. Using a web framework (such as `axum` or `actix-web` in Rust), Only1MCP will expose a lightweight dashboard accessible via browser. This dashboard will include visual charts (via libraries like D3.js or Chart.js) for metrics like request latency, throughput, error rates, and cache hits. Users can add/remove servers through a web form, update configurations, and view logs. The web UI will greatly aid in enterprise scenarios where remote monitoring is key.
- **Desktop GUI (Phase 4):** For maximum user-friendliness, a full **GUI application** is planned using **Tauri** (which allows building cross-platform desktop apps with Rust backends and a webview frontend). The GUI will have the same functions as the web dashboard but packaged as a native app for Windows/Mac/Linux. This phase focuses on polish: intuitive drag-and-drop to add a new MCP server, toggle switches for features, and rich notifications (e.g., if a server goes down or a new version is available).
- **Dynamic Discovery & Registration:** Only1MCP will include an optional service discovery mechanism. For example, it can integrate with mDNS/Bonjour or a service registry (Consul, etc.) to automatically discover MCP servers on a local network or within a cluster. Newly discovered servers can be auto-registered, and dead servers auto-removed, if this mode is enabled. This dynamic discovery ensures **scalability** in environments with many ephemeral tool servers.
- **Load Balancing & Failover:** The aggregator will support connecting multiple instances of the *same* tool server type and distributing calls among them. Several algorithms (configurable) will be available for load balancing: round-robin, least-connections, or even AI-driven selection. If one server instance fails to respond, Only1MCP can automatically **fail over** to a backup instance. Health checks (implemented as periodic pings in a Tokio task) will mark servers as up or down. This feature provides **high availability**, crucial for production use.
- **Batching and Parallel Execution:** Where possible, Only1MCP will **batch multiple requests** to reduce latency and context size. For example, if an AI query would sequentially call two different tools, Only1MCP can detect this (or the AI can request it) and perform the calls in parallel or as a single composite call if the underlying servers support it. Batching results in fewer round-trips to the AI and can merge multiple tool schemas into one payload when listing tools, saving tokens. (This concept is inspired by community ideas like the mcp-batchit project, which demonstrated combining spec requests【25†L283-L290】.)
- **Response Caching and Compression:** The aggregator will cache responses for idempotent tool calls (e.g., a “read current stock price” tool) using an in-memory store or Redis (via the `dashmap` crate or `redis-rs`). Cached results will be returned instantly to the AI if the same request repeats, dramatically lowering latency. Additionally, Only1MCP can compress large responses or tool lists (using gzip or Brotli) before sending them to the AI client, to save bandwidth and possibly tokens (if the AI client supports compressed inputs).
- **Enhanced Security (Auth & Encryption):** Build upon core security with features like **OAuth2/JWT authentication** for clients (so multiple users can have separate tokens/permissions), **role-based access control** to restrict certain tools to certain roles, and end-to-end encryption options. For instance, if Only1MCP is used across a network, it can enforce that all backend MCP server connections use TLS. We will also integrate secure secret storage for any API keys needed by MCP servers (like storing them encrypted and only injecting at call time). These measures ensure Only1MCP can be safely deployed in enterprise environments.
- **Monitoring & Analytics:** Introduce **real-time monitoring** in Phase 3/4 where the web/GUI is available. Under the hood, Only1MCP will collect metrics (using `metrics` and `prometheus` Rust crates) such as number of requests, average response time per server, error counts, cache hits, etc. Users can view these metrics in the dashboard or export them to external systems (Prometheus/Grafana integration). Logging will be structured (JSON logs optional) to feed into log management systems. This transparency helps in tuning performance and quickly diagnosing issues.
- **Plugin System & Extensibility:** Only1MCP will be architected with modularity in mind. We plan to allow **plugins** to extend functionality. For example, a plugin might implement a custom scheduling algorithm or integrate a new type of backend not originally supported. Possible approaches include a dynamic loading system for Rust library plugins or even running WebAssembly plugins in a sandbox (using `wasmtime`). This would let advanced users add features without modifying the core codebase, and allows the community to contribute integrations.
- **Advanced Innovations:** *Beyond the basics, Only1MCP will explore cutting-edge improvements:*  
  - **AI-Assisted Routing:** Incorporate a lightweight ML model (via crates like `tract` or `burn`) to predict usage patterns and pre-warm certain connections or choose the fastest server for a given request based on historical data. For example, if a particular MCP server tends to respond faster to image analysis queries, the model could learn to route those tasks to it preferentially.  
  - **Zero-Downtime Updates:** Implement hot-reloading of the Only1MCP binary itself or its configuration with minimal disruption. Strategies might involve running a new version in parallel and switching ports, or using in-memory state transfer. The goal is that upgrading Only1MCP (or its internal plugin modules) does not require taking the whole system offline.  
  - **Integrated Container Management:** For users who *do* want to run tool servers in containers, Only1MCP can integrate with container engines (like **Podman** or use the Docker API via the `bollard` crate). Through a simple command, the user could instruct Only1MCP to fetch/build a container image for an MCP server and run it. This essentially combines the deployment convenience of Docker’s MCP Toolkit with the flexibility of a standalone app – but again, it’s optional and would run containers in a controlled way, without imposing Docker Desktop on all users.  
  - **UX & Accessibility:** Ensure the tool is accessible to a broad range of users. Planned features include **internationalization (i18n)** support (using the `fluent` crate for translations of UI text), theming and color customization in the TUI/GUI, and extensive **documentation**. Documentation will likely be generated using mdBook (for a rich text guide) and kept up-to-date as features evolve. 

## Technical Details

### Architecture and Components

```text
+---------------------------+          (MCP requests)         +-------------------+
|    AI Application (LLM)   |  --HTTP/S-->  Only1MCP Proxy  -->|  MCP Server #1    |
+---------------------------+               (Rust)            +-------------------+
            |                                   |\__           +-------------------+
            |                                   |   \--> ...-->|  MCP Server #2    |
            |                                   |               +-------------------+
            |                                   |               +-------------------+
            |                                   \-------------->|  MCP Server #N    |
            v                                                    +-------------------+
    (Unified response)
```

*Option A – Only1MCP as a unified front-end:* In this design, an AI agent (or any MCP client) issues tool calls to a **single URL** (Only1MCP), rather than juggling multiple MCP endpoints. Only1MCP maintains connections to each configured MCP **resource server** on the backend. When a request comes in, it **parses the tool identifier** to determine which backend server should handle it, forwards the request, and then **streams back** the result to the AI. The AI model is oblivious to the multiple servers behind the scenes – it sees one consistent interface. This greatly simplifies prompt context, since the model only needs to know about one “mega-server.” It also means we can introduce cross-cutting improvements (like caching or load balancing) transparently at the proxy layer.

```text
[ AI Agent ] 
     |                   Only1MCP (Rust Proxy)
     |                    |   (Unified Interface)
     v                    |   
 +-------+          +-----+-------+                 
 | LLM   | --HTTP-->|  Proxy Core |--->[ External MCP Service A ]
 +-------+          |(Routing, etc)|--->[ External MCP Service B ]
                   /+-------------+\
                  /     Container   \
                 v    Manager (Opt.) v
           [ Containerized MCP X ]   [ Containerized MCP Y ]
```

*Option B – Only1MCP with container management:* In this variant, the **Proxy Core** of Only1MCP still performs the same role of routing and unification. However, if a tool server is not already running externally, Only1MCP’s **Container Manager** can launch it (for example, spin up *MCP X* and *MCP Y* in containers). Those then behave like external servers from the core’s perspective (accessible via HTTP calls), but Only1MCP has lifecycle control over them (it could restart them, shut them down, or update them by pulling new images). This approach is useful for self-contained deployments or quick experiments with new tools: the user can just tell Only1MCP what tool they want (perhaps by name or image) and the aggregator fetches and runs it. The diagram shows an example with two containerized servers alongside two purely external ones, to highlight that Only1MCP can handle a mix of both.

**Internal Structure (Modules):**
- **Server Registry:** Thread-safe list of active MCP servers (`Arc<RwLock<HashMap<String, ServerInfo>>>`) with runtime updates.
- **HTTP Server / Router:** Axum-based listener that parses incoming MCP requests and forwards them via `reqwest` to the correct backend. Supports streaming (SSE/chunked) pass-through.
- **Middleware & Context Manager:** Caching, batching, compression, and authentication (API keys/JWT) applied uniformly.
- **Config & HotSwap Handler:** Watches for config changes (file or CLI-triggered) via `tokio::sync::watch` channel; updates registry atomically.
- **CLI/TUI Interface Layer:** Phase 1 CLI via `clap`; Phase 2 TUI via `tui-rs` for real-time operations.
- **Web/GUI Interface:** Phase 3 web server (REST/WebSocket) for dashboards; Phase 4 Tauri desktop app.

**Security Considerations:**
- TLS (via `rustls`) for client↔Only1MCP and Only1MCP↔server links.
- Optional OAuth2/JWT-based auth, RBAC, and encrypted secret storage.
- Audit logging of all tool invocations with structured logs.

### Implementation Notes and Pseudocode

**Hot-Swap Configuration (Tokio watch channel):**
```rust
use tokio::sync::{watch, RwLock};
use std::sync::Arc;

struct Config { servers: Vec<ServerInfo> }
type ServerRegistry = Arc<RwLock<Vec<ServerInfo>>>;

async fn hot_swap_handler(registry: ServerRegistry, mut rx: watch::Receiver<Config>) {
    while rx.changed().await.is_ok() {
        let new_servers = rx.borrow().servers.clone();
        let mut reg = registry.write().await;
        *reg = new_servers;  // Swap without downtime
    }
}
```

**Routing Sketch (Axum):**
```rust
use axum::{routing::post, Json, Router};
use std::sync::Arc;

async fn call_tool(Json(req): Json<McpCall>, registry: Arc<ServerRegistry>) -> impl IntoResponse {
    let target = resolve_target(&req.tool_id, registry).await?;
    // forward to target via reqwest, stream response back
}
```

**Caching Layer (DashMap) & Compression:**
```rust
use dashmap::DashMap;
static CACHE: Lazy<DashMap<CacheKey, CachedValue>> = Lazy::new(DashMap::new);
// On GET-like calls: check CACHE before forwarding; apply gzip/brotli on large responses
```

## Top-Level Development Strategy (Roadmap)

| **Phase (Sprint)** | **Focus & Milestones** | **Timeline** | **Key Tools** |
|---|---|---|---|
| **Phase 1: CLI MVP (S1–S3)** | Core proxy (routing, registry), hot-swap config, basic auth/logging, E2E tests | ~4–6 weeks | tokio, axum/hyper, reqwest, clap, serde |
| **Phase 2: TUI (S4–S5)** | Terminal dashboard, controls, health checks, basic caching & perf tuning | ~3 weeks | tui-rs, crossterm, metrics |
| **Phase 3: Web (S6–S7)** | Web dashboard (REST/WebSocket), Prometheus metrics, LB strategies, TLS/JWT | ~4 weeks | axum/actix-web, rustls, prometheus |
| **Phase 4: GUI (S8–S9)** | Tauri desktop app, plugin system (dynlib/WASM), final optimization & docs | ~4 weeks | tauri, wasmtime/libloading |

**Process & Quality:** Git-based workflow, CI (GitHub Actions), `cargo test` + integration tests, `cargo-audit`, property tests (`proptest`), fuzzing (`cargo-fuzz`), benchmarks (`criterion`) and soak tests.

**Risks & Mitigations:** MCP spec churn (modular adapters), performance bottlenecks (profiling/zero-copy), Windows/Linux differences (early multi-OS CI builds).

**Success Metrics:** <10ms proxy overhead per call; 50–70% prompt token reduction for multi-tool sessions; stable multi-week uptime; positive community adoption.

## References and Comparative Analysis

- **Official MCP Intro/Explainers** – Humanloop/Anthropic overviews of MCP as a universal method to connect AI to tools【24†L55-L63】.  
- **Docker MCP Toolkit** – Catalog of **200+ MCP servers**, Docker Desktop-powered one-click launches【6†L97-L104】.  
- **TBXark/mcp-proxy** – Go-based proxy aggregating multiple MCP servers behind a single HTTP entrypoint【2†L307-L315】.  
- **VeriTeknik PluggedIn MCP Proxy** – Node/TS proxy with web playground, unified interface, SSE/STDIO support【16†L373-L381】.  
- **MCPEz (PulseMCP)** – Python proxy that aggregates multiple MCP services into a single endpoint【17†L29-L34】.  
- **Reddit r/AI_Agents** – User report of *“explosion of URLs and tools”* when connecting many MCP servers; motivation for a proxy aggregator【11†L503-L510】.  
- **Token Overhead Discussions** – Combining/limiting tool specs and using on-demand loading to reduce prompt size【25†L227-L236】【25†L275-L283】【25†L283-L290】.  

---

*© 2025 Only1MCP Project — Rust-first, efficient, secure MCP aggregation.*
