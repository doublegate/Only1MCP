//! Configuration hot-reload implementation using notify and arc-swap

use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use arc_swap::ArcSwap;
use std::sync::Arc;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::watch;
use tracing::{info, warn, error, debug};

use crate::config::Config;
use crate::error::{Error, Result};

/// Configuration loader with hot-reload support
///
/// The ConfigLoader watches a configuration file for changes and automatically
/// reloads the configuration when the file is modified. It uses:
/// - notify-debouncer-full: Debounced file watching (500ms) to handle rapid changes
/// - arc-swap: Lock-free atomic config updates for high-performance reads
/// - tokio::sync::watch: Broadcast channel for notifying subscribers of updates
///
/// # Example
///
/// ```no_run
/// use std::path::PathBuf;
/// use only1mcp::config::ConfigLoader;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let loader = ConfigLoader::new(PathBuf::from("config.yaml"))?
///         .watch()?;
///
///     let config = loader.get_config();
///     println!("Server port: {}", config.server.port);
///
///     // Subscribe to reload events
///     let mut reload_rx = loader.subscribe();
///     tokio::spawn(async move {
///         while reload_rx.changed().await.is_ok() {
///             let new_config = reload_rx.borrow();
///             println!("Config reloaded! New port: {}", new_config.server.port);
///         }
///     });
///
///     Ok(())
/// }
/// ```
pub struct ConfigLoader {
    /// Current configuration (atomic updates via ArcSwap)
    config: Arc<ArcSwap<Config>>,

    /// Configuration file path
    config_path: PathBuf,

    /// Reload notification channel (sender)
    reload_tx: watch::Sender<Arc<Config>>,

    /// Reload notification channel (receiver)
    reload_rx: watch::Receiver<Arc<Config>>,

    /// File watcher (kept alive - dropping this stops watching)
    _watcher: Option<Debouncer<RecommendedWatcher, FileIdMap>>,
}

impl ConfigLoader {
    /// Create a new config loader
    ///
    /// Loads the initial configuration from the specified path.
    /// Does not start watching for changes until `watch()` is called.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the configuration file (YAML or TOML)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The configuration file cannot be read
    /// - The configuration file has invalid syntax
    /// - The configuration fails validation
    pub fn new(config_path: PathBuf) -> Result<Self> {
        // Load initial configuration
        let initial_config = Config::from_file(&config_path)?;

        // Validate initial config
        initial_config.validate()?;

        let config_arc = Arc::new(initial_config);

        // Create watch channel for reload notifications
        let (reload_tx, reload_rx) = watch::channel(config_arc.clone());

        // Create ArcSwap for atomic config updates
        let config = Arc::new(ArcSwap::from_pointee((*config_arc).clone()));

        info!("Configuration loaded from: {}", config_path.display());

        Ok(Self {
            config,
            config_path,
            reload_tx,
            reload_rx,
            _watcher: None,
        })
    }

    /// Start watching for configuration changes
    ///
    /// Starts a file watcher that monitors the configuration file for changes.
    /// When changes are detected (after 500ms debounce), the configuration is
    /// automatically reloaded, validated, and atomically swapped if valid.
    ///
    /// # Errors
    ///
    /// Returns an error if the file watcher cannot be created or started.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use only1mcp::config::ConfigLoader;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let loader = ConfigLoader::new(PathBuf::from("config.yaml"))?
    ///     .watch()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn watch(mut self) -> Result<Self> {
        let config_path = self.config_path.clone();
        let config = self.config.clone();
        let reload_tx = self.reload_tx.clone();

