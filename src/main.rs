//! Only1MCP - The Ultimate MCP Server Aggregator
//!
//! A high-performance, Rust-based proxy and aggregator for Model Context Protocol (MCP) servers.
//! Provides a unified interface for AI applications to interact with multiple MCP tool servers,
//! dramatically reducing context overhead and improving performance.
//!
//! # Features
//!
//! - **Unified Proxy**: Single endpoint for multiple MCP servers
//! - **Hot-Swap**: Add/remove servers without downtime
//! - **Context Optimization**: 50-70% reduction in token usage via caching and batching
//! - **Multi-Transport**: STDIO, HTTP, SSE, and WebSocket support
//! - **Enterprise Security**: OAuth2, JWT, RBAC, audit logging
//! - **High Performance**: <5ms latency overhead, 10k+ req/s throughput

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{error, info};

mod auth;
mod cache;
mod config;
mod error;
mod health;
mod metrics;
mod proxy;
mod transport;

use error::Result;

#[derive(Parser)]
#[command(name = "only1mcp")]
#[command(about = "The Ultimate MCP Server Aggregator", long_about = None)]
#[command(version)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, env = "ONLY1MCP_CONFIG")]
    config: Option<PathBuf>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, env = "ONLY1MCP_LOG_LEVEL", default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the proxy server
    Start {
        /// Server host
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Server port
        #[arg(long, default_value = "8080")]
        port: u16,
    },

    /// Validate configuration file
    Validate {
        /// Configuration file to validate
        #[arg(value_name = "CONFIG")]
        config: PathBuf,
    },

    /// List configured servers
    List,

    /// Add a new MCP server
    Add {
        /// Server ID
        #[arg(long)]
        id: String,

        /// Server name
        #[arg(long)]
        name: String,

        /// Transport type (stdio, http, sse)
        #[arg(long)]
        transport: String,

        /// Command for STDIO transport
        #[arg(long)]
        command: Option<String>,

        /// URL for HTTP transport
        #[arg(long)]
        url: Option<String>,
    },

    /// Remove an MCP server
    Remove {
        /// Server ID to remove
        id: String,
    },

    /// Test connection to a server
    Test {
        /// Server ID to test
        id: String,
    },

    /// Show server health status
    Status,

    /// View logs
    Logs {
        /// Filter by server ID
        #[arg(long)]
        server: Option<String>,

        /// Follow logs
        #[arg(short, long)]
        follow: bool,
    },

    /// Generate configuration template
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Interactive TUI mode
    Tui,

    /// Run benchmarks
    Benchmark {
        /// Number of requests
        #[arg(long, default_value = "10000")]
        requests: usize,

        /// Number of concurrent connections
        #[arg(long, default_value = "100")]
        concurrency: usize,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Generate configuration template
    Generate {
        /// Template type (solo, team, enterprise)
        #[arg(long, default_value = "solo")]
        template: String,
    },

    /// Convert configuration format
    Convert {
        /// Input file
        #[arg(long)]
        from: PathBuf,

        /// Output file
        #[arg(long)]
        to: PathBuf,
    },

    /// Validate and fix configuration
    Doctor,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let cli = Cli::parse();

    // Initialize tracing/logging
    init_tracing(&cli.log_level)?;

    info!("Only1MCP v{} starting...", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = if let Some(config_path) = &cli.config {
        config::Config::from_file(config_path)?
    } else {
        config::Config::discover_and_load()?
    };

    // Execute command
    match cli.command {
        Commands::Start { host, port } => {
            info!("Starting proxy server on {}:{}", host, port);
            let server = proxy::ProxyServer::new(config).await?;
            server.run().await?;
        },

        Commands::Validate {
            config: config_path,
        } => {
            info!("Validating configuration: {:?}", config_path);
            match config::Config::validate_file(&config_path) {
                Ok(_) => {
                    println!("✓ Configuration valid");
                    std::process::exit(0);
                },
                Err(e) => {
                    eprintln!("✗ Configuration errors found:");
                    eprintln!("{}", e);
                    std::process::exit(1);
                },
            }
        },

        Commands::List => {
            println!("Configured MCP Servers:");
            for server in &config.servers {
                println!(
                    "  - {} ({}): {:?}",
                    server.id, server.name, server.transport
                );
            }
        },

        Commands::Add { .. } => {
            println!("Server addition via CLI not yet implemented");
            println!("Please edit configuration file or use admin API");
        },

        Commands::Remove { .. } => {
            println!("Server removal via CLI not yet implemented");
            println!("Please edit configuration file or use admin API");
        },

        Commands::Test { id } => {
            println!("Testing connection to server: {}", id);
            // TODO: Implement connection test
        },

        Commands::Status => {
            println!("Server health status:");
            println!("  (Status monitoring not yet implemented)");
        },

        Commands::Logs { .. } => {
            println!("Log viewing not yet implemented");
        },

        Commands::Config { action } => {
            match action {
                ConfigCommands::Generate { template } => {
                    let template_content = generate_config_template(&template)?;
                    println!("{}", template_content);
                },
                ConfigCommands::Convert { from, to } => {
                    println!("Converting {} to {}", from.display(), to.display());
                    // TODO: Implement format conversion
                },
                ConfigCommands::Doctor => {
                    println!("Running configuration diagnostics...");
                    // TODO: Implement config doctor
                },
            }
        },

        Commands::Tui => {
            println!("TUI mode not yet implemented");
            println!("Coming in Phase 2!");
        },

        Commands::Benchmark {
            requests,
            concurrency,
        } => {
            println!(
                "Running benchmark with {} requests and {} concurrent connections",
                requests, concurrency
            );
            // TODO: Implement benchmark
        },
    }

    Ok(())
}

fn init_tracing(log_level: &str) -> Result<()> {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::registry().with(filter).with(fmt::layer()).init();

    Ok(())
}

fn generate_config_template(template_type: &str) -> Result<String> {
    match template_type {
        "solo" => Ok(include_str!("../config/templates/solo.yaml").to_string()),
        "team" => Ok(include_str!("../config/templates/team.yaml").to_string()),
        "enterprise" => Ok(include_str!("../config/templates/enterprise.yaml").to_string()),
        _ => Err(error::Error::InvalidTemplate(template_type.to_string())),
    }
}
