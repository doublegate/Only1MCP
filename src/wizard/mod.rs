//! Interactive Configuration Wizard
//!
//! Provides a friendly command-line interface for creating Only1MCP configurations

use crate::config::schema::*;
use crate::error::Result;
use std::io::{self, Write};
use std::path::PathBuf;

/// Configuration wizard
pub struct ConfigWizard {
    output_path: Option<PathBuf>,
}

impl ConfigWizard {
    pub fn new() -> Self {
        Self { output_path: None }
    }

    /// Set the output path for the configuration file
    pub fn with_output(mut self, path: PathBuf) -> Self {
        self.output_path = Some(path);
        self
    }

    /// Run the interactive wizard
    pub async fn run(&self) -> Result<ServerConfig> {
        println!("\n╔═══════════════════════════════════════════════════════════╗");
        println!("║         Welcome to the Only1MCP Configuration Wizard     ║");
        println!("╚═══════════════════════════════════════════════════════════╝\n");

        // Step 1: Choose deployment type
        let deployment = self.prompt_deployment_type()?;

        // Step 2: Basic server configuration
        let server_config = self.prompt_server_config()?;

        // Step 3: MCP servers
        let mcp_servers = self.prompt_mcp_servers()?;

        // Step 4: Load balancing
        let load_balancing = self.prompt_load_balancing()?;

        // Step 5: Optional features
        let features = self.prompt_features()?;

        // Build configuration
        let config = ServerConfig {
            host: server_config.host,
            port: server_config.port,
            workers: server_config.workers,
            mcp_servers,
            load_balancing: Some(load_balancing),
            cache: features.cache,
            rate_limiting: features.rate_limiting,
            auth: features.auth,
            admin: features.admin,
            metrics: features.metrics,
            hot_reload: Some(features.hot_reload),
            logging: Some(features.logging),
        };

        // Save configuration
        if let Some(path) = &self.output_path {
            self.save_config(&config, path)?;
            println!("\n✅ Configuration saved to: {}", path.display());
        }

        Ok(config)
    }

    fn prompt_deployment_type(&self) -> Result<DeploymentType> {
        println!("📋 Step 1: Deployment Type");
        println!("────────────────────────────────────────────────────────────");
        println!("1. Solo Developer    - Single user, local development");
        println!("2. Small Team        - 5-20 users, basic authentication");
        println!("3. Enterprise        - Large scale, full features");
        println!();

        let choice = self.prompt_number("Select deployment type (1-3)", 1, 3)?;

        Ok(match choice {
            1 => DeploymentType::Solo,
            2 => DeploymentType::Team,
            3 => DeploymentType::Enterprise,
            _ => DeploymentType::Solo,
        })
    }

    fn prompt_server_config(&self) -> Result<BasicServerConfig> {
        println!("\n📋 Step 2: Server Configuration");
        println!("────────────────────────────────────────────────────────────");

        let host = self.prompt_string(
            "Host to bind to",
            Some("0.0.0.0"),
        )?;

        let port = self.prompt_number(
            "Port to bind to",
            1,
            65535,
        )? as u16;

        let workers = if self.prompt_yes_no("Auto-detect number of workers?", true)? {
            None
        } else {
            Some(self.prompt_number("Number of worker threads", 1, 128)? as u16)
        };

        Ok(BasicServerConfig {
            host,
            port,
            workers,
        })
    }