        // Create debounced file watcher (500ms debounce)
        // This prevents reload storms when editors save multiple times
        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: std::result::Result<Vec<DebouncedEvent>, Vec<notify::Error>>| {
                match result {
                    Ok(events) => {
                        for event in events {
                            if event.paths.contains(&config_path) {
                                debug!("Config file changed: {:?}", event.kind);
                                if let Err(e) = Self::reload_config_internal(
                                    &config_path,
                                    &config,
                                    &reload_tx,
                                ) {
                                    error!("Failed to reload config: {}", e);

                                    // Update error metrics
                                    #[cfg(feature = "metrics")]
                                    {
                                        use crate::metrics::CONFIG_RELOAD_ERRORS;
                                        CONFIG_RELOAD_ERRORS.inc();
                                    }
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        for e in errors {
                            error!("File watcher error: {}", e);
                        }
                    }
                }
            },
        ).map_err(|e| Error::Config(format!("Failed to create file watcher: {}", e)))?;

        // Watch the config file
        debouncer
            .watcher()
            .watch(self.config_path.as_path(), RecursiveMode::NonRecursive)
            .map_err(|e| Error::Config(format!("Failed to watch config file: {}", e)))?;

        info!("File watcher started for: {}", self.config_path.display());

        self._watcher = Some(debouncer);
        Ok(self)
    }

    /// Get current configuration
    ///
    /// Returns an Arc to the current configuration. This operation is lock-free
    /// and extremely fast, making it suitable for hot paths like request handling.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use only1mcp::config::ConfigLoader;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let loader = ConfigLoader::new(PathBuf::from("config.yaml"))?;
    /// let config = loader.get_config();
    /// println!("Server running on {}:{}", config.server.host, config.server.port);
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_config(&self) -> Arc<Config> {
        self.config.load_full()
    }

    /// Subscribe to configuration reload events
    ///
    /// Returns a watch::Receiver that receives notifications whenever the
    /// configuration is successfully reloaded. Multiple subscribers can
    /// independently receive reload notifications.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use only1mcp::config::ConfigLoader;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let loader = ConfigLoader::new(PathBuf::from("config.yaml"))?.watch()?;
    /// let mut reload_rx = loader.subscribe();
    ///
    /// tokio::spawn(async move {
    ///     while reload_rx.changed().await.is_ok() {
    ///         let config = reload_rx.borrow();
    ///         println!("Config reloaded: {} servers", config.servers.len());
    ///     }
    /// });
    /// # Ok(())
    /// # }
    /// ```
    pub fn subscribe(&self) -> watch::Receiver<Arc<Config>> {
        self.reload_rx.clone()
    }

    /// Internal config reload logic
    ///
    /// This function is called by the file watcher when changes are detected.
    /// It performs the following steps:
    /// 1. Load new configuration from file
    /// 2. Validate the new configuration
    /// 3. Atomically swap the configuration (if valid)
    /// 4. Notify all subscribers
    /// 5. Update metrics
    ///
    /// If any step fails, the old configuration is preserved.
    fn reload_config_internal(
        path: &Path,
        config: &Arc<ArcSwap<Config>>,
        reload_tx: &watch::Sender<Arc<Config>>,
    ) -> Result<()> {
        info!("Reloading configuration from: {}", path.display());

        // Load new configuration
        let new_config = Config::from_file(path)?;

        // Validate configuration
        new_config.validate()?;

        // Atomic swap (this is lock-free and extremely fast)
        let new_config_arc = Arc::new(new_config);
        config.store(new_config_arc.clone());

        // Notify subscribers (they'll receive Arc<Config> without copying data)
        let _ = reload_tx.send(new_config_arc);

        info!("Configuration reloaded successfully from: {}", path.display());

        // Update metrics
        #[cfg(feature = "metrics")]
        {
            use crate::metrics::CONFIG_RELOAD_TOTAL;
            CONFIG_RELOAD_TOTAL.inc();
        }

        Ok(())
    }

    /// Manually trigger a config reload
    ///
    /// Forces an immediate reload of the configuration file, bypassing the
    /// file watcher. Useful for testing or programmatic configuration updates.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be loaded or validated.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use std::path::PathBuf;
    /// # use only1mcp::config::ConfigLoader;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let loader = ConfigLoader::new(PathBuf::from("config.yaml"))?;
    /// loader.reload()?;
    /// println!("Configuration manually reloaded");
    /// # Ok(())
    /// # }
    /// ```
    pub fn reload(&self) -> Result<()> {
        Self::reload_config_internal(
            &self.config_path,
            &self.config,
            &self.reload_tx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_loader_initial_load() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_yaml = r#"
server:
  host: "127.0.0.1"
  port: 8080
servers: []
"#;
        fs::write(&temp_file, config_yaml).unwrap();

        let loader = ConfigLoader::new(temp_file.path().to_path_buf()).unwrap();
        let config = loader.get_config();

        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 8080);
    }

    #[tokio::test]
    async fn test_config_hot_reload() {
        let temp_file = NamedTempFile::new().unwrap();
        let initial_config = r#"
server:
  host: "127.0.0.1"
  port: 8080
servers: []
"#;
        fs::write(&temp_file, initial_config).unwrap();

        let loader = ConfigLoader::new(temp_file.path().to_path_buf())
            .unwrap()
            .watch()
            .unwrap();

        let mut reload_rx = loader.subscribe();

        // Give watcher time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Modify config file
        let new_config = r#"
server:
  host: "0.0.0.0"
  port: 9090
servers: []
"#;
        fs::write(&temp_file, new_config).unwrap();

        // Wait for reload notification (with timeout)
        tokio::select! {
            _ = reload_rx.changed() => {
                let config = loader.get_config();
                assert_eq!(config.server.host, "0.0.0.0");
                assert_eq!(config.server.port, 9090);
            }
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                panic!("Config reload timeout");
            }
        }
    }

