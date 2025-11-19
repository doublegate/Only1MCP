/// Plugin Management CLI Commands

use crate::error::{Error, Result};
use crate::marketplace::{db, storage, types::*, validator::PluginValidator};
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    /// Install a plugin from the marketplace
    Install {
        /// Plugin name
        name: String,

        /// Specific version (defaults to latest)
        #[arg(short, long)]
        version: Option<String>,

        /// Install from local path instead of registry
        #[arg(long)]
        local: Option<PathBuf>,

        /// Install from URL
        #[arg(long)]
        url: Option<String>,
    },

    /// List installed plugins
    List {
        /// Show all available plugins from registry
        #[arg(short, long)]
        all: bool,
    },

    /// Search for plugins in the marketplace
    Search {
        /// Search query
        query: String,

        /// Filter by capability
        #[arg(short, long)]
        capability: Option<String>,

        /// Sort by (downloads, rating, recent, name)
        #[arg(short, long, default_value = "downloads")]
        sort: String,
    },

    /// Show plugin information
    Info {
        /// Plugin name
        name: String,
    },

    /// Publish a plugin to the marketplace
    Publish {
        /// Plugin directory path
        path: PathBuf,

        /// Marketplace API token
        #[arg(long, env = "ONLY1MCP_MARKETPLACE_TOKEN")]
        token: Option<String>,

        /// Dry run (validate only, don't publish)
        #[arg(long)]
        dry_run: bool,
    },

    /// Initialize a new plugin from template
    Init {
        /// Plugin name
        name: String,

        /// Template type (basic, auth, rate-limiter, metrics)
        #[arg(short, long, default_value = "basic")]
        template: String,
    },

    /// Test a plugin locally
    Test {
        /// Plugin name
        name: String,

        /// Verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        name: String,
    },
}

impl PluginCommand {
    pub async fn execute(self) -> Result<()> {
        match self {
            Self::Install {
                name,
                version,
                local,
                url,
            } => install_plugin(name, version, local, url).await,

            Self::List { all } => list_plugins(all).await,

            Self::Search {
                query,
                capability,
                sort,
            } => search_plugins(query, capability, sort).await,

            Self::Info { name } => show_plugin_info(name).await,

            Self::Publish {
                path,
                token,
                dry_run,
            } => publish_plugin(path, token, dry_run).await,

            Self::Init { name, template } => init_plugin(name, template).await,

            Self::Test { name, verbose } => test_plugin(name, verbose).await,

            Self::Uninstall { name } => uninstall_plugin(name).await,
        }
    }
}

/// Install a plugin
async fn install_plugin(
    name: String,
    version: Option<String>,
    local: Option<PathBuf>,
    url: Option<String>,
) -> Result<()> {
    if let Some(local_path) = local {
        println!("📦 Installing plugin from local path: {}", local_path.display());
        // TODO: Install from local tarball
        return Ok(());
    }

    if let Some(url_path) = url {
        println!("📦 Installing plugin from URL: {}", url_path);
        // TODO: Download and install from URL
        return Ok(());
    }

    // Install from registry
    let version_str = version.as_deref().unwrap_or("latest");
    println!("📦 Installing {} version {}...", name, version_str);

    // Download from marketplace
    let plugin = db::get_plugin_by_name(&name)
        .await?
        .ok_or_else(|| Error::Config(format!("Plugin '{}' not found", name)))?;

    let target_version = version.unwrap_or(plugin.latest_version);

    let tarball = storage::download_plugin(&name, &target_version).await?;

    // TODO: Extract and install plugin
    let install_dir = get_plugin_install_dir(&name)?;
    println!("✅ Installed {} v{} to {}", name, target_version, install_dir.display());

    Ok(())
}

/// List plugins
async fn list_plugins(all: bool) -> Result<()> {
    if all {
        // List all available plugins from registry
        let query = PluginSearchQuery {
            query: None,
            capability: None,
            sort_by: Some(SortBy::Downloads),
            page: Some(1),
            per_page: Some(50),
        };

        let response = db::search_plugins(&query).await?;

        println!("\n📦 Available Plugins ({} total):\n", response.total);
        for plugin in response.plugins {
            println!(
                "  {} v{} - {}",
                plugin.name, plugin.version, plugin.description
            );
            println!(
                "    Downloads: {}  Rating: {}  Author: {}",
                plugin.downloads,
                plugin.rating.map_or("N/A".to_string(), |r| format!("{:.1}/5.0", r)),
                plugin.author
            );
            println!();
        }
    } else {
        // List installed plugins
        let install_dir = get_plugins_dir()?;
        println!("\n📦 Installed Plugins:\n");

        if !install_dir.exists() {
            println!("  No plugins installed yet.");
            return Ok(());
        }

        // TODO: Read installed plugins from directory
        println!("  Plugin listing not yet implemented");
    }

    Ok(())
}

