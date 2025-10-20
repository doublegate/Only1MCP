//! STDIO transport implementation for local MCP servers.
//!
//! Manages process spawning, bidirectional communication through pipes,
//! MCP protocol initialization handshake, and security sandboxing.

use crate::error::Result;
use crate::types::{McpRequest, McpResponse, ServerId};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Failed to spawn process: {0}")]
    ProcessSpawnFailed(std::io::Error),

    #[error("No stdin available")]
    NoStdin,

    #[error("No stdout available")]
    NoStdout,

    #[error("No stderr available")]
    NoStderr,

    #[error("Request timeout")]
    Timeout,

    #[error("Process unhealthy")]
    ProcessUnhealthy,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("MCP protocol error: {0}")]
    ProtocolError(String),

    #[error("Initialization failed: {0}")]
    InitializationFailed(String),

    #[error("Invalid server response: {0}")]
    InvalidResponse(String),

    #[error("Connection not initialized")]
    NotInitialized,

    #[error("Connection in invalid state: {0:?}")]
    InvalidState(StdioConnectionState),
}

/// Connection lifecycle states for MCP STDIO servers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioConnectionState {
    /// Process spawned, not yet initialized
    Spawned,
    /// Initialize request sent, waiting for response
    Initializing,
    /// Initialized and ready for requests
    Ready,
    /// Connection closed or failed
    Closed,
}

/// Server capabilities returned during MCP initialization.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
}

impl ServerCapabilities {
    pub fn supports_tools(&self) -> bool {
        self.tools.is_some()
    }

    pub fn supports_resources(&self) -> bool {
        self.resources.is_some()
    }

    pub fn supports_prompts(&self) -> bool {
        self.prompts.is_some()
    }
}

/// Configuration for STDIO transport.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StdioConfig {
    /// Command to execute
    pub command: String,
    /// Command arguments
    pub args: Vec<String>,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Working directory
    pub cwd: Option<String>,
    /// Request timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum memory in MB
    pub max_memory_mb: Option<u64>,
    /// Maximum CPU percentage
    pub max_cpu_percent: Option<u32>,
    /// Enable security sandbox
    pub sandbox: bool,
}

impl Default for StdioConfig {
    fn default() -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            env: HashMap::new(),
            cwd: None,
            timeout_ms: 30000,
            max_memory_mb: Some(512),
            max_cpu_percent: Some(50),
            sandbox: true,
        }
    }
}

