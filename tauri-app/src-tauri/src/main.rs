// Only1MCP Tauri Desktop App - Rust Backend

#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use only1mcp::{config::Config, proxy::ProxyServer, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, State, Window};
use tokio::sync::RwLock;

// Application state
struct AppState {
    proxy: Arc<RwLock<Option<ProxyServer>>>,
    config: Arc<RwLock<Option<Config>>>,
}

// Proxy status
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProxyStatus {
    running: bool,
    uptime_seconds: Option<u64>,
    version: String,
    config_path: Option<String>,
}

// Metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsData {
    requests_total: u64,
    requests_per_second: f64,
    avg_latency_ms: f64,
    active_connections: u32,
    cache_hit_rate: f64,
    error_rate: f64,
}

// Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ServerInfo {
    id: String,
    name: String,
    status: String,
    health: String,
    requests: u64,
}

#[tauri::command]
async fn get_status(state: State<'_, AppState>) -> Result<ProxyStatus, String> {
    let proxy_guard = state.proxy.read().await;

    let running = proxy_guard.is_some();
    let config_guard = state.config.read().await;

    Ok(ProxyStatus {
        running,
        uptime_seconds: if running { Some(0) } else { None }, // TODO: Track start time
        version: env!("CARGO_PKG_VERSION").to_string(),
        config_path: config_guard
            .as_ref()
            .and_then(|_| Some("~/.only1mcp/config.yaml".to_string())),
    })
}

#[tauri::command]
async fn start_proxy(
    config_path: String,
    state: State<'_, AppState>,
    window: Window,
) -> Result<(), String> {
    // Load configuration
    let config_pathbuf = PathBuf::from(&config_path);
    let config = Config::from_file(&config_pathbuf).map_err(|e| e.to_string())?;

    // Create proxy server
    let proxy = ProxyServer::new(config.clone()).await.map_err(|e| e.to_string())?;

    // Store in state
    {
        let mut proxy_guard = state.proxy.write().await;
        *proxy_guard = Some(proxy);
    }

    {
        let mut config_guard = state.config.write().await;
        *config_guard = Some(config);
    }

    // Start background task to emit metrics
    let app_handle = window.app_handle();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // TODO: Collect real metrics from proxy
            let metrics = MetricsData {
                requests_total: 0,
                requests_per_second: 0.0,
                avg_latency_ms: 0.0,
                active_connections: 0,
                cache_hit_rate: 0.0,
                error_rate: 0.0,
            };

            let _ = app_handle.emit_all("metrics-update", metrics);
        }
    });

    Ok(())
}

#[tauri::command]
async fn stop_proxy(state: State<'_, AppState>) -> Result<(), String> {
    let mut proxy_guard = state.proxy.write().await;

    if let Some(proxy) = proxy_guard.take() {
        // TODO: Graceful shutdown
        drop(proxy);
    }

    let mut config_guard = state.config.write().await;
    *config_guard = None;

    Ok(())
}

#[tauri::command]
async fn load_config(path: String) -> Result<Config, String> {
    let config_path = PathBuf::from(path);
    Config::from_file(&config_path).map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_config(path: String, config: Config) -> Result<(), String> {
    let config_path = PathBuf::from(path);

    let yaml = serde_yaml::to_string(&config).map_err(|e| e.to_string())?;

    std::fs::write(config_path, yaml).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_servers(state: State<'_, AppState>) -> Result<Vec<ServerInfo>, String> {
    let config_guard = state.config.read().await;

    if let Some(ref config) = *config_guard {
        let servers: Vec<ServerInfo> = config
            .servers
            .iter()
            .map(|server| ServerInfo {
                id: server.id.clone(),
                name: server.name.clone(),
                status: "unknown".to_string(),
                health: "unknown".to_string(),
                requests: 0,
            })
            .collect();

        Ok(servers)
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
async fn get_metrics(state: State<'_, AppState>) -> Result<MetricsData, String> {
    let proxy_guard = state.proxy.read().await;

    if proxy_guard.is_some() {
        // TODO: Get real metrics from proxy
        Ok(MetricsData {
            requests_total: 0,
            requests_per_second: 0.0,
            avg_latency_ms: 0.0,
            active_connections: 0,
            cache_hit_rate: 0.0,
            error_rate: 0.0,
        })
    } else {
        Err("Proxy not running".to_string())
    }
}

#[tauri::command]
async fn validate_config(config_str: String) -> Result<bool, String> {
    let config: Config = serde_yaml::from_str(&config_str).map_err(|e| e.to_string())?;

    // TODO: Validate config more thoroughly
    Ok(!config.servers.is_empty())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            proxy: Arc::new(RwLock::new(None)),
            config: Arc::new(RwLock::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_proxy,
            stop_proxy,
            load_config,
            save_config,
            get_servers,
            get_metrics,
            validate_config
        ])
        .setup(|app| {
            // Set up system tray
            use tauri::{CustomMenuItem, SystemTray, SystemTrayEvent, SystemTrayMenu};

            let start = CustomMenuItem::new("start".to_string(), "Start Proxy");
            let stop = CustomMenuItem::new("stop".to_string(), "Stop Proxy");
            let quit = CustomMenuItem::new("quit".to_string(), "Quit");
            let tray_menu = SystemTrayMenu::new()
                .add_item(start)
                .add_item(stop)
                .add_native_item(tauri::SystemTrayMenuItem::Separator)
                .add_item(quit);

            let tray = SystemTray::new().with_menu(tray_menu);

            app.handle()
                .system_tray()
                .map(|st| {
                    st.on_event(|app, event| match event {
                        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                            "quit" => {
                                std::process::exit(0);
                            }
                            "start" => {
                                // TODO: Trigger start via IPC
                            }
                            "stop" => {
                                // TODO: Trigger stop via IPC
                            }
                            _ => {}
                        },
                        _ => {}
                    })
                })
                .ok();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