    fn prompt_mcp_servers(&self) -> Result<Vec<McpServerConfig>> {
        println!("\n📋 Step 3: MCP Servers");
        println!("────────────────────────────────────────────────────────────");

        let mut servers = Vec::new();

        loop {
            println!("\nAdd MCP Server #{}", servers.len() + 1);

            let name = self.prompt_string("Server name (e.g., 'filesystem')", None)?;

            println!("\nTransport type:");
            println!("1. HTTP/HTTPS");
            println!("2. STDIO (local process)");
            println!("3. SSE (Server-Sent Events)");
            println!("4. WebSocket");

            let transport_type = self.prompt_number("Select transport (1-4)", 1, 4)?;

            let transport = match transport_type {
                1 => {
                    let url = self.prompt_string("HTTP/HTTPS URL", None)?;
                    Transport::Http { url }
                }
                2 => {
                    let command = self.prompt_string("Command to execute", None)?;
                    let args = self.prompt_string("Arguments (space-separated, or empty)", Some(""))?;
                    let args: Vec<String> = if args.is_empty() {
                        Vec::new()
                    } else {
                        args.split_whitespace().map(String::from).collect()
                    };
                    Transport::Stdio { command, args }
                }
                3 => {
                    let url = self.prompt_string("SSE URL", None)?;
                    Transport::Sse { url }
                }
                4 => {
                    let url = self.prompt_string("WebSocket URL", None)?;
                    Transport::WebSocket { url }
                }
                _ => unreachable!(),
            };

            let weight = if self.prompt_yes_no("Configure weight for load balancing?", false)? {
                Some(self.prompt_number("Weight (1-100)", 1, 100)? as u32)
            } else {
                None
            };

            servers.push(McpServerConfig {
                name,
                transport,
                weight,
                enabled: true,
                timeout_ms: None,
                max_retries: None,
            });

            if !self.prompt_yes_no("\nAdd another MCP server?", false)? {
                break;
            }
        }

        Ok(servers)
    }

    fn prompt_load_balancing(&self) -> Result<LoadBalancingConfig> {
        println!("\n📋 Step 4: Load Balancing");
        println!("────────────────────────────────────────────────────────────");
        println!("1. Round Robin       - Distribute requests evenly");
        println!("2. Least Connections - Route to least busy server");
        println!("3. Consistent Hash   - Sticky sessions based on request");
        println!("4. Random            - Random server selection");
        println!();

        let algorithm = match self.prompt_number("Select algorithm (1-4)", 1, 4)? {
            1 => LoadBalancingAlgorithm::RoundRobin,
            2 => LoadBalancingAlgorithm::LeastConnections,
            3 => LoadBalancingAlgorithm::ConsistentHash,
            4 => LoadBalancingAlgorithm::Random,
            _ => LoadBalancingAlgorithm::RoundRobin,
        };

        let health_check = if self.prompt_yes_no("Enable health checking?", true)? {
            Some(HealthCheckConfig {
                interval_secs: self.prompt_number("Check interval (seconds)", 1, 300)? as u64,
                timeout_ms: self.prompt_number("Check timeout (milliseconds)", 100, 10000)? as u64,
                unhealthy_threshold: 3,
                healthy_threshold: 2,
            })
        } else {
            None
        };

        Ok(LoadBalancingConfig {
            algorithm,
            health_check,
        })
    }