/// STDIO transport handler managing process lifecycle and MCP protocol.
pub struct StdioTransport {
    /// Active STDIO processes
    processes: Arc<DashMap<ServerId, Arc<StdioProcess>>>,
    /// Connection state per server
    connection_states: Arc<DashMap<ServerId, StdioConnectionState>>,
    /// Server capabilities per server (from initialize response)
    server_capabilities: Arc<DashMap<ServerId, ServerCapabilities>>,
    /// Initialization locks per server (prevent concurrent init)
    init_locks: Arc<DashMap<ServerId, Arc<Mutex<()>>>>,
    /// Process metrics
    metrics: Arc<ProcessMetrics>,
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioTransport {
    /// Create a new STDIO transport handler.
    pub fn new() -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            connection_states: Arc::new(DashMap::new()),
            server_capabilities: Arc::new(DashMap::new()),
            init_locks: Arc::new(DashMap::new()),
            metrics: Arc::new(ProcessMetrics::default()),
        }
    }

    /// Perform MCP protocol initialization handshake with a STDIO server.
    async fn initialize_connection(
        &self,
        server_id: &str,
        process: &StdioProcess,
    ) -> std::result::Result<ServerCapabilities, TransportError> {
        info!("Initializing MCP connection for server: {}", server_id);

        // Step 1: Send initialize request
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "roots": {
                        "listChanged": true
                    },
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "Only1MCP",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }
        });

        process.send_json(&init_request).await?;
        debug!("Sent initialize request to {}", server_id);

        // WORKAROUND: NPX-based processes need time to fully initialize their I/O streams
        // before they're ready to receive requests. Without this delay, we may try to read
        // from stdout before the final process in the chain (npx → sh → node → server) has
        // set up its stdio properly, resulting in broken pipe or connection closed errors.
        tokio::time::sleep(Duration::from_millis(300)).await;

        // Step 2: Read initialize response with timeout
        let init_response = tokio::time::timeout(Duration::from_secs(30), process.receive_json())
            .await
            .map_err(|_| {
                TransportError::InitializationFailed(
                    "Timeout waiting for initialize response".into(),
                )
            })??;

        // Step 3: Validate response
        if init_response.get("jsonrpc") != Some(&json!("2.0")) {
            return Err(TransportError::ProtocolError(
                "Invalid JSON-RPC version".into(),
            ));
        }

        let result = init_response
            .get("result")
            .ok_or_else(|| TransportError::InvalidResponse("Missing result field".into()))?;

        let protocol_version = result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| TransportError::ProtocolError("Missing protocolVersion".into()))?;

        if protocol_version != "2024-11-05" {
            warn!(
                "Server {} using different protocol version: {}",
                server_id, protocol_version
            );
        }

        // Step 4: Extract server capabilities
        let capabilities = result
            .get("capabilities")
            .ok_or_else(|| TransportError::InvalidResponse("Missing capabilities".into()))?;
        let server_capabilities: ServerCapabilities = serde_json::from_value(capabilities.clone())?;

        // Step 5: Log server info
        if let Some(server_info) = result.get("serverInfo") {
            info!(
                "Server {} initialized: {} v{}",
                server_id,
                server_info.get("name").and_then(|n| n.as_str()).unwrap_or("unknown"),
                server_info.get("version").and_then(|v| v.as_str()).unwrap_or("unknown")
            );
        }

        // Step 6: Send initialized notification
        let initialized_notification = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });

        process.send_json(&initialized_notification).await?;
        debug!("Sent initialized notification to {}", server_id);

        Ok(server_capabilities)
    }

    /// Initialize a STDIO connection with full MCP handshake (with retries).
    async fn initialize_stdio_connection(
        &self,
        server_id: &str,
        config: &StdioConfig,
    ) -> std::result::Result<(), TransportError> {
        // Set state to Initializing
        self.connection_states
            .insert(server_id.to_string(), StdioConnectionState::Initializing);

        // Perform handshake with retry logic
        let mut attempts = 0;
        const MAX_RETRIES: u32 = 3;

        let capabilities = loop {
            // Spawn new process for this attempt (or get existing healthy one)
            let process = self.get_or_create_process(server_id.to_string(), config).await?;

            match self.initialize_connection(server_id, &process).await {
                Ok(caps) => break caps,
                Err(e) if attempts < MAX_RETRIES => {
                    attempts += 1;
                    warn!(
                        "Initialization attempt {} failed for {}: {}. Retrying...",
                        attempts, server_id, e
                    );

                    // Remove failed process so next attempt spawns a new one
                    self.processes.remove(&server_id.to_string());

                    tokio::time::sleep(Duration::from_millis(500 * attempts as u64)).await;
                },
                Err(e) => {
                    self.connection_states
                        .insert(server_id.to_string(), StdioConnectionState::Closed);
                    self.processes.remove(&server_id.to_string());
                    self.metrics.init_failures.fetch_add(1, Ordering::Relaxed);
                    return Err(e);
                },
            }
        };

        // Store capabilities
        self.server_capabilities.insert(server_id.to_string(), capabilities);

        // Update state to Ready
        self.connection_states
            .insert(server_id.to_string(), StdioConnectionState::Ready);

        info!("STDIO server {} initialized successfully", server_id);
        Ok(())
    }

    /// Send a request to a STDIO MCP server with explicit config.
    pub async fn send_request_with_config(
        &self,
        server_id: ServerId,
        config: &StdioConfig,
        request: McpRequest,
    ) -> std::result::Result<McpResponse, TransportError> {
        // Check if connection needs initialization
        let needs_init = {
            let state = self.connection_states.get(&server_id);
            state.map(|s| *s.value() != StdioConnectionState::Ready).unwrap_or(true)
        };

        if needs_init {
            // Acquire initialization lock to prevent concurrent initialization
            let init_lock = self
                .init_locks
                .entry(server_id.clone())
                .or_insert_with(|| Arc::new(Mutex::new(())));
            let _guard = init_lock.lock().await;

            // Double-check state after acquiring lock (another task may have initialized)
            let state = self.connection_states.get(&server_id);
            if state.map(|s| *s.value() != StdioConnectionState::Ready).unwrap_or(true) {
                // Perform initialization
                let start = std::time::Instant::now();
                self.initialize_stdio_connection(&server_id, config).await?;
                let duration = start.elapsed();
                debug!("Initialization took {:?} for {}", duration, server_id);
                self.metrics
                    .init_duration_sum
                    .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
            }
        }

        // Get process
        let process = self.processes.get(&server_id).ok_or(TransportError::ProcessUnhealthy)?;

        // Send request as JSON-RPC
        let request_json = serde_json::to_value(&request)?;
        process.send_json(&request_json).await?;

        // Read response with timeout
        let response_json = tokio::time::timeout(
            Duration::from_millis(config.timeout_ms),
            process.receive_json(),
        )
        .await
        .map_err(|_| TransportError::Timeout)??;

        // Parse response
        let response: McpResponse = serde_json::from_value(response_json)?;

        self.metrics.requests_sent.fetch_add(1, Ordering::Relaxed);
        Ok(response)
    }

    /// Send a request to a STDIO MCP server (convenience method using default config).
    pub async fn send_request(
        &self,
        server_id: &str,
        request: McpRequest,
    ) -> std::result::Result<McpResponse, TransportError> {
        // Use default config for simplicity
        let config = StdioConfig {
            command: "mcp-server".to_string(),
            args: vec![],
            cwd: None,
            env: HashMap::new(),
            timeout_ms: 30000,
            max_memory_mb: None,
            max_cpu_percent: None,
            sandbox: true,
        };

        self.send_request_with_config(server_id.to_string(), &config, request).await
    }

    /// Resolve NPX package to direct node command.
    ///
    /// Converts: `npx -y @modelcontextprotocol/server-NAME`
    /// To: `node /path/to/cache/server-NAME/index.js`
    ///
    /// This eliminates the multi-process chain complexity (npx → node → sh → server)
    /// and provides more reliable pipe management in sandboxed environments.
    fn resolve_npx_to_node(config: &StdioConfig) -> Option<StdioConfig> {
        // Check if this is an NPX command
        if config.command != "npx" {
            return None;
        }

        // Extract package name from args
        // Expected: ["-y", "@modelcontextprotocol/server-NAME"] or similar
        let package_name = config.args.iter().find(|arg| !arg.starts_with('-'))?;

        debug!("Attempting to resolve NPX package: {}", package_name);

        // Try to find the package in NPX cache
        let cache_path = Self::find_npx_package(package_name)?;

        info!(
            "Resolved NPX package {} to: {}",
            package_name,
            cache_path.display()
        );

        // Create new config with node command
        // IMPORTANT: Disable sandbox for NPX-resolved packages because:
        // 1. They're from npm registry (trusted source)
        // 2. Node.js worker threads need unrestricted process limits
        // 3. RLIMIT_NPROC even at 50 can cause uv_thread_create failures
        Some(StdioConfig {
            command: "node".to_string(),
            args: vec![cache_path.to_string_lossy().to_string()],
            env: config.env.clone(),
            timeout_ms: config.timeout_ms,
            max_memory_mb: config.max_memory_mb,
            max_cpu_percent: config.max_cpu_percent,
            sandbox: false, // Disable sandbox for NPX packages
            cwd: config.cwd.clone(),
        })
    }

    /// Find NPX package in cache.
    fn find_npx_package(package_name: &str) -> Option<std::path::PathBuf> {
        // Get npm cache directory
        let cache_dir = Self::get_npm_cache_dir()?;
        let npx_cache = cache_dir.join("_npx");

        if !npx_cache.exists() {
            warn!("NPX cache directory not found: {}", npx_cache.display());
            return None;
        }

        // Extract package name without scope
        // @modelcontextprotocol/server-NAME -> server-NAME
        let package_name_only = package_name.split('/').next_back().unwrap_or(package_name);

        debug!(
            "Searching for package: {} in {}",
            package_name_only,
            npx_cache.display()
        );

        // Search patterns for the entry point
        let search_patterns = vec![
            format!("{}*/**/index.js", package_name_only),
            format!("{}*/**/dist/index.js", package_name_only),
            format!("{}/index.js", package_name_only),
            format!("{}/dist/index.js", package_name_only),
        ];

        // Try each pattern
        for pattern in search_patterns {
            if let Some(path) = Self::glob_find(&npx_cache, &pattern) {
                debug!("Found via pattern '{}': {}", pattern, path.display());
                return Some(path);
            }
        }

        warn!("Could not find entry point for package: {}", package_name);
        None
    }

    /// Get npm cache directory.
    fn get_npm_cache_dir() -> Option<std::path::PathBuf> {
        use std::path::PathBuf;
        use std::process::Command;

        // Try to get from npm config
        let output = Command::new("npm").args(["config", "get", "cache"]).output().ok()?;

        if output.status.success() {
            let cache_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let cache_path = PathBuf::from(cache_str);

            if cache_path.exists() {
                debug!("NPM cache directory: {}", cache_path.display());
                return Some(cache_path);
            }
        }

        // Fallback to default location
        let home = std::env::var("HOME").ok()?;
        let default_cache = PathBuf::from(home).join(".npm");

        if default_cache.exists() {
            debug!("Using default NPM cache: {}", default_cache.display());
            return Some(default_cache);
        }

        None
    }

    /// Helper to check if a path matches a glob pattern (reduces nesting).
    fn matches_glob_pattern(path_str: &str, pattern_parts: &[&str]) -> bool {
        let mut pos = 0;
        for part in pattern_parts {
            if let Some(idx) = path_str[pos..].find(part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }
        true
    }

    /// Simple glob-like search for files matching a pattern.
    fn glob_find(base: &std::path::Path, pattern: &str) -> Option<std::path::PathBuf> {
        use walkdir::WalkDir;

        // Convert glob pattern to matching logic
        let pattern_parts: Vec<&str> = pattern.split("**").collect();

        for entry in WalkDir::new(base).max_depth(10).follow_links(false).into_iter().flatten() {
            let path = entry.path();
            let path_str = path.to_string_lossy();

            // Simple pattern matching
            let matches = if pattern.contains("**") {
                // Check if path contains all pattern parts in order
                Self::matches_glob_pattern(&path_str, &pattern_parts)
            } else {
                path_str.ends_with(pattern)
            };

            if matches && path.is_file() {
                return Some(path.to_path_buf());
            }
        }

        None
    }

    /// Get existing or spawn new STDIO process.
    async fn get_or_create_process(
        &self,
        server_id: ServerId,
        config: &StdioConfig,
    ) -> std::result::Result<Arc<StdioProcess>, TransportError> {
        // Check if process exists and is healthy
        if let Some(process) = self.processes.get(&server_id) {
            if process.is_healthy().await {
                return Ok(process.clone());
            }
            // Process unhealthy, remove it
            self.processes.remove(&server_id);
            warn!("Removed unhealthy process for server {}", server_id);
        }

        // Try to resolve NPX to node if applicable
        let resolved_config = Self::resolve_npx_to_node(config).unwrap_or_else(|| config.clone());

        if resolved_config.command != config.command {
            info!(
                "Resolved NPX package to direct node execution: {} → {}",
                config.command,
                resolved_config.args.first().unwrap_or(&"?".to_string())
            );
        }

        // Spawn new process with security restrictions (using resolved config)
        let mut command = Command::new(&resolved_config.command);
        command
            .args(&resolved_config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set working directory
        if let Some(cwd) = &resolved_config.cwd {
            command.current_dir(cwd);
        }

        // Apply environment variables
        for (key, value) in &resolved_config.env {
            command.env(key, value);
        }

        // Security: Restrict process capabilities (Linux)
        #[cfg(target_os = "linux")]
        if resolved_config.sandbox {
            #[allow(unused_imports)]
            use std::os::unix::process::CommandExt;

            // Run as non-root user if we're root
            unsafe {
                if libc::getuid() == 0 {
                    command.uid(1000); // Run as user 1000
                    command.gid(1000);
                }
            }

            // Set resource limits
            // Clone values needed in closure
            let max_cpu = resolved_config.max_cpu_percent;
            let max_memory_mb = resolved_config.max_memory_mb;

            // SAFETY: pre_exec is called before fork, in a single-threaded context
            unsafe {
                command.pre_exec(move || {
                    // Limit CPU time
                    if let Some(max_cpu) = max_cpu {
                        let cpu_limit = (max_cpu as u64) * 10; // Convert percentage to deciseconds
                        let rlimit = libc::rlimit {
                            rlim_cur: cpu_limit,
                            rlim_max: cpu_limit,
                        };
                        libc::setrlimit(libc::RLIMIT_CPU, &rlimit);
                    }

                    // Limit memory
                    if let Some(max_memory_mb) = max_memory_mb {
                        let memory_bytes = max_memory_mb * 1024 * 1024;
                        let rlimit = libc::rlimit {
                            rlim_cur: memory_bytes,
                            rlim_max: memory_bytes,
                        };
                        libc::setrlimit(libc::RLIMIT_AS, &rlimit);
                    }

                    // Limit number of processes
                    // IMPORTANT: Node.js MCP servers need ~20-30 threads for worker pools
                    // Setting this too low (e.g., 10) causes uv_thread_create failures
                    let rlimit = libc::rlimit {
                        rlim_cur: 50, // Increased from 10 to 50 for Node.js worker threads
                        rlim_max: 50,
                    };
                    libc::setrlimit(libc::RLIMIT_NPROC, &rlimit);

                    Ok(())
                });
            }
        }

        let mut child = command.spawn().map_err(TransportError::ProcessSpawnFailed)?;

        let stdin = child.stdin.take().ok_or(TransportError::NoStdin)?;
        let stdout = child.stdout.take().ok_or(TransportError::NoStdout)?;
        let stderr = child.stderr.take().ok_or(TransportError::NoStderr)?;

        let process = Arc::new(StdioProcess::new(
            server_id.clone(),
            child,
            stdin,
            stdout,
            stderr,
        ));

        // Store in process map
        self.processes.insert(server_id.clone(), process.clone());
        self.metrics.processes_spawned.fetch_add(1, Ordering::Relaxed);

        info!(
            "Spawned STDIO process for server {}: {}",
            server_id, resolved_config.command
        );

        Ok(process)
    }

    /// Kill a specific process.
    pub async fn kill_process(&self, server_id: &ServerId) -> Result<()> {
        if let Some((_, process)) = self.processes.remove(server_id) {
            process.kill().await?;
            info!("Killed process for server {}", server_id);
        }
        Ok(())
    }

    /// Kill all processes.
    pub async fn kill_all(&self) -> Result<()> {
        let processes: Vec<_> = self.processes.iter().map(|entry| entry.value().clone()).collect();

        for process in processes {
            process.kill().await?;
        }

        self.processes.clear();
        info!("Killed all STDIO processes");
        Ok(())
    }
}

/// STDIO process wrapper with bidirectional communication.
pub struct StdioProcess {
    /// Child process handle
    child: Arc<Mutex<Child>>,
    /// Stdin writer (to server)
    stdin: Arc<Mutex<ChildStdin>>,
    /// Stdout reader (from server)
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    /// Stderr reader (for diagnostics)
    stderr: Arc<Mutex<BufReader<ChildStderr>>>,
    /// Process health status
    healthy: Arc<AtomicBool>,
}

impl StdioProcess {
    /// Create a new STDIO process wrapper.
    /// Automatically starts a background task to drain stderr to prevent blocking.
    fn new(
        server_id: String,
        child: Child,
        stdin: ChildStdin,
        stdout: ChildStdout,
        stderr: ChildStderr,
    ) -> Self {
        let stderr = Arc::new(Mutex::new(BufReader::new(stderr)));
        let stderr_clone = stderr.clone();
        let server_id_clone = server_id.clone();

        // CRITICAL: Spawn background task to continuously drain stderr
        // This prevents the stderr buffer from filling up and blocking the process.
        // STDIO MCP servers often print startup messages and logs to stderr, and if
        // we don't read them, the 64KB pipe buffer fills up, causing the process to
        // block on stderr writes and become unresponsive.
        tokio::spawn(Self::drain_stderr(stderr_clone, server_id_clone));

        Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            stderr,
            healthy: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Send a JSON-RPC message to the STDIO server (line-delimited JSON).
    pub async fn send_json(
        &self,
        value: &serde_json::Value,
    ) -> std::result::Result<(), TransportError> {
        let mut stdin = self.stdin.lock().await;

        // Serialize to JSON and add newline (MCP STDIO uses line-delimited JSON)
        let json_str = serde_json::to_string(value)?;
        stdin.write_all(json_str.as_bytes()).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;

        debug!(
            "Sent JSON-RPC message: {}",
            json_str.chars().take(100).collect::<String>()
        );
        Ok(())
    }

    /// Receive a JSON-RPC message from the STDIO server (line-delimited JSON).
    /// Skips non-JSON lines (like startup messages) until a valid JSON-RPC message is found.
    pub async fn receive_json(&self) -> std::result::Result<serde_json::Value, TransportError> {
        let mut stdout = self.stdout.lock().await;

        // Read lines until we find valid JSON
        loop {
            let mut line = String::new();
            let bytes_read = stdout.read_line(&mut line).await?;

            if bytes_read == 0 {
                return Err(TransportError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Connection closed",
                )));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                // Skip empty lines
                continue;
            }

            // Try to parse as JSON
            match serde_json::from_str::<serde_json::Value>(trimmed) {
                Ok(value) if value.is_object() => {
                    // Valid JSON object - check if it's JSON-RPC
                    if value.get("jsonrpc").is_some()
                        || value.get("method").is_some()
                        || value.get("result").is_some()
                    {
                        debug!(
                            "Received JSON-RPC message: {}",
                            trimmed.chars().take(100).collect::<String>()
                        );
                        return Ok(value);
                    } else {
                        // Valid JSON but not JSON-RPC, skip
                        debug!(
                            "Skipping non-JSON-RPC message: {}",
                            trimmed.chars().take(50).collect::<String>()
                        );
                        continue;
                    }
                },
                Ok(_) => {
                    // Valid JSON but not an object (array, string, etc), skip
                    debug!(
                        "Skipping non-object JSON: {}",
                        trimmed.chars().take(50).collect::<String>()
                    );
                    continue;
                },
                Err(_) => {
                    // Not valid JSON - likely a startup message or log
                    debug!(
                        "Skipping non-JSON line: {}",
                        trimmed.chars().take(50).collect::<String>()
                    );
                    continue;
                },
            }
        }
    }

    /// Send a message to the STDIO server (legacy binary method - deprecated).
    pub async fn send(&self, data: Vec<u8>) -> std::result::Result<(), TransportError> {
        let value: serde_json::Value = serde_json::from_slice(&data)?;
        self.send_json(&value).await
    }

    /// Receive a message from the STDIO server (legacy binary method - deprecated).
    pub async fn receive(&self) -> std::result::Result<Vec<u8>, TransportError> {
        let value = self.receive_json().await?;
        Ok(serde_json::to_vec(&value)?)
    }

    /// Read any available stderr output (non-blocking).
    pub async fn read_stderr(&self) -> Option<String> {
        let mut stderr = self.stderr.lock().await;
        let mut buffer = Vec::new();

        // Try to read available data (non-blocking)
        match tokio::time::timeout(Duration::from_millis(10), stderr.read_to_end(&mut buffer)).await
        {
            Ok(Ok(_)) if !buffer.is_empty() => Some(String::from_utf8_lossy(&buffer).to_string()),
            _ => None,
        }
    }

    /// Check if the process is still running and responsive.
    pub async fn is_healthy(&self) -> bool {
        let mut child = self.child.lock().await;

        // Check if process is still running
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process has exited
                error!("STDIO process exited with status: {:?}", status);
                self.healthy.store(false, Ordering::Relaxed);
                false
            },
            Ok(None) => {
                // Process still running
                self.healthy.load(Ordering::Relaxed)
            },
            Err(e) => {
                error!("Failed to check process status: {}", e);
                false
            },
        }
    }

    /// Kill the process.
    pub async fn kill(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        child.kill().await?;
        self.healthy.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// Send a ping to check if the process is responsive.
    pub async fn ping(&self) -> bool {
        // Send a simple ping request
        let ping_request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "id": 0
        });

        if let Ok(data) = serde_json::to_vec(&ping_request) {
            if self.send(data).await.is_ok() {
                // Try to receive response with short timeout
                if let Ok(Ok(_)) =
                    tokio::time::timeout(Duration::from_secs(1), self.receive()).await
                {
                    return true;
                }
            }
        }

        false
    }

    /// Background task to drain stderr (prevents blocking). Reduces nesting.
    async fn drain_stderr(stderr: Arc<Mutex<BufReader<ChildStderr>>>, server_id: String) {
        let mut stderr_lock = stderr.lock().await;
        let mut line = String::new();
        loop {
            line.clear();
            match stderr_lock.read_line(&mut line).await {
                Ok(0) => {
                    // EOF - process has exited
                    debug!("stderr [{}]: EOF reached", server_id);
                    break;
                },
                Ok(_) => {
                    // Log stderr output for debugging (skip empty lines)
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        debug!("stderr [{}]: {}", server_id, trimmed);
                    }
                },
                Err(e) => {
                    debug!("stderr [{}]: Read error: {}", server_id, e);
                    break;
                },
            }
        }
    }
}

/// Metrics for STDIO processes and MCP initialization.
#[derive(Default)]
pub struct ProcessMetrics {
    pub processes_spawned: AtomicU64,
    pub processes_killed: AtomicU64,
    pub requests_sent: AtomicU64,
    pub responses_received: AtomicU64,
    pub errors: AtomicU64,
    pub init_failures: AtomicU64,
    pub init_duration_sum: AtomicU64, // Total initialization time in milliseconds
}

impl Drop for StdioProcess {
    fn drop(&mut self) {
        // Attempt to kill the process when dropped
        let child = self.child.clone();
        tokio::spawn(async move {
            let mut child_guard = child.lock().await;
            let _ = child_guard.kill().await;
        });
    }
}