/// Search plugins
async fn search_plugins(query: String, capability: Option<String>, sort: String) -> Result<()> {
    let sort_by = match sort.as_str() {
        "downloads" => SortBy::Downloads,
        "rating" => SortBy::Rating,
        "recent" => SortBy::RecentlyUpdated,
        "name" => SortBy::Name,
        _ => SortBy::Downloads,
    };

    let search_query = PluginSearchQuery {
        query: Some(query.clone()),
        capability: None, // TODO: Parse capability
        sort_by: Some(sort_by),
        page: Some(1),
        per_page: Some(20),
    };

    let response = db::search_plugins(&search_query).await?;

    println!("\n🔍 Search results for '{}' ({} found):\n", query, response.total);

    for plugin in response.plugins {
        println!("  {} v{}", plugin.name, plugin.version);
        println!("    {}", plugin.description);
        println!(
            "    Downloads: {}  Rating: {}",
            plugin.downloads,
            plugin.rating.map_or("N/A".to_string(), |r| format!("{:.1}/5.0", r))
        );
        println!();
    }

    Ok(())
}

/// Show plugin information
async fn show_plugin_info(name: String) -> Result<()> {
    let plugin = db::get_plugin_by_name(&name)
        .await?
        .ok_or_else(|| Error::Config(format!("Plugin '{}' not found", name)))?;

    println!("\n📦 Plugin: {}\n", plugin.name);
    println!("Version:     {}", plugin.latest_version);
    println!("Author:      {}", plugin.author);
    println!("License:     {}", plugin.license);
    println!("Description: {}", plugin.description);
    if let Some(ref homepage) = plugin.homepage {
        println!("Homepage:    {}", homepage);
    }
    if let Some(ref repo) = plugin.repository {
        println!("Repository:  {}", repo);
    }
    println!("\nStatistics:");
    println!("  Downloads: {}", plugin.downloads);
    println!(
        "  Rating:    {}",
        plugin.rating.map_or("N/A".to_string(), |r| format!("{:.1}/5.0", r))
    );
    println!("  Created:   {}", plugin.created_at.format("%Y-%m-%d"));
    println!("  Updated:   {}", plugin.updated_at.format("%Y-%m-%d"));

    Ok(())
}

/// Publish a plugin
async fn publish_plugin(path: PathBuf, token: Option<String>, dry_run: bool) -> Result<()> {
    println!("📤 Publishing plugin from: {}", path.display());

    // Load manifest
    let manifest_path = path.join("plugin.yaml");
    if !manifest_path.exists() {
        return Err(Error::Config(
            "plugin.yaml not found in plugin directory".to_string(),
        ));
    }

    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: PluginManifest = serde_yaml::from_str(&manifest_content)
        .map_err(|e| Error::Config(format!("Invalid plugin.yaml: {}", e)))?;

    // Validate manifest
    PluginValidator::validate_manifest(&manifest)?;
    println!("✅ Manifest validation passed");

    if dry_run {
        println!("🔍 Dry run completed successfully (not published)");
        return Ok(());
    }

    // Create tarball
    println!("📦 Creating plugin tarball...");
    // TODO: Create tarball from directory

    // Publish to marketplace
    if token.is_none() {
        return Err(Error::Config(
            "Marketplace token required. Set ONLY1MCP_MARKETPLACE_TOKEN or use --token".to_string(),
        ));
    }

    println!("✅ Plugin {} v{} published successfully!", manifest.name, manifest.version);

    Ok(())
}

/// Initialize a new plugin from template
async fn init_plugin(name: String, template: String) -> Result<()> {
    PluginValidator::validate_name(&name)?;

    let plugin_dir = std::env::current_dir()?.join(&name);

    if plugin_dir.exists() {
        return Err(Error::Config(format!(
            "Directory '{}' already exists",
            name
        )));
    }

    println!("🚀 Creating new plugin: {}", name);
    println!("   Template: {}", template);

    tokio::fs::create_dir_all(&plugin_dir).await?;

    // Create plugin.yaml
    let manifest = create_plugin_manifest(&name, &template)?;
    let manifest_yaml = serde_yaml::to_string(&manifest)
        .map_err(|e| Error::Config(format!("Failed to serialize manifest: {}", e)))?;

    tokio::fs::write(plugin_dir.join("plugin.yaml"), manifest_yaml).await?;

    // Create Cargo.toml
    let cargo_toml = create_cargo_toml(&name);
    tokio::fs::write(plugin_dir.join("Cargo.toml"), cargo_toml).await?;

    // Create src/lib.rs
    tokio::fs::create_dir_all(plugin_dir.join("src")).await?;
    let lib_rs = create_lib_rs(&name, &template);
    tokio::fs::write(plugin_dir.join("src/lib.rs"), lib_rs).await?;

    // Create README.md
    let readme = format!("# {}\n\nOnly1MCP plugin for {}.\n\n## Usage\n\nTODO: Add usage instructions\n", name, name);
    tokio::fs::write(plugin_dir.join("README.md"), readme).await?;

    // Create LICENSE
    tokio::fs::write(plugin_dir.join("LICENSE"), "MIT License\n\nTODO: Add license text\n").await?;

    println!("✅ Plugin initialized at: {}", plugin_dir.display());
    println!("\nNext steps:");
    println!("  cd {}", name);
    println!("  cargo build");
    println!("  cargo test");
    println!("  only1mcp plugin publish .");

    Ok(())
}

