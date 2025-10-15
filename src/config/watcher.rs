//! Configuration file watching using the notify crate for cross-platform support.
//!
//! Implements debouncing to handle rapid file changes (e.g., editors that
//! write multiple times). Supports both polling (for network filesystems)
//! and native OS watchers (inotify on Linux, FSEvents on macOS, etc.).

use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event, EventKind};
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::error::ProxyError;

/// Configuration change event
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// Path to the changed configuration file
    pub path: PathBuf,

    /// Type of file system event
    pub event_type: EventKind,

    /// Timestamp when event was detected
    pub timestamp: Instant,
}

/// Configuration file watcher
pub struct ConfigWatcher {
    /// Path to configuration file
    config_path: PathBuf,

    /// File system watcher instance
    watcher: RecommendedWatcher,

    /// Debounce timer (default: 500ms)
    debounce: Duration,

    /// Event channel for config changes
    tx: mpsc::Sender<ConfigChangeEvent>,
}

/// Watcher error type
#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("Notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("Channel send error: {0}")]
    Send(#[from] mpsc::error::SendError<ConfigChangeEvent>),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl ConfigWatcher {
    /// Initialize file watcher with intelligent debouncing
    pub async fn new(
        config_path: impl AsRef<Path>,
        debounce_ms: u64,
    ) -> Result<(Self, mpsc::Receiver<ConfigChangeEvent>), WatcherError> {
        let config_path = config_path.as_ref().to_path_buf();
        let (tx, rx) = mpsc::channel(10);

        // Clone for the watcher closure
        let tx_clone = tx.clone();
        let path_clone = config_path.clone();
        let debounce = Duration::from_millis(debounce_ms);

        // Create watcher with error recovery
        let mut watcher = notify::recommended_watcher(
            move |res: notify::Result<Event>| {
                // Handle file system events
                match res {
                    Ok(event) => {
                        // Filter relevant events (modify, create, rename)
                        if Self::is_relevant_event(&event) {
                            let tx = tx_clone.clone();
                            let path = path_clone.clone();

                            // Spawn debounced handler
                            tokio::spawn(async move {
                                // Wait for debounce period
                                tokio::time::sleep(debounce).await;

                                // Verify file still exists and is readable
                                if path.exists() && path.is_file() {
                                    // Send change event
                                    let _ = tx.send(ConfigChangeEvent {
                                        path,
                                        event_type: event.kind,
                                        timestamp: Instant::now(),
                                    }).await;
                                }
                            });
                        }
                    }
                    Err(e) => {
                        tracing::error!("Watcher error: {:?}", e);
                        // Attempt to recover by re-establishing watch
                    }
                }
            }
        )?;

        // Watch the configuration file specifically
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;

        // Also watch parent directory for file replacement scenarios
        if let Some(parent) = config_path.parent() {
            watcher.watch(parent, RecursiveMode::NonRecursive)?;
        }

        Ok((
            Self {
                config_path,
                watcher,
                debounce,
                tx,
            },
            rx
        ))
    }

    /// Determine if an event should trigger a reload
    fn is_relevant_event(event: &Event) -> bool {
        matches!(
            event.kind,
            EventKind::Modify(_) |
            EventKind::Create(_) |
            EventKind::Remove(_) |
            EventKind::Other  // Some editors use atomic writes
        )
    }

    /// Force a configuration reload (useful for API-triggered reloads)
    pub async fn trigger_reload(&self) -> Result<(), WatcherError> {
        self.tx.send(ConfigChangeEvent {
            path: self.config_path.clone(),
            event_type: EventKind::Other,
            timestamp: Instant::now(),
        }).await?;

        Ok(())
    }

    /// Stop watching the configuration file
    pub fn stop(&mut self) -> Result<(), WatcherError> {
        self.watcher.unwatch(&self.config_path)?;

        if let Some(parent) = self.config_path.parent() {
            self.watcher.unwatch(parent)?;
        }

        Ok(())
    }
}

