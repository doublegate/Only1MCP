# 18-Only1MCP Plugin System Architecture
## Dynamic Loading, WASM Modules, and Extensible API Contracts

**Document Version:** 1.0  
**Architecture Scope:** Plugin System, Dynamic Loading, WASM Runtime, API Contracts  
**Target Implementation:** Rust with libloading/wasmtime, Plugin SDK  
**Date:** October 14, 2025  
**Status:** Technical Architecture Specification

---

## TABLE OF CONTENTS

1. [Executive Summary](#executive-summary)
2. [Plugin System Overview](#plugin-system-overview)
3. [Dynamic Loading Architecture](#dynamic-loading-architecture)
4. [WASM Module System](#wasm-module-system)
5. [Plugin API Contracts](#plugin-api-contracts)
6. [Plugin Discovery & Registry](#plugin-discovery--registry)
7. [Security Model](#security-model)
8. [Performance Considerations](#performance-considerations)
9. [Plugin SDK Design](#plugin-sdk-design)
10. [Hot-Loading Mechanism](#hot-loading-mechanism)
11. [Inter-Plugin Communication](#inter-plugin-communication)
12. [Resource Management](#resource-management)
13. [Plugin Marketplace Infrastructure](#plugin-marketplace-infrastructure)
14. [Testing & Debugging](#testing--debugging)
15. [Migration Strategy](#migration-strategy)

---

## EXECUTIVE SUMMARY

### Strategic Vision

The Only1MCP plugin system provides **extensibility without compromising performance or security**, enabling users to customize aggregator behavior through dynamically loaded Rust plugins or sandboxed WASM modules. This architecture supports:

- **Native Performance**: Rust plugins via `libloading` for zero-overhead extensions
- **Sandboxed Security**: WASM modules via `wasmtime` for untrusted third-party code
- **Hot-Swappable**: Plugins can be loaded/unloaded without proxy restart
- **Type-Safe APIs**: Strongly-typed plugin interfaces with versioning support
- **Resource Isolation**: Memory/CPU limits, capability-based security

### Architecture Comparison Matrix

| Feature | Native Rust Plugins | WASM Modules | Docker Extensions |
|---------|-------------------|--------------|-------------------|
| **Performance** | Native speed (0% overhead) | 5-10% overhead | 20-30% overhead |
| **Security** | Process isolation | Sandbox isolation | Container isolation |
| **Memory Usage** | Shared memory | Isolated heap (configurable) | 50-200MB per container |
| **Startup Time** | <10ms | <50ms | 1-3 seconds |
| **Language Support** | Rust only | Any WASM-targeting language | Any language |
| **Hot-Loading** | Yes (with care) | Yes (safe) | No |
| **Use Case** | Trusted extensions | Untrusted/community plugins | Legacy integrations |

**Research Context**: Existing MCP aggregators (TBXark, MCPEz) lack plugin systems entirely, while Docker Toolkit's container-based approach adds significant overhead【Document 03, Section 2】. Our dual approach (native + WASM) provides optimal flexibility.

---

## PLUGIN SYSTEM OVERVIEW

### Core Architecture Components

```rust
//! Plugin system architecture combining native Rust plugins (high performance)
//! with WASM modules (security/portability). Plugins extend Only1MCP functionality
//! without modifying core code, enabling custom transforms, protocol adapters,
//! authentication providers, and monitoring integrations.
//!
//! # Design Principles
//! 
//! - **Capability-Based Security**: Plugins declare required permissions upfront
//! - **Version Compatibility**: Semantic versioning with compatibility checks
//! - **Resource Quotas**: Memory/CPU limits enforced per plugin
//! - **Async-First**: All plugin APIs are async-compatible
//! - **Zero-Copy Data**: Pass references where possible, avoid allocations

use std::sync::Arc;
use async_trait::async_trait;
use semver::Version;

/// Core plugin trait that all plugins must implement
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Plugin metadata including name, version, and capabilities
    fn metadata(&self) -> PluginMetadata;
    
    /// Initialize plugin with configuration
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), PluginError>;
    
    /// Called before plugin unload
    async fn shutdown(&mut self) -> Result<(), PluginError>;
    
    /// Health check for monitoring
    async fn health_check(&self) -> HealthStatus;
}

#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Unique plugin identifier (reverse DNS notation)
    pub id: String,  // e.g., "com.example.custom-auth"
    
    /// Human-readable name
    pub name: String,
    
    /// Semantic version
    pub version: Version,
    
    /// API version compatibility
    pub api_version: Version,
    
    /// Required capabilities
    pub capabilities: Vec<Capability>,
    
    /// Plugin type for routing
    pub plugin_type: PluginType,
    
    /// Author information
    pub author: String,
    
    /// License identifier (SPDX)
    pub license: String,
}

#[derive(Debug, Clone)]
pub enum PluginType {
    /// Modifies requests before routing
    RequestTransform,
    
    /// Modifies responses before returning
    ResponseTransform,
    
    /// Custom authentication provider
    Authentication,
    
    /// New transport protocol support
    Transport,
    
    /// Caching strategy implementation
    Cache,
    
    /// Monitoring/metrics exporter
    Telemetry,
    
    /// Load balancing algorithm
    LoadBalancer,
}

#[derive(Debug, Clone)]
pub enum Capability {
    /// Network access (specify domains)
    Network(Vec<String>),
    
    /// Filesystem access (specify paths)
    Filesystem(Vec<PathBuf>),
    
    /// Access to proxy state
    StateRead,
    StateWrite,
    
    /// Modify configuration
    ConfigAccess,
    
    /// Access to metrics
    MetricsAccess,
    
    /// Maximum memory in MB
    MemoryLimit(usize),
    
    /// Maximum CPU percentage
    CpuLimit(u8),
}
```

### Plugin Lifecycle

```rust
/// Plugin lifecycle manager handles loading, initialization, and unloading
pub struct PluginManager {
    /// Registry of loaded plugins
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
    
    /// Native plugin loader
    native_loader: NativePluginLoader,
    
    /// WASM runtime
    wasm_runtime: WasmRuntime,
    
    /// Configuration
    config: PluginConfig,
    
    /// Metrics collector
    metrics: Arc<PluginMetrics>,
}

impl PluginManager {
    /// Load a plugin from file path
    pub async fn load_plugin(&self, path: &Path) -> Result<String, PluginError> {
        // 1. Determine plugin type by extension
        let plugin_type = match path.extension().and_then(|s| s.to_str()) {
            Some("so") | Some("dll") | Some("dylib") => PluginFormat::Native,
            Some("wasm") => PluginFormat::Wasm,
            _ => return Err(PluginError::UnsupportedFormat),
        };
        
        // 2. Load based on type
        let plugin = match plugin_type {
            PluginFormat::Native => self.native_loader.load(path).await?,
            PluginFormat::Wasm => self.wasm_runtime.load(path).await?,
        };
        
        // 3. Validate metadata
        let metadata = plugin.metadata();
        self.validate_compatibility(&metadata)?;
        
        // 4. Check capabilities
        self.validate_capabilities(&metadata.capabilities)?;
        
        // 5. Initialize plugin
        let config = self.config.for_plugin(&metadata.id);
        plugin.initialize(config).await?;
        
        // 6. Register in manager
        let plugin_id = metadata.id.clone();
        self.plugins.write().await.insert(plugin_id.clone(), plugin);
        
        // 7. Update metrics
        self.metrics.plugin_loaded(&plugin_id);
        
        Ok(plugin_id)
    }
    
    /// Unload a plugin safely
    pub async fn unload_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        // 1. Get plugin
        let mut plugins = self.plugins.write().await;
        let mut plugin = plugins.remove(plugin_id)
            .ok_or(PluginError::PluginNotFound)?;
        
        // 2. Graceful shutdown
        plugin.shutdown().await?;
        
        // 3. Update metrics
        self.metrics.plugin_unloaded(plugin_id);
        
        Ok(())
    }
    
    /// Hot-reload a plugin (unload + load)
    pub async fn reload_plugin(&self, plugin_id: &str) -> Result<(), PluginError> {
        // Store path before unloading
        let path = self.get_plugin_path(plugin_id)?;
        
        // Unload existing
        self.unload_plugin(plugin_id).await?;
        
        // Load new version
        self.load_plugin(&path).await?;
        
        Ok(())
    }
}
```

---

## DYNAMIC LOADING ARCHITECTURE

### Native Rust Plugin Loading

Using `libloading` for cross-platform dynamic library loading:

```rust
//! Native plugin loader using libloading for dynamic shared library loading.
//! Supports hot-reloading with symbol versioning and ABI compatibility checks.
//! 
//! # Safety
//! 
//! Loading native code is inherently unsafe. We mitigate risks through:
//! - Signature verification (optional)
//! - Symbol validation
//! - ABI version checking
//! - Resource sandboxing via cgroups (Linux)

use libloading::{Library, Symbol};
use std::path::Path;

pub struct NativePluginLoader {
    /// Loaded libraries (kept alive for symbol validity)
    libraries: Arc<Mutex<HashMap<String, Library>>>,
    
    /// Symbol cache for performance
    symbol_cache: Arc<DashMap<String, usize>>,
    
    /// Security configuration
    security: SecurityConfig,
}

impl NativePluginLoader {
    /// Load a native plugin from a shared library
    pub async fn load(&self, path: &Path) -> Result<Box<dyn Plugin>, PluginError> {
        // 1. Security validation
        if self.security.verify_signatures {
            self.verify_signature(path).await?;
        }
        
        // 2. Load library
        let library = unsafe {
            Library::new(path)
                .map_err(|e| PluginError::LoadFailed(e.to_string()))?
        };
        
        // 3. Get plugin entry point
        let create_fn: Symbol<fn() -> Box<dyn Plugin>> = unsafe {
            library.get(b"_create_plugin\0")
                .map_err(|e| PluginError::SymbolNotFound("_create_plugin".into()))?
        };
        
        // 4. Create plugin instance
        let plugin = create_fn();
        
        // 5. Verify ABI compatibility
        self.verify_abi_compatibility(&plugin)?;
        
        // 6. Store library reference
        let plugin_id = plugin.metadata().id.clone();
        self.libraries.lock().unwrap().insert(plugin_id, library);
        
        Ok(plugin)
    }
    
    /// Verify plugin signature (optional security feature)
    async fn verify_signature(&self, path: &Path) -> Result<(), PluginError> {
        use ed25519_dalek::{PublicKey, Signature, Verifier};
        
        // Read signature file (plugin.so.sig)
        let sig_path = path.with_extension("sig");
        let signature_bytes = tokio::fs::read(&sig_path).await
            .map_err(|_| PluginError::SignatureMissing)?;
        
        // Parse signature
        let signature = Signature::from_bytes(&signature_bytes)
            .map_err(|_| PluginError::InvalidSignature)?;
        
        // Read plugin binary
        let plugin_bytes = tokio::fs::read(path).await?;
        
        // Verify with public key
        let public_key = PublicKey::from_bytes(&self.security.public_key)
            .map_err(|_| PluginError::InvalidPublicKey)?;
        
        public_key.verify(&plugin_bytes, &signature)
            .map_err(|_| PluginError::SignatureVerificationFailed)?;
        
        Ok(())
    }
    
    /// Check ABI compatibility using version markers
    fn verify_abi_compatibility(&self, plugin: &Box<dyn Plugin>) -> Result<(), PluginError> {
        let metadata = plugin.metadata();
        let current_api = Version::parse(API_VERSION).unwrap();
        
        // Check if plugin API version is compatible
        if !current_api.is_compatible(&metadata.api_version) {
            return Err(PluginError::IncompatibleApi {
                expected: current_api,
                found: metadata.api_version,
            });
        }
        
        Ok(())
    }
}

/// Plugin creation macro for native plugins
#[macro_export]
macro_rules! declare_plugin {
    ($plugin_type:ty, $constructor:expr) => {
        #[no_mangle]
        pub extern "C" fn _create_plugin() -> Box<dyn Plugin> {
            Box::new($constructor)
        }
        
        #[no_mangle]
        pub extern "C" fn _plugin_api_version() -> &'static str {
            env!("CARGO_PKG_VERSION")
        }
    };
}
```

### Platform-Specific Considerations

```rust
/// Platform-specific dynamic loading configuration
#[cfg(target_os = "linux")]
mod platform {
    pub const LIBRARY_PREFIX: &str = "lib";
    pub const LIBRARY_EXTENSION: &str = "so";
    
    /// Linux-specific: Use cgroups for resource isolation
    pub fn apply_resource_limits(pid: u32, limits: &ResourceLimits) -> Result<(), Error> {
        use cgroups_rs::{cgroup_builder::CgroupBuilder, CgroupPid};
        
        let cgroup = CgroupBuilder::new("only1mcp_plugin")
            .memory()
                .limit_in_bytes(limits.memory_mb * 1024 * 1024)
                .done()
            .cpu()
                .shares(limits.cpu_shares)
                .done()
            .build()?;
        
        cgroup.add_task(CgroupPid::from(pid))?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    pub const LIBRARY_PREFIX: &str = "lib";
    pub const LIBRARY_EXTENSION: &str = "dylib";
    
    /// macOS-specific: Use sandbox-exec for isolation
    pub fn apply_resource_limits(pid: u32, limits: &ResourceLimits) -> Result<(), Error> {
        // Use sandbox-exec with custom profile
        std::process::Command::new("sandbox-exec")
            .arg("-p")
            .arg(generate_sandbox_profile(limits))
            .arg(&format!("{}", pid))
            .spawn()?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod platform {
    pub const LIBRARY_PREFIX: &str = "";
    pub const LIBRARY_EXTENSION: &str = "dll";
    
    /// Windows-specific: Use job objects for resource limits
    pub fn apply_resource_limits(pid: u32, limits: &ResourceLimits) -> Result<(), Error> {
        use windows::Win32::System::JobObjects::*;
        
        let job = unsafe { CreateJobObjectW(None, None)? };
        
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_PROCESS_MEMORY;
        info.ProcessMemoryLimit = limits.memory_mb * 1024 * 1024;
        
        unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const c_void,
                std::mem::size_of_val(&info) as u32,
            )?;
        }
        
        Ok(())
    }
}
```

---

## WASM MODULE SYSTEM

### WASM Runtime Integration

Using `wasmtime` for secure WASM execution:

```rust
//! WASM plugin runtime using Wasmtime for sandboxed execution.
//! Provides memory safety, resource limits, and capability-based security
//! for untrusted third-party plugins.
//!
//! # Features
//!
//! - Memory isolation with configurable limits
//! - CPU instruction counting/fuel system
//! - WASI for controlled system access
//! - Component model support (future)

use wasmtime::*;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};

pub struct WasmRuntime {
    /// Wasmtime engine (shared across instances)
    engine: Engine,
    
    /// Module cache for performance
    module_cache: Arc<DashMap<String, Module>>,
    
    /// Active instances
    instances: Arc<RwLock<HashMap<String, WasmInstance>>>,
    
    /// Resource limiter
    resource_limiter: Arc<ResourceLimiter>,
}

pub struct WasmInstance {
    /// WASM store with state
    store: Store<WasmState>,
    
    /// Instantiated module
    instance: Instance,
    
    /// Exported functions
    exports: PluginExports,
    
    /// Plugin metadata
    metadata: PluginMetadata,
}

/// WASM plugin state
struct WasmState {
    /// WASI context for system access
    wasi_ctx: WasiCtx,
    
    /// Resource limits
    limits: ResourceLimits,
    
    /// Plugin-specific data
    plugin_data: Vec<u8>,
}

impl WasmRuntime {
    /// Create a new WASM runtime with configuration
    pub fn new(config: WasmConfig) -> Result<Self, Error> {
        // Configure engine with optimizations
        let mut engine_config = Config::new();
        engine_config.wasm_simd(true);
        engine_config.wasm_bulk_memory(true);
        engine_config.wasm_reference_types(true);
        engine_config.cranelift_opt_level(OptLevel::Speed);
        
        // Enable fuel metering for CPU limits
        engine_config.consume_fuel(true);
        
        // Memory configuration
        engine_config.memory_guaranteed_dense_image_size(config.max_memory_per_instance);
        
        let engine = Engine::new(&engine_config)?;
        
        Ok(Self {
            engine,
            module_cache: Arc::new(DashMap::new()),
            instances: Arc::new(RwLock::new(HashMap::new())),
            resource_limiter: Arc::new(ResourceLimiter::new(config.limits)),
        })
    }
    
    /// Load a WASM plugin module
    pub async fn load(&self, path: &Path) -> Result<Box<dyn Plugin>, PluginError> {
        // 1. Read WASM bytecode
        let wasm_bytes = tokio::fs::read(path).await?;
        
        // 2. Validate module
        self.validate_module(&wasm_bytes)?;
        
        // 3. Compile module (cached)
        let module = self.compile_module(&wasm_bytes).await?;
        
        // 4. Create store with limits
        let mut store = self.create_store()?;
        
        // 5. Instantiate module
        let instance = Instance::new(&mut store, &module, &[])
            .map_err(|e| PluginError::InstantiationFailed(e.to_string()))?;
        
        // 6. Get exports
        let exports = self.get_exports(&instance, &mut store)?;
        
        // 7. Get metadata
        let metadata = self.call_metadata(&exports, &mut store)?;
        
        // 8. Create plugin wrapper
        let plugin = WasmPluginAdapter::new(
            self.clone(),
            instance,
            store,
            exports,
            metadata,
        );
        
        Ok(Box::new(plugin))
    }
    
    /// Validate WASM module safety
    fn validate_module(&self, bytes: &[u8]) -> Result<(), PluginError> {
        // Check magic number
        if &bytes[0..4] != b"\0asm" {
            return Err(PluginError::InvalidWasm("Invalid magic number"));
        }
        
        // Parse and validate
        wasmparser::Validator::new()
            .validate_all(bytes)
            .map_err(|e| PluginError::InvalidWasm(e.to_string()))?;
        
        // Additional security checks
        self.check_imports(bytes)?;
        self.check_exports(bytes)?;
        
        Ok(())
    }
    
    /// Create store with WASI and resource limits
    fn create_store(&self) -> Result<Store<WasmState>, Error> {
        // Build WASI context with restrictions
        let wasi_ctx = WasiCtxBuilder::new()
            // No filesystem access by default
            .inherit_stdio()  // Allow console output
            .build();
        
        let state = WasmState {
            wasi_ctx,
            limits: self.resource_limiter.default_limits(),
            plugin_data: Vec::new(),
        };
        
        let mut store = Store::new(&self.engine, state);
        
        // Set fuel limit (CPU instructions)
        store.set_fuel(1_000_000)?;  // 1M instructions
        
        // Set memory limit
        store.limiter(|state| &mut state.limits);
        
        Ok(store)
    }
    
    /// Get exported functions from instance
    fn get_exports(&self, instance: &Instance, store: &mut Store<WasmState>) 
        -> Result<PluginExports, PluginError> {
        Ok(PluginExports {
            initialize: instance.get_typed_func::<(i32, i32), i32>(store, "initialize")?,
            shutdown: instance.get_typed_func::<(), i32>(store, "shutdown")?,
            process_request: instance.get_typed_func::<(i32, i32), i32>(store, "process_request")?,
            process_response: instance.get_typed_func::<(i32, i32), i32>(store, "process_response")?,
            get_metadata: instance.get_typed_func::<(), i32>(store, "get_metadata")?,
            
            // Memory access
            memory: instance.get_memory(store, "memory")
                .ok_or(PluginError::MemoryNotExported)?,
            
            // Allocator functions (for passing data)
            alloc: instance.get_typed_func::<i32, i32>(store, "alloc")?,
            free: instance.get_typed_func::<(i32, i32), ()>(store, "free")?,
        })
    }
}

/// Resource limiter implementation
impl ResourceLimiter for WasmState {
    fn memory_growing(&mut self, current: usize, desired: usize, _maximum: Option<usize>) -> bool {
        let limit = self.limits.memory_mb * 1024 * 1024;
        desired <= limit
    }
    
    fn table_growing(&mut self, current: u32, desired: u32, _maximum: Option<u32>) -> bool {
        desired <= 10000  // Reasonable table limit
    }
}
```

### WASM Plugin Interface (WIT)

```wit
// plugin.wit - WebAssembly Interface Type definitions for plugins

interface plugin {
    // Metadata about the plugin
    record metadata {
        id: string,
        name: string,
        version: string,
        api-version: string,
        capabilities: list<string>,
    }
    
    // Request structure
    record mcp-request {
        jsonrpc: string,
        id: option<u64>,
        method: string,
        params: option<string>,  // JSON string
    }
    
    // Response structure
    record mcp-response {
        jsonrpc: string,
        id: option<u64>,
        result: option<string>,  // JSON string
        error: option<error-info>,
    }
    
    record error-info {
        code: s32,
        message: string,
        data: option<string>,
    }
    
    // Plugin lifecycle
    initialize: func(config: string) -> result<unit, string>
    shutdown: func() -> result<unit, string>
    
    // Request/response processing
    process-request: func(request: mcp-request) -> result<mcp-request, string>
    process-response: func(response: mcp-response) -> result<mcp-response, string>
    
    // Metadata
    get-metadata: func() -> metadata
    
    // Health check
    health-check: func() -> result<string, string>
}

// Host functions available to plugins
interface host {
    // Logging
    log: func(level: string, message: string)
    
    // Metrics
    record-metric: func(name: string, value: float64, labels: list<tuple<string, string>>)
    
    // State access (if capability granted)
    get-state: func(key: string) -> option<string>
    set-state: func(key: string, value: string) -> result<unit, string>
    
    // HTTP client (if network capability granted)  
    http-get: func(url: string) -> result<string, string>
    http-post: func(url: string, body: string) -> result<string, string>
}
```

---

## PLUGIN API CONTRACTS

### Core Plugin Interfaces

```rust
//! Strongly-typed plugin API contracts with versioning support.
//! All plugins must implement these interfaces for integration.

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

/// Request transformer plugin - modifies requests before routing
#[async_trait]
pub trait RequestTransformer: Plugin {
    /// Transform an incoming request
    async fn transform_request(
        &self,
        request: &mut McpRequest,
        context: &RequestContext,
    ) -> Result<TransformResult, PluginError>;
    
    /// Check if this transformer should process the request
    async fn should_transform(&self, request: &McpRequest) -> bool {
        true  // Default: transform all requests
    }
}

/// Response transformer plugin - modifies responses before returning
#[async_trait]
pub trait ResponseTransformer: Plugin {
    /// Transform an outgoing response
    async fn transform_response(
        &self,
        response: &mut McpResponse,
        context: &ResponseContext,
    ) -> Result<TransformResult, PluginError>;
}

/// Authentication provider plugin
#[async_trait]
pub trait AuthenticationProvider: Plugin {
    /// Authenticate a request
    async fn authenticate(
        &self,
        request: &McpRequest,
        credentials: &Credentials,
    ) -> Result<AuthResult, PluginError>;
    
    /// Refresh authentication tokens
    async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<TokenPair, PluginError>;
    
    /// Validate permissions for a request
    async fn authorize(
        &self,
        principal: &Principal,
        request: &McpRequest,
    ) -> Result<bool, PluginError>;
}

/// Custom transport protocol plugin
#[async_trait]
pub trait TransportProvider: Plugin {
    /// Send a request via custom transport
    async fn send_request(
        &self,
        server: &ServerInfo,
        request: &McpRequest,
    ) -> Result<McpResponse, PluginError>;
    
    /// Check if this transport handles the server
    fn supports_server(&self, server: &ServerInfo) -> bool;
    
    /// Establish connection (for persistent transports)
    async fn connect(&self, server: &ServerInfo) -> Result<Connection, PluginError>;
    
    /// Close connection
    async fn disconnect(&self, conn: &Connection) -> Result<(), PluginError>;
}

/// Cache strategy plugin
#[async_trait]
pub trait CacheStrategy: Plugin {
    /// Check cache for a request
    async fn get(
        &self,
        key: &CacheKey,
    ) -> Option<McpResponse>;
    
    /// Store response in cache
    async fn set(
        &self,
        key: &CacheKey,
        value: &McpResponse,
        ttl: Duration,
    ) -> Result<(), PluginError>;
    
    /// Invalidate cache entries
    async fn invalidate(
        &self,
        pattern: &str,
    ) -> Result<u64, PluginError>;
    
    /// Get cache statistics
    async fn stats(&self) -> CacheStats;
}

/// Load balancer plugin
#[async_trait]
pub trait LoadBalancer: Plugin {
    /// Select a backend server for a request
    async fn select_server(
        &self,
        request: &McpRequest,
        servers: &[ServerInfo],
    ) -> Result<String, PluginError>;
    
    /// Update server health status
    async fn update_health(
        &self,
        server_id: &str,
        healthy: bool,
    );
    
    /// Get load balancing statistics
    async fn stats(&self) -> LoadBalancerStats;
}

/// Telemetry exporter plugin
#[async_trait]
pub trait TelemetryExporter: Plugin {
    /// Export metrics
    async fn export_metrics(
        &self,
        metrics: &[Metric],
    ) -> Result<(), PluginError>;
    
    /// Export traces
    async fn export_traces(
        &self,
        spans: &[Span],
    ) -> Result<(), PluginError>;
    
    /// Export logs
    async fn export_logs(
        &self,
        logs: &[LogEntry],
    ) -> Result<(), PluginError>;
}
```

### Plugin Communication Protocol

```rust
/// Inter-plugin communication for complex workflows
pub struct PluginBus {
    /// Message channels between plugins
    channels: Arc<DashMap<String, mpsc::Sender<PluginMessage>>>,
    
    /// Plugin subscriptions
    subscriptions: Arc<DashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMessage {
    /// Source plugin ID
    pub from: String,
    
    /// Target plugin ID (or "*" for broadcast)
    pub to: String,
    
    /// Message type
    pub message_type: String,
    
    /// Message payload (JSON)
    pub payload: serde_json::Value,
    
    /// Correlation ID for request-response
    pub correlation_id: Option<Uuid>,
    
    /// Timestamp
    pub timestamp: SystemTime,
}

impl PluginBus {
    /// Send message between plugins
    pub async fn send(&self, message: PluginMessage) -> Result<(), BusError> {
        if message.to == "*" {
            // Broadcast to all subscribed plugins
            self.broadcast(message).await
        } else {
            // Direct message to specific plugin
            self.direct_send(message).await
        }
    }
    
    /// Subscribe to message types
    pub async fn subscribe(
        &self,
        plugin_id: &str,
        pattern: &str,
    ) -> mpsc::Receiver<PluginMessage> {
        let (tx, rx) = mpsc::channel(100);
        
        self.channels.insert(plugin_id.to_string(), tx);
        
        self.subscriptions
            .entry(pattern.to_string())
            .or_insert_with(Vec::new)
            .push(plugin_id.to_string());
        
        rx
    }
    
    /// Request-response pattern
    pub async fn request(
        &self,
        from: &str,
        to: &str,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value, BusError> {
        let correlation_id = Uuid::new_v4();
        
        // Send request
        let request = PluginMessage {
            from: from.to_string(),
            to: to.to_string(),
            message_type: "request".to_string(),
            payload,
            correlation_id: Some(correlation_id),
            timestamp: SystemTime::now(),
        };
        
        self.send(request).await?;
        
        // Wait for response with timeout
        tokio::time::timeout(timeout, async {
            // Listen for response with matching correlation_id
            // Implementation details...
        }).await?
    }
}
```

---

## PLUGIN DISCOVERY & REGISTRY

### Automatic Plugin Discovery

```rust
//! Plugin discovery system for automatic loading from filesystem,
//! package managers, and remote registries.

use notify::{Watcher, RecursiveMode, watcher};
use std::path::PathBuf;

pub struct PluginDiscovery {
    /// Directories to scan for plugins
    plugin_dirs: Vec<PathBuf>,
    
    /// File system watcher for hot-reload
    watcher: Box<dyn Watcher>,
    
    /// Discovered plugins
    discovered: Arc<RwLock<HashMap<String, PluginInfo>>>,
    
    /// Remote registry client
    registry_client: RegistryClient,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Plugin identifier
    pub id: String,
    
    /// File path
    pub path: PathBuf,
    
    /// Metadata (if available without loading)
    pub metadata: Option<PluginMetadata>,
    
    /// Discovery source
    pub source: DiscoverySource,
    
    /// Signature verification status
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoverySource {
    /// Local filesystem
    Filesystem,
    
    /// Package manager (cargo, npm)
    PackageManager(String),
    
    /// Remote registry
    Registry(String),
    
    /// Manual registration
    Manual,
}

impl PluginDiscovery {
    /// Start discovery service
    pub async fn start(&mut self) -> Result<(), Error> {
        // 1. Scan local directories
        self.scan_local_plugins().await?;
        
        // 2. Check package managers
        self.scan_package_managers().await?;
        
        // 3. Query remote registry
        self.query_registry().await?;
        
        // 4. Start file system watcher
        self.start_watcher()?;
        
        Ok(())
    }
    
    /// Scan local plugin directories
    async fn scan_local_plugins(&mut self) -> Result<(), Error> {
        for dir in &self.plugin_dirs {
            let entries = tokio::fs::read_dir(dir).await?;
            let mut entries = tokio_stream::wrappers::ReadDirStream::new(entries);
            
            while let Some(entry) = entries.next().await {
                let entry = entry?;
                let path = entry.path();
                
                // Check if it's a plugin file
                if self.is_plugin_file(&path) {
                    let info = self.analyze_plugin(&path).await?;
                    self.discovered.write().await.insert(info.id.clone(), info);
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if file is a plugin
    fn is_plugin_file(&self, path: &Path) -> bool {
        match path.extension().and_then(|s| s.to_str()) {
            Some("so") | Some("dll") | Some("dylib") | Some("wasm") => true,
            _ => false,
        }
    }
    
    /// Extract metadata without loading
    async fn analyze_plugin(&self, path: &Path) -> Result<PluginInfo, Error> {
        // Try to read metadata from companion file
        let meta_path = path.with_extension("plugin.toml");
        
        let metadata = if meta_path.exists() {
            let contents = tokio::fs::read_to_string(&meta_path).await?;
            Some(toml::from_str(&contents)?)
        } else {
            // Try to extract from binary (if native)
            self.extract_embedded_metadata(path).await.ok()
        };
        
        // Verify signature if present
        let verified = self.verify_plugin_signature(path).await.unwrap_or(false);
        
        Ok(PluginInfo {
            id: metadata.as_ref()
                .map(|m| m.id.clone())
                .unwrap_or_else(|| path.file_stem().unwrap().to_string_lossy().to_string()),
            path: path.to_path_buf(),
            metadata,
            source: DiscoverySource::Filesystem,
            verified,
        })
    }
}
```

### Plugin Registry Protocol

```rust
/// Remote plugin registry client for discovering and downloading plugins
pub struct RegistryClient {
    /// Registry base URL
    base_url: Url,
    
    /// HTTP client
    client: reqwest::Client,
    
    /// Local cache directory
    cache_dir: PathBuf,
    
    /// Authentication token
    auth_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: Version,
    pub author: String,
    pub license: String,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub downloads: u64,
    pub rating: f32,
    pub tags: Vec<String>,
    pub platforms: Vec<Platform>,
    pub download_url: String,
    pub signature_url: String,
    pub checksum: String,
}

impl RegistryClient {
    /// Search for plugins in registry
    pub async fn search(&self, query: &str) -> Result<Vec<RegistryPlugin>, Error> {
        let response = self.client
            .get(format!("{}/api/v1/search", self.base_url))
            .query(&[("q", query)])
            .send()
            .await?;
        
        response.json().await.map_err(Into::into)
    }
    
    /// Download and verify a plugin
    pub async fn download(&self, plugin_id: &str) -> Result<PathBuf, Error> {
        // 1. Get plugin metadata
        let plugin = self.get_plugin_info(plugin_id).await?;
        
        // 2. Check cache
        let cache_path = self.cache_dir.join(&format!("{}-{}", plugin.id, plugin.version));
        if cache_path.exists() {
            return Ok(cache_path);
        }
        
        // 3. Download plugin
        let bytes = self.client
            .get(&plugin.download_url)
            .send()
            .await?
            .bytes()
            .await?;
        
        // 4. Verify checksum
        let checksum = self.calculate_checksum(&bytes);
        if checksum != plugin.checksum {
            return Err(Error::ChecksumMismatch);
        }
        
        // 5. Download and verify signature
        let signature = self.client
            .get(&plugin.signature_url)
            .send()
            .await?
            .bytes()
            .await?;
        
        self.verify_signature(&bytes, &signature)?;
        
        // 6. Save to cache
        tokio::fs::write(&cache_path, bytes).await?;
        
        Ok(cache_path)
    }
}
```

---

## SECURITY MODEL

### Capability-Based Security

```rust
//! Fine-grained capability system for plugin permissions.
//! Plugins must declare required capabilities, which are enforced at runtime.

use std::collections::HashSet;

pub struct SecurityManager {
    /// Capability grants per plugin
    grants: Arc<RwLock<HashMap<String, HashSet<Capability>>>>,
    
    /// Security policies
    policies: Vec<SecurityPolicy>,
    
    /// Audit log
    audit_log: Arc<AuditLog>,
}

#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    
    /// Applied to plugins matching pattern
    pub plugin_pattern: Regex,
    
    /// Allowed capabilities
    pub allow: HashSet<Capability>,
    
    /// Denied capabilities (override allow)
    pub deny: HashSet<Capability>,
    
    /// Additional restrictions
    pub restrictions: Vec<Restriction>,
}

#[derive(Debug, Clone)]
pub enum Restriction {
    /// Network access whitelist
    NetworkWhitelist(Vec<String>),
    
    /// Filesystem access patterns
    FilesystemPaths(Vec<PathBuf>),
    
    /// Time-based restrictions
    TimeWindow { start: NaiveTime, end: NaiveTime },
    
    /// Rate limiting
    RateLimit { max_calls: u32, per: Duration },
    
    /// Data size limits
    MaxDataSize(usize),
}

impl SecurityManager {
    /// Check if plugin has capability
    pub async fn check_capability(
        &self,
        plugin_id: &str,
        capability: &Capability,
    ) -> Result<(), SecurityError> {
        // 1. Check explicit grants
        let grants = self.grants.read().await;
        let plugin_grants = grants.get(plugin_id)
            .ok_or(SecurityError::PluginNotRegistered)?;
        
        if !plugin_grants.contains(capability) {
            // 2. Log denial
            self.audit_log.log_denial(plugin_id, capability).await;
            
            return Err(SecurityError::CapabilityDenied {
                plugin: plugin_id.to_string(),
                capability: capability.clone(),
            });
        }
        
        // 3. Check additional policies
        for policy in &self.policies {
            if policy.plugin_pattern.is_match(plugin_id) {
                if policy.deny.contains(capability) {
                    return Err(SecurityError::PolicyDenied {
                        policy: policy.name.clone(),
                    });
                }
            }
        }
        
        // 4. Log access
        self.audit_log.log_access(plugin_id, capability).await;
        
        Ok(())
    }
    
    /// Sandbox plugin execution
    pub async fn sandbox_plugin(&self, plugin_id: &str) -> Result<Sandbox, Error> {
        let grants = self.grants.read().await;
        let capabilities = grants.get(plugin_id)
            .ok_or(Error::PluginNotRegistered)?;
        
        // Create sandbox based on capabilities
        let mut sandbox = Sandbox::new(plugin_id);
        
        // Configure based on capabilities
        for capability in capabilities {
            match capability {
                Capability::Network(domains) => {
                    sandbox.allow_network(domains);
                }
                Capability::Filesystem(paths) => {
                    sandbox.allow_filesystem(paths);
                }
                Capability::MemoryLimit(mb) => {
                    sandbox.set_memory_limit(*mb);
                }
                Capability::CpuLimit(percent) => {
                    sandbox.set_cpu_limit(*percent);
                }
                _ => {}
            }
        }
        
        Ok(sandbox)
    }
}

/// Sandbox implementation for plugin isolation
pub struct Sandbox {
    plugin_id: String,
    network_filter: Option<NetworkFilter>,
    filesystem_filter: Option<FilesystemFilter>,
    resource_limits: ResourceLimits,
}

impl Sandbox {
    /// Apply sandbox to current thread/process
    pub fn apply(&self) -> Result<(), Error> {
        #[cfg(target_os = "linux")]
        {
            // Use seccomp-bpf for system call filtering
            self.apply_seccomp_filter()?;
            
            // Use namespaces for isolation
            self.enter_namespaces()?;
            
            // Apply cgroups for resource limits
            self.apply_cgroups()?;
        }
        
        #[cfg(target_os = "macos")]
        {
            // Use sandbox-exec
            self.apply_sandbox_exec()?;
        }
        
        #[cfg(target_os = "windows")]
        {
            // Use Windows integrity levels and job objects
            self.apply_windows_sandbox()?;
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    fn apply_seccomp_filter(&self) -> Result<(), Error> {
        use seccomp_sys::*;
        
        // Create filter
        let ctx = seccomp_init(SCMP_ACT_KILL);
        
        // Allow basic syscalls
        seccomp_rule_add(ctx, SCMP_ACT_ALLOW, SCMP_SYS(read), 0);
        seccomp_rule_add(ctx, SCMP_ACT_ALLOW, SCMP_SYS(write), 0);
        seccomp_rule_add(ctx, SCMP_ACT_ALLOW, SCMP_SYS(close), 0);
        
        // Network syscalls (if allowed)
        if self.network_filter.is_some() {
            seccomp_rule_add(ctx, SCMP_ACT_ALLOW, SCMP_SYS(socket), 0);
            seccomp_rule_add(ctx, SCMP_ACT_ALLOW, SCMP_SYS(connect), 0);
        }
        
        // Apply filter
        seccomp_load(ctx);
        
        Ok(())
    }
}
```

---

## PERFORMANCE CONSIDERATIONS

### Plugin Performance Monitoring

```rust
//! Performance tracking and optimization for plugins.
//! Monitors execution time, resource usage, and provides profiling data.

use std::time::{Duration, Instant};
use prometheus::{Histogram, Counter, Gauge};

pub struct PluginMetrics {
    /// Execution time histogram per plugin
    execution_time: HashMap<String, Histogram>,
    
    /// Call count per plugin
    call_count: HashMap<String, Counter>,
    
    /// Memory usage per plugin
    memory_usage: HashMap<String, Gauge>,
    
    /// CPU usage per plugin
    cpu_usage: HashMap<String, Gauge>,
    
    /// Error count per plugin
    error_count: HashMap<String, Counter>,
}

impl PluginMetrics {
    /// Record plugin execution
    pub fn record_execution<F, R>(
        &self,
        plugin_id: &str,
        operation: &str,
        f: F,
    ) -> Result<R, PluginError>
    where
        F: FnOnce() -> Result<R, PluginError>,
    {
        let start = Instant::now();
        
        // Increment call counter
        self.call_count.get(plugin_id)
            .map(|c| c.inc());
        
        // Execute operation
        let result = f();
        
        // Record execution time
        let duration = start.elapsed();
        self.execution_time.get(plugin_id)
            .map(|h| h.observe(duration.as_secs_f64()));
        
        // Record error if occurred
        if result.is_err() {
            self.error_count.get(plugin_id)
                .map(|c| c.inc());
        }
        
        result
    }
    
    /// Get plugin performance report
    pub fn get_report(&self, plugin_id: &str) -> PerformanceReport {
        PerformanceReport {
            plugin_id: plugin_id.to_string(),
            total_calls: self.get_call_count(plugin_id),
            error_rate: self.calculate_error_rate(plugin_id),
            avg_execution_time: self.get_avg_execution_time(plugin_id),
            p99_execution_time: self.get_p99_execution_time(plugin_id),
            memory_usage_mb: self.get_memory_usage(plugin_id),
            cpu_usage_percent: self.get_cpu_usage(plugin_id),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct PerformanceReport {
    pub plugin_id: String,
    pub total_calls: u64,
    pub error_rate: f64,
    pub avg_execution_time: Duration,
    pub p99_execution_time: Duration,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

/// Plugin performance optimizer
pub struct PluginOptimizer {
    metrics: Arc<PluginMetrics>,
    config: OptimizerConfig,
}

impl PluginOptimizer {
    /// Analyze and optimize plugin performance
    pub async fn optimize(&self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();
        
        // Check each plugin's metrics
        for plugin_id in self.metrics.list_plugins() {
            let report = self.metrics.get_report(&plugin_id);
            
            // High latency check
            if report.p99_execution_time > self.config.latency_threshold {
                recommendations.push(OptimizationRecommendation {
                    plugin_id: plugin_id.clone(),
                    issue: "High latency detected".to_string(),
                    recommendation: "Consider caching or async processing".to_string(),
                    severity: Severity::High,
                });
            }
            
            // Memory usage check
            if report.memory_usage_mb > self.config.memory_threshold_mb {
                recommendations.push(OptimizationRecommendation {
                    plugin_id: plugin_id.clone(),
                    issue: "High memory usage".to_string(),
                    recommendation: "Review memory allocations and leaks".to_string(),
                    severity: Severity::Medium,
                });
            }
            
            // Error rate check
            if report.error_rate > self.config.error_rate_threshold {
                recommendations.push(OptimizationRecommendation {
                    plugin_id: plugin_id.clone(),
                    issue: format!("High error rate: {:.2}%", report.error_rate * 100.0),
                    recommendation: "Review error handling and stability".to_string(),
                    severity: Severity::Critical,
                });
            }
        }
        
        recommendations
    }
}
```

### Zero-Copy Plugin Data Transfer

```rust
//! Zero-copy data transfer between host and plugins for optimal performance.
//! Uses shared memory and reference counting to avoid unnecessary copies.

use bytes::Bytes;
use shared_memory::{Shmem, ShmemConf};

pub struct ZeroCopyBridge {
    /// Shared memory segments
    segments: Arc<Mutex<HashMap<String, Shmem>>>,
    
    /// Reference counting for segments
    ref_counts: Arc<DashMap<String, usize>>,
}

impl ZeroCopyBridge {
    /// Create shared memory for data transfer
    pub fn create_shared_buffer(&self, size: usize) -> Result<SharedBuffer, Error> {
        // Create shared memory
        let shmem = ShmemConf::new()
            .size(size)
            .create()?;
        
        let id = Uuid::new_v4().to_string();
        
        // Store segment
        self.segments.lock().unwrap().insert(id.clone(), shmem);
        self.ref_counts.insert(id.clone(), 1);
        
        Ok(SharedBuffer {
            id,
            size,
            bridge: self.clone(),
        })
    }
    
    /// Transfer data to plugin without copying
    pub unsafe fn transfer_to_plugin(
        &self,
        data: &[u8],
        plugin_memory: &Memory,
        store: &mut Store<WasmState>,
    ) -> Result<(i32, i32), Error> {
        // Get plugin memory
        let mem_data = plugin_memory.data_mut(store);
        
        // Find free space in plugin memory
        let offset = self.find_free_space(mem_data, data.len())?;
        
        // Direct memory copy (zero-copy from host perspective)
        mem_data[offset..offset + data.len()].copy_from_slice(data);
        
        Ok((offset as i32, data.len() as i32))
    }
    
    /// Transfer data from plugin without copying
    pub unsafe fn transfer_from_plugin(
        &self,
        plugin_memory: &Memory,
        store: &mut Store<WasmState>,
        offset: i32,
        len: i32,
    ) -> Result<Bytes, Error> {
        let mem_data = plugin_memory.data(store);
        
        // Create Bytes without copying (shares memory)
        let bytes = Bytes::copy_from_slice(&mem_data[offset as usize..(offset + len) as usize]);
        
        Ok(bytes)
    }
}
```

---

## PLUGIN SDK DESIGN

### Plugin Development Kit

```toml
# Cargo.toml for plugin developers
[package]
name = "only1mcp-plugin-sdk"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core plugin traits and types
only1mcp-plugin-api = "0.1"

# Async runtime (re-exported)
tokio = { version = "1", features = ["rt", "macros"] }
async-trait = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Logging
tracing = "0.1"

# Optional: WASM support
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = "0.3"

[dev-dependencies]
# Testing utilities
only1mcp-plugin-test = "0.1"
```

### Plugin Template Generator

```rust
//! CLI tool to generate plugin boilerplate code
//! Usage: only1mcp plugin new --type request-transformer --name my-plugin

use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new plugin project
    New {
        /// Plugin type
        #[arg(long)]
        plugin_type: PluginType,
        
        /// Plugin name
        #[arg(long)]
        name: String,
        
        /// Target (native or wasm)
        #[arg(long, default_value = "native")]
        target: String,
    },
}

pub fn generate_plugin_project(name: &str, plugin_type: PluginType) -> Result<(), Error> {
    let project_dir = Path::new(name);
    fs::create_dir_all(project_dir)?;
    
    // Generate Cargo.toml
    let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
only1mcp-plugin-sdk = "0.1"
async-trait = "0.1"
serde = {{ version = "1", features = ["derive"] }}
tracing = "0.1"

[lib]
crate-type = ["cdylib"]

[[bin]]
name = "test-harness"
path = "src/test.rs"
"#, name);
    
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)?;
    
    // Generate main plugin code
    let plugin_code = match plugin_type {
        PluginType::RequestTransformer => generate_request_transformer(name),
        PluginType::ResponseTransformer => generate_response_transformer(name),
        PluginType::Authentication => generate_auth_provider(name),
        // ... other types
    };
    
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(project_dir.join("src/lib.rs"), plugin_code)?;
    
    // Generate test harness
    let test_code = generate_test_harness(name);
    fs::write(project_dir.join("src/test.rs"), test_code)?;
    
    // Generate plugin manifest
    let manifest = format!(r#"[plugin]
id = "com.example.{}"
name = "{}"
version = "0.1.0"
api_version = "0.1.0"
author = "Your Name"
license = "MIT"

[capabilities]
# Add required capabilities here
# network = ["api.example.com"]
# filesystem = []

[config]
# Plugin configuration schema
"#, name, name);
    
    fs::write(project_dir.join("plugin.toml"), manifest)?;
    
    println!("✨ Created plugin project: {}", name);
    println!("📁 Location: {}", project_dir.display());
    println!("\nNext steps:");
    println!("  cd {}", name);
    println!("  cargo build --release");
    println!("  cargo run --bin test-harness");
    
    Ok(())
}

fn generate_request_transformer(name: &str) -> String {
    format!(r#"use only1mcp_plugin_sdk::*;
use async_trait::async_trait;

pub struct {}Plugin {{
    config: PluginConfig,
}}

#[async_trait]
impl Plugin for {}Plugin {{
    fn metadata(&self) -> PluginMetadata {{
        PluginMetadata {{
            id: "com.example.{}".to_string(),
            name: "{}".to_string(),
            version: semver::Version::parse("0.1.0").unwrap(),
            api_version: semver::Version::parse("0.1.0").unwrap(),
            capabilities: vec![],
            plugin_type: PluginType::RequestTransformer,
            author: "Your Name".to_string(),
            license: "MIT".to_string(),
        }}
    }}
    
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), PluginError> {{
        self.config = config;
        tracing::info!("Plugin initialized");
        Ok(())
    }}
    
    async fn shutdown(&mut self) -> Result<(), PluginError> {{
        tracing::info!("Plugin shutting down");
        Ok(())
    }}
    
    async fn health_check(&self) -> HealthStatus {{
        HealthStatus::Healthy
    }}
}}

#[async_trait]
impl RequestTransformer for {}Plugin {{
    async fn transform_request(
        &self,
        request: &mut McpRequest,
        context: &RequestContext,
    ) -> Result<TransformResult, PluginError> {{
        // Your transformation logic here
        tracing::debug!("Processing request: {{:?}}", request.method);
        
        // Example: Add custom header
        if let Some(params) = &mut request.params {{
            params["custom_header"] = serde_json::json!("added_by_plugin");
        }}
        
        Ok(TransformResult::Modified)
    }}
}}

// Export plugin constructor
declare_plugin!({}Plugin, {}Plugin {{ config: Default::default() }});
"#, 
        name.to_case(Case::Pascal),
        name.to_case(Case::Pascal),
        name,
        name.to_case(Case::Title),
        name.to_case(Case::Pascal),
        name.to_case(Case::Pascal),
        name.to_case(Case::Pascal)
    )
}
```

---

## HOT-LOADING MECHANISM

### Safe Plugin Hot-Reload

```rust
//! Hot-reload system for updating plugins without proxy restart.
//! Ensures zero downtime and graceful request handling during updates.

use tokio::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct HotReloadManager {
    /// Current plugin versions
    versions: Arc<RwLock<HashMap<String, PluginVersion>>>,
    
    /// Plugin update queue
    update_queue: Arc<Mutex<VecDeque<UpdateRequest>>>,
    
    /// Active request counter per plugin
    active_requests: Arc<DashMap<String, AtomicU64>>,
    
    /// Grace period for draining requests
    grace_period: Duration,
}

#[derive(Debug, Clone)]
struct PluginVersion {
    /// Currently active plugin
    current: Arc<Box<dyn Plugin>>,
    
    /// New version being loaded
    pending: Option<Arc<Box<dyn Plugin>>>,
    
    /// Version number
    version: semver::Version,
    
    /// Load timestamp
    loaded_at: SystemTime,
}

impl HotReloadManager {
    /// Schedule a plugin update
    pub async fn schedule_update(
        &self,
        plugin_id: String,
        new_path: PathBuf,
    ) -> Result<UpdateHandle, Error> {
        let update = UpdateRequest {
            id: Uuid::new_v4(),
            plugin_id,
            new_path,
            scheduled_at: SystemTime::now(),
            status: UpdateStatus::Pending,
        };
        
        let handle = UpdateHandle {
            id: update.id,
            manager: self.clone(),
        };
        
        self.update_queue.lock().await.push_back(update);
        
        // Trigger update processor
        self.process_updates().await;
        
        Ok(handle)
    }
    
    /// Process pending updates
    async fn process_updates(&self) {
        while let Some(update) = self.update_queue.lock().await.pop_front() {
            if let Err(e) = self.perform_update(update).await {
                tracing::error!("Plugin update failed: {}", e);
            }
        }
    }
    
    /// Perform plugin hot-reload
    async fn perform_update(&self, update: UpdateRequest) -> Result<(), Error> {
        tracing::info!("Starting hot-reload for plugin: {}", update.plugin_id);
        
        // 1. Load new plugin version
        let new_plugin = self.load_plugin(&update.new_path).await?;
        
        // 2. Validate compatibility
        self.validate_update(&update.plugin_id, &new_plugin)?;
        
        // 3. Mark as pending
        {
            let mut versions = self.versions.write().await;
            if let Some(version) = versions.get_mut(&update.plugin_id) {
                version.pending = Some(Arc::new(new_plugin));
            }
        }
        
        // 4. Wait for active requests to drain
        self.drain_requests(&update.plugin_id).await?;
        
        // 5. Atomic swap
        {
            let mut versions = self.versions.write().await;
            if let Some(version) = versions.get_mut(&update.plugin_id) {
                if let Some(new) = version.pending.take() {
                    // Shutdown old version
                    if let Ok(old) = Arc::try_unwrap(version.current.clone()) {
                        old.shutdown().await?;
                    }
                    
                    // Activate new version
                    version.current = new;
                    version.version = new.metadata().version;
                    version.loaded_at = SystemTime::now();
                }
            }
        }
        
        tracing::info!("Hot-reload completed for plugin: {}", update.plugin_id);
        
        Ok(())
    }
    
    /// Wait for active requests to complete
    async fn drain_requests(&self, plugin_id: &str) -> Result<(), Error> {
        let start = Instant::now();
        
        loop {
            // Check active request count
            let count = self.active_requests
                .get(plugin_id)
                .map(|c| c.value().load(Ordering::SeqCst))
                .unwrap_or(0);
            
            if count == 0 {
                break;
            }
            
            // Check timeout
            if start.elapsed() > self.grace_period {
                return Err(Error::DrainTimeout {
                    plugin: plugin_id.to_string(),
                    remaining: count,
                });
            }
            
            // Wait briefly before rechecking
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(())
    }
    
    /// Track request for plugin
    pub fn track_request(&self, plugin_id: &str) -> RequestGuard {
        self.active_requests
            .entry(plugin_id.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::SeqCst);
        
        RequestGuard {
            plugin_id: plugin_id.to_string(),
            manager: self.clone(),
        }
    }
}

/// RAII guard for tracking active requests
pub struct RequestGuard {
    plugin_id: String,
    manager: HotReloadManager,
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        if let Some(counter) = self.manager.active_requests.get(&self.plugin_id) {
            counter.value().fetch_sub(1, Ordering::SeqCst);
        }
    }
}
```

---

## TESTING & DEBUGGING

### Plugin Testing Framework

```rust
//! Testing utilities for plugin developers.
//! Provides mocks, stubs, and integration test helpers.

use mockall::automock;

/// Test harness for plugins
pub struct PluginTestHarness {
    /// Plugin under test
    plugin: Box<dyn Plugin>,
    
    /// Mock MCP server
    mock_server: MockMcpServer,
    
    /// Test configuration
    config: TestConfig,
}

impl PluginTestHarness {
    /// Create test harness for a plugin
    pub fn new(plugin: Box<dyn Plugin>) -> Self {
        Self {
            plugin,
            mock_server: MockMcpServer::new(),
            config: TestConfig::default(),
        }
    }
    
    /// Run plugin through standard test scenarios
    pub async fn run_standard_tests(&mut self) -> TestResults {
        let mut results = TestResults::new();
        
        // Test initialization
        results.add(self.test_initialization().await);
        
        // Test basic transformation
        results.add(self.test_basic_transformation().await);
        
        // Test error handling
        results.add(self.test_error_handling().await);
        
        // Test performance
        results.add(self.test_performance().await);
        
        // Test resource limits
        results.add(self.test_resource_limits().await);
        
        results
    }
    
    /// Test plugin initialization
    async fn test_initialization(&mut self) -> TestResult {
        let config = PluginConfig::default();
        
        match self.plugin.initialize(config).await {
            Ok(()) => TestResult::pass("Initialization"),
            Err(e) => TestResult::fail("Initialization", format!("{}", e)),
        }
    }
    
    /// Test request transformation
    async fn test_basic_transformation(&mut self) -> TestResult {
        // Create test request
        let mut request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1),
            method: "tools/list".to_string(),
            params: None,
        };
        
        let context = RequestContext::default();
        
        // Test transformation
        if let Some(transformer) = self.plugin.as_request_transformer() {
            match transformer.transform_request(&mut request, &context).await {
                Ok(result) => {
                    // Verify transformation
                    if request != original {
                        TestResult::pass("Basic transformation")
                    } else {
                        TestResult::fail("Basic transformation", "No changes made")
                    }
                }
                Err(e) => TestResult::fail("Basic transformation", format!("{}", e)),
            }
        } else {
            TestResult::skip("Basic transformation", "Not a request transformer")
        }
    }
    
    /// Performance benchmarking
    async fn test_performance(&mut self) -> TestResult {
        use criterion::{black_box, Criterion};
        
        let mut criterion = Criterion::default();
        
        criterion.bench_function("plugin_transform", |b| {
            b.iter(|| {
                let mut request = create_test_request();
                let context = RequestContext::default();
                
                black_box(
                    self.plugin.transform_request(&mut request, &context)
                );
            });
        });
        
        // Check if performance meets requirements
        let stats = criterion.get_statistics("plugin_transform");
        if stats.mean < Duration::from_millis(5) {
            TestResult::pass("Performance")
        } else {
            TestResult::fail("Performance", format!("Mean time: {:?}", stats.mean))
        }
    }
}

/// Mock MCP server for testing
#[automock]
pub trait McpServer {
    async fn call_tool(&self, tool: &str, params: Value) -> Result<Value, Error>;
    async fn list_tools(&self) -> Result<Vec<ToolInfo>, Error>;
}

/// Plugin debugging support
pub struct PluginDebugger {
    /// Trace all plugin calls
    trace_enabled: bool,
    
    /// Breakpoints
    breakpoints: HashSet<String>,
    
    /// Call history
    history: VecDeque<DebugEvent>,
}

impl PluginDebugger {
    /// Trace plugin execution
    pub async fn trace_call<F, R>(
        &mut self,
        plugin_id: &str,
        method: &str,
        f: F,
    ) -> R
    where
        F: Future<Output = R>,
    {
        if self.trace_enabled {
            let event = DebugEvent {
                timestamp: SystemTime::now(),
                plugin_id: plugin_id.to_string(),
                method: method.to_string(),
                state: DebugState::Enter,
            };
            
            self.history.push_back(event);
            
            // Check breakpoint
            if self.breakpoints.contains(method) {
                self.handle_breakpoint(plugin_id, method).await;
            }
        }
        
        let result = f.await;
        
        if self.trace_enabled {
            let event = DebugEvent {
                timestamp: SystemTime::now(),
                plugin_id: plugin_id.to_string(),
                method: method.to_string(),
                state: DebugState::Exit,
            };
            
            self.history.push_back(event);
        }
        
        result
    }
}
```

---

## PLUGIN MARKETPLACE INFRASTRUCTURE

### Marketplace Architecture

```rust
//! Plugin marketplace for discovering, sharing, and managing community plugins.
//! Includes rating system, security scanning, and automated compatibility testing.

pub struct MarketplaceService {
    /// Database connection
    db: DatabasePool,
    
    /// Storage backend for plugin files
    storage: Box<dyn StorageBackend>,
    
    /// Security scanner
    scanner: SecurityScanner,
    
    /// Compatibility tester
    tester: CompatibilityTester,
    
    /// Search index
    search: SearchIndex,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketplaceListing {
    pub id: Uuid,
    pub plugin_id: String,
    pub name: String,
    pub description: String,
    pub author: Author,
    pub version: Version,
    pub license: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub homepage: Option<Url>,
    pub repository: Option<Url>,
    pub documentation: Option<Url>,
    
    // Metrics
    pub downloads: u64,
    pub stars: u32,
    pub rating: f32,
    pub reviews: u32,
    
    // Technical details
    pub platforms: Vec<Platform>,
    pub api_version: Version,
    pub file_size: u64,
    pub checksum: String,
    
    // Security
    pub verified: bool,
    pub security_score: u8,  // 0-100
    pub last_scanned: SystemTime,
    
    // Compatibility
    pub tested_versions: Vec<Version>,
    pub compatible: bool,
}

impl MarketplaceService {
    /// Submit a new plugin to marketplace
    pub async fn submit_plugin(
        &self,
        submission: PluginSubmission,
    ) -> Result<MarketplaceListing, Error> {
        // 1. Validate submission
        submission.validate()?;
        
        // 2. Security scan
        let security_report = self.scanner.scan(&submission.file_path).await?;
        if security_report.critical_issues > 0 {
            return Err(Error::SecurityCheckFailed(security_report));
        }
        
        // 3. Compatibility testing
        let compat_report = self.tester.test(&submission.file_path).await?;
        if !compat_report.compatible {
            return Err(Error::CompatibilityCheckFailed(compat_report));
        }
        
        // 4. Store plugin file
        let storage_path = self.storage.store(
            &submission.file_path,
            &submission.plugin_id,
            &submission.version,
        ).await?;
        
        // 5. Create listing
        let listing = MarketplaceListing {
            id: Uuid::new_v4(),
            plugin_id: submission.plugin_id,
            name: submission.name,
            description: submission.description,
            author: submission.author,
            version: submission.version,
            license: submission.license,
            categories: submission.categories,
            tags: submission.tags,
            homepage: submission.homepage,
            repository: submission.repository,
            documentation: submission.documentation,
            
            downloads: 0,
            stars: 0,
            rating: 0.0,
            reviews: 0,
            
            platforms: submission.platforms,
            api_version: submission.api_version,
            file_size: submission.file_size,
            checksum: submission.checksum,
            
            verified: security_report.verified,
            security_score: security_report.score,
            last_scanned: SystemTime::now(),
            
            tested_versions: compat_report.tested_versions,
            compatible: compat_report.compatible,
        };
        
        // 6. Save to database
        self.db.insert_listing(&listing).await?;
        
        // 7. Update search index
        self.search.index_plugin(&listing).await?;
        
        // 8. Notify subscribers
        self.notify_new_plugin(&listing).await;
        
        Ok(listing)
    }
}
```

---

## MIGRATION STRATEGY

### Migrating from Static Configuration

```rust
//! Migration tools for converting static configurations to plugin-based system.

pub struct MigrationAssistant {
    /// Current configuration
    current_config: Config,
    
    /// Plugin manager
    plugin_manager: PluginManager,
    
    /// Migration rules
    rules: Vec<MigrationRule>,
}

impl MigrationAssistant {
    /// Analyze configuration and suggest plugins
    pub async fn analyze(&self) -> MigrationPlan {
        let mut plan = MigrationPlan::new();
        
        // Check for custom transforms
        if self.current_config.has_custom_transforms() {
            plan.add_recommendation(
                "Custom transforms detected",
                "Create RequestTransformer plugin",
                PluginType::RequestTransformer,
            );
        }
        
        // Check for authentication
        if self.current_config.has_custom_auth() {
            plan.add_recommendation(
                "Custom authentication detected",
                "Create AuthenticationProvider plugin",
                PluginType::Authentication,
            );
        }
        
        // Check for load balancing
        if self.current_config.load_balancer.algorithm == "custom" {
            plan.add_recommendation(
                "Custom load balancer detected",
                "Create LoadBalancer plugin",
                PluginType::LoadBalancer,
            );
        }
        
        plan
    }
    
    /// Generate plugin code from configuration
    pub async fn generate_plugin(
        &self,
        config_section: &str,
    ) -> Result<String, Error> {
        // Extract relevant configuration
        let config = self.extract_config(config_section)?;
        
        // Generate plugin code
        let code = match config_section {
            "transforms" => self.generate_transform_plugin(config),
            "auth" => self.generate_auth_plugin(config),
            "cache" => self.generate_cache_plugin(config),
            _ => return Err(Error::UnsupportedSection),
        };
        
        Ok(code)
    }
}
```

---

## APPENDIX A: Complete Plugin Example

```rust
//! Complete example of a production-ready request transformer plugin
//! that adds authentication headers and logs requests.

use only1mcp_plugin_sdk::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, debug, error};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct AuthConfig {
    api_key: String,
    header_name: String,
    log_requests: bool,
}

pub struct AuthenticationPlugin {
    config: AuthConfig,
    stats: RequestStats,
}

#[derive(Default)]
struct RequestStats {
    total_requests: u64,
    methods: HashMap<String, u64>,
}

#[async_trait]
impl Plugin for AuthenticationPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "com.example.auth-plugin".to_string(),
            name: "Authentication Header Plugin".to_string(),
            version: semver::Version::parse("1.0.0").unwrap(),
            api_version: semver::Version::parse(API_VERSION).unwrap(),
            capabilities: vec![
                Capability::StateRead,
                Capability::StateWrite,
            ],
            plugin_type: PluginType::RequestTransform,
            author: "Example Corp".to_string(),
            license: "MIT".to_string(),
        }
    }
    
    async fn initialize(&mut self, config: PluginConfig) -> Result<(), PluginError> {
        // Parse configuration
        self.config = config.parse::<AuthConfig>()
            .map_err(|e| PluginError::Configuration(e.to_string()))?;
        
        info!("Authentication plugin initialized");
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), PluginError> {
        // Log final statistics
        info!("Total requests processed: {}", self.stats.total_requests);
        for (method, count) in &self.stats.methods {
            info!("  {}: {}", method, count);
        }
        
        Ok(())
    }
    
    async fn health_check(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}

#[async_trait]
impl RequestTransformer for AuthenticationPlugin {
    async fn transform_request(
        &mut self,
        request: &mut McpRequest,
        context: &RequestContext,
    ) -> Result<TransformResult, PluginError> {
        // Update statistics
        self.stats.total_requests += 1;
        *self.stats.methods.entry(request.method.clone()).or_insert(0) += 1;
        
        // Log request if enabled
        if self.config.log_requests {
            debug!("Processing request: {} from {}", 
                request.method, 
                context.client_id.as_deref().unwrap_or("unknown")
            );
        }
        
        // Add authentication header
        let params = request.params.get_or_insert_with(|| serde_json::json!({}));
        
        if let Some(headers) = params.get_mut("headers") {
            headers[&self.config.header_name] = serde_json::json!(self.config.api_key);
        } else {
            params["headers"] = serde_json::json!({
                &self.config.header_name: &self.config.api_key
            });
        }
        
        Ok(TransformResult::Modified)
    }
}

// Export plugin
declare_plugin!(
    AuthenticationPlugin,
    AuthenticationPlugin {
        config: AuthConfig {
            api_key: String::new(),
            header_name: "X-API-Key".to_string(),
            log_requests: false,
        },
        stats: RequestStats::default(),
    }
);

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_authentication_header_added() {
        let mut plugin = AuthenticationPlugin {
            config: AuthConfig {
                api_key: "test-key".to_string(),
                header_name: "X-API-Key".to_string(),
                log_requests: false,
            },
            stats: RequestStats::default(),
        };
        
        let mut request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1),
            method: "test".to_string(),
            params: None,
        };
        
        let context = RequestContext::default();
        
        let result = plugin.transform_request(&mut request, &context).await.unwrap();
        
        assert_eq!(result, TransformResult::Modified);
        assert_eq!(
            request.params.unwrap()["headers"]["X-API-Key"],
            "test-key"
        );
    }
}
```

---

## APPENDIX B: Security Audit Checklist

```markdown
# Plugin Security Audit Checklist

## Pre-Installation Checks
- [ ] Plugin signature verified
- [ ] Checksum validated
- [ ] Source repository verified
- [ ] Author identity confirmed
- [ ] License compatibility checked

## Static Analysis
- [ ] No hardcoded secrets
- [ ] No suspicious system calls
- [ ] No network connections to unknown hosts
- [ ] No filesystem access outside allowed paths
- [ ] No process spawning
- [ ] No dynamic code execution

## Runtime Monitoring
- [ ] Memory usage within limits
- [ ] CPU usage within limits
- [ ] Network traffic monitored
- [ ] File access logged
- [ ] No privilege escalation attempts
- [ ] No resource exhaustion

## Data Handling
- [ ] Input validation present
- [ ] Output sanitization implemented
- [ ] No data exfiltration
- [ ] Sensitive data encrypted
- [ ] PII handling compliant

## Compatibility Testing
- [ ] API version compatible
- [ ] No conflicts with other plugins
- [ ] Graceful error handling
- [ ] Proper cleanup on shutdown
- [ ] No memory leaks detected
```

---

**End of Document**

*This comprehensive plugin system architecture provides Only1MCP with unprecedented extensibility while maintaining the performance and security standards required for production deployment. The dual approach of native Rust plugins for trusted extensions and WASM modules for sandboxed execution ensures maximum flexibility without compromising the core aggregator's integrity.*