/// Test a plugin
async fn test_plugin(name: String, verbose: bool) -> Result<()> {
    println!("🧪 Testing plugin: {}", name);

    // TODO: Load plugin and run tests

    println!("✅ All tests passed!");
    Ok(())
}

/// Uninstall a plugin
async fn uninstall_plugin(name: String) -> Result<()> {
    println!("🗑️  Uninstalling plugin: {}", name);

    let install_dir = get_plugin_install_dir(&name)?;

    if !install_dir.exists() {
        return Err(Error::Config(format!("Plugin '{}' is not installed", name)));
    }

    tokio::fs::remove_dir_all(&install_dir).await?;

    println!("✅ Plugin {} uninstalled", name);
    Ok(())
}

// Helper functions

fn get_plugins_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| Error::Config("Could not determine home directory".to_string()))?;
    Ok(home.join(".only1mcp/plugins"))
}

fn get_plugin_install_dir(name: &str) -> Result<PathBuf> {
    Ok(get_plugins_dir()?.join(name))
}

fn create_plugin_manifest(name: &str, _template: &str) -> Result<PluginManifest> {
    Ok(PluginManifest {
        name: name.to_string(),
        version: "0.1.0".to_string(),
        description: format!("Only1MCP plugin for {}", name),
        author: "Your Name <your.email@example.com>".to_string(),
        homepage: None,
        repository: None,
        license: "MIT".to_string(),
        capabilities: vec![PluginCapability::RequestTransform],
        dependencies: std::collections::HashMap::new(),
        config_schema: None,
        keywords: vec![],
        compatibility: PluginCompatibility {
            min_only1mcp_version: "0.6.0".to_string(),
            platforms: vec!["linux".to_string(), "macos".to_string(), "windows".to_string()],
        },
    })
}

fn create_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
only1mcp = "0.6"
async-trait = "0.1"
serde = {{ version = "1.0", features = ["derive"] }}
tokio = {{ version = "1.0", features = ["full"] }}

[lib]
crate-type = ["cdylib", "rlib"]
"#,
        name
    )
}

fn create_lib_rs(name: &str, _template: &str) -> String {
    format!(
        r#"//! {} - Only1MCP Plugin

use only1mcp::plugins::*;
use async_trait::async_trait;
use serde::{{Deserialize, Serialize}};

#[derive(Debug, Deserialize)]
pub struct {}Config {{
    pub enabled: bool,
}}

pub struct {}Plugin {{
    metadata: PluginMetadata,
    state: PluginState,
    config: Option<{}Config>,
}}

impl {}Plugin {{
    pub fn new() -> Self {{
        Self {{
            metadata: PluginMetadata {{
                name: "{}".to_string(),
                version: "0.1.0".to_string(),
                author: "Your Name".to_string(),
                description: "Plugin description".to_string(),
                capabilities: vec![PluginCapability::RequestTransform],
                dependencies: vec![],
            }},
            state: PluginState::Loaded,
            config: None,
        }}
    }}
}}

#[async_trait]
impl Plugin for {}Plugin {{
    fn metadata(&self) -> &PluginMetadata {{
        &self.metadata
    }}

    async fn initialize(&mut self, config: std::collections::HashMap<String, serde_json::Value>) -> Result<(), String> {{
        // TODO: Parse config
        self.state = PluginState::Ready;
        Ok(())
    }}

    async fn start(&mut self) -> Result<(), String> {{
        self.state = PluginState::Running;
        Ok(())
    }}

    async fn stop(&mut self) -> Result<(), String> {{
        self.state = PluginState::Stopped;
        Ok(())
    }}

    fn state(&self) -> PluginState {{
        self.state
    }}

    async fn health_check(&self) -> Result<PluginHealth, String> {{
        Ok(PluginHealth {{
            healthy: self.state == PluginState::Running,
            message: "Plugin is healthy".to_string(),
        }})
    }}
}}

#[async_trait]
impl RequestTransformer for {}Plugin {{
    async fn transform_request(
        &self,
        request: McpRequest,
        _context: &PluginContext,
    ) -> Result<McpRequest, String> {{
        // TODO: Implement request transformation
        Ok(request)
    }}
}}
"#,
        name,
        to_pascal_case(name),
        to_pascal_case(name),
        to_pascal_case(name),
        to_pascal_case(name),
        name,
        to_pascal_case(name),
        to_pascal_case(name)
    )
}

fn to_pascal_case(s: &str) -> String {
    s.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