    #[tokio::test]
    async fn test_invalid_config_keeps_old() {
        let temp_file = NamedTempFile::new().unwrap();
        let initial_config = r#"
server:
  host: "127.0.0.1"
  port: 8080
servers: []
"#;
        fs::write(&temp_file, initial_config).unwrap();

        let loader = ConfigLoader::new(temp_file.path().to_path_buf())
            .unwrap()
            .watch()
            .unwrap();

        let config_before = loader.get_config();

        // Give watcher time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Write invalid YAML
        fs::write(&temp_file, "invalid: yaml: content: [[[").unwrap();

        // Wait for debounce + processing
        tokio::time::sleep(Duration::from_secs(1)).await;

        let config_after = loader.get_config();
        assert_eq!(config_before.server.port, config_after.server.port);
        assert_eq!(config_before.server.host, config_after.server.host);
    }

    #[test]
    fn test_missing_file_error() {
        let result = ConfigLoader::new(PathBuf::from("/nonexistent/config.yaml"));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let temp_file = NamedTempFile::new().unwrap();
        let initial_config = r#"
server:
  host: "127.0.0.1"
  port: 8080
servers: []
"#;
        fs::write(&temp_file, initial_config).unwrap();

        let loader = ConfigLoader::new(temp_file.path().to_path_buf())
            .unwrap()
            .watch()
            .unwrap();

        let mut reload_rx1 = loader.subscribe();
        let mut reload_rx2 = loader.subscribe();

        // Give watcher time to start
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Modify config file
        let new_config = r#"
server:
  host: "0.0.0.0"
  port: 3000
servers: []
"#;
        fs::write(&temp_file, new_config).unwrap();

        // Both subscribers should receive the update
        tokio::select! {
            _ = reload_rx1.changed() => {}
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                panic!("Subscriber 1 timeout");
            }
        }

        tokio::select! {
            _ = reload_rx2.changed() => {}
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                panic!("Subscriber 2 timeout");
            }
        }

        let config = loader.get_config();
        assert_eq!(config.server.port, 3000);
    }

    #[tokio::test]
    async fn test_manual_reload() {
        let temp_file = NamedTempFile::new().unwrap();
        let initial_config = r#"
server:
  host: "127.0.0.1"
  port: 8080
servers: []
"#;
        fs::write(&temp_file, initial_config).unwrap();

        let loader = ConfigLoader::new(temp_file.path().to_path_buf()).unwrap();

        // Modify config file (without watcher)
        let new_config = r#"
server:
  host: "0.0.0.0"
  port: 5000
servers: []
"#;
        fs::write(&temp_file, new_config).unwrap();

        // Manually trigger reload
        loader.reload().unwrap();

        let config = loader.get_config();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 5000);
    }
}
