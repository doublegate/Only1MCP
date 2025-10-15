//! STDIO transport implementation for local MCP servers.
//!
//! Manages process spawning, bidirectional communication through pipes,
//! and security sandboxing for untrusted MCP servers.

use crate::error::{Error, Result};
use crate::types::{ServerId, McpRequest, McpResponse};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::collections::HashMap;
use tokio::process::{Child, ChildStdin, ChildStdout, ChildStderr, Command};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use dashmap::DashMap;
use tracing::{info, error, debug, warn};
use serde::{Deserialize, Serialize};

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

/// STDIO transport handler managing process lifecycle.
pub struct StdioTransport {
    /// Active STDIO processes
    processes: Arc<DashMap<ServerId, Arc<StdioProcess>>>,
    /// Process metrics
    metrics: Arc<ProcessMetrics>,
}

impl StdioTransport {
    /// Create a new STDIO transport handler.
    pub fn new() -> Self {
        Self {
            processes: Arc::new(DashMap::new()),
            metrics: Arc::new(ProcessMetrics::default()),
        }
    }

    /// Send a request to a STDIO MCP server.
    pub async fn send_request(
        &self,
        server_id: ServerId,
        config: &StdioConfig,
        request: McpRequest,
    ) -> Result<McpResponse, TransportError> {
        // Get or create STDIO process
        let process = self.get_or_create_process(
            server_id.clone(),
            config
        ).await?;

        // Send request through stdin
        let request_bytes = serde_json::to_vec(&request)?;
        process.send(request_bytes).await?;

        // Read response from stdout with timeout
        let response_bytes = tokio::time::timeout(
            Duration::from_millis(config.timeout_ms),
            process.receive()
        ).await
            .map_err(|_| TransportError::Timeout)?
            ?;

        // Parse response
        let response: McpResponse = serde_json::from_slice(&response_bytes)?;

        self.metrics.requests_sent.fetch_add(1, Ordering::Relaxed);
        Ok(response)
    }

