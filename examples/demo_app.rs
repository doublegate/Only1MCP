#!/usr/bin/env cargo
//! Demo Application for Only1MCP
//!
//! This application demonstrates the key features of Only1MCP including:
//! - GUI backend integration
//! - Real-time metrics tracking
//! - Server management
//! - Event logging
//!
//! Run with: cargo run --example demo_app

use chrono::Utc;
use only1mcp::gui_simple::{EventSeverity, GuiBackend, ServerInfo, SystemEvent};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() {
    println!("🚀 Only1MCP Demo Application");
    println!("============================\n");

    // Create GUI backend
    let backend = GuiBackend::new();
    println!("✓ Created GUI backend");

    // Simulate starting the proxy
    backend.set_running(true).await;
    backend
        .log_event(SystemEvent {
            timestamp: Utc::now(),
            event_type: "proxy_started".to_string(),
            message: "Proxy server started successfully".to_string(),
            severity: EventSeverity::Info,
        })
        .await;
    println!("✓ Started proxy server");

    // Register some MCP servers
    println!("\n📡 Registering MCP servers...");

    backend
        .add_server(ServerInfo {
            id: "filesystem-1".to_string(),
            name: "Filesystem MCP Server".to_string(),
            transport_type: "stdio".to_string(),
            healthy: true,
            enabled: true,
        })
        .await;
    println!("  ✓ Registered: Filesystem MCP Server");

    backend
        .add_server(ServerInfo {
            id: "github-1".to_string(),
            name: "GitHub MCP Server".to_string(),
            transport_type: "http".to_string(),
            healthy: true,
            enabled: true,
        })
        .await;
    println!("  ✓ Registered: GitHub MCP Server");

    backend
        .add_server(ServerInfo {
            id: "brave-search-1".to_string(),
            name: "Brave Search MCP Server".to_string(),
            transport_type: "sse".to_string(),
            healthy: true,
            enabled: true,
        })
        .await;
    println!("  ✓ Registered: Brave Search MCP Server");

    // Log events
    backend
        .log_event(SystemEvent {
            timestamp: Utc::now(),
            event_type: "server_registered".to_string(),
            message: "All MCP servers registered".to_string(),
            severity: EventSeverity::Info,
        })
        .await;

    // Simulate traffic and metrics
    println!("\n📊 Simulating traffic...");

    for i in 0..10 {
        // Simulate incoming connections
        backend.set_active_connections(i * 5 + 10).await;

        // Record various metrics
        backend
            .record_metric("request_count".to_string(), (i * 100) as f64)
            .await;
        backend
            .record_metric("cpu_usage".to_string(), 50.0 + (i as f64 * 2.5))
            .await;
        backend
            .record_metric("memory_usage".to_string(), 60.0 + (i as f64 * 1.5))
            .await;
        backend
            .record_metric(
                "response_time_ms".to_string(),
                25.0 + (rand::random::<f64>() * 10.0),
            )
            .await;

        // Occasionally simulate server health changes
        if i == 5 {
            backend.update_server_health("github-1", false).await;
            backend
                .log_event(SystemEvent {
                    timestamp: Utc::now(),
                    event_type: "server_unhealthy".to_string(),
                    message: "GitHub MCP Server marked as unhealthy".to_string(),
                    severity: EventSeverity::Warning,
                })
                .await;
            println!("  ⚠️  Server health changed");
        }

        if i == 7 {
            backend.update_server_health("github-1", true).await;
            backend
                .log_event(SystemEvent {
                    timestamp: Utc::now(),
                    event_type: "server_recovered".to_string(),
                    message: "GitHub MCP Server recovered".to_string(),
                    severity: EventSeverity::Info,
                })
                .await;
            println!("  ✓ Server recovered");
        }

        print!("  Progress: [");
        for j in 0..10 {
            if j <= i {
                print!("█");
            } else {
                print!("░");
            }
        }
        println!("] {}%", (i + 1) * 10);

        sleep(Duration::from_millis(500)).await;
    }

    // Display dashboard data
    println!("\n📈 Dashboard Summary");
    println!("====================");

    let dashboard = backend.get_dashboard().await;
    println!("Status: {}", if dashboard.is_running { "🟢 RUNNING" } else { "🔴 STOPPED" });
    println!("Uptime: {} seconds", dashboard.uptime_seconds);
    println!("Active Connections: {}", dashboard.active_connections);
    println!(
        "Servers: {}/{} healthy",
        dashboard.healthy_servers, dashboard.total_servers
    );
    println!("Version: {}", dashboard.version);

    // Display server list
    println!("\n🖥️  Server Status");
    println!("=================");

    let servers = backend.get_servers().await;
    for server in servers {
        let health_icon = if server.healthy { "🟢" } else { "🔴" };
        let enabled_icon = if server.enabled { "✓" } else { "✗" };
        println!(
            "{} {} {} [{}] ({})",
            health_icon, enabled_icon, server.name, server.transport_type, server.id
        );
    }

    // Display metrics
    println!("\n📊 Metrics");
    println!("==========");

    let metric_names = backend.list_metrics().await;
    for name in &metric_names {
        if let Some(points) = backend.get_metric(name).await {
            if let Some(latest) = points.last() {
                println!("{}: {:.2}", name, latest.value);
            }
        }
    }

    // Display recent events
    println!("\n📝 Recent Events");
    println!("================");

    let events = backend.get_events(5).await;
    for event in events {
        let severity_icon = match event.severity {
            EventSeverity::Info => "ℹ️ ",
            EventSeverity::Warning => "⚠️ ",
            EventSeverity::Error => "❌",
            EventSeverity::Critical => "🚨",
        };
        println!(
            "{} [{}] {}",
            severity_icon,
            event.timestamp.format("%H:%M:%S"),
            event.message
        );
    }

    // Display system stats
    println!("\n📋 System Statistics");
    println!("====================");

    let stats = backend.get_stats().await;
    println!("Total Servers: {}", stats.total_servers);
    println!("Healthy Servers: {}", stats.healthy_servers);
    println!("Active Connections: {}", stats.active_connections);
    println!("Total Metrics: {}", stats.total_metrics);
    println!("Total Events: {}", stats.total_events);
    println!("Uptime: {} seconds", stats.uptime_seconds);

    // Simulate shutdown
    println!("\n🛑 Shutting down...");
    backend.set_running(false).await;
    backend
        .log_event(SystemEvent {
            timestamp: Utc::now(),
            event_type: "proxy_stopped".to_string(),
            message: "Proxy server stopped gracefully".to_string(),
            severity: EventSeverity::Info,
        })
        .await;

    println!("✓ Demo completed successfully!");
    println!("\n💡 Next steps:");
    println!("  • Open examples/dashboard.html in your browser");
    println!("  • Try cargo run --example demo_app");
    println!("  • Read examples/GETTING_STARTED.md");
    println!("  • Explore examples/INTEGRATION_GUIDE.md");
}