    fn prompt_features(&self) -> Result<FeaturesConfig> {
        println!("\n📋 Step 5: Optional Features");
        println!("────────────────────────────────────────────────────────────");

        let cache = if self.prompt_yes_no("Enable response caching?", true)? {
            Some(CacheConfig {
                enabled: true,
                max_size_mb: self.prompt_number("Cache size (MB)", 10, 10000)? as u64,
                ttl_secs: self.prompt_number("TTL (seconds)", 60, 86400)? as u64,
            })
        } else {
            None
        };

        let rate_limiting = if self.prompt_yes_no("Enable rate limiting?", true)? {
            Some(RateLimitConfig {
                enabled: true,
                requests_per_minute: self.prompt_number("Requests per minute", 10, 10000)? as u32,
                burst_size: None,
            })
        } else {
            None
        };

        let auth = if self.prompt_yes_no("Enable authentication?", false)? {
            println!("\nAuthentication type:");
            println!("1. JWT");
            println!("2. OAuth2");
            println!("3. Basic Auth");

            let auth_type = self.prompt_number("Select type (1-3)", 1, 3)?;

            Some(AuthConfig {
                jwt: if auth_type == 1 {
                    Some(JwtConfig {
                        secret: self.prompt_string("JWT secret", None)?,
                        algorithm: "HS256".to_string(),
                        expiry_secs: 3600,
                    })
                } else {
                    None
                },
                oauth: if auth_type == 2 {
                    Some(OAuth2Config {
                        client_id: self.prompt_string("OAuth2 client ID", None)?,
                        client_secret: self.prompt_string("OAuth2 client secret", None)?,
                        auth_url: self.prompt_string("Authorization URL", None)?,
                        token_url: self.prompt_string("Token URL", None)?,
                    })
                } else {
                    None
                },
                basic: None,
            })
        } else {
            None
        };

        let admin = if self.prompt_yes_no("Enable admin API?", true)? {
            Some(AdminConfig {
                enabled: true,
                port: self.prompt_number("Admin port", 1, 65535)? as u16,
            })
        } else {
            None
        };

        let metrics = if self.prompt_yes_no("Enable Prometheus metrics?", true)? {
            Some(MetricsConfig {
                enabled: true,
                port: self.prompt_number("Metrics port", 1, 65535)? as u16,
            })
        } else {
            None
        };

        let hot_reload = self.prompt_yes_no("Enable configuration hot-reload?", true)?;

        let log_level = if self.prompt_yes_no("Configure logging?", true)? {
            println!("\nLog level:");
            println!("1. Error");
            println!("2. Warn");
            println!("3. Info");
            println!("4. Debug");
            println!("5. Trace");

            let level = match self.prompt_number("Select level (1-5)", 1, 5)? {
                1 => "error",
                2 => "warn",
                3 => "info",
                4 => "debug",
                5 => "trace",
                _ => "info",
            };

            level.to_string()
        } else {
            "info".to_string()
        };

        let logging = LoggingConfig {
            level: log_level,
            file: None,
            json: false,
        };

        Ok(FeaturesConfig {
            cache,
            rate_limiting,
            auth,
            admin,
            metrics,
            hot_reload,
            logging,
        })
    }

    fn prompt_string(&self, prompt: &str, default: Option<&str>) -> Result<String> {
        print!("{}", prompt);
        if let Some(def) = default {
            print!(" [{}]", def);
        }
        print!(": ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        Ok(if input.is_empty() {
            default.unwrap_or("").to_string()
        } else {
            input.to_string()
        })
    }

    fn prompt_number(&self, prompt: &str, min: u32, max: u32) -> Result<u32> {
        loop {
            print!("{} ({}-{}): ", prompt, min, max);
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            match input.trim().parse::<u32>() {
                Ok(n) if n >= min && n <= max => return Ok(n),
                Ok(n) => println!("❌ Please enter a number between {} and {} (got {})", min, max, n),
                Err(_) => println!("❌ Please enter a valid number"),
            }
        }
    }

    fn prompt_yes_no(&self, prompt: &str, default: bool) -> Result<bool> {
        let default_str = if default { "Y/n" } else { "y/N" };
        print!("{} [{}]: ", prompt, default_str);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        Ok(match input.as_str() {
            "" => default,
            "y" | "yes" => true,
            "n" | "no" => false,
            _ => default,
        })
    }

    fn save_config(&self, config: &ServerConfig, path: &PathBuf) -> Result<()> {
        let yaml = serde_yaml::to_string(config)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }
}

// Helper structs
#[derive(Debug)]
enum DeploymentType {
    Solo,
    Team,
    Enterprise,
}

#[derive(Debug)]
struct BasicServerConfig {
    host: String,
    port: u16,
    workers: Option<u16>,
}

#[derive(Debug)]
struct FeaturesConfig {
    cache: Option<CacheConfig>,
    rate_limiting: Option<RateLimitConfig>,
    auth: Option<AuthConfig>,
    admin: Option<AdminConfig>,
    metrics: Option<MetricsConfig>,
    hot_reload: bool,
    logging: LoggingConfig,
}

impl Default for ConfigWizard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wizard_creation() {
        let wizard = ConfigWizard::new();
        assert!(wizard.output_path.is_none());

        let wizard = wizard.with_output(PathBuf::from("test.yaml"));
        assert!(wizard.output_path.is_some());
    }
}