    /// Get existing or spawn new STDIO process.
    async fn get_or_create_process(
        &self,
        server_id: ServerId,
        config: &StdioConfig,
    ) -> Result<Arc<StdioProcess>, TransportError> {
        // Check if process exists and is healthy
        if let Some(process) = self.processes.get(&server_id) {
            if process.is_healthy().await {
                return Ok(process.clone());
            }
            // Process unhealthy, remove it
            self.processes.remove(&server_id);
            warn!("Removed unhealthy process for server {}", server_id);
        }

        // Spawn new process with security restrictions
        let mut command = Command::new(&config.command);
        command
            .args(&config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Set working directory
        if let Some(cwd) = &config.cwd {
            command.current_dir(cwd);
        }

        // Apply environment variables
        for (key, value) in &config.env {
            command.env(key, value);
        }

        // Security: Restrict process capabilities (Linux)
        #[cfg(target_os = "linux")]
        if config.sandbox {
            use std::os::unix::process::CommandExt;

            // Run as non-root user if we're root
            unsafe {
                if libc::getuid() == 0 {
                    command.uid(1000);  // Run as user 1000
                    command.gid(1000);
                }
            }

            // Set resource limits
            command.pre_exec(move || {
                // Limit CPU time
                if let Some(max_cpu) = config.max_cpu_percent {
                    let cpu_limit = (max_cpu as u64) * 10; // Convert percentage to deciseconds
                    let rlimit = libc::rlimit {
                        rlim_cur: cpu_limit,
                        rlim_max: cpu_limit,
                    };
                    unsafe {
                        libc::setrlimit(libc::RLIMIT_CPU, &rlimit);
                    }
                }

                // Limit memory
                if let Some(max_memory_mb) = config.max_memory_mb {
                    let memory_bytes = max_memory_mb * 1024 * 1024;
                    let rlimit = libc::rlimit {
                        rlim_cur: memory_bytes,
                        rlim_max: memory_bytes,
                    };
                    unsafe {
                        libc::setrlimit(libc::RLIMIT_AS, &rlimit);
                    }
                }

                // Limit number of processes
                let rlimit = libc::rlimit {
                    rlim_cur: 10,
                    rlim_max: 10,
                };
                unsafe {
                    libc::setrlimit(libc::RLIMIT_NPROC, &rlimit);
                }

                Ok(())
            });
        }

        let mut child = command.spawn()
            .map_err(|e| TransportError::ProcessSpawnFailed(e))?;

        let stdin = child.stdin.take()
            .ok_or(TransportError::NoStdin)?;
        let stdout = child.stdout.take()
            .ok_or(TransportError::NoStdout)?;
        let stderr = child.stderr.take()
            .ok_or(TransportError::NoStderr)?;

        let process = Arc::new(StdioProcess::new(
            child,
            stdin,
            stdout,
            stderr,
        ));

        // Store in process map
        self.processes.insert(server_id.clone(), process.clone());
        self.metrics.processes_spawned.fetch_add(1, Ordering::Relaxed);

        info!("Spawned STDIO process for server {}: {}", server_id, config.command);
        Ok(process)
    }

    /// Kill a specific process.
    pub async fn kill_process(&self, server_id: &ServerId) -> Result<(), Error> {
        if let Some((_, process)) = self.processes.remove(server_id) {
            process.kill().await?;
            info!("Killed process for server {}", server_id);
        }
        Ok(())
    }

    /// Kill all processes.
    pub async fn kill_all(&self) -> Result<(), Error> {
        let processes: Vec<_> = self.processes.iter()
            .map(|entry| entry.value().clone())
            .collect();

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
    fn new(
        child: Child,
        stdin: ChildStdin,
        stdout: ChildStdout,
        stderr: ChildStderr,
    ) -> Self {
        Self {
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
            stderr: Arc::new(Mutex::new(BufReader::new(stderr))),
            healthy: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Send a message to the STDIO server.
    pub async fn send(&self, data: Vec<u8>) -> Result<(), TransportError> {
        let mut stdin = self.stdin.lock().await;

        // Write length-prefixed message (4 bytes length + data)
        let len = data.len() as u32;
        stdin.write_u32(len).await?;
        stdin.write_all(&data).await?;
        stdin.flush().await?;

        debug!("Sent {} bytes to STDIO process", data.len());
        Ok(())
    }

    /// Receive a message from the STDIO server.
    pub async fn receive(&self) -> Result<Vec<u8>, TransportError> {
        let mut stdout = self.stdout.lock().await;

        // Read length prefix
        let len = stdout.read_u32().await?;

        if len > 10_000_000 {
            // Sanity check: reject messages larger than 10MB
            error!("Received message too large: {} bytes", len);
            return Err(TransportError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message too large"
            )));
        }

        // Read message data
        let mut buffer = vec![0u8; len as usize];
        stdout.read_exact(&mut buffer).await?;

        debug!("Received {} bytes from STDIO process", buffer.len());
        Ok(buffer)
    }

    /// Read any available stderr output (non-blocking).
    pub async fn read_stderr(&self) -> Option<String> {
        let mut stderr = self.stderr.lock().await;
        let mut buffer = Vec::new();

        // Try to read available data (non-blocking)
        match tokio::time::timeout(
            Duration::from_millis(10),
            stderr.read_to_end(&mut buffer)
        ).await {
            Ok(Ok(_)) if !buffer.is_empty() => {
                Some(String::from_utf8_lossy(&buffer).to_string())
            }
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
            }
        }
    }

    /// Kill the process.
    pub async fn kill(&self) -> Result<(), Error> {
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
                if let Ok(Ok(_)) = tokio::time::timeout(
                    Duration::from_secs(1),
                    self.receive()
                ).await {
                    return true;
                }
            }
        }

        false
    }
}

/// Metrics for STDIO processes.
#[derive(Default)]
pub struct ProcessMetrics {
    pub processes_spawned: std::sync::atomic::AtomicU64,
    pub processes_killed: std::sync::atomic::AtomicU64,
    pub requests_sent: std::sync::atomic::AtomicU64,
    pub responses_received: std::sync::atomic::AtomicU64,
    pub errors: std::sync::atomic::AtomicU64,
}

impl Drop for StdioProcess {
    fn drop(&mut self) {
        // Attempt to kill the process when dropped
        let child = self.child.clone();
        tokio::spawn(async move {
            if let Ok(mut child) = child.lock().await {
                let _ = child.kill().await;
            }
        });
    }
}
