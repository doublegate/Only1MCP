//! Daemon lifecycle management for Only1MCP
//!
//! Provides Unix daemon functionality including:
//! - Process daemonization (fork/detach)
//! - PID file management
//! - Process lifecycle tracking
//! - Graceful shutdown coordination

use crate::error::{Error, Result};
use daemonize::Daemonize;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub mod signals;

/// Daemon manager for Only1MCP
///
/// Handles daemonization, PID file management, and process lifecycle.
#[derive(Debug)]
pub struct DaemonManager {
    pid_file: PathBuf,
    log_file: PathBuf,
    config_dir: PathBuf,
}

impl DaemonManager {
    /// Create a new daemon manager
    ///
    /// Uses `~/.config/only1mcp/` as the base directory for all daemon files.
    pub fn new() -> Result<Self> {
        let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config).join("only1mcp")
        } else {
            dirs::home_dir()
                .ok_or_else(|| Error::Config("Cannot determine home directory".into()))?
                .join(".config")
                .join("only1mcp")
        };

        // Ensure directory exists
        fs::create_dir_all(&config_dir)
            .map_err(|e| Error::Config(format!("Failed to create config directory: {}", e)))?;

        Ok(Self {
            pid_file: config_dir.join("only1mcp.pid"),
            log_file: config_dir.join("only1mcp.log"),
            config_dir,
        })
    }

    /// Daemonize the current process
    ///
    /// Forks the process, detaches from the controlling terminal, and redirects
    /// stdout/stderr to the log file. The parent process exits, leaving the child
    /// as a background daemon.
    #[cfg(unix)]
    pub fn daemonize(&self) -> Result<()> {
        use std::fs::OpenOptions;

        info!("Daemonizing process...");

        // Open log file for stdout/stderr redirection
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .map_err(|e| Error::Server(format!("Failed to open log file: {}", e)))?;

        let daemon =
            Daemonize::new()
                .pid_file(&self.pid_file)
                .working_directory("/tmp")
                .stdout(log_file.try_clone().map_err(|e| {
                    Error::Server(format!("Failed to clone log file handle: {}", e))
                })?)
                .stderr(log_file);

        daemon
            .start()
            .map_err(|e| Error::Server(format!("Failed to daemonize: {}", e)))?;

        info!("Process daemonized successfully");
        Ok(())
    }

    /// Daemonize the current process (non-Unix platforms - not supported)
    #[cfg(not(unix))]
    pub fn daemonize(&self) -> Result<()> {
        Err(Error::Server(
            "Daemon mode is not supported on this platform. Use --foreground flag.".into(),
        ))
    }

    /// Check if a daemon instance is currently running
    ///
    /// Returns `true` if a PID file exists and the process is alive.
    /// Automatically cleans up stale PID files.
    pub fn is_running(&self) -> bool {
        if !self.pid_file.exists() {
            return false;
        }

        // Read PID and check if process exists
        match fs::read_to_string(&self.pid_file) {
            Ok(pid_str) => match pid_str.trim().parse::<i32>() {
                Ok(pid) => {
                    // Send signal 0 to check if process exists (doesn't actually send a signal)
                    match signal::kill(Pid::from_raw(pid), None) {
                        Ok(_) => {
                            // Process exists
                            true
                        },
                        Err(_) => {
                            // Process doesn't exist, clean up stale PID file
                            warn!("Stale PID file detected, cleaning up");
                            let _ = fs::remove_file(&self.pid_file);
                            false
                        },
                    }
                },
                Err(_) => {
                    warn!("Invalid PID in file, removing");
                    let _ = fs::remove_file(&self.pid_file);
                    false
                },
            },
            Err(_) => {
                let _ = fs::remove_file(&self.pid_file);
                false
            },
        }
    }

    /// Stop a running daemon instance
    ///
    /// Sends SIGTERM to gracefully shutdown the daemon. If the process doesn't
    /// exit within 30 seconds, sends SIGKILL.
    #[cfg(unix)]
    pub fn stop(&self) -> Result<()> {
        use std::thread;
        use std::time::Duration;

        if !self.pid_file.exists() {
            return Err(Error::Server(
                "No running instance found (PID file missing)".into(),
            ));
        }

        let pid_str = fs::read_to_string(&self.pid_file)
            .map_err(|e| Error::Server(format!("Failed to read PID file: {}", e)))?;

        let pid = pid_str
            .trim()
            .parse::<i32>()
            .map_err(|_| Error::Server("Invalid PID file format".into()))?;

        info!("Sending SIGTERM to process {}", pid);

        // Send SIGTERM for graceful shutdown
        signal::kill(Pid::from_raw(pid), Signal::SIGTERM)
            .map_err(|e| Error::Server(format!("Failed to send SIGTERM: {}", e)))?;

        // Wait for process to exit (with timeout)
        for i in 0..30 {
            // 30 iterations * 100ms = 3 seconds
            thread::sleep(Duration::from_millis(100));

            // Check if process still exists
            if signal::kill(Pid::from_raw(pid), None).is_err() {
                // Process no longer exists
                info!("Process exited gracefully");
                let _ = fs::remove_file(&self.pid_file);
                return Ok(());
            }

            if i == 29 {
                // Last iteration
                warn!("Process did not respond to SIGTERM after 3 seconds");
            }
        }

        // If still running, send SIGKILL
        warn!("Process did not exit gracefully, sending SIGKILL");
        signal::kill(Pid::from_raw(pid), Signal::SIGKILL)
            .map_err(|e| Error::Server(format!("Failed to send SIGKILL: {}", e)))?;

        // Give it a moment to die
        thread::sleep(Duration::from_millis(500));

        let _ = fs::remove_file(&self.pid_file);
        info!("Process forcefully terminated");

        Ok(())
    }

    /// Stop a running daemon instance (non-Unix platforms - not supported)
    #[cfg(not(unix))]
    pub fn stop(&self) -> Result<()> {
        Err(Error::Server(
            "Daemon stop is not supported on this platform".into(),
        ))
    }

    /// Get the path to the log file
    pub fn get_log_path(&self) -> &Path {
        &self.log_file
    }

    /// Get the path to the PID file
    pub fn get_pid_path(&self) -> &Path {
        &self.pid_file
    }

    /// Get the configuration directory
    pub fn get_config_dir(&self) -> &Path {
        &self.config_dir
    }
}

impl Default for DaemonManager {
    fn default() -> Self {
        Self::new().expect("Failed to create DaemonManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daemon_manager_creation() {
        let mgr = DaemonManager::new().unwrap();
        assert!(mgr.get_pid_path().to_string_lossy().contains("only1mcp.pid"));
        assert!(mgr.get_log_path().to_string_lossy().contains("only1mcp.log"));
    }

    #[test]
    fn test_is_running_no_pid_file() {
        let mgr = DaemonManager::new().unwrap();
        // Ensure no PID file exists
        let _ = fs::remove_file(mgr.get_pid_path());
        assert!(!mgr.is_running());
    }
}