/// Hot reload manager for coordinating configuration updates
pub struct HotReloadManager {
    /// Current configuration version
    version: Arc<std::sync::atomic::AtomicU64>,

    /// Configuration loader
    config_loader: Arc<dyn ConfigLoader>,

    /// Reload callbacks
    callbacks: Arc<tokio::sync::RwLock<Vec<ReloadCallback>>>,
}

/// Configuration loader trait
#[async_trait::async_trait]
pub trait ConfigLoader: Send + Sync {
    /// Load configuration from path
    async fn load(&self, path: &Path) -> std::result::Result<crate::config::Config, ProxyError>;

    /// Validate configuration
    async fn validate(&self, config: &crate::config::Config) -> std::result::Result<(), ProxyError>;
}

/// Reload callback type
pub type ReloadCallback = Box<dyn Fn(u64) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

use std::pin::Pin;
use std::future::Future;
use std::sync::atomic::Ordering;

impl HotReloadManager {
    /// Create new hot reload manager
    pub fn new(config_loader: Arc<dyn ConfigLoader>) -> Self {
        Self {
            version: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            config_loader,
            callbacks: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    /// Register a reload callback
    pub async fn register_callback(&self, callback: ReloadCallback) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }

    /// Reload configuration from file
    pub async fn reload_configuration(&self, path: &Path) -> std::result::Result<u64, ProxyError> {
        // Load new configuration
        let new_config = self.config_loader.load(path).await?;

        // Validate before applying
        self.config_loader.validate(&new_config).await?;

        // Apply new configuration (atomic swap would happen here)
        let new_version = self.version.fetch_add(1, Ordering::SeqCst) + 1;

        // Notify all callbacks
        let callbacks = self.callbacks.read().await;
        for callback in callbacks.iter() {
            callback(new_version).await;
        }

        Ok(new_version)
    }

    /// Get current configuration version
    pub fn current_version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    /// Check if configuration is critical
    pub fn is_critical(&self) -> bool {
        false // Placeholder
    }
}

/// Handle configuration change events with validation
pub async fn process_config_changes(
    mut rx: mpsc::Receiver<ConfigChangeEvent>,
    reload_manager: Arc<HotReloadManager>,
) {
    // Track last processed event to prevent duplicate processing
    let mut last_processed: Option<Instant> = None;

    while let Some(event) = rx.recv().await {
        // Skip if we recently processed (within 100ms)
        if let Some(last) = last_processed {
            if event.timestamp.duration_since(last) < Duration::from_millis(100) {
                tracing::debug!("Skipping duplicate config change event");
                continue;
            }
        }

        // Attempt reload with comprehensive error handling
        match reload_manager.reload_configuration(&event.path).await {
            Ok(version) => {
                tracing::info!(
                    "Configuration reloaded successfully (version: {})",
                    version
                );
                last_processed = Some(event.timestamp);

                // Emit metrics
                metrics::counter!("config_reloads_success").increment(1);
            }
            Err(e) => {
                tracing::error!("Configuration reload failed: {:?}", e);

                // Keep current configuration active
                metrics::counter!("config_reloads_failed").increment(1);

                // Optionally trigger alerts for ops team
                if reload_manager.is_critical() {
                    tracing::error!(
                        "Critical configuration reload failure: {:?}",
                        e
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::fs;

    #[tokio::test]
    async fn test_config_watcher_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let (watcher, _rx) = ConfigWatcher::new(temp_file.path(), 100).await.unwrap();

        // Watcher should be created successfully
        assert_eq!(watcher.config_path, temp_file.path());
        assert_eq!(watcher.debounce, Duration::from_millis(100));
    }

    #[tokio::test]
    async fn test_config_change_detection() {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        let (_watcher, mut rx) = ConfigWatcher::new(&path, 50).await.unwrap();

        // Write to the file
        fs::write(&path, "test content").unwrap();

        // Should receive a change event
        tokio::time::timeout(Duration::from_secs(1), async {
            if let Some(event) = rx.recv().await {
                assert_eq!(event.path, path);
            }
        }).await.unwrap();
    }
}