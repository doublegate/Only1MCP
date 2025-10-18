//! Configuration validation logic

use crate::config::Config;
use crate::error::{Error, Result};

impl Config {
    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate server config
        if self.server.port == 0 {
            return Err(Error::Config("Server port must be non-zero".to_string()));
        }

        if self.server.max_connections == 0 {
            return Err(Error::Config(
                "max_connections must be non-zero".to_string(),
            ));
        }

        // Validate TLS config
        if self.server.tls.enabled {
            if self.server.tls.cert_path.is_none() {
                return Err(Error::Config(
                    "TLS enabled but cert_path not specified".to_string(),
                ));
            }
            if self.server.tls.key_path.is_none() {
                return Err(Error::Config(
                    "TLS enabled but key_path not specified".to_string(),
                ));
            }
        }

        // Validate backend servers
        if self.servers.is_empty() {
            tracing::warn!("No backend servers configured");
        }

        for server in &self.servers {
            if server.id.is_empty() {
                return Err(Error::Config("Server ID cannot be empty".to_string()));
            }
            if server.name.is_empty() {
                return Err(Error::Config(format!(
                    "Server {} has empty name",
                    server.id
                )));
            }
            if server.weight == 0 {
                return Err(Error::Config(format!(
                    "Server {} has zero weight",
                    server.id
                )));
            }

            // Validate health check config
            if server.health_check.enabled {
                if server.health_check.interval_seconds == 0 {
                    return Err(Error::Config(format!(
                        "Server {} has zero health check interval",
                        server.id
                    )));
                }
                if server.health_check.timeout_seconds == 0 {
                    return Err(Error::Config(format!(
                        "Server {} has zero health check timeout",
                        server.id
                    )));
                }
                if server.health_check.timeout_seconds >= server.health_check.interval_seconds {
                    return Err(Error::Config(format!(
                        "Server {} health check timeout must be less than interval",
                        server.id
                    )));
                }
            }
        }

        // Validate load balancer config
        let valid_algorithms = [
            "round_robin",
            "least_connections",
            "consistent_hash",
            "random",
            "weighted_random",
        ];
        if !valid_algorithms.contains(&self.proxy.load_balancer.algorithm.as_str()) {
            return Err(Error::Config(format!(
                "Invalid load balancer algorithm: {}. Valid options: {:?}",
                self.proxy.load_balancer.algorithm, valid_algorithms
            )));
        }

        if self.proxy.load_balancer.virtual_nodes == 0 {
            return Err(Error::Config("virtual_nodes must be non-zero".to_string()));
        }

        // Validate connection pool config
        if self.proxy.connection_pool.max_per_backend == 0 {
            return Err(Error::Config(
                "max_per_backend must be non-zero".to_string(),
            ));
        }

        if self.proxy.connection_pool.min_idle > self.proxy.connection_pool.max_per_backend {
            return Err(Error::Config(
                "min_idle cannot be greater than max_per_backend".to_string(),
            ));
        }

        // Validate cache config
        if self.context_optimization.cache.enabled {
            if self.context_optimization.cache.max_entries == 0 {
                return Err(Error::Config(
                    "cache max_entries must be non-zero".to_string(),
                ));
            }
            if self.context_optimization.cache.ttl_seconds == 0 {
                return Err(Error::Config(
                    "cache ttl_seconds must be non-zero".to_string(),
                ));
            }
        }

        // Validate batching config
        if self.context_optimization.batching.enabled {
            if self.context_optimization.batching.max_batch_size == 0 {
                return Err(Error::Config(
                    "batching max_batch_size must be non-zero".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let mut config = Config::default();
        config.server.port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_tls_without_cert() {
        let mut config = Config::default();
        config.server.tls.enabled = true;
        assert!(config.validate().is_err());
    }
}
