# Only1MCP Architecture Diagrams

## Comprehensive Technical Visualization for Initial Development

**Document Version:** 1.0
**Date:** October 14, 2025
**Status:** Technical Architecture Visualization
**Purpose:** Detailed Mermaid diagrams for system architecture, data flows, security, and implementation patterns

---

## Table of Contents

- [Only1MCP Architecture Diagrams](#only1mcp-architecture-diagrams)
  - [Comprehensive Technical Visualization for Initial Development](#comprehensive-technical-visualization-for-initial-development)
  - [Table of Contents](#table-of-contents)
  - [1. Overall System Architecture](#1-overall-system-architecture)
    - [High-Level System Overview](#high-level-system-overview)
  - [2. Core Component Architecture](#2-core-component-architecture)
    - [Internal Component Relationships](#internal-component-relationships)
  - [3. Request Routing and Transport Layer](#3-request-routing-and-transport-layer)
    - [Multi-Transport Architecture](#multi-transport-architecture)
    - [STDIO Process Management](#stdio-process-management)
    - [HTTP Connection Pooling](#http-connection-pooling)
  - [4. Security Architecture](#4-security-architecture)
    - [Defense-in-Depth Security Layers](#defense-in-depth-security-layers)
  - [5. Authentication and Authorization Flow](#5-authentication-and-authorization-flow)
    - [Complete Auth Flow with RBAC](#complete-auth-flow-with-rbac)
  - [6. Context Optimization Pipeline](#6-context-optimization-pipeline)
    - [Token Reduction Architecture](#token-reduction-architecture)
  - [7. Caching Strategy Architecture](#7-caching-strategy-architecture)
    - [Multi-Layer Cache Design](#multi-layer-cache-design)
  - [8. Hot-Reload and Zero-Downtime Pattern](#8-hot-reload-and-zero-downtime-pattern)
    - [Configuration Hot-Reload Mechanism](#configuration-hot-reload-mechanism)
  - [9. Load Balancing Architecture](#9-load-balancing-architecture)
    - [Consistent Hashing with Health-Aware Fallback](#consistent-hashing-with-health-aware-fallback)
    - [1. Consistent Hashing (Primary)](#1-consistent-hashing-primary)
    - [2. Least Connections (Fallback)](#2-least-connections-fallback)
  - [10. Health Checking and Circuit Breaker](#10-health-checking-and-circuit-breaker)
    - [Hybrid Health Monitoring](#hybrid-health-monitoring)
    - [Health Check Implementation](#health-check-implementation)
  - [11. Plugin System Architecture](#11-plugin-system-architecture)
    - [Native Rust + WASM Dual Architecture](#native-rust--wasm-dual-architecture)
  - [12. Data Flow - Complete Request Lifecycle](#12-data-flow---complete-request-lifecycle)
    - [End-to-End Request Processing](#end-to-end-request-processing)
  - [13. Connection Pool Management](#13-connection-pool-management)
    - [Per-Backend Connection Pooling](#per-backend-connection-pooling)
  - [14. Monitoring and Observability](#14-monitoring-and-observability)
    - [Comprehensive Observability Stack](#comprehensive-observability-stack)
    - [Performance Metrics](#performance-metrics)
  - [15. Configuration Management](#15-configuration-management)
    - [Configuration Schema and Validation](#configuration-schema-and-validation)
  - [Summary](#summary)

---

## 1. Overall System Architecture

### High-Level System Overview

```mermaid
graph TB
    subgraph "AI Clients"
        Claude[Claude Desktop]
        VSCode[VS Code + Cline]
        API[API Clients]
    end

    subgraph "Only1MCP Aggregator - Rust/Axum"
        subgraph "Ingress Layer"
            TLS[TLS Termination<br/>Rustls 1.3]
            Auth[Authentication<br/>OAuth2/JWT/API Keys]
            RateLimit[Rate Limiter<br/>60 req/min default]
        end

        subgraph "Core Proxy Engine"
            Router[Request Router<br/>Axum Framework]
            Registry[Server Registry<br/>Arc RwLock HashMap]
            LB[Load Balancer<br/>Consistent Hash]

            subgraph "Context Optimizer"
                Cache[Multi-Layer Cache<br/>DashMap LRU]
                Batcher[Request Batcher<br/>100ms windows]
                Compressor[Compression Engine<br/>zstd/gzip/brotli]
                DynLoad[Dynamic Tool Loader<br/>Lazy schema loading]
            end
        end

        subgraph "Transport Handlers"
            STDIO[STDIO Handler<br/>Process Management]
            HTTP[HTTP Handler<br/>reqwest + pools]
            SSE[SSE Handler<br/>EventSource]
            WS[WebSocket Handler<br/>tokio-tungstenite]
        end

        subgraph "Backend Coordination"
            HealthCheck[Health Checker<br/>Active + Passive]
            CircuitBreaker[Circuit Breaker<br/>50% error threshold]
            ConnPool[Connection Pools<br/>50-100 per backend]
        end

        subgraph "Management & Observability"
            AdminAPI[Admin API<br/>Configuration CRUD]
            Metrics[Prometheus Metrics<br/>OpenTelemetry]
            Audit[Audit Logger<br/>Cryptographic chain]
        end
    end

    subgraph "Backend MCP Servers"
        FS[Filesystem MCP<br/>STDIO: npx]
        GH[GitHub MCP<br/>HTTP: OAuth]
        WEB[Web Search MCP<br/>STDIO: uvx]
        CUSTOM[Custom Servers<br/>Any Transport]
    end

    subgraph "Configuration & State"
        ConfigFile[config.yaml<br/>Hot-reloadable]
        ConfigWatch[File Watcher<br/>notify-rs]
        SecretStore[Secret Manager<br/>Keyring + Env]
    end

    %% Client connections
    Claude -->|MCP Protocol<br/>STDIO/HTTP| TLS
    VSCode -->|MCP Protocol<br/>STDIO/HTTP| TLS
    API -->|MCP Protocol<br/>HTTP/REST| TLS

    %% Ingress flow
    TLS --> Auth
    Auth --> RateLimit
    RateLimit --> Router

    %% Router connections
    Router --> Registry
    Router --> Cache
    Registry --> LB

    %% Load balancer to optimizers
    LB --> Batcher
    LB --> DynLoad
    Batcher --> Compressor

    %% Transport selection
    Router --> STDIO
    Router --> HTTP
    Router --> SSE
    Router --> WS

    %% Backend connections
    STDIO <-->|stdin/stdout<br/>JSON-RPC| FS
    HTTP <-->|HTTP POST<br/>JSON-RPC| GH
    STDIO <-->|stdin/stdout<br/>JSON-RPC| WEB
    HTTP <-->|Various| CUSTOM

    %% Backend coordination
    STDIO --> ConnPool
    HTTP --> ConnPool
    ConnPool --> HealthCheck
    HealthCheck --> CircuitBreaker
    CircuitBreaker --> Router

    %% Configuration flow
    ConfigFile -.->|Watch| ConfigWatch
    ConfigWatch -.->|Reload Events| Registry
    SecretStore -.->|Secrets| Auth

    %% Management connections
    AdminAPI --> Registry
    Router --> Metrics
    Auth --> Audit
    Router --> Audit

    classDef ingress fill:#e1f5ff,stroke:#01579b,stroke-width:3px
    classDef core fill:#fff3e0,stroke:#e65100,stroke-width:3px
    classDef backend fill:#f3e5f5,stroke:#4a148c,stroke-width:3px
    classDef config fill:#e8f5e9,stroke:#1b5e20,stroke-width:3px

    class TLS,Auth,RateLimit ingress
    class Router,Registry,LB,Cache,Batcher core
    class FS,GH,WEB,CUSTOM backend
    class ConfigFile,ConfigWatch,SecretStore config
```

**Key Metrics & Targets:**

- **Latency Overhead:** <5ms (p99)
- **Throughput:** 10k+ req/s
- **Token Reduction:** 50-70% via optimization
- **Cache Hit Rate:** >70%
- **Uptime:** >99.9%

---

## 2. Core Component Architecture

### Internal Component Relationships

```mermaid
graph TB
    subgraph "Core Components - Rust Implementation"
        subgraph "State Management"
            AppState[AppState<br/>Arc Shared State]
            ServerReg[ServerRegistry<br/>Arc RwLock HashMap]
            HashRing[ConsistentHashRing<br/>BTreeMap with vnodes]
            ConfigState[Configuration<br/>Arc Config]
        end

        subgraph "Proxy Engine"
            AxumServer[Axum Server<br/>Multi-threaded Tokio]
            RouteHandler[Route Handlers<br/>Extractors + State]
            Middleware[Middleware Stack<br/>Tower Layers]
        end

        subgraph "Request Processing"
            RequestValidator[Request Validator<br/>Schema Validation]
            ToolMatcher[Tool Matcher<br/>Regex + Priority]
            ServerSelector[Server Selector<br/>Consistent Hash]
            RequestForwarder[Request Forwarder<br/>Transport Abstraction]
        end

        subgraph "Response Processing"
            ResponseAggregator[Response Aggregator<br/>Batch Combining]
            ResponseCache[Response Cache<br/>DashMap + TTL]
            ErrorHandler[Error Handler<br/>Resilience Patterns]
        end

        subgraph "Background Tasks"
            HealthMonitor[Health Monitor<br/>Tokio Task]
            ConfigWatcher[Config Watcher<br/>notify + debounce]
            CacheEvictor[Cache Evictor<br/>LRU + TTL]
            MetricsCollector[Metrics Collector<br/>Prometheus]
        end
    end

    %% State connections
    AppState --> ServerReg
    AppState --> ResponseCache
    AppState --> ConfigState
    ServerReg --> HashRing

    %% Proxy flow
    AxumServer --> RouteHandler
    RouteHandler --> Middleware
    Middleware --> RequestValidator

    %% Request processing
    RequestValidator --> ToolMatcher
    ToolMatcher --> ServerSelector
    ServerSelector --> HashRing
    ServerSelector --> RequestForwarder

    %% Response flow
    RequestForwarder --> ResponseAggregator
    ResponseAggregator --> ResponseCache
    ResponseAggregator --> ErrorHandler
    ErrorHandler --> RouteHandler

    %% Background task connections
    HealthMonitor -.->|Updates| ServerReg
    ConfigWatcher -.->|Reloads| ConfigState
    ConfigState -.->|Triggers| ServerReg
    CacheEvictor -.->|Cleans| ResponseCache
    RouteHandler -.->|Records| MetricsCollector

    classDef state fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef process fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef background fill:#f3e5f5,stroke:#6a1b9a,stroke-width:2px

    class AppState,ServerReg,HashRing,ConfigState state
    class RequestValidator,ToolMatcher,ServerSelector process
    class HealthMonitor,ConfigWatcher,CacheEvictor,MetricsCollector background
```

**Component Details:**

| Component | Technology | Concurrency Model | Performance Target |
|-----------|-----------|-------------------|-------------------|
| AppState | Arc<T> | Lock-free reads | <1ns clone overhead |
| ServerRegistry | Arc<RwLock<HashMap>> | Multi-reader, single-writer | <100ns read |
| ResponseCache | DashMap<K,V> | Lock-free sharding | <50ns lookup |
| AxumServer | Tokio runtime | Multi-threaded work-stealing | 10k req/s |

---

## 3. Request Routing and Transport Layer

### Multi-Transport Architecture

```mermaid
graph TB
    subgraph "Client Request"
        ClientReq[MCP Request<br/>JSON-RPC 2.0]
    end

    subgraph "Transport Router"
        TransportSelect{Transport<br/>Selection}
    end

    subgraph "STDIO Transport"
        STDIOCheck{Process<br/>Exists?}
        STDIOSpawn[Spawn Process<br/>tokio::process]
        STDIOPool[Process Pool<br/>DashMap pid → handle]
        STDIOWrite[Write stdin<br/>length-prefixed]
        STDIORead[Read stdout<br/>buffered async]
        STDIOError[Read stderr<br/>logging only]
    end

    subgraph "HTTP/HTTPS Transport"
        HTTPPool[Connection Pool<br/>reqwest client]
        HTTPAuth[Add Auth Headers<br/>Bearer/API Key]
        HTTPRequest[HTTP POST<br/>Content-Type: json]
        HTTPTimeout[Timeout Guard<br/>5s default]
        HTTPRetry{Retry?<br/>3 attempts}
    end

    subgraph "SSE Transport - Legacy"
        SSEConnect[EventSource Connect<br/>GET with Accept: text/event-stream]
        SSEStream[Parse Event Stream<br/>data: field extraction]
        SSEReconnect[Auto-reconnect<br/>Last-Event-ID]
    end

    subgraph "WebSocket Transport"
        WSUpgrade[WebSocket Upgrade<br/>HTTP 101]
        WSFraming[Message Framing<br/>Text frames]
        WSPing[Keepalive Ping<br/>30s interval]
        WSBidirectional[Full-Duplex<br/>tokio::select!]
    end

    subgraph "Response Processing"
        ParseJSON[Parse JSON-RPC<br/>serde_json]
        ValidateResp[Validate Response<br/>Schema check]
        ErrorMap[Error Mapping<br/>Standardize errors]
    end

    %% Flow from client
    ClientReq --> TransportSelect

    %% STDIO path
    TransportSelect -->|Transport::Stdio| STDIOCheck
    STDIOCheck -->|No| STDIOSpawn
    STDIOCheck -->|Yes| STDIOPool
    STDIOSpawn --> STDIOPool
    STDIOPool --> STDIOWrite
    STDIOWrite --> STDIORead
    STDIORead --> ParseJSON
    STDIOPool -.->|diagnostics| STDIOError

    %% HTTP path
    TransportSelect -->|Transport::Http| HTTPPool
    HTTPPool --> HTTPAuth
    HTTPAuth --> HTTPRequest
    HTTPRequest --> HTTPTimeout
    HTTPTimeout --> HTTPRetry
    HTTPRetry -->|Success| ParseJSON
    HTTPRetry -->|Retry| HTTPRequest
    HTTPRetry -->|Fail| ErrorMap

    %% SSE path
    TransportSelect -->|Transport::Sse| SSEConnect
    SSEConnect --> SSEStream
    SSEStream --> ParseJSON
    SSEStream -->|Disconnect| SSEReconnect
    SSEReconnect --> SSEConnect

    %% WebSocket path
    TransportSelect -->|Transport::WebSocket| WSUpgrade
    WSUpgrade --> WSFraming
    WSFraming --> WSBidirectional
    WSBidirectional --> WSPing
    WSBidirectional --> ParseJSON

    %% Response validation
    ParseJSON --> ValidateResp
    ValidateResp --> ErrorMap

    classDef transport fill:#e1f5ff,stroke:#01579b,stroke-width:2px
    classDef process fill:#fff3e0,stroke:#e65100,stroke-width:2px

    class STDIOPool,HTTPPool,SSEConnect,WSUpgrade transport
    class ParseJSON,ValidateResp,ErrorMap process
```

**Transport-Specific Implementation Details:**

### STDIO Process Management

```rust
// Process lifecycle: spawn → communicate → terminate
// stdin: length-prefix (u32 BE) + JSON payload
// stdout: length-prefix (u32 BE) + JSON response
// stderr: UTF-8 log lines (non-blocking)
// Graceful shutdown: SIGTERM → 5s wait → SIGKILL
```

### HTTP Connection Pooling

```rust
// reqwest::Client with connection pool
// max_connections_per_host: 100
// pool_idle_timeout: 90s
// connection_verbose: false (perf)
// HTTP/2 preferred, fallback to HTTP/1.1
```

---

## 4. Security Architecture

### Defense-in-Depth Security Layers

```mermaid
graph TB
    subgraph "Network Security Layer"
        TLS13[TLS 1.3<br/>Rustls<br/>Post-quantum KEX]
        Firewall[Firewall Rules<br/>Allowlist IPs]
        DDoS[DDoS Protection<br/>Rate limiting]
    end

    subgraph "Authentication Layer"
        AuthSelect{Auth<br/>Method}

        subgraph "OAuth2/OIDC"
            OAuthProvider[Provider Config<br/>Okta/Azure AD/GitHub]
            OAuthExchange[Code Exchange<br/>PKCE flow]
            TokenValidate[Token Validation<br/>JWT verify + expiry]
        end

        subgraph "API Key"
            APIKeyExtract[Extract from Header<br/>X-API-Key/Bearer]
            APIKeyHash[Compare Hash<br/>bcrypt/Argon2]
            APIKeyRotate[Auto-rotation<br/>30-90 days]
        end

        subgraph "mTLS"
            ClientCert[Client Certificate<br/>X.509 validation]
            CertRevoke[Revocation Check<br/>OCSP/CRL]
        end
    end

    subgraph "Authorization Layer - RBAC"
        RoleCheck{User<br/>Role?}

        AdminPerms[Admin Role<br/>Full access]
        DevPerms[Developer Role<br/>Tool subset]
        ReadOnlyPerms[Read-Only Role<br/>List/view only]

        ToolACL[Tool-Level ACL<br/>Granular permissions]
        ServerACL[Server-Level ACL<br/>Backend restrictions]
    end

    subgraph "Request Security"
        InputValidation[Input Validation<br/>Schema + sanitization]
        SSRFProtection[SSRF Protection<br/>Block private IPs]
        CommandWhitelist[Command Allowlist<br/>node/npx/python/uvx]
        PayloadLimit[Payload Size Limit<br/>100MB default]
    end

    subgraph "Audit & Monitoring"
        AuditLog[Audit Logger<br/>Cryptographic chain]
        AnomalyDetect[Anomaly Detection<br/>Rate/pattern analysis]
        AlertSystem[Alert System<br/>Slack/PagerDuty]
    end

    subgraph "Data Protection"
        SecretMgmt[Secret Management<br/>Keyring/Vault]
        Encryption[At-Rest Encryption<br/>AES-256-GCM]
        SecureErase[Secure Memory Erase<br/>zeroize crate]
    end

    %% Network flow
    TLS13 --> Firewall
    Firewall --> DDoS
    DDoS --> AuthSelect

    %% Authentication flows
    AuthSelect -->|OAuth2/OIDC| OAuthProvider
    OAuthProvider --> OAuthExchange
    OAuthExchange --> TokenValidate

    AuthSelect -->|API Key| APIKeyExtract
    APIKeyExtract --> APIKeyHash
    APIKeyHash --> APIKeyRotate

    AuthSelect -->|mTLS| ClientCert
    ClientCert --> CertRevoke

    %% Authorization
    TokenValidate --> RoleCheck
    APIKeyHash --> RoleCheck
    CertRevoke --> RoleCheck

    RoleCheck -->|admin| AdminPerms
    RoleCheck -->|developer| DevPerms
    RoleCheck -->|readonly| ReadOnlyPerms

    AdminPerms --> ToolACL
    DevPerms --> ToolACL
    ReadOnlyPerms --> ToolACL
    ToolACL --> ServerACL

    %% Request security
    ServerACL --> InputValidation
    InputValidation --> SSRFProtection
    SSRFProtection --> CommandWhitelist
    CommandWhitelist --> PayloadLimit

    %% Audit trail
    RoleCheck -.->|Log| AuditLog
    PayloadLimit -.->|Log| AuditLog
    AuditLog --> AnomalyDetect
    AnomalyDetect -->|Threshold| AlertSystem

    %% Data protection
    SecretMgmt -.->|Provides| APIKeyHash
    SecretMgmt -.->|Provides| OAuthExchange
    Encryption -.->|Protects| AuditLog
    SecureErase -.->|Cleans| TokenValidate

    classDef network fill:#ffebee,stroke:#b71c1c,stroke-width:3px
    classDef auth fill:#e8eaf6,stroke:#283593,stroke-width:3px
    classDef authz fill:#f3e5f5,stroke:#6a1b9a,stroke-width:3px
    classDef protection fill:#e8f5e9,stroke:#2e7d32,stroke-width:3px

    class TLS13,Firewall,DDoS network
    class OAuthProvider,APIKeyExtract,ClientCert auth
    class RoleCheck,ToolACL,ServerACL authz
    class SecretMgmt,Encryption,SecureErase protection
```

**Security Hardening Checklist:**

- [x] TLS 1.3 with modern cipher suites only
- [x] No hardcoded secrets (keyring + environment variables)
- [x] Input validation on all external data
- [x] SSRF protection (block 127.0.0.0/8, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
- [x] Command allowlisting (no arbitrary shell execution)
- [x] Rate limiting per user/IP (60 req/min default)
- [x] Audit logging with tamper detection (SHA3 chain)
- [x] Secure memory handling (zeroize for secrets)
- [x] Regular dependency audits (cargo audit)

---

## 5. Authentication and Authorization Flow

### Complete Auth Flow with RBAC

```mermaid
sequenceDiagram
    autonumber
    participant Client as AI Client
    participant Proxy as Only1MCP Proxy
    participant AuthProvider as OAuth Provider<br/>(Okta/Azure AD)
    participant RBAC as RBAC Engine
    participant Backend as Backend MCP Server
    participant Audit as Audit Logger

    Note over Client,Audit: OAuth2/OIDC Flow

    Client->>Proxy: Initial Request<br/>No credentials
    Proxy->>Client: 401 Unauthorized<br/>WWW-Authenticate: OAuth2

    Client->>AuthProvider: GET /authorize<br/>client_id, redirect_uri, state, PKCE
    AuthProvider->>Client: User login page
    Client->>AuthProvider: POST credentials
    AuthProvider->>Client: 302 Redirect with code

    Client->>Proxy: GET /callback?code=xyz&state=abc
    Proxy->>AuthProvider: POST /token<br/>code, code_verifier (PKCE)
    AuthProvider->>Proxy: access_token, refresh_token, id_token

    Proxy->>Proxy: Validate JWT signature<br/>Check expiry, issuer, audience
    Proxy->>RBAC: Extract claims<br/>sub, roles, permissions

    Note over Proxy,Audit: Authorization Check

    Client->>Proxy: MCP Request<br/>Authorization: Bearer <token>
    Proxy->>Proxy: Extract JWT from header
    Proxy->>Proxy: Verify token signature + expiry

    Proxy->>RBAC: Check permission<br/>user_id, tool_name, server_id
    RBAC->>RBAC: Load user roles<br/>(admin/developer/readonly)
    RBAC->>RBAC: Check tool ACL<br/>Role → Tool mapping
    RBAC->>RBAC: Check server ACL<br/>Role → Server mapping

    alt Permission Granted
        RBAC->>Proxy: Authorized ✓
        Proxy->>Audit: Log: ACCESS_GRANTED<br/>user, tool, timestamp
        Proxy->>Backend: Forward MCP Request
        Backend->>Proxy: MCP Response
        Proxy->>Client: 200 OK + Response
    else Permission Denied
        RBAC->>Proxy: Unauthorized ✗
        Proxy->>Audit: Log: ACCESS_DENIED<br/>user, tool, reason, timestamp
        Proxy->>Client: 403 Forbidden<br/>Insufficient permissions
    end

    Note over Client,Audit: Token Refresh Flow

    Client->>Proxy: Request with expired token
    Proxy->>Proxy: Detect token expiry
    Proxy->>AuthProvider: POST /token<br/>grant_type=refresh_token
    AuthProvider->>Proxy: New access_token
    Proxy->>Client: New token in response header

    Note over Client,Audit: Admin Operations

    Client->>Proxy: Admin API call<br/>(add server, change config)
    Proxy->>RBAC: Check role = admin
    alt Is Admin
        RBAC->>Proxy: Authorized ✓
        Proxy->>Audit: Log: ADMIN_ACTION<br/>action, user, before/after state
        Proxy->>Proxy: Execute admin action
        Proxy->>Client: 200 OK
    else Not Admin
        RBAC->>Proxy: Unauthorized ✗
        Proxy->>Audit: Log: ADMIN_DENIED<br/>user, attempted action
        Proxy->>Client: 403 Forbidden
    end
```

**RBAC Configuration Example:**

```yaml
rbac:
  roles:
    - name: admin
      permissions:
        - "*"  # Full access

    - name: developer
      permissions:
        - "filesystem:*"
        - "github:*"
        - "web_search"
        - "!admin:*"  # Explicit deny admin tools

    - name: readonly
      permissions:
        - "filesystem:read_file"
        - "filesystem:list_directory"
        - "web_search"

  users:
    - email: "admin@company.com"
      roles: ["admin"]

    - email: "dev@company.com"
      roles: ["developer"]

    - email: "analyst@company.com"
      roles: ["readonly"]
```

---

## 6. Context Optimization Pipeline

### Token Reduction Architecture

```mermaid
graph TB
    subgraph "Incoming Request"
        MCPRequest[MCP Request<br/>JSON-RPC]
        RequestSize[Original Size<br/>~3,200 tokens/server]
    end

    subgraph "Optimization Stage 1: Cache Check"
        CacheKey[Generate Cache Key<br/>Hash: tool_name + args]
        CacheLookup{Cache<br/>Hit?}
        CacheHit[Return Cached<br/>90% latency reduction]
        CacheValidate[Validate TTL<br/>Check freshness]
    end

    subgraph "Optimization Stage 2: Request Batching"
        BatchQueue[Batch Queue<br/>Time window: 100ms]
        BatchAccumulate{Window<br/>Expired?}
        BatchCombine[Combine Requests<br/>Single backend call]
        BatchMetrics[Record Batch Size<br/>Target: 30-50 requests]
    end

    subgraph "Optimization Stage 3: Dynamic Loading"
        ToolRegistry[Tool Registry<br/>Lazy schema storage]
        SchemaCheck{Schema<br/>Loaded?}
        SchemaStub[Return Tool Stub<br/>Minimal metadata]
        SchemaLoad[Load Full Schema<br/>On-demand fetch]
        SchemaPrediction[Predictive Loading<br/>ML-based prefetch]
    end

    subgraph "Optimization Stage 4: Payload Compression"
        CompressSelect{Payload<br/>Size?}
        CompressGzip[Gzip<br/>Fast, 60% reduction]
        CompressZstd[Zstd<br/>Best ratio, 70% reduction]
        CompressBrotli[Brotli<br/>Best for text, 75% reduction]
        CompressDictionary[Dictionary Training<br/>MCP-specific patterns]
    end

    subgraph "Optimization Stage 5: Response Trimming"
        ResponseParse[Parse Response<br/>Extract fields]
        FieldFilter[Filter Unnecessary<br/>Remove verbose fields]
        JSONMinify[Minify JSON<br/>Remove whitespace]
        SchemaValidate[Validate Against<br/>MCP schema]
    end

    subgraph "Metrics & Monitoring"
        TokenCounter[Token Counter<br/>tiktoken-rs]
        SavingsCalc[Calculate Savings<br/>Baseline vs Optimized]
        MetricsExport[Export to Prometheus<br/>tokens_saved_total]
    end

    subgraph "Optimized Response"
        OptimizedResp[Optimized Response<br/>50-70% token reduction]
        CacheStore[Store in Cache<br/>TTL: 5-60 min]
    end

    %% Flow
    MCPRequest --> RequestSize
    RequestSize --> CacheKey
    CacheKey --> CacheLookup

    %% Cache hit path
    CacheLookup -->|Hit| CacheValidate
    CacheValidate -->|Valid| CacheHit
    CacheValidate -->|Stale| BatchQueue
    CacheHit --> OptimizedResp

    %% Cache miss path
    CacheLookup -->|Miss| BatchQueue

    %% Batching
    BatchQueue --> BatchAccumulate
    BatchAccumulate -->|No| BatchQueue
    BatchAccumulate -->|Yes| BatchCombine
    BatchCombine --> BatchMetrics
    BatchMetrics --> ToolRegistry

    %% Dynamic loading
    ToolRegistry --> SchemaCheck
    SchemaCheck -->|Loaded| CompressSelect
    SchemaCheck -->|Not Loaded| SchemaStub
    SchemaStub --> SchemaPrediction
    SchemaPrediction --> SchemaLoad
    SchemaLoad --> CompressSelect

    %% Compression
    CompressSelect -->|<1KB| CompressGzip
    CompressSelect -->|1-10KB| CompressZstd
    CompressSelect -->|>10KB| CompressBrotli
    CompressGzip --> CompressDictionary
    CompressZstd --> CompressDictionary
    CompressBrotli --> CompressDictionary
    CompressDictionary --> ResponseParse

    %% Response trimming
    ResponseParse --> FieldFilter
    FieldFilter --> JSONMinify
    JSONMinify --> SchemaValidate
    SchemaValidate --> TokenCounter

    %% Metrics
    TokenCounter --> SavingsCalc
    SavingsCalc --> MetricsExport
    SchemaValidate --> OptimizedResp

    %% Cache store
    OptimizedResp --> CacheStore

    classDef optimization fill:#e8f5e9,stroke:#2e7d32,stroke-width:3px
    classDef metrics fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px

    class CacheKey,BatchCombine,SchemaLoad,CompressZstd,FieldFilter optimization
    class TokenCounter,SavingsCalc,MetricsExport metrics
```

**Optimization Performance Targets:**

| Technique | Token Reduction | Latency Impact | Cache Hit Rate |
|-----------|----------------|----------------|----------------|
| **Response Caching** | 70-90% (on hit) | -85% latency | >70% target |
| **Request Batching** | 30-50% | +50-100ms | N/A |
| **Dynamic Loading** | 90% (initial) | <10ms | N/A |
| **Compression** | 60-75% | +2-5ms | N/A |
| **Payload Trimming** | 40-60% | <1ms | N/A |
| **Combined** | **50-70%** | **<5ms** | **>70%** |

---

## 7. Caching Strategy Architecture

### Multi-Layer Cache Design

```mermaid
graph TB
    subgraph "Cache Layer Architecture"
        subgraph "L1 Cache: Hot Tools"
            L1Hot[DashMap<br/>In-Memory]
            L1TTL[TTL: 5 minutes]
            L1Size[Max: 1000 entries]
            L1Evict[LRU Eviction]
        end

        subgraph "L2 Cache: Warm Resources"
            L2Warm[DashMap<br/>In-Memory]
            L2TTL[TTL: 30 minutes]
            L2Size[Max: 5000 entries]
            L2Evict[LRU Eviction]
        end

        subgraph "L3 Cache: Cold Prompts"
            L3Cold[DashMap<br/>In-Memory]
            L3TTL[TTL: 2 hours]
            L3Size[Max: 10000 entries]
            L3Evict[LRU Eviction]
        end

        subgraph "L4 Cache: Persistent (Optional)"
            L4Disk[Redis/RocksDB<br/>Disk-backed]
            L4TTL[TTL: 24 hours]
            L4Size[Max: 100GB]
        end
    end

    subgraph "Cache Key Generation"
        KeyGen[Cache Key Generator]
        HashAlgo[BLAKE3 Hash<br/>Fast + secure]
        KeyFormat[Format:<br/>version:method:args_hash]
    end

    subgraph "Cache Operations"
        CacheGet{Get<br/>Operation}
        CacheSet[Set Operation<br/>With TTL]
        CacheInvalidate[Invalidate<br/>Pattern-based]
        CacheWarmup[Warmup<br/>Predictive prefetch]
    end

    subgraph "Cache Metrics"
        HitRate[Hit Rate Counter<br/>Per layer]
        MissRate[Miss Rate Counter]
        EvictionCount[Eviction Counter]
        SizeMonitor[Size Monitor<br/>Memory pressure]
    end

    subgraph "Request Flow"
        IncomingReq[Incoming Request]
        CheckL1{In L1?}
        CheckL2{In L2?}
        CheckL3{In L3?}
        CheckL4{In L4?}
        BackendCall[Call Backend<br/>Cache miss]
        PromoteCache[Promote to Higher Layer<br/>Access-based]
    end

    %% Key generation
    IncomingReq --> KeyGen
    KeyGen --> HashAlgo
    HashAlgo --> KeyFormat
    KeyFormat --> CheckL1

    %% Cache layer checks
    CheckL1 -->|Hit| L1Hot
    CheckL1 -->|Miss| CheckL2
    CheckL2 -->|Hit| L2Warm
    CheckL2 -->|Miss| CheckL3
    CheckL3 -->|Hit| L3Cold
    CheckL3 -->|Miss| CheckL4
    CheckL4 -->|Hit| L4Disk
    CheckL4 -->|Miss| BackendCall

    %% Promotion on hit
    L2Warm --> PromoteCache
    L3Cold --> PromoteCache
    L4Disk --> PromoteCache
    PromoteCache --> L1Hot

    %% Backend response caching
    BackendCall --> CacheSet
    CacheSet --> L1Hot
    CacheSet --> L2Warm
    CacheSet --> L3Cold
    CacheSet --> L4Disk

    %% Cache operations
    CacheGet --> CheckL1
    CacheInvalidate -.->|Clear| L1Hot
    CacheInvalidate -.->|Clear| L2Warm
    CacheInvalidate -.->|Clear| L3Cold
    CacheWarmup -.->|Prefetch| L1Hot

    %% Eviction policies
    L1Hot --> L1Evict
    L1Evict -.->|Demote| L2Warm
    L2Warm --> L2Evict
    L2Evict -.->|Demote| L3Cold
    L3Cold --> L3Evict
    L3Evict -.->|Demote| L4Disk

    %% Metrics
    L1Hot -.->|Record| HitRate
    CheckL1 -.->|Record| MissRate
    L1Evict -.->|Record| EvictionCount
    L1Hot -.->|Monitor| SizeMonitor

    classDef cache fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef operation fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef metrics fill:#f3e5f5,stroke:#6a1b9a,stroke-width:2px

    class L1Hot,L2Warm,L3Cold,L4Disk cache
    class CacheGet,CacheSet,CacheInvalidate operation
    class HitRate,MissRate,EvictionCount metrics
```

**Cache Configuration Example:**

```yaml
context_optimization:
  cache:
    enabled: true

    # L1: Hot tools (frequently used)
    l1_hot:
      max_entries: 1000
      ttl_seconds: 300      # 5 minutes
      eviction: lru

    # L2: Warm resources (moderately used)
    l2_warm:
      max_entries: 5000
      ttl_seconds: 1800     # 30 minutes
      eviction: lru

    # L3: Cold prompts (rarely used)
    l3_cold:
      max_entries: 10000
      ttl_seconds: 7200     # 2 hours
      eviction: lru

    # L4: Persistent cache (optional)
    l4_persistent:
      enabled: false
      backend: redis
      url: "redis://localhost:6379"
      ttl_seconds: 86400    # 24 hours
      max_size_mb: 100000   # 100GB

    # Tool-specific overrides
    tool_overrides:
      read_file:
        ttl_seconds: 60     # Files change frequently
      list_directory:
        ttl_seconds: 600    # Directories more stable
      web_search:
        ttl_seconds: 3600   # Search results stable for 1 hour
```

---

## 8. Hot-Reload and Zero-Downtime Pattern

### Configuration Hot-Reload Mechanism

```mermaid
sequenceDiagram
    autonumber
    participant ConfigFile as config.yaml<br/>File System
    participant Watcher as File Watcher<br/>notify-rs
    participant Debouncer as Debouncer<br/>500ms window
    participant Validator as Config Validator<br/>Schema check
    participant RegistryActive as Active Registry<br/>Arc RwLock
    participant RegistryStandby as Standby Registry<br/>Build new state
    participant HealthChecker as Health Checker<br/>Validate backends
    participant ConnectionDrain as Connection Drainer<br/>Graceful transition
    participant Clients as Active Clients<br/>In-flight requests

    Note over ConfigFile,Clients: Configuration Change Initiated

    ConfigFile->>Watcher: File modification event<br/>inotify/FSEvents
    Watcher->>Debouncer: Raw file event
    
    Note over Debouncer: Wait 500ms for<br/>additional changes
    
    Debouncer->>Debouncer: Accumulate events
    Debouncer->>Validator: Debounced event
    
    Validator->>ConfigFile: Read new configuration
    ConfigFile->>Validator: YAML content
    
    Validator->>Validator: Parse YAML<br/>serde_yaml
    Validator->>Validator: Validate schema<br/>Check required fields
    Validator->>Validator: Validate backends<br/>URLs, commands, auth
    
    alt Validation Failed
        Validator->>RegistryActive: Keep current config
        Validator->>Watcher: Log error + continue
        Note over Validator,Watcher: Invalid config rejected<br/>System remains stable
    else Validation Succeeded
        Validator->>RegistryStandby: Build new registry
        
        RegistryStandby->>RegistryStandby: Clone current state
        RegistryStandby->>RegistryStandby: Apply config changes
        RegistryStandby->>RegistryStandby: Initialize new backends
        RegistryStandby->>RegistryStandby: Build hash ring
        
        RegistryStandby->>HealthChecker: Validate new backends
        
        par Health Check All New Backends
            HealthChecker->>HealthChecker: Check backend 1
            HealthChecker->>HealthChecker: Check backend 2
            HealthChecker->>HealthChecker: Check backend N
        end
        
        alt All Health Checks Pass
            HealthChecker->>RegistryStandby: All healthy ✓
            
            Note over RegistryActive,Clients: Begin Atomic Swap
            
            RegistryStandby->>RegistryActive: Request write lock
            RegistryActive->>Clients: Tag in-flight requests<br/>with version ID
            
            Note over Clients: New requests block<br/>briefly (~10-50ms)
            
            RegistryActive->>ConnectionDrain: Identify removed backends
            ConnectionDrain->>ConnectionDrain: Mark for draining
            ConnectionDrain->>ConnectionDrain: Wait for in-flight<br/>completion (30s timeout)
            
            ConnectionDrain->>RegistryActive: Draining complete
            
            RegistryActive->>RegistryActive: Atomic pointer swap<br/>Arc::swap()
            RegistryActive->>Clients: Release write lock
            
            Note over Clients: New requests resume<br/>with new config
            
            RegistryActive->>RegistryStandby: Old registry dropped
            RegistryStandby->>RegistryStandby: Cleanup old connections
            
            RegistryActive->>Watcher: Hot-reload complete ✓<br/>Latency: <100ms
            
        else Health Check Failed
            HealthChecker->>RegistryStandby: Health check failed ✗
            RegistryStandby->>RegistryStandby: Rollback changes
            RegistryStandby->>RegistryActive: Keep current config
            RegistryActive->>Watcher: Log error + continue
            
            Note over RegistryActive,Watcher: Rollback complete<br/>No downtime
        end
    end
    
    Note over ConfigFile,Clients: Hot-Reload Complete<br/>Zero Dropped Requests
```

**Dual-Registry Pattern Implementation:**

```rust
// Dual registry for atomic swaps
pub struct ServerRegistry {
    // Active registry serving requests
    active: Arc<RwLock<RegistryState>>,
    
    // Standby registry for building new state
    standby: Arc<RwLock<Option<RegistryState>>>,
    
    // Configuration version counter
    version: Arc<AtomicU64>,
    
    // File watcher
    watcher: Arc<ConfigWatcher>,
}

// Zero-downtime swap operation
pub async fn hot_swap(&self, new_config: Config) -> Result<(), Error> {
    // 1. Build standby registry
    let new_state = RegistryState::from_config(&new_config).await?;
    
    // 2. Health check all new backends
    new_state.health_check_all().await?;
    
    // 3. Acquire write lock (blocks new requests briefly)
    let mut active = self.active.write().await;
    
    // 4. Atomic swap
    let old_state = std::mem::replace(&mut *active, new_state);
    
    // 5. Increment version
    self.version.fetch_add(1, Ordering::SeqCst);
    
    // 6. Release lock (requests resume with new config)
    drop(active);
    
    // 7. Drain old connections (async, non-blocking)
    tokio::spawn(async move {
        old_state.graceful_shutdown(Duration::from_secs(30)).await;
    });
    
    Ok(())
}
```

**Connection Draining Algorithm:**

```
1. Mark backend as "draining" in registry
2. Stop routing new requests to backend
3. Wait for in-flight requests to complete:
   - Set deadline (30s default)
   - Poll active connection count
   - Log progress every 5s
4. Force-close connections after deadline
5. Clean up resources (file handles, memory)
6. Log final drain statistics
```

---

## 9. Load Balancing Architecture

### Consistent Hashing with Health-Aware Fallback

```mermaid
graph TB
    subgraph "Request Arrival"
        IncomingReq[Incoming MCP Request<br/>tool_name: 'read_file']
    end

    subgraph "Primary: Consistent Hash"
        HashKey[Extract Hash Key<br/>tool_name]
        HashFunc[Hash Function<br/>xxHash3 64-bit]
        VirtualNodes[Virtual Node Ring<br/>BTreeMap sorted by hash]
        
        subgraph "Hash Ring - 150 vnodes/server"
            VNode1[Server A - vnode 0<br/>hash: 0x001A...]
            VNode2[Server A - vnode 1<br/>hash: 0x0F3C...]
            VNode3[Server B - vnode 0<br/>hash: 0x1B2D...]
            VNode4[Server B - vnode 1<br/>hash: 0x2E4F...]
            VNode5[Server C - vnode 0<br/>hash: 0x3A5C...]
            VNodeN[... 450 vnodes total<br/>3 servers × 150]
        end
        
        RingLookup[Binary Search<br/>O log n lookup]
        ServerSelect[Selected Server<br/>Clockwise on ring]
    end

    subgraph "Health Check"
        HealthFilter{Server<br/>Healthy?}
        HealthStatus[Health Status<br/>Active monitoring]
    end

    subgraph "Fallback: Least Connections"
        PowerOfTwo[Power of Two Choices<br/>Sample 2 random]
        ConnCount[Connection Counter<br/>Per backend]
        SelectLeast[Select Server<br/>Fewer connections]
    end

    subgraph "Circuit Breaker"
        CircuitState{Circuit<br/>State?}
        CircuitClosed[Closed<br/>Normal operation]
        CircuitOpen[Open<br/>Fast-fail]
        CircuitHalfOpen[Half-Open<br/>Testing recovery]
        
        ErrorRate[Error Rate Monitor<br/>50% threshold]
        ErrorCount[Error Counter<br/>Sliding window]
    end

    subgraph "Final Selection"
        FinalServer[Selected Backend<br/>Server B]
        RoutingMetrics[Record Metrics<br/>Prometheus]
    end

    %% Primary flow
    IncomingReq --> HashKey
    HashKey --> HashFunc
    HashFunc --> VirtualNodes
    
    VirtualNodes --> VNode1
    VirtualNodes --> VNode2
    VirtualNodes --> VNode3
    VirtualNodes --> VNode4
    VirtualNodes --> VNode5
    VirtualNodes --> VNodeN
    
    VNode1 --> RingLookup
    VNode2 --> RingLookup
    VNode3 --> RingLookup
    VNode4 --> RingLookup
    VNode5 --> RingLookup
    VNodeN --> RingLookup
    
    RingLookup --> ServerSelect
    ServerSelect --> HealthFilter
    
    %% Health check
    HealthFilter -->|Healthy| HealthStatus
    HealthStatus --> CircuitState
    
    HealthFilter -->|Unhealthy| PowerOfTwo
    
    %% Fallback
    PowerOfTwo --> ConnCount
    ConnCount --> SelectLeast
    SelectLeast --> HealthFilter
    
    %% Circuit breaker
    CircuitState -->|Closed| CircuitClosed
    CircuitState -->|Open| CircuitOpen
    CircuitState -->|Half-Open| CircuitHalfOpen
    
    CircuitClosed --> FinalServer
    CircuitHalfOpen --> FinalServer
    
    CircuitOpen --> ErrorRate
    ErrorRate --> PowerOfTwo
    
    FinalServer -.->|Success| ErrorCount
    FinalServer -.->|Error| ErrorCount
    ErrorCount -.->|Update| ErrorRate
    
    %% Final
    FinalServer --> RoutingMetrics

    classDef primary fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef fallback fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef health fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    
    class HashFunc,VirtualNodes,RingLookup primary
    class PowerOfTwo,ConnCount,SelectLeast fallback
    class HealthFilter,CircuitState,ErrorRate health
```

**Load Balancing Algorithms:**

### 1. Consistent Hashing (Primary)

```rust
// Consistent hash ring implementation
pub struct ConsistentHashRing {
    ring: BTreeMap<u64, (ServerId, u32)>,  // hash -> (server, vnode)
    virtual_nodes: u32,  // 150-200 recommended
}

// Add server: O(V) where V = virtual_nodes
pub fn add_server(&mut self, server_id: &ServerId) {
    for vnode in 0..self.virtual_nodes {
        let key = format!("{}:{}", server_id, vnode);
        let hash = xxhash_rust::xxh3::xxh3_64(key.as_bytes());
        self.ring.insert(hash, (server_id.clone(), vnode));
    }
}

// Lookup: O(log N) where N = total virtual nodes
pub fn get_server(&self, request: &McpRequest) -> Option<&ServerId> {
    let key = request.method.as_str();
    let hash = xxhash_rust::xxh3::xxh3_64(key.as_bytes());
    
    // Find first node >= hash (clockwise on ring)
    self.ring
        .range(hash..)
        .next()
        .or_else(|| self.ring.iter().next())  // Wrap around
        .map(|(_, (id, _))| id)
}
```

**Benefits:**

- Minimal key remapping on server add/remove (only K/n keys)
- Even load distribution with sufficient virtual nodes
- Deterministic routing (same request → same server)

### 2. Least Connections (Fallback)

```rust
// Power of Two Choices algorithm: O(1)
pub fn select_least_loaded(&self, backends: &[ServerId]) -> ServerId {
    use rand::seq::SliceRandom;
    
    // Sample 2 random servers
    let sample: Vec<_> = backends
        .choose_multiple(&mut rand::thread_rng(), 2)
        .collect();
    
    // Return server with fewer connections
    sample.iter()
        .min_by_key(|id| self.connection_count(id))
        .unwrap()
        .clone()
}
```

**Benefits:**

- Balances load without global state
- Converges to optimal distribution
- Fast selection (constant time)

---

## 10. Health Checking and Circuit Breaker

### Hybrid Health Monitoring

```mermaid
stateDiagram-v2
    [*] --> Healthy: Initial State

    state Healthy {
        [*] --> ActiveProbe
        ActiveProbe --> SuccessCount: Probe OK
        SuccessCount --> ActiveProbe: Continue
    }

    state Unhealthy {
        [*] --> FailureDetection
        FailureDetection --> AttemptRecovery: Wait interval
        AttemptRecovery --> RetryProbe: After 30s
    }

    state Degraded {
        [*] --> PartialFailure
        PartialFailure --> MonitorRate: Track error rate
        MonitorRate --> PartialFailure: Continue
    }

    Healthy --> Degraded: Error rate > 10%<br/>fall=3 consecutive
    Degraded --> Unhealthy: Error rate > 50%<br/>Circuit opens
    Unhealthy --> Degraded: Probe success<br/>rise=2 consecutive
    Degraded --> Healthy: Error rate < 5%<br/>Sustained recovery
    
    Healthy --> Unhealthy: Critical failure<br/>Connection refused

    note right of Healthy
        Active Probes: 10s interval
        Passive: Monitor all requests
        Circuit: CLOSED
    end note

    note right of Degraded
        Active Probes: 5s interval
        Reduced traffic: 50%
        Circuit: HALF-OPEN
    end note

    note right of Unhealthy
        Active Probes: 30s interval
        No traffic routed
        Circuit: OPEN
    end note
```

### Health Check Implementation

```mermaid
graph TB
    subgraph "Active Health Checking"
        ProbeScheduler[Probe Scheduler<br/>tokio::interval]
        ProbeInterval[Interval: 10s healthy<br/>5s degraded, 30s unhealthy]
        
        subgraph "STDIO Health"
            STDIOProbe[Send health check<br/>JSON-RPC: health/status]
            STDIOTimeout[Timeout: 5s]
            STDIOResponse[Parse response<br/>Check exit code]
        end
        
        subgraph "HTTP Health"
            HTTPProbe[GET /health<br/>or POST /mcp]
            HTTPTimeout[Timeout: 5s]
            HTTPResponse[Check status 200<br/>Parse JSON]
        end
    end

    subgraph "Passive Health Checking"
        RequestMonitor[Monitor All Requests<br/>Real traffic]
        ErrorDetector[Error Detector<br/>Track failures]
        LatencyMonitor[Latency Monitor<br/>Track p99]
        
        subgraph "Error Types"
            ConnectionError[Connection Refused<br/>DNS failure]
            TimeoutError[Request Timeout<br/>> 30s]
            ProtocolError[Invalid Response<br/>Parse error]
            ApplicationError[5xx Status<br/>Backend error]
        end
    end

    subgraph "Circuit Breaker State Machine"
        CircuitClosed[CLOSED<br/>Normal Operation]
        CircuitOpen[OPEN<br/>Fast Fail]
        CircuitHalfOpen[HALF-OPEN<br/>Testing Recovery]
        
        ErrorThreshold{Error Rate<br/>> 50%?}
        ConsecutiveFails{Consecutive<br/>Fails > 3?}
        SuccessTest{Test<br/>Success?}
        TimerExpired{30s<br/>Timer?}
    end

    subgraph "Health Status Update"
        HealthRegistry[Health Registry<br/>Arc RwLock HashMap]
        HealthMetrics[Metrics Export<br/>Prometheus]
        HealthLog[Structured Logging<br/>State transitions]
    end

    %% Active probing
    ProbeScheduler --> ProbeInterval
    ProbeInterval --> STDIOProbe
    ProbeInterval --> HTTPProbe
    
    STDIOProbe --> STDIOTimeout
    STDIOTimeout --> STDIOResponse
    
    HTTPProbe --> HTTPTimeout
    HTTPTimeout --> HTTPResponse
    
    STDIOResponse --> HealthRegistry
    HTTPResponse --> HealthRegistry

    %% Passive monitoring
    RequestMonitor --> ErrorDetector
    RequestMonitor --> LatencyMonitor
    
    ErrorDetector --> ConnectionError
    ErrorDetector --> TimeoutError
    ErrorDetector --> ProtocolError
    ErrorDetector --> ApplicationError
    
    ConnectionError --> ErrorThreshold
    TimeoutError --> ErrorThreshold
    ProtocolError --> ErrorThreshold
    ApplicationError --> ErrorThreshold
    
    LatencyMonitor --> ErrorThreshold

    %% Circuit breaker
    CircuitClosed --> ErrorThreshold
    ErrorThreshold -->|Yes| ConsecutiveFails
    ConsecutiveFails -->|Yes| CircuitOpen
    
    CircuitOpen --> TimerExpired
    TimerExpired -->|Yes| CircuitHalfOpen
    
    CircuitHalfOpen --> SuccessTest
    SuccessTest -->|Yes| CircuitClosed
    SuccessTest -->|No| CircuitOpen
    
    ErrorThreshold -->|No| CircuitClosed

    %% Status updates
    CircuitClosed --> HealthRegistry
    CircuitOpen --> HealthRegistry
    CircuitHalfOpen --> HealthRegistry
    
    HealthRegistry --> HealthMetrics
    HealthRegistry --> HealthLog

    classDef active fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef passive fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef circuit fill:#ffebee,stroke:#b71c1c,stroke-width:3px
    
    class ProbeScheduler,STDIOProbe,HTTPProbe active
    class RequestMonitor,ErrorDetector,LatencyMonitor passive
    class CircuitClosed,CircuitOpen,CircuitHalfOpen circuit
```

**Health Check Configuration:**

```yaml
health:
  # Active health checking
  active:
    enabled: true
    interval_healthy: 10s
    interval_degraded: 5s
    interval_unhealthy: 30s
    timeout: 5s
    
    # Health check method per transport
    stdio:
      command: "health_check"  # Special MCP method
    http:
      endpoint: "/health"      # Standard endpoint
      method: GET
      expected_status: 200
    
  # Passive health checking
  passive:
    enabled: true
    window_size: 100          # Rolling window of requests
    error_threshold: 0.5      # 50% error rate
    latency_threshold_ms: 5000  # 5s p99 latency
    
  # Circuit breaker
  circuit_breaker:
    enabled: true
    error_threshold: 0.5      # Open at 50% errors
    consecutive_failures: 3   # or 3 consecutive fails
    recovery_timeout: 30s     # Test recovery after 30s
    half_open_requests: 10    # Allow 10 test requests
    
  # State transitions
  transitions:
    healthy_to_degraded:
      fall: 3                 # 3 consecutive failures
      error_rate: 0.1         # or 10% error rate
    degraded_to_unhealthy:
      fall: 5                 # 5 consecutive failures
      error_rate: 0.5         # or 50% error rate
    unhealthy_to_degraded:
      rise: 2                 # 2 consecutive successes
    degraded_to_healthy:
      rise: 5                 # 5 consecutive successes
      duration: 60s           # Sustained for 1 minute
```

---

## 11. Plugin System Architecture

### Native Rust + WASM Dual Architecture

```mermaid
graph TB
    subgraph "Plugin Host - Only1MCP Core"
        PluginManager[Plugin Manager<br/>Lifecycle control]
        PluginRegistry[Plugin Registry<br/>Loaded plugins]
        CapabilityChecker[Capability Checker<br/>Permission validation]
    end

    subgraph "Native Rust Plugins"
        subgraph "Dynamic Loading - libloading"
            DynLib[Shared Library<br/>.so / .dylib / .dll]
            LoadSymbol[Load Symbols<br/>Unsafe FFI]
            PluginInit[Plugin Init Function<br/>extern "C"]
        end
        
        subgraph "Plugin Types"
            AuthPlugin[Auth Plugin<br/>Custom providers]
            TransformPlugin[Transform Plugin<br/>Request/response]
            ProtocolPlugin[Protocol Plugin<br/>Custom transports]
            MonitorPlugin[Monitor Plugin<br/>Observability]
        end
        
        NativePerf[Performance<br/>Native speed, 0% overhead]
    end

    subgraph "WASM Plugins"
        subgraph "WASM Runtime - wasmtime"
            WASMModule[WASM Module<br/>.wasm file]
            WASMInstance[Instance<br/>Sandboxed execution]
            WASISupport[WASI Support<br/>Limited syscalls]
        end
        
        subgraph "Security Sandbox"
            MemoryLimit[Memory Limit<br/>64MB default]
            CPUQuota[CPU Quota<br/>Time limits]
            CapabilityList[Capability List<br/>Allowed operations]
            ResourceMonitor[Resource Monitor<br/>Enforcement]
        end
        
        WASMPerf[Performance<br/>5-10% overhead]
    end

    subgraph "Plugin API Contract"
        PluginTrait[Plugin Trait<br/>Rust interface]
        
        subgraph "Core Methods"
            OnLoad[on_load <br/>Initialization]
            OnRequest[on_request<br/>Pre-processing]
            OnResponse[on_response<br/>Post-processing]
            OnUnload[on_unload<br/>Cleanup]
        end
        
        VersionCheck[Version Check<br/>SemVer compatibility]
    end

    subgraph "Plugin Communication"
        HostFunctions[Host Functions<br/>Provided to plugins]
        
        subgraph "Available APIs"
            LogAPI[Logging API<br/>tracing integration]
            MetricAPI[Metrics API<br/>Prometheus]
            ConfigAPI[Config API<br/>Read settings]
            StorageAPI[Storage API<br/>Persistent data]
        end
    end

    subgraph "Plugin Discovery"
        ScanDir[Scan Directory<br/>/plugins]
        ManifestParse[Parse Manifest<br/>plugin.toml]
        DependencyResolve[Resolve Dependencies<br/>Plugin graph]
    end

    subgraph "Hot-Loading"
        FileWatch[Watch Plugin Dir<br/>notify-rs]
        Reload[Reload Plugin<br/>Graceful swap]
        StatePreserve[Preserve State<br/>Migrate data]
    end

    %% Plugin loading flow
    PluginManager --> ScanDir
    ScanDir --> ManifestParse
    ManifestParse --> DependencyResolve
    DependencyResolve --> PluginRegistry

    %% Native plugin path
    PluginRegistry --> DynLib
    DynLib --> LoadSymbol
    LoadSymbol --> PluginInit
    PluginInit --> AuthPlugin
    PluginInit --> TransformPlugin
    PluginInit --> ProtocolPlugin
    PluginInit --> MonitorPlugin
    
    AuthPlugin --> NativePerf
    TransformPlugin --> NativePerf
    ProtocolPlugin --> NativePerf
    MonitorPlugin --> NativePerf

    %% WASM plugin path
    PluginRegistry --> WASMModule
    WASMModule --> WASMInstance
    WASMInstance --> WASISupport
    WASISupport --> MemoryLimit
    MemoryLimit --> CPUQuota
    CPUQuota --> CapabilityList
    CapabilityList --> ResourceMonitor
    ResourceMonitor --> WASMPerf

    %% API contract
    NativePerf --> PluginTrait
    WASMPerf --> PluginTrait
    PluginTrait --> OnLoad
    PluginTrait --> OnRequest
    PluginTrait --> OnResponse
    PluginTrait --> OnUnload
    PluginTrait --> VersionCheck

    %% Capability checking
    OnRequest --> CapabilityChecker
    OnResponse --> CapabilityChecker

    %% Host functions
    CapabilityChecker --> HostFunctions
    HostFunctions --> LogAPI
    HostFunctions --> MetricAPI
    HostFunctions --> ConfigAPI
    HostFunctions --> StorageAPI

    %% Hot-loading
    FileWatch --> Reload
    Reload --> StatePreserve
    StatePreserve --> PluginManager

    classDef native fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    classDef wasm fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef security fill:#ffebee,stroke:#b71c1c,stroke-width:2px
    
    class DynLib,LoadSymbol,AuthPlugin,TransformPlugin native
    class WASMModule,WASMInstance,WASISupport wasm
    class CapabilityChecker,MemoryLimit,CPUQuota security
```

**Plugin Manifest Example:**

```toml
# plugin.toml - Plugin metadata
[plugin]
name = "custom-auth"
version = "1.0.0"
api_version = "1.0"
author = "Your Company"
description = "Custom OAuth2 provider integration"

[plugin.type]
kind = "native"  # or "wasm"
path = "libcustom_auth.so"

[plugin.capabilities]
required = [
    "network.http",      # Can make HTTP requests
    "config.read",       # Can read configuration
    "secrets.read",      # Can read secrets (with permission)
]

[plugin.dependencies]
# Other plugins this plugin depends on
plugins = []

# Rust crates (for native plugins)
crates = [
    { name = "oauth2", version = "4.4" },
    { name = "reqwest", version = "0.11" },
]

[plugin.config]
# Default configuration values
oauth_provider = "custom_provider"
client_id_env = "CUSTOM_CLIENT_ID"
client_secret_env = "CUSTOM_CLIENT_SECRET"
```

---

## 12. Data Flow - Complete Request Lifecycle

### End-to-End Request Processing

```mermaid
sequenceDiagram
    autonumber
    participant Client as AI Client<br/>(Claude Desktop)
    participant TLS as TLS Layer<br/>Rustls
    participant Auth as Auth Middleware<br/>JWT/OAuth2
    participant RateLimit as Rate Limiter<br/>60 req/min
    participant Router as Request Router<br/>Axum
    participant Cache as Response Cache<br/>DashMap LRU
    participant Optimizer as Context Optimizer<br/>Batch/Compress
    participant Registry as Server Registry<br/>Arc RwLock
    participant LB as Load Balancer<br/>Consistent Hash
    participant Health as Health Checker<br/>Circuit Breaker
    participant Transport as Transport Handler<br/>STDIO/HTTP/SSE
    participant Backend as Backend MCP Server<br/>Filesystem/GitHub
    participant Metrics as Metrics Collector<br/>Prometheus
    participant Audit as Audit Logger<br/>Cryptographic chain

    Note over Client,Audit: Phase 1: Connection & Authentication

    Client->>TLS: MCP Request (HTTPS)<br/>tool/call: read_file
    TLS->>TLS: TLS 1.3 handshake<br/>Verify certificate
    TLS->>Auth: Decrypted request
    
    Auth->>Auth: Extract JWT from header
    Auth->>Auth: Verify signature + expiry
    Auth->>Auth: Extract user claims
    Auth->>RateLimit: Authenticated user
    
    RateLimit->>RateLimit: Check rate limit<br/>60 req/min per user
    alt Rate Limit Exceeded
        RateLimit->>Client: 429 Too Many Requests<br/>Retry-After: 60s
    else Rate Limit OK
        RateLimit->>Audit: Log: REQUEST_RECEIVED<br/>user, tool, timestamp
        RateLimit->>Router: Forward request
    end

    Note over Router,Backend: Phase 2: Routing & Optimization

    Router->>Cache: Check cache<br/>key: hash(tool_name, args)
    
    alt Cache Hit (70% probability)
        Cache->>Router: Cached response<br/>Latency: <1ms
        Router->>Metrics: Record: cache_hit
        Router->>Client: 200 OK + cached response
    else Cache Miss
        Cache->>Metrics: Record: cache_miss
        Cache->>Optimizer: Cache miss, optimize
        
        Optimizer->>Optimizer: Add to batch queue<br/>Window: 100ms
        
        alt Batch Window Active
            Note over Optimizer: Wait for more requests<br/>or timeout
        else Batch Window Expired
            Optimizer->>Optimizer: Combine 30 requests
            Optimizer->>Optimizer: Compress payload<br/>zstd: 70% reduction
        end
        
        Optimizer->>Registry: Get available servers<br/>for tool: read_file
        Registry->>Registry: Filter by tool capability
        Registry->>Registry: Check server health
        
        Registry->>LB: Servers: [fs-server-1, fs-server-2]
        LB->>LB: Hash request key<br/>xxHash3(tool_name)
        LB->>LB: Lookup in hash ring<br/>Binary search: O(log n)
        LB->>Health: Selected: fs-server-1
        
        Health->>Health: Check circuit breaker
        alt Circuit Open
            Health->>LB: Server unavailable
            LB->>LB: Fallback: least connections
            LB->>Health: Selected: fs-server-2
        else Circuit Closed
            Health->>Transport: Forward to fs-server-1
        end

        Note over Transport,Backend: Phase 3: Backend Communication

        Transport->>Transport: Determine transport<br/>Type: STDIO
        Transport->>Transport: Get or spawn process<br/>npx @mcp/filesystem
        Transport->>Backend: Write to stdin<br/>length-prefix + JSON
        
        Backend->>Backend: Read file: /home/user/doc.txt
        Backend->>Transport: Write to stdout<br/>length-prefix + JSON
        
        Transport->>Transport: Parse response<br/>Validate JSON-RPC
        Transport->>Optimizer: Backend response

        Note over Optimizer,Client: Phase 4: Response Processing

        Optimizer->>Optimizer: Decompress response
        Optimizer->>Optimizer: Trim payload<br/>Remove verbose fields
        Optimizer->>Optimizer: Calculate token savings<br/>Baseline vs Optimized
        
        Optimizer->>Cache: Store response<br/>TTL: 5 minutes
        Optimizer->>Metrics: Record: tokens_saved<br/>optimization_ratio
        
        Optimizer->>Router: Optimized response
        Router->>Audit: Log: REQUEST_COMPLETE<br/>user, tool, latency, status
        Router->>TLS: Response payload
        TLS->>Client: 200 OK + response
    end

    Note over Metrics,Audit: Phase 5: Observability

    Metrics->>Metrics: Update counters<br/>requests_total++
    Metrics->>Metrics: Record histogram<br/>request_duration_seconds
    Metrics->>Metrics: Update gauges<br/>active_connections
    
    Audit->>Audit: Cryptographic hash<br/>Chain to previous event
    Audit->>Audit: Append to audit log<br/>Immutable record

    Note over Client,Audit: Request Complete<br/>Total Latency: <20ms (cached)<br/>or <100ms (backend)
```

**Performance Breakdown:**

| Phase | Component | Latency (p50) | Latency (p99) |
|-------|-----------|---------------|---------------|
| **1. TLS + Auth** | Rustls + JWT | 1-2ms | 5ms |
| **2. Cache Lookup** | DashMap | <1ms | 1ms |
| **3. Optimization** | Batch/Compress | 2-3ms | 10ms |
| **4. Load Balancing** | Hash + Health | <1ms | 2ms |
| **5. Backend Call** | STDIO/HTTP | 10-50ms | 200ms |
| **6. Response Process** | Cache Store | 1-2ms | 5ms |
| **Total (Cache Hit)** | | **<5ms** | **10ms** |
| **Total (Cache Miss)** | | **20-60ms** | **200ms** |

---

## 13. Connection Pool Management

### Per-Backend Connection Pooling

```mermaid
graph TB
    subgraph "Connection Pool Architecture"
        subgraph "Pool Manager"
            PoolConfig[Pool Configuration<br/>Per backend]
            PoolFactory[Connection Factory<br/>Create connections]
            PoolState[Pool State<br/>Arc RwLock]
        end

        subgraph "Pool for Backend A - HTTP"
            subgraph "Active Connections"
                ConnA1[Connection 1<br/>In use]
                ConnA2[Connection 2<br/>In use]
                ConnA3[Connection 3<br/>In use]
            end
            
            subgraph "Idle Connections"
                ConnA4[Connection 4<br/>Idle: 30s]
                ConnA5[Connection 5<br/>Idle: 15s]
            end
            
            PoolLimitA[Max: 100 connections]
            IdleMinA[Min Idle: 10]
        end

        subgraph "Pool for Backend B - STDIO"
            subgraph "Process Pool"
                ProcB1[Process 1<br/>PID: 12345]
                ProcB2[Process 2<br/>PID: 12346]
                ProcB3[Process 3<br/>PID: 12347]
            end
            
            ProcessLimit[Max: 5 processes]
            ReusePolicy[Reuse Strategy<br/>Round-robin]
        end
    end

    subgraph "Connection Lifecycle"
        Acquire[Acquire Connection<br/>From pool]
        
        CheckAvail{Idle<br/>Available?}
        CreateNew[Create New<br/>Connection]
        WaitTimeout[Wait for Release<br/>Timeout: 30s]
        
        UseConn[Use Connection<br/>Execute request]
        
        Release[Release Connection<br/>Back to pool]
        ValidateConn[Validate Connection<br/>Health check]
        ReturnIdle[Return to Idle Pool]
        Close[Close Connection<br/>Max idle time exceeded]
    end

    subgraph "Pool Maintenance"
        IdleMonitor[Idle Monitor<br/>Background task]
        IdleTimeout[Idle Timeout Check<br/>5 min default]
        IdleEvict[Evict Stale<br/>Close connection]
        
        HealthProbe[Health Probe<br/>Test connections]
        RemoveBad[Remove Unhealthy<br/>From pool]
        
        PoolResize[Dynamic Resize<br/>Based on load]
    end

    subgraph "Metrics"
        PoolMetrics[Pool Metrics]
        ActiveCount[Active Count<br/>Gauge]
        IdleCount[Idle Count<br/>Gauge]
        WaitTime[Wait Time<br/>Histogram]
        CreateRate[Creation Rate<br/>Counter]
    end

    %% Pool manager
    PoolConfig --> PoolFactory
    PoolFactory --> PoolState
    PoolState --> PoolLimitA
    PoolState --> ProcessLimit

    %% Connection lifecycle
    Acquire --> CheckAvail
    CheckAvail -->|Yes| UseConn
    CheckAvail -->|No| CreateNew
    CheckAvail -->|Pool Full| WaitTimeout
    
    CreateNew --> PoolFactory
    PoolFactory --> UseConn
    WaitTimeout --> UseConn
    
    UseConn --> Release
    Release --> ValidateConn
    ValidateConn -->|Healthy| ReturnIdle
    ValidateConn -->|Unhealthy| Close
    ReturnIdle --> ConnA4
    ReturnIdle --> ConnA5

    %% Maintenance
    IdleMonitor --> IdleTimeout
    IdleTimeout --> IdleEvict
    IdleEvict --> Close
    
    HealthProbe --> ValidateConn
    RemoveBad --> Close
    
    PoolResize --> PoolFactory
    PoolResize --> IdleEvict

    %% Metrics
    UseConn -.->|Record| ActiveCount
    ReturnIdle -.->|Record| IdleCount
    WaitTimeout -.->|Record| WaitTime
    CreateNew -.->|Record| CreateRate

    classDef pool fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef lifecycle fill:#fff3e0,stroke:#e65100,stroke-width:2px
    classDef maintenance fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    
    class PoolConfig,PoolFactory,PoolLimitA pool
    class Acquire,UseConn,Release lifecycle
    class IdleMonitor,HealthProbe,PoolResize maintenance
```

**Connection Pool Configuration:**

```yaml
proxy:
  connection_pool:
    # Global pool settings
    max_per_backend: 100         # Maximum connections per backend
    min_idle: 10                 # Minimum idle connections to maintain
    max_idle_time_ms: 300000     # Close idle connections after 5 min
    connection_timeout_ms: 30000 # Timeout when acquiring connection
    validation_timeout_ms: 5000  # Timeout for connection validation
    
    # HTTP-specific
    http:
      keep_alive: true           # Enable HTTP keep-alive
      tcp_nodelay: true          # Disable Nagle's algorithm
      pool_idle_timeout_ms: 90000 # 90s idle timeout
      
    # STDIO-specific
    stdio:
      max_processes: 5           # Maximum concurrent processes
      reuse_strategy: "round_robin"  # or "least_used"
      process_spawn_timeout_ms: 5000
      process_idle_timeout_ms: 600000  # 10 min
      
    # Maintenance
    maintenance_interval_ms: 60000  # Check every minute
    health_check_on_acquire: true   # Validate before use
    health_check_on_release: true   # Validate after use
```

---

## 14. Monitoring and Observability

### Comprehensive Observability Stack

```mermaid
graph TB
    subgraph "Application Instrumentation"
        AppCode[Only1MCP Application<br/>Rust Code]
        
        subgraph "Tracing"
            TracingMacro[tracing::instrument<br/>Function decorators]
            SpanContext[Span Context<br/>Nested traces]
            EventLog[Event Logging<br/>Structured logs]
        end
        
        subgraph "Metrics"
            PrometheusClient[Prometheus Client<br/>Registry + collectors]
            
            subgraph "Metric Types"
                Counter[Counters<br/>requests_total]
                Gauge[Gauges<br/>active_connections]
                Histogram[Histograms<br/>request_duration]
                Summary[Summaries<br/>quantiles]
            end
        end
    end

    subgraph "Data Collection"
        subgraph "Logs"
            LogCollector[Log Collector<br/>tracing-subscriber]
            LogFormat[Format: JSON<br/>Structured output]
            LogLevels[Levels: ERROR, WARN<br/>INFO, DEBUG, TRACE]
        end
        
        subgraph "Traces"
            OTELExporter[OTLP Exporter<br/>OpenTelemetry]
            TraceID[Trace ID<br/>Request correlation]
            SpanID[Span ID<br/>Operation tracking]
        end
        
        subgraph "Metrics Export"
            MetricsEndpoint[/metrics Endpoint<br/>Prometheus format]
            ScrapeInterval[Scrape: 15s]
        end
    end

    subgraph "Observability Backends"
        subgraph "Metrics Backend"
            Prometheus[Prometheus<br/>Time-series DB]
            PromQuery[PromQL Queries<br/>Ad-hoc analysis]
            AlertManager[AlertManager<br/>Alert routing]
        end
        
        subgraph "Visualization"
            Grafana[Grafana<br/>Dashboards]
            
            subgraph "Dashboards"
                DashOverview[Overview<br/>Key metrics]
                DashPerf[Performance<br/>Latency breakdown]
                DashCache[Cache<br/>Hit rates, savings]
                DashBackends[Backends<br/>Health, load]
                DashSecurity[Security<br/>Auth, rate limits]
            end
        end
        
        subgraph "Tracing Backend"
            Jaeger[Jaeger<br/>Distributed tracing]
            TraceView[Trace Viewer<br/>Request flow]
            DepGraph[Dependency Graph<br/>Service map]
        end
        
        subgraph "Log Backend"
            LogStorage[Log Storage<br/>Loki/ELK]
            LogSearch[Log Search<br/>Full-text + filters]
            LogAggregation[Log Aggregation<br/>Count, group, analyze]
        end
    end

    subgraph "Alerting"
        AlertRules[Alert Rules<br/>PromQL expressions]
        
        subgraph "Alert Types"
            HighLatency[High Latency<br/>p99 > 200ms]
            LowCacheHit[Low Cache Hit<br/><50%]
            BackendDown[Backend Down<br/>Health = unhealthy]
            ErrorSpike[Error Spike<br/>Rate > 5%]
            RateLimitHit[Rate Limit Hit<br/>Frequent 429s]
        end
        
        AlertChannel[Alert Channels<br/>Slack/PagerDuty/Email]
    end

    %% Application flow
    AppCode --> TracingMacro
    AppCode --> PrometheusClient
    
    TracingMacro --> SpanContext
    SpanContext --> EventLog
    
    PrometheusClient --> Counter
    PrometheusClient --> Gauge
    PrometheusClient --> Histogram
    PrometheusClient --> Summary

    %% Collection
    EventLog --> LogCollector
    LogCollector --> LogFormat
    LogFormat --> LogLevels
    
    SpanContext --> OTELExporter
    OTELExporter --> TraceID
    OTELExporter --> SpanID
    
    Counter --> MetricsEndpoint
    Gauge --> MetricsEndpoint
    Histogram --> MetricsEndpoint
    Summary --> MetricsEndpoint
    MetricsEndpoint --> ScrapeInterval

    %% Backends
    LogLevels --> LogStorage
    LogStorage --> LogSearch
    LogStorage --> LogAggregation
    
    TraceID --> Jaeger
    SpanID --> Jaeger
    Jaeger --> TraceView
    Jaeger --> DepGraph
    
    ScrapeInterval --> Prometheus
    Prometheus --> PromQuery
    Prometheus --> Grafana
    
    Grafana --> DashOverview
    Grafana --> DashPerf
    Grafana --> DashCache
    Grafana --> DashBackends
    Grafana --> DashSecurity

    %% Alerting
    Prometheus --> AlertRules
    AlertRules --> HighLatency
    AlertRules --> LowCacheHit
    AlertRules --> BackendDown
    AlertRules --> ErrorSpike
    AlertRules --> RateLimitHit
    
    HighLatency --> AlertManager
    LowCacheHit --> AlertManager
    BackendDown --> AlertManager
    ErrorSpike --> AlertManager
    RateLimitHit --> AlertManager
    
    AlertManager --> AlertChannel

    classDef instrument fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    classDef backend fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef alert fill:#ffebee,stroke:#b71c1c,stroke-width:2px
    
    class TracingMacro,PrometheusClient,LogCollector instrument
    class Prometheus,Grafana,Jaeger,LogStorage backend
    class AlertRules,HighLatency,BackendDown,ErrorSpike alert
```

**Key Metrics to Monitor:**

### Performance Metrics

```
# Request metrics
only1mcp_requests_total{method, status}           # Counter
only1mcp_request_duration_seconds{method}         # Histogram
only1mcp_requests_in_flight{method}               # Gauge

# Cache metrics
only1mcp_cache_hits_total{layer}                  # Counter
only1mcp_cache_misses_total{layer}                # Counter
only1mcp_cache_size_bytes{layer}                  # Gauge
only1mcp_cache_evictions_total{layer}             # Counter

# Backend metrics
only1mcp_backend_requests_total{backend, status}  # Counter
only1mcp_backend_duration_seconds{backend}        # Histogram
only1mcp_backend_health{backend}                  # Gauge (0=down, 1=up)

# Context optimization
only1mcp_tokens_baseline{server}                  # Gauge
only1mcp_tokens_optimized{server}                 # Gauge
only1mcp_tokens_saved_total                       # Counter
only1mcp_optimization_ratio                       # Histogram

# Connection pools
only1mcp_pool_connections_active{backend}         # Gauge
only1mcp_pool_connections_idle{backend}           # Gauge
only1mcp_pool_wait_duration_seconds{backend}      # Histogram
```

---

## 15. Configuration Management

### Configuration Schema and Validation

```mermaid
graph TB
    subgraph "Configuration Sources"
        YAMLFile[config.yaml<br/>Primary config]
        TOMLFile[config.toml<br/>Alternative format]
        EnvVars[Environment Variables<br/>ONLY1MCP_*]
        CLIArgs[CLI Arguments<br/>--config, --port]
    end

    subgraph "Configuration Loading"
        Loader[Config Loader<br/>Layered loading]
        Merge[Merge Strategy<br/>CLI > Env > File > Defaults]
        Validation[Schema Validation<br/>serde + custom]
    end

    subgraph "Configuration Schema"
        subgraph "Server Config"
            ServerHost[host: String]
            ServerPort[port: u16]
            ServerTLS[tls: TlsConfig]
            ServerWorkers[worker_threads: usize]
        end
        
        subgraph "Backend Servers"
            ServerList[servers: Vec Server Info]
            ServerID[id: String]
            ServerTransport[transport: Transport]
            ServerRouting[routing: RoutingConfig]
        end
        
        subgraph "Proxy Behavior"
            LoadBalancer[load_balancer: LBConfig]
            ConnPool[connection_pool: PoolConfig]
            HotReload[hot_reload: ReloadConfig]
        end
        
        subgraph "Context Optimization"
            CacheConfig[cache: CacheConfig]
            BatchConfig[batching: BatchConfig]
            CompressConfig[compression: CompressConfig]
        end
        
        subgraph "Authentication"
            AuthAdmin[admin: AuthConfig]
            AuthClient[client: AuthConfig]
            RateLimit[rate_limit: RateLimitConfig]
        end
        
        subgraph "Observability"
            MetricsConfig[metrics: MetricsConfig]
            LoggingConfig[logging: LoggingConfig]
            AuditConfig[audit: AuditConfig]
        end
    end

    subgraph "Configuration Watching"
        FileWatcher[File Watcher<br/>notify-rs]
        Debouncer[Debounce Events<br/>500ms window]
        ReloadTrigger[Reload Trigger<br/>Tokio channel]
    end

    subgraph "Configuration Validation"
        SchemaCheck[Schema Check<br/>Required fields]
        TypeCheck[Type Check<br/>Port ranges, URLs]
        LogicCheck[Logic Check<br/>Constraints]
        
        subgraph "Validation Rules"
            PortRange[Port: 1-65535]
            URLFormat[Valid URLs]
            FileExists[File paths exist]
            NoDuplicates[No duplicate IDs]
            ValidTransport[Transport type valid]
        end
    end

    subgraph "Configuration Application"
        AtomicSwap[Atomic Config Swap<br/>Arc swap]
        RegistryUpdate[Update Registry<br/>Server list]
        PoolUpdate[Update Pools<br/>Connection limits]
        CacheUpdate[Update Cache<br/>Size/TTL]
    end

    subgraph "Configuration Storage"
        ActiveConfig[Active Config<br/>Arc Config]
        ConfigVersion[Version Counter<br/>AtomicU64]
        ConfigHistory[Config History<br/>Last 10 versions]
    end

    %% Loading flow
    YAMLFile --> Loader
    TOMLFile --> Loader
    EnvVars --> Loader
    CLIArgs --> Loader
    
    Loader --> Merge
    Merge --> Validation

    %% Schema
    Validation --> ServerHost
    Validation --> ServerList
    Validation --> LoadBalancer
    Validation --> CacheConfig
    Validation --> AuthAdmin
    Validation --> MetricsConfig
    
    ServerHost --> ServerPort
    ServerPort --> ServerTLS
    ServerTLS --> ServerWorkers
    
    ServerList --> ServerID
    ServerID --> ServerTransport
    ServerTransport --> ServerRouting
    
    LoadBalancer --> ConnPool
    ConnPool --> HotReload
    
    CacheConfig --> BatchConfig
    BatchConfig --> CompressConfig
    
    AuthAdmin --> AuthClient
    AuthClient --> RateLimit
    
    MetricsConfig --> LoggingConfig
    LoggingConfig --> AuditConfig

    %% Validation rules
    Validation --> SchemaCheck
    SchemaCheck --> TypeCheck
    TypeCheck --> LogicCheck
    
    LogicCheck --> PortRange
    LogicCheck --> URLFormat
    LogicCheck --> FileExists
    LogicCheck --> NoDuplicates
    LogicCheck --> ValidTransport

    %% Watching
    YAMLFile -.->|Watch| FileWatcher
    FileWatcher --> Debouncer
    Debouncer --> ReloadTrigger
    ReloadTrigger --> Loader

    %% Application
    Validation --> AtomicSwap
    AtomicSwap --> RegistryUpdate
    AtomicSwap --> PoolUpdate
    AtomicSwap --> CacheUpdate
    
    AtomicSwap --> ActiveConfig
    AtomicSwap --> ConfigVersion
    AtomicSwap --> ConfigHistory

    classDef source fill:#e8f5e9,stroke:#2e7d32,stroke-width:2px
    classDef schema fill:#e3f2fd,stroke:#0d47a1,stroke-width:2px
    classDef validation fill:#fff3e0,stroke:#e65100,stroke-width:2px
    
    class YAMLFile,EnvVars,CLIArgs source
    class ServerHost,ServerList,CacheConfig,AuthAdmin schema
    class SchemaCheck,TypeCheck,PortRange,URLFormat validation
```

**Configuration Precedence:**

```
CLI Arguments (highest)
  ↓
Environment Variables
  ↓
Configuration File
  ↓
Default Values (lowest)
```

**Example Configuration with All Sections:**

```yaml
version: "1.0"

# Server configuration
server:
  host: "0.0.0.0"
  port: 8080
  worker_threads: 0  # Auto-detect CPU cores
  
  tls:
    enabled: true
    cert_path: "/etc/ssl/cert.pem"
    key_path: "/etc/ssl/key.pem"
    
  admin:
    enabled: true
    port: 8081
    path: "/admin"

# Backend MCP servers
servers:
  - id: "filesystem"
    name: "Filesystem MCP"
    transport:
      type: "stdio"
      command: "npx"
      args: ["@modelcontextprotocol/server-filesystem", "/home"]
    routing:
      tools: ["read_file", "write_file", "list_directory"]
      priority: 100
    health:
      enabled: true
      interval_seconds: 10
      timeout_seconds: 5

# Proxy behavior
proxy:
  load_balancer:
    algorithm: "consistent_hash"
    hash_key: "tool_name"
    virtual_nodes: 150
    
  connection_pool:
    max_per_backend: 100
    min_idle: 10
    max_idle_time_ms: 300000
    
  hot_reload:
    enabled: true
    watch_interval_ms: 1000
    debounce_ms: 500

# Context optimization
context_optimization:
  cache:
    enabled: true
    max_entries: 10000
    max_size_mb: 500
    ttl_seconds: 300
    
  batching:
    enabled: true
    max_batch_size: 50
    batch_window_ms: 100
    
  compression:
    enabled: true
    algorithm: "zstd"
    level: 3

# Authentication
auth:
  admin:
    enabled: true
    type: "api_key"
    api_key_env: "ONLY1MCP_ADMIN_KEY"
    
  client:
    enabled: true
    type: "oauth2"
    oauth2:
      provider: "okta"
      client_id_env: "OAUTH_CLIENT_ID"
      client_secret_env: "OAUTH_CLIENT_SECRET"
      issuer: "https://company.okta.com"
      
  rate_limit:
    enabled: true
    requests_per_minute: 60
    burst: 10

# Observability
observability:
  metrics:
    enabled: true
    type: "prometheus"
    port: 9090
    path: "/metrics"
    
  logging:
    level: "info"
    format: "json"
    
  audit:
    enabled: true
    log_requests: true
    log_admin: true
```

---

## Summary

This comprehensive architecture diagram collection provides detailed technical visualization of Only1MCP's core systems for the initial development phase:

1. **Overall System Architecture** - High-level component interaction
2. **Core Components** - Internal Rust module relationships
3. **Transport Layer** - Multi-protocol support (STDIO/HTTP/SSE/WebSocket)
4. **Security** - Defense-in-depth with TLS, auth, RBAC
5. **Authentication** - Complete OAuth2/OIDC flow with RBAC
6. **Context Optimization** - 50-70% token reduction pipeline
7. **Caching** - Multi-layer cache architecture
8. **Hot-Reload** - Zero-downtime configuration updates
9. **Load Balancing** - Consistent hashing + health-aware routing
10. **Health Checking** - Active + passive monitoring with circuit breaker
11. **Plugin System** - Native Rust + WASM dual architecture
12. **Request Lifecycle** - Complete end-to-end data flow
13. **Connection Pooling** - Per-backend pool management
14. **Observability** - Metrics, logs, traces, alerts
15. **Configuration** - Schema, validation, hot-reload

These diagrams serve as the definitive technical reference for implementing Only1MCP's core features with:

- **<5ms latency overhead** (p99)
- **10k+ req/s throughput**
- **50-70% token reduction**
- **>99.9% uptime**
- **Zero-downtime operations**

All patterns and architectures are production-ready, validated by research, and aligned with Rust best practices for performance, safety, and maintainability.

---

**Document End**
**Version:** 1.0
**Last Updated:** October 14, 2025
**Status:** Technical Specification - Initial Development Phase
