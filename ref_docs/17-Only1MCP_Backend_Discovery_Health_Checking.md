# Only1MCP Backend Server Discovery & Health Checking
## Zero-Config Service Discovery and Intelligent Health Monitoring

**Document Version:** 1.0  
**Component Focus:** Service Discovery, Health Checks, Circuit Breakers, Failure Detection  
**Performance Target:** <100ms Discovery, <5s Health Detection, 99.9% Accuracy  
**Date:** November 2024  
**Status:** Production Implementation Specification

---

## TABLE OF CONTENTS

1. [Executive Summary](#executive-summary)
2. [Service Discovery Architecture](#service-discovery-architecture)
3. [mDNS Zero-Config Discovery](#mdns-zero-config-discovery)
4. [Consul Production Integration](#consul-production-integration)
5. [Health Check Protocols](#health-check-protocols)
6. [Circuit Breaker Implementation](#circuit-breaker-implementation)
7. [Failure Detection Algorithms](#failure-detection-algorithms)
8. [Health-Aware Load Balancing](#health-aware-load-balancing)
9. [Recovery & Remediation](#recovery--remediation)
10. [Monitoring & Alerting](#monitoring--alerting)
11. [Production Deployment](#production-deployment)
12. [Performance Optimization](#performance-optimization)
13. [Testing Strategies](#testing-strategies)
14. [Implementation Checklist](#implementation-checklist)

---

## EXECUTIVE SUMMARY

### The Discovery & Health Challenge

MCP server environments face critical operational challenges:
- **Manual Configuration**: Users spend 30+ minutes editing JSON for each server
- **Silent Failures**: Servers die without notification, causing request failures
- **No Auto-Discovery**: New servers require manual configuration updates
- **Health Check Overhead**: Aggressive checking impacts performance
- **Cascading Failures**: One bad server can overwhelm others

### Only1MCP Solution

**Service Discovery Stack:**
- **Development**: mDNS/Bonjour for zero-config local discovery
- **Production**: Consul integration with DNS interface
- **Hybrid Mode**: Manual + automatic discovery combined

**Health Monitoring:**
- **Active Checks**: 10s intervals with exponential backoff
- **Passive Monitoring**: Circuit breakers on request failures
- **Composite Health**: Latency + error rate + resource usage

**Quantitative Impact:**
- **Discovery Time**: Manual 30min → Automatic <100ms
- **Failure Detection**: 30-60s → <5s with circuit breakers
- **False Positives**: <1% with adaptive thresholds
- **Recovery Time**: 5min manual → 10s automatic

---

## SERVICE DISCOVERY ARCHITECTURE

### Multi-Layer Discovery Strategy

```rust
//! Service discovery orchestrates multiple discovery mechanisms
//! to automatically find and register MCP servers across environments.

use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// Core trait for discovery providers
#[async_trait]
pub trait ServiceDiscovery: Send + Sync {
    /// Discover available services
    async fn discover(&self) -> Result<Vec<ServiceEndpoint>, DiscoveryError>;
    
    /// Register a service for discovery
    async fn register(&self, service: ServiceEndpoint) -> Result<(), DiscoveryError>;
    
    /// Deregister a service
    async fn deregister(&self, id: &str) -> Result<(), DiscoveryError>;
    
    /// Watch for service changes
    async fn watch(&self) -> Result<ServiceWatcher, DiscoveryError>;
}

/// Discovered service endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// Unique service identifier
    pub id: String,
    
    /// Service name (e.g., "filesystem-mcp")
    pub name: String,
    
    /// Service version
    pub version: String,
    
    /// Network address
    pub address: String,
    
    /// Port number
    pub port: u16,
    
    /// Transport type (stdio, http, sse)
    pub transport: TransportType,
    
    /// Service metadata
    pub metadata: HashMap<String, String>,
    
    /// Health check endpoint
    pub health_endpoint: Option<String>,
    
    /// Discovery timestamp
    pub discovered_at: SystemTime,
}

/// Multi-provider discovery orchestrator
pub struct DiscoveryOrchestrator {
    /// Active discovery providers
    providers: Vec<Arc<dyn ServiceDiscovery>>,
    
    /// Discovered services cache
    services: Arc<RwLock<HashMap<String, ServiceEndpoint>>>,
    
    /// Configuration
    config: DiscoveryConfig,
    
    /// Service registry integration
    registry: Arc<ServerRegistry>,
    
    /// Metrics collector
    metrics: Arc<DiscoveryMetrics>,
}

impl DiscoveryOrchestrator {
    /// Initialize discovery with configured providers
    pub async fn new(config: DiscoveryConfig) -> Result<Self, Error> {
        let mut providers: Vec<Arc<dyn ServiceDiscovery>> = Vec::new();
        
        // Add mDNS provider for local discovery
        if config.mdns.enabled {
            providers.push(Arc::new(
                MdnsDiscovery::new(config.mdns.clone()).await?
            ));
        }
        
        // Add Consul provider for production
        if config.consul.enabled {
            providers.push(Arc::new(
                ConsulDiscovery::new(config.consul.clone()).await?
            ));
        }
        
        // Add static provider for manual entries
        if !config.static_services.is_empty() {
            providers.push(Arc::new(
                StaticDiscovery::new(config.static_services.clone())
            ));
        }
        
        Ok(Self {
            providers,
            services: Arc::new(RwLock::new(HashMap::new())),
            config,
            registry: Arc::new(ServerRegistry::new()),
            metrics: Arc::new(DiscoveryMetrics::new()),
        })
    }
    
    /// Start continuous discovery process
    pub async fn start_discovery(self: Arc<Self>) -> Result<(), Error> {
        // Initial discovery sweep
        self.discover_all().await?;
        
        // Start watchers for each provider
        for provider in &self.providers {
            let orchestrator = self.clone();
            let provider = provider.clone();
            
            tokio::spawn(async move {
                orchestrator.watch_provider(provider).await;
            });
        }
        
        // Periodic refresh
        let orchestrator = self.clone();
        tokio::spawn(async move {
            orchestrator.periodic_refresh().await;
        });
        
        Ok(())
    }
    
    /// Discover services from all providers
    async fn discover_all(&self) -> Result<(), Error> {
        let mut all_services = Vec::new();
        
        // Query each provider in parallel
        let futures: Vec<_> = self.providers
            .iter()
            .map(|p| p.discover())
            .collect();
        
        let results = futures::future::join_all(futures).await;
        
        for (provider_idx, result) in results.into_iter().enumerate() {
            match result {
                Ok(services) => {
                    tracing::info!(
                        "Provider {} discovered {} services",
                        provider_idx,
                        services.len()
                    );
                    all_services.extend(services);
                    self.metrics.discoveries.inc();
                }
                Err(e) => {
                    tracing::warn!(
                        "Discovery provider {} failed: {:?}",
                        provider_idx,
                        e
                    );
                    self.metrics.discovery_errors.inc();
                }
            }
        }
        
        // Deduplicate and merge services
        let merged = self.merge_services(all_services).await?;
        
        // Update registry
        self.update_registry(merged).await?;
        
        Ok(())
    }
}
```

---

## MDNS ZERO-CONFIG DISCOVERY

### Local Network Service Advertisement

```rust
//! mDNS (Multicast DNS) provides zero-configuration service discovery
//! for local network environments, perfect for development.

use mdns_sd::{ServiceDaemon, ServiceInfo, ServiceEvent};
use std::net::IpAddr;
use tokio::sync::mpsc;

/// mDNS service discovery implementation
pub struct MdnsDiscovery {
    /// mDNS daemon
    daemon: ServiceDaemon,
    
    /// Service type for MCP servers
    service_type: String,
    
    /// Configuration
    config: MdnsConfig,
    
    /// Discovered services
    services: Arc<DashMap<String, ServiceEndpoint>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MdnsConfig {
    /// Service type (e.g., "_mcp._tcp")
    pub service_type: String,
    
    /// Domain (usually "local")
    pub domain: String,
    
    /// Network interface to bind
    pub interface: Option<String>,
    
    /// Discovery timeout
    pub timeout_ms: u64,
    
    /// Enable IPv6
    pub enable_ipv6: bool,
}

impl MdnsDiscovery {
    /// Create new mDNS discovery instance
    pub async fn new(config: MdnsConfig) -> Result<Self, Error> {
        let daemon = ServiceDaemon::new()?;
        
        Ok(Self {
            daemon,
            service_type: format!("{}.{}", config.service_type, config.domain),
            config,
            services: Arc::new(DashMap::new()),
        })
    }
    
    /// Register Only1MCP as discoverable service
    pub async fn register_self(&self, port: u16) -> Result<(), Error> {
        let service_info = ServiceInfo::new(
            &self.service_type,
            "only1mcp",
            "local.",
            "",
            port,
            Some(HashMap::from([
                ("version", env!("CARGO_PKG_VERSION")),
                ("transport", "http"),
                ("api_version", "1.0"),
            ])),
        )?;
        
        self.daemon.register(service_info)?;
        
        tracing::info!(
            "Registered mDNS service: {}:{}",
            self.service_type,
            port
        );
        
        Ok(())
    }
    
    /// Browse for MCP services on the network
    pub async fn browse_services(&self) -> Result<Vec<ServiceEndpoint>, Error> {
        let (tx, mut rx) = mpsc::channel(100);
        let service_type = self.service_type.clone();
        let services = self.services.clone();
        
        // Start browsing
        let browse_handle = self.daemon.browse(&service_type)?;
        
        // Process events
        tokio::spawn(async move {
            while let Ok(event) = browse_handle.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let endpoint = Self::info_to_endpoint(info);
                        services.insert(endpoint.id.clone(), endpoint.clone());
                        let _ = tx.send(endpoint).await;
                    }
                    ServiceEvent::ServiceRemoved(name, _) => {
                        services.remove(&name);
                    }
                    _ => {}
                }
            }
        });
        
        // Collect services for timeout duration
        let mut discovered = Vec::new();
        let timeout = Duration::from_millis(self.config.timeout_ms);
        let deadline = tokio::time::Instant::now() + timeout;
        
        loop {
            tokio::select! {
                Some(service) = rx.recv() => {
                    discovered.push(service);
                }
                _ = tokio::time::sleep_until(deadline) => {
                    break;
                }
            }
        }
        
        tracing::info!(
            "mDNS discovery found {} services",
            discovered.len()
        );
        
        Ok(discovered)
    }
    
    /// Convert mDNS ServiceInfo to ServiceEndpoint
    fn info_to_endpoint(info: ServiceInfo) -> ServiceEndpoint {
        let metadata: HashMap<String, String> = info
            .get_properties()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        
        ServiceEndpoint {
            id: info.get_fullname().to_string(),
            name: info.get_instance_name().to_string(),
            version: metadata.get("version")
                .unwrap_or(&"unknown".to_string())
                .clone(),
            address: info.get_addresses()
                .iter()
                .next()
                .map(|a| a.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            port: info.get_port(),
            transport: TransportType::from_str(
                metadata.get("transport").unwrap_or(&"http")
            ).unwrap_or(TransportType::Http),
            metadata,
            health_endpoint: metadata.get("health_endpoint").cloned(),
            discovered_at: SystemTime::now(),
        }
    }
}

#[async_trait]
impl ServiceDiscovery for MdnsDiscovery {
    async fn discover(&self) -> Result<Vec<ServiceEndpoint>, DiscoveryError> {
        self.browse_services().await
            .map_err(|e| DiscoveryError::Provider(e.to_string()))
    }
    
    async fn register(&self, service: ServiceEndpoint) -> Result<(), DiscoveryError> {
        // mDNS doesn't support external registration
        Err(DiscoveryError::NotSupported("mDNS auto-registers only"))
    }
    
    async fn deregister(&self, _id: &str) -> Result<(), DiscoveryError> {
        // Services auto-deregister when they disappear
        Ok(())
    }
    
    async fn watch(&self) -> Result<ServiceWatcher, DiscoveryError> {
        // Return continuous watcher
        Ok(ServiceWatcher::new(self.services.clone()))
    }
}
```

---

## CONSUL PRODUCTION INTEGRATION

### Enterprise Service Discovery

```rust
//! Consul provides production-grade service discovery with
//! health checking, multi-datacenter support, and DNS interface.

use consul_api_client::{Client as ConsulClient, Service, Health, Check};

/// Consul service discovery implementation
pub struct ConsulDiscovery {
    /// Consul API client
    client: ConsulClient,
    
    /// Configuration
    config: ConsulConfig,
    
    /// Service cache
    cache: Arc<DashMap<String, ServiceEndpoint>>,
    
    /// Health check manager
    health_manager: Arc<HealthCheckManager>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConsulConfig {
    /// Consul server URL
    pub url: String,
    
    /// Datacenter name
    pub datacenter: Option<String>,
    
    /// Service tag filter
    pub tags: Vec<String>,
    
    /// ACL token
    pub token: Option<String>,
    
    /// Health check interval
    pub check_interval: Duration,
    
    /// Deregister critical after
    pub deregister_after: Duration,
    
    /// Enable passing checks only
    pub passing_only: bool,
}

impl ConsulDiscovery {
    /// Initialize Consul discovery client
    pub async fn new(config: ConsulConfig) -> Result<Self, Error> {
        let mut client_config = consul_api_client::Config::new();
        client_config.address = config.url.clone();
        
        if let Some(token) = &config.token {
            client_config.token = Some(token.clone());
        }
        
        if let Some(dc) = &config.datacenter {
            client_config.datacenter = Some(dc.clone());
        }
        
        let client = ConsulClient::new(client_config)?;
        let health_manager = Arc::new(HealthCheckManager::new());
        
        Ok(Self {
            client,
            config,
            cache: Arc::new(DashMap::new()),
            health_manager,
        })
    }
    
    /// Register Only1MCP with Consul
    pub async fn register_service(&self, port: u16) -> Result<(), Error> {
        let service = Service {
            id: Some("only1mcp-proxy".to_string()),
            name: "only1mcp".to_string(),
            tags: Some(vec![
                "mcp-proxy".to_string(),
                "version-1.0".to_string(),
                format!("transport-{}", "http"),
            ]),
            address: Some(self.get_local_ip().await?),
            port: Some(port),
            meta: Some(HashMap::from([
                ("version".to_string(), env!("CARGO_PKG_VERSION").to_string()),
                ("capabilities".to_string(), "aggregation,caching,auth".to_string()),
            ])),
            check: Some(Check::http(
                format!("http://localhost:{}/health", port),
                self.config.check_interval,
            )),
            ..Default::default()
        };
        
        self.client.agent.service_register(service).await?;
        
        // Enable maintenance mode if needed
        if std::env::var("MAINTENANCE_MODE").is_ok() {
            self.client.agent
                .enable_service_maintenance("only1mcp-proxy", "Maintenance mode")
                .await?;
        }
        
        tracing::info!("Registered with Consul as only1mcp-proxy");
        
        Ok(())
    }
    
    /// Discover MCP services from Consul
    pub async fn discover_services(&self) -> Result<Vec<ServiceEndpoint>, Error> {
        // Query all services with MCP tag
        let services = self.client
            .catalog
            .services(Some(&["mcp"]))
            .await?;
        
        let mut endpoints = Vec::new();
        
        for (service_name, tags) in services.iter() {
            // Skip our own service
            if service_name == "only1mcp" {
                continue;
            }
            
            // Get service instances
            let instances = self.client
                .health
                .service(
                    service_name,
                    Some(&self.config.tags),
                    self.config.passing_only,
                    None,
                )
                .await?;
            
            for instance in instances {
                if let Some(service) = instance.service {
                    let endpoint = self.consul_to_endpoint(service, instance.checks);
                    endpoints.push(endpoint);
                }
            }
        }
        
        // Update cache
        for endpoint in &endpoints {
            self.cache.insert(endpoint.id.clone(), endpoint.clone());
        }
        
        tracing::info!(
            "Consul discovery found {} healthy services",
            endpoints.len()
        );
        
        Ok(endpoints)
    }
    
    /// Convert Consul service to ServiceEndpoint
    fn consul_to_endpoint(
        &self,
        service: consul_api_client::Service,
        checks: Vec<consul_api_client::Check>,
    ) -> ServiceEndpoint {
        let health_status = self.aggregate_health_status(&checks);
        
        ServiceEndpoint {
            id: service.id.unwrap_or_else(|| service.name.clone()),
            name: service.name,
            version: service.meta
                .as_ref()
                .and_then(|m| m.get("version"))
                .cloned()
                .unwrap_or_else(|| "unknown".to_string()),
            address: service.address.unwrap_or_else(|| "localhost".to_string()),
            port: service.port.unwrap_or(0),
            transport: self.parse_transport(&service.tags),
            metadata: service.meta.unwrap_or_default(),
            health_endpoint: Some(format!(
                "http://{}:{}/health",
                service.address.as_ref().unwrap_or(&"localhost".to_string()),
                service.port.unwrap_or(0)
            )),
            discovered_at: SystemTime::now(),
        }
    }
    
    /// Watch for service changes using blocking queries
    pub async fn watch_services(self: Arc<Self>) -> Result<(), Error> {
        let mut index = 0u64;
        
        loop {
            // Long-polling query with index
            let result = self.client
                .health
                .service_with_index(
                    "mcp",
                    Some(&self.config.tags),
                    self.config.passing_only,
                    Some(index),
                    Some(Duration::from_secs(30)), // Block for 30s
                )
                .await;
            
            match result {
                Ok((services, new_index)) => {
                    if new_index > index {
                        // Services changed
                        tracing::debug!("Consul watch detected changes");
                        self.process_service_changes(services).await?;
                        index = new_index;
                    }
                }
                Err(e) => {
                    tracing::warn!("Consul watch error: {:?}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }
}
```

---

## HEALTH CHECK PROTOCOLS

### Hybrid Active/Passive Health Monitoring

```rust
//! Health checking combines active probes with passive request monitoring
//! to quickly detect failures while minimizing overhead.

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

/// Comprehensive health check manager
pub struct HealthCheckManager {
    /// Active health checkers per backend
    checkers: Arc<DashMap<String, HealthChecker>>,
    
    /// Health status cache
    status_cache: Arc<DashMap<String, HealthStatus>>,
    
    /// Configuration
    config: HealthConfig,
    
    /// Metrics
    metrics: Arc<HealthMetrics>,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    /// Overall health state
    pub state: HealthState,
    
    /// Last successful check
    pub last_success: Instant,
    
    /// Last failed check  
    pub last_failure: Option<Instant>,
    
    /// Consecutive successes
    pub success_count: u32,
    
    /// Consecutive failures
    pub failure_count: u32,
    
    /// Average latency (ms)
    pub avg_latency: f64,
    
    /// Error rate (0.0 - 1.0)
    pub error_rate: f64,
    
    /// Resource usage
    pub resources: ResourceMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthState {
    /// Service is healthy and accepting requests
    Healthy,
    
    /// Service is degraded but operational
    Degraded,
    
    /// Service is unhealthy and not accepting requests
    Unhealthy,
    
    /// Health status unknown (not checked yet)
    Unknown,
}

/// Individual backend health checker
pub struct HealthChecker {
    /// Backend identifier
    backend_id: String,
    
    /// Health check endpoint
    endpoint: String,
    
    /// Check interval
    interval: Duration,
    
    /// Check timeout
    timeout: Duration,
    
    /// Failure threshold (fall)
    failure_threshold: u32,
    
    /// Success threshold (rise)
    success_threshold: u32,
    
    /// Current status
    status: Arc<RwLock<HealthStatus>>,
    
    /// HTTP client for checks
    http_client: reqwest::Client,
    
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
}

impl HealthChecker {
    /// Start continuous health checking
    pub async fn start(self: Arc<Self>) {
        let mut interval = tokio::time::interval(self.interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        
        while !self.shutdown.load(Ordering::Relaxed) {
            interval.tick().await;
            
            // Perform health check
            let result = self.perform_check().await;
            
            // Update status
            self.update_status(result).await;
        }
    }
    
    /// Perform single health check
    async fn perform_check(&self) -> HealthCheckResult {
        let start = Instant::now();
        
        // Build health check request
        let request = if self.endpoint.contains("/mcp/health") {
            // MCP-specific health check
            self.http_client
                .post(&self.endpoint)
                .json(&json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "health/check",
                    "params": {}
                }))
                .timeout(self.timeout)
        } else {
            // Generic HTTP health check
            self.http_client
                .get(&self.endpoint)
                .timeout(self.timeout)
        };
        
        match request.send().await {
            Ok(response) => {
                let latency = start.elapsed();
                
                if response.status().is_success() {
                    // Parse health response if available
                    if let Ok(body) = response.json::<HealthResponse>().await {
                        HealthCheckResult::Success {
                            latency,
                            details: Some(body),
                        }
                    } else {
                        HealthCheckResult::Success {
                            latency,
                            details: None,
                        }
                    }
                } else {
                    HealthCheckResult::Failure {
                        reason: format!("HTTP {}", response.status()),
                        latency: Some(latency),
                    }
                }
            }
            Err(e) => {
                let latency = start.elapsed();
                
                HealthCheckResult::Failure {
                    reason: if e.is_timeout() {
                        format!("Timeout after {:?}", self.timeout)
                    } else if e.is_connect() {
                        "Connection refused".to_string()
                    } else {
                        e.to_string()
                    },
                    latency: Some(latency),
                }
            }
        }
    }
    
    /// Update health status based on check result
    async fn update_status(&self, result: HealthCheckResult) {
        let mut status = self.status.write().await;
        
        match result {
            HealthCheckResult::Success { latency, details } => {
                status.last_success = Instant::now();
                status.success_count += 1;
                status.failure_count = 0;
                
                // Update latency moving average
                status.avg_latency = (status.avg_latency * 0.9) + 
                    (latency.as_millis() as f64 * 0.1);
                
                // Update error rate (exponential decay)
                status.error_rate *= 0.95;
                
                // Parse resource metrics if available
                if let Some(details) = details {
                    status.resources = details.resources;
                }
                
                // Determine state based on thresholds
                if status.success_count >= self.success_threshold {
                    status.state = HealthState::Healthy;
                }
            }
            HealthCheckResult::Failure { reason, latency } => {
                status.last_failure = Some(Instant::now());
                status.failure_count += 1;
                status.success_count = 0;
                
                // Update error rate
                status.error_rate = (status.error_rate * 0.9) + 0.1;
                
                // Log failure reason
                tracing::warn!(
                    "Health check failed for {}: {}",
                    self.backend_id,
                    reason
                );
                
                // Determine state based on thresholds
                if status.failure_count >= self.failure_threshold {
                    status.state = HealthState::Unhealthy;
                } else if status.failure_count > 0 {
                    status.state = HealthState::Degraded;
                }
            }
        }
        
        // Emit metrics
        self.emit_metrics(&status);
    }
}

/// Passive health monitoring through request analysis
pub struct PassiveHealthMonitor {
    /// Request success/failure tracking
    request_stats: Arc<DashMap<String, RequestStats>>,
    
    /// Configuration
    config: PassiveHealthConfig,
    
    /// Circuit breaker manager
    circuit_breakers: Arc<CircuitBreakerManager>,
}

impl PassiveHealthMonitor {
    /// Record request outcome for passive monitoring
    pub async fn record_request(
        &self,
        backend_id: &str,
        success: bool,
        latency: Duration,
    ) {
        let mut stats = self.request_stats
            .entry(backend_id.to_string())
            .or_insert_with(RequestStats::new);
        
        stats.record(success, latency);
        
        // Check if circuit breaker should trip
        if stats.should_trip_circuit_breaker(&self.config) {
            self.circuit_breakers
                .trip(backend_id)
                .await;
            
            tracing::warn!(
                "Circuit breaker tripped for {} (error_rate: {:.2}%, latency: {:?})",
                backend_id,
                stats.error_rate() * 100.0,
                stats.p99_latency()
            );
        }
    }
    
    /// Get current passive health assessment
    pub async fn assess_health(&self, backend_id: &str) -> HealthState {
        if let Some(stats) = self.request_stats.get(backend_id) {
            if stats.error_rate() > self.config.unhealthy_threshold {
                HealthState::Unhealthy
            } else if stats.error_rate() > self.config.degraded_threshold {
                HealthState::Degraded
            } else {
                HealthState::Healthy
            }
        } else {
            HealthState::Unknown
        }
    }
}
```

---

## CIRCUIT BREAKER IMPLEMENTATION

### Failure Isolation and Recovery

```rust
//! Circuit breakers prevent cascading failures by temporarily
//! disabling requests to failing backends.

use std::sync::atomic::{AtomicU32, AtomicI64};

/// Circuit breaker state machine
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    /// Normal operation, requests allowed
    Closed,
    
    /// Failing, requests blocked
    Open,
    
    /// Testing recovery, limited requests
    HalfOpen,
}

/// Circuit breaker for individual backend
pub struct CircuitBreaker {
    /// Backend identifier
    backend_id: String,
    
    /// Current state
    state: Arc<RwLock<CircuitState>>,
    
    /// Failure counter
    failure_count: Arc<AtomicU32>,
    
    /// Success counter (for half-open)
    success_count: Arc<AtomicU32>,
    
    /// Last state change timestamp
    last_state_change: Arc<AtomicI64>,
    
    /// Configuration
    config: CircuitBreakerConfig,
    
    /// State change listeners
    listeners: Arc<RwLock<Vec<StateChangeListener>>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failures needed to open circuit
    pub failure_threshold: u32,
    
    /// Successes needed to close circuit
    pub success_threshold: u32,
    
    /// Time before attempting recovery
    pub timeout: Duration,
    
    /// Error rate threshold (0.0 - 1.0)
    pub error_rate_threshold: f64,
    
    /// Half-open test request limit
    pub half_open_limit: u32,
    
    /// Exponential backoff multiplier
    pub backoff_multiplier: f64,
    
    /// Maximum backoff duration
    pub max_backoff: Duration,
}

impl CircuitBreaker {
    /// Create new circuit breaker
    pub fn new(backend_id: String, config: CircuitBreakerConfig) -> Self {
        Self {
            backend_id,
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(AtomicU32::new(0)),
            success_count: Arc::new(AtomicU32::new(0)),
            last_state_change: Arc::new(AtomicI64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            )),
            config,
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Check if request should be allowed
    pub async fn should_allow_request(&self) -> bool {
        let state = self.state.read().await;
        
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout expired
                let last_change = self.last_state_change.load(Ordering::Relaxed);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                
                if now - last_change > self.config.timeout.as_secs() as i64 {
                    // Transition to half-open
                    drop(state);
                    self.transition_to_half_open().await;
                    true
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests for testing
                let success = self.success_count.load(Ordering::Relaxed);
                let failure = self.failure_count.load(Ordering::Relaxed);
                
                (success + failure) < self.config.half_open_limit
            }
        }
    }
    
    /// Record successful request
    pub async fn record_success(&self) {
        let state = self.state.read().await.clone();
        
        match state {
            CircuitState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            }
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if count >= self.config.success_threshold {
                    drop(state);
                    self.transition_to_closed().await;
                }
            }
            CircuitState::Open => {
                // Shouldn't happen, but reset if it does
                tracing::warn!(
                    "Success recorded in open state for {}",
                    self.backend_id
                );
            }
        }
    }
    
    /// Record failed request
    pub async fn record_failure(&self) {
        let state = self.state.read().await.clone();
        
        match state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if count >= self.config.failure_threshold {
                    drop(state);
                    self.transition_to_open().await;
                }
            }
            CircuitState::HalfOpen => {
                // Single failure in half-open returns to open
                drop(state);
                self.transition_to_open().await;
            }
            CircuitState::Open => {
                // Already open, update failure count for metrics
                self.failure_count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    /// Transition to open state
    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Open;
        
        self.update_timestamp();
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        
        tracing::error!(
            "Circuit breaker OPEN for backend {}",
            self.backend_id
        );
        
        // Notify listeners
        self.notify_listeners(CircuitState::Open).await;
    }
    
    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::HalfOpen;
        
        self.update_timestamp();
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        
        tracing::info!(
            "Circuit breaker HALF-OPEN for backend {} (testing recovery)",
            self.backend_id
        );
        
        // Notify listeners
        self.notify_listeners(CircuitState::HalfOpen).await;
    }
    
    /// Transition to closed state
    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitState::Closed;
        
        self.update_timestamp();
        self.failure_count.store(0, Ordering::Relaxed);
        self.success_count.store(0, Ordering::Relaxed);
        
        tracing::info!(
            "Circuit breaker CLOSED for backend {} (recovered)",
            self.backend_id
        );
        
        // Notify listeners
        self.notify_listeners(CircuitState::Closed).await;
    }
}
```

---

## FAILURE DETECTION ALGORITHMS

### Advanced Failure Detection

```rust
//! Sophisticated failure detection using phi-accrual and
//! adaptive threshold algorithms for accurate detection.

use statrs::distribution::{Normal, ContinuousCDF};

/// Phi Accrual Failure Detector
/// Based on "The φ Accrual Failure Detector" by Hayashibara et al.
pub struct PhiAccrualDetector {
    /// Heartbeat history window
    window_size: usize,
    
    /// Heartbeat intervals
    intervals: VecDeque<Duration>,
    
    /// Last heartbeat timestamp
    last_heartbeat: Arc<RwLock<Instant>>,
    
    /// Phi threshold for failure
    threshold: f64,
    
    /// Statistics
    stats: Arc<RwLock<HeartbeatStats>>,
}

impl PhiAccrualDetector {
    /// Create new Phi Accrual detector
    pub fn new(threshold: f64, window_size: usize) -> Self {
        Self {
            window_size,
            intervals: VecDeque::with_capacity(window_size),
            last_heartbeat: Arc::new(RwLock::new(Instant::now())),
            threshold,
            stats: Arc::new(RwLock::new(HeartbeatStats::default())),
        }
    }
    
    /// Record heartbeat
    pub async fn heartbeat(&mut self) {
        let now = Instant::now();
        let last = *self.last_heartbeat.read().await;
        let interval = now - last;
        
        // Update window
        if self.intervals.len() >= self.window_size {
            self.intervals.pop_front();
        }
        self.intervals.push_back(interval);
        
        // Update statistics
        self.update_stats().await;
        
        // Update last heartbeat
        *self.last_heartbeat.write().await = now;
    }
    
    /// Calculate phi value
    pub async fn phi(&self) -> f64 {
        let now = Instant::now();
        let last = *self.last_heartbeat.read().await;
        let elapsed = now - last;
        
        let stats = self.stats.read().await;
        
        if stats.count < 2 {
            // Not enough data
            return 0.0;
        }
        
        // Calculate probability using normal distribution
        let normal = Normal::new(stats.mean, stats.std_dev).unwrap();
        let p_later = 1.0 - normal.cdf(elapsed.as_secs_f64());
        
        // Convert to phi
        if p_later > 0.0 {
            -p_later.log10()
        } else {
            f64::MAX
        }
    }
    
    /// Check if node should be considered failed
    pub async fn is_failed(&self) -> bool {
        self.phi().await > self.threshold
    }
    
    /// Update statistics from intervals
    async fn update_stats(&self) {
        if self.intervals.is_empty() {
            return;
        }
        
        let values: Vec<f64> = self.intervals
            .iter()
            .map(|d| d.as_secs_f64())
            .collect();
        
        let count = values.len() as f64;
        let mean = values.iter().sum::<f64>() / count;
        
        let variance = values
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / count;
        
        let std_dev = variance.sqrt();
        
        *self.stats.write().await = HeartbeatStats {
            count: count as u64,
            mean,
            std_dev,
            min: values.iter().cloned().fold(f64::INFINITY, f64::min),
            max: values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        };
    }
}

/// Adaptive threshold failure detector
pub struct AdaptiveThresholdDetector {
    /// Response time history
    history: Arc<RwLock<BoundedHistogram>>,
    
    /// Current threshold
    threshold: Arc<RwLock<Duration>>,
    
    /// Configuration
    config: AdaptiveConfig,
}

impl AdaptiveThresholdDetector {
    /// Determine if a response time indicates failure
    pub async fn is_failure(&self, response_time: Duration) -> bool {
        let threshold = *self.threshold.read().await;
        
        if response_time > threshold {
            // Potential failure, but adapt threshold
            self.adapt_threshold(response_time).await;
            true
        } else {
            // Success, update history
            self.history.write().await.record(response_time);
            self.adapt_threshold(response_time).await;
            false
        }
    }
    
    /// Adapt threshold based on recent history
    async fn adapt_threshold(&self, latest: Duration) {
        let history = self.history.read().await;
        
        // Calculate percentile-based threshold
        let p99 = history.percentile(0.99);
        let p95 = history.percentile(0.95);
        
        // Adaptive formula: threshold = p95 + k * (p99 - p95)
        let k = self.config.sensitivity; // 1.0 = normal, >1.0 = more sensitive
        let new_threshold = p95 + 
            Duration::from_secs_f64(k * (p99 - p95).as_secs_f64());
        
        // Apply bounds
        let new_threshold = new_threshold
            .max(self.config.min_threshold)
            .min(self.config.max_threshold);
        
        *self.threshold.write().await = new_threshold;
    }
}
```

---

## HEALTH-AWARE LOAD BALANCING

### Intelligent Request Distribution

```rust
//! Load balancing that considers real-time health metrics
//! for optimal request distribution.

/// Health-aware load balancer
pub struct HealthAwareBalancer {
    /// Base load balancing algorithm
    base_algorithm: LoadBalancerType,
    
    /// Health score calculator
    health_scorer: Arc<HealthScorer>,
    
    /// Backend weights
    weights: Arc<DashMap<String, f64>>,
    
    /// Configuration
    config: HealthAwareConfig,
}

impl HealthAwareBalancer {
    /// Select backend considering health scores
    pub async fn select_backend(
        &self,
        available_backends: &[String],
        request_context: &RequestContext,
    ) -> Result<String, SelectionError> {
        if available_backends.is_empty() {
            return Err(SelectionError::NoBackendsAvailable);
        }
        
        // Calculate weighted scores
        let mut scored_backends = Vec::new();
        
        for backend_id in available_backends {
            let health_score = self.health_scorer
                .calculate_score(backend_id)
                .await?;
            
            let weight = self.weights
                .get(backend_id)
                .map(|w| *w)
                .unwrap_or(1.0);
            
            let final_score = health_score * weight;
            
            scored_backends.push((backend_id.clone(), final_score));
        }
        
        // Sort by score (highest first)
        scored_backends.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // Apply selection strategy
        match self.config.strategy {
            SelectionStrategy::BestScore => {
                // Always pick highest score
                Ok(scored_backends[0].0.clone())
            }
            SelectionStrategy::WeightedRandom => {
                // Weighted random selection
                self.weighted_random_select(&scored_backends)
            }
            SelectionStrategy::PowerOfTwo => {
                // Power of two choices
                self.power_of_two_select(&scored_backends)
            }
        }
    }
    
    /// Power of Two Choices selection
    fn power_of_two_select(
        &self,
        scored_backends: &[(String, f64)],
    ) -> Result<String, SelectionError> {
        let mut rng = rand::thread_rng();
        
        // Pick two random backends
        let idx1 = rng.gen_range(0..scored_backends.len());
        let idx2 = rng.gen_range(0..scored_backends.len());
        
        // Return the one with better score
        if scored_backends[idx1].1 >= scored_backends[idx2].1 {
            Ok(scored_backends[idx1].0.clone())
        } else {
            Ok(scored_backends[idx2].0.clone())
        }
    }
}

/// Health score calculator
pub struct HealthScorer {
    /// Health data sources
    health_manager: Arc<HealthCheckManager>,
    circuit_breakers: Arc<CircuitBreakerManager>,
    passive_monitor: Arc<PassiveHealthMonitor>,
    
    /// Scoring configuration
    config: ScoringConfig,
}

impl HealthScorer {
    /// Calculate composite health score (0.0 - 1.0)
    pub async fn calculate_score(&self, backend_id: &str) -> Result<f64, Error> {
        let mut score = 1.0;
        
        // Factor 1: Active health check status
        if let Some(status) = self.health_manager.get_status(backend_id).await {
            score *= match status.state {
                HealthState::Healthy => 1.0,
                HealthState::Degraded => 0.5,
                HealthState::Unhealthy => 0.0,
                HealthState::Unknown => 0.3,
            };
        }
        
        // Factor 2: Circuit breaker state
        if let Some(breaker) = self.circuit_breakers.get(backend_id).await {
            score *= match breaker.state().await {
                CircuitState::Closed => 1.0,
                CircuitState::HalfOpen => 0.5,
                CircuitState::Open => 0.0,
            };
        }
        
        // Factor 3: Request success rate
        if let Some(stats) = self.passive_monitor.get_stats(backend_id).await {
            score *= 1.0 - stats.error_rate();
        }
        
        // Factor 4: Response time
        if let Some(latency) = self.passive_monitor.get_p50_latency(backend_id).await {
            let latency_factor = 1.0 - (latency.as_millis() as f64 / 
                self.config.max_acceptable_latency_ms);
            score *= latency_factor.max(0.0);
        }
        
        // Factor 5: Resource utilization
        if let Some(resources) = self.health_manager.get_resources(backend_id).await {
            let cpu_factor = 1.0 - (resources.cpu_usage / 100.0);
            let mem_factor = 1.0 - (resources.memory_usage / 100.0);
            score *= (cpu_factor * 0.5 + mem_factor * 0.5);
        }
        
        Ok(score.max(0.0).min(1.0))
    }
}
```

---

## RECOVERY & REMEDIATION

### Automated Recovery Procedures

```rust
//! Automated recovery mechanisms to restore failed services
//! and maintain system stability.

/// Recovery orchestrator
pub struct RecoveryOrchestrator {
    /// Recovery strategies
    strategies: Vec<Box<dyn RecoveryStrategy>>,
    
    /// Recovery history
    history: Arc<DashMap<String, RecoveryHistory>>,
    
    /// Configuration
    config: RecoveryConfig,
}

#[async_trait]
trait RecoveryStrategy: Send + Sync {
    /// Attempt recovery for a failed backend
    async fn attempt_recovery(
        &self,
        backend_id: &str,
        failure_info: &FailureInfo,
    ) -> Result<RecoveryResult, RecoveryError>;
    
    /// Check if strategy is applicable
    fn is_applicable(&self, failure_info: &FailureInfo) -> bool;
}

/// Restart recovery strategy
struct RestartStrategy {
    process_manager: Arc<ProcessManager>,
    max_restarts: u32,
}

#[async_trait]
impl RecoveryStrategy for RestartStrategy {
    async fn attempt_recovery(
        &self,
        backend_id: &str,
        failure_info: &FailureInfo,
    ) -> Result<RecoveryResult, RecoveryError> {
        tracing::info!("Attempting restart recovery for {}", backend_id);
        
        // Check restart limit
        let restart_count = self.process_manager
            .get_restart_count(backend_id)
            .await;
        
        if restart_count >= self.max_restarts {
            return Err(RecoveryError::LimitExceeded);
        }
        
        // Stop the process
        self.process_manager.stop_process(backend_id).await?;
        
        // Wait for cleanup
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Start the process
        self.process_manager.start_process(backend_id).await?;
        
        // Wait for startup
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // Verify health
        let health = self.process_manager
            .check_process_health(backend_id)
            .await?;
        
        if health {
            Ok(RecoveryResult::Success {
                strategy: "restart".to_string(),
                duration: Duration::from_secs(7),
            })
        } else {
            Err(RecoveryError::RecoveryFailed)
        }
    }
    
    fn is_applicable(&self, failure_info: &FailureInfo) -> bool {
        matches!(failure_info.failure_type, 
            FailureType::ProcessCrash | 
            FailureType::Unresponsive
        )
    }
}

/// Connection reset strategy
struct ConnectionResetStrategy {
    connection_pool: Arc<ConnectionPool>,
}

#[async_trait]
impl RecoveryStrategy for ConnectionResetStrategy {
    async fn attempt_recovery(
        &self,
        backend_id: &str,
        _failure_info: &FailureInfo,
    ) -> Result<RecoveryResult, RecoveryError> {
        tracing::info!("Resetting connections for {}", backend_id);
        
        // Close all existing connections
        self.connection_pool
            .close_all_connections(backend_id)
            .await?;
        
        // Create new connections
        self.connection_pool
            .create_connections(backend_id, 5)
            .await?;
        
        Ok(RecoveryResult::Success {
            strategy: "connection_reset".to_string(),
            duration: Duration::from_millis(500),
        })
    }
    
    fn is_applicable(&self, failure_info: &FailureInfo) -> bool {
        matches!(failure_info.failure_type,
            FailureType::ConnectionError |
            FailureType::Timeout
        )
    }
}
```

---

## MONITORING & ALERTING

### Health Metrics and Dashboards

```rust
//! Comprehensive monitoring for service discovery and health checking
//! with Prometheus metrics and alert integration.

/// Health monitoring metrics collector
pub struct HealthMonitoringMetrics {
    /// Discovery metrics
    services_discovered: Counter,
    discovery_duration: Histogram,
    discovery_errors: Counter,
    
    /// Health check metrics
    health_checks_total: CounterVec,
    health_check_duration: HistogramVec,
    health_status: GaugeVec,
    
    /// Circuit breaker metrics
    circuit_breaker_state: GaugeVec,
    circuit_breaker_trips: Counter,
    
    /// Recovery metrics
    recovery_attempts: Counter,
    recovery_success: Counter,
    recovery_duration: Histogram,
}

impl HealthMonitoringMetrics {
    /// Register all metrics with Prometheus
    pub fn register() -> Result<Self, Error> {
        let services_discovered = register_counter!(
            "only1mcp_services_discovered_total",
            "Total number of services discovered"
        )?;
        
        let health_checks_total = register_counter_vec!(
            "only1mcp_health_checks_total",
            "Total health checks performed",
            &["backend", "result"]
        )?;
        
        let health_check_duration = register_histogram_vec!(
            "only1mcp_health_check_duration_seconds",
            "Health check duration in seconds",
            &["backend", "type"],
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
        )?;
        
        let health_status = register_gauge_vec!(
            "only1mcp_health_status",
            "Current health status (0=unknown, 1=healthy, 2=degraded, 3=unhealthy)",
            &["backend"]
        )?;
        
        let circuit_breaker_state = register_gauge_vec!(
            "only1mcp_circuit_breaker_state",
            "Circuit breaker state (0=closed, 1=open, 2=half-open)",
            &["backend"]
        )?;
        
        Ok(Self {
            services_discovered,
            discovery_duration,
            discovery_errors,
            health_checks_total,
            health_check_duration,
            health_status,
            circuit_breaker_state,
            circuit_breaker_trips,
            recovery_attempts,
            recovery_success,
            recovery_duration,
        })
    }
    
    /// Export metrics for Grafana dashboard
    pub fn export_dashboard_config() -> &'static str {
        include_str!("../dashboards/health-monitoring.json")
    }
}

/// Alert manager for health events
pub struct HealthAlertManager {
    /// Alert channels
    channels: Vec<Box<dyn AlertChannel>>,
    
    /// Alert rules
    rules: Vec<AlertRule>,
    
    /// Alert state
    active_alerts: Arc<DashMap<String, Alert>>,
}

impl HealthAlertManager {
    /// Check and fire alerts based on current state
    pub async fn evaluate_alerts(&self, context: &HealthContext) {
        for rule in &self.rules {
            if rule.evaluate(context) {
                self.fire_alert(&rule, context).await;
            } else {
                self.resolve_alert(&rule.id).await;
            }
        }
    }
    
    /// Fire an alert through configured channels
    async fn fire_alert(&self, rule: &AlertRule, context: &HealthContext) {
        let alert = Alert {
            id: rule.id.clone(),
            severity: rule.severity,
            title: rule.title.clone(),
            description: rule.format_description(context),
            timestamp: SystemTime::now(),
            labels: rule.labels.clone(),
        };
        
        // Check if already active
        if self.active_alerts.contains_key(&alert.id) {
            return;
        }
        
        // Store active alert
        self.active_alerts.insert(alert.id.clone(), alert.clone());
        
        // Send through all channels
        for channel in &self.channels {
            if let Err(e) = channel.send_alert(&alert).await {
                tracing::error!(
                    "Failed to send alert through channel: {:?}",
                    e
                );
            }
        }
    }
}
```

---

## PRODUCTION DEPLOYMENT

### Production Configuration Guide

```yaml
# production-health.yaml
# Production health checking and service discovery configuration

service_discovery:
  # Use Consul in production
  consul:
    enabled: true
    url: "http://consul.service.consul:8500"
    datacenter: "us-east-1"
    
    # Service registration
    service_name: "only1mcp"
    service_tags:
      - "mcp-proxy"
      - "production"
      - "version-1.0"
    
    # Health check
    check_interval: 10s
    check_timeout: 5s
    deregister_after: 30s
    
    # ACL token (use environment variable)
    token: "${CONSUL_TOKEN}"
  
  # Disable mDNS in production
  mdns:
    enabled: false
  
  # Static services for critical backends
  static_services:
    - id: "critical-mcp-1"
      name: "Critical MCP Server"
      address: "10.0.1.50"
      port: 9001
      health_endpoint: "/health"

health_checking:
  # Active health checks
  active:
    enabled: true
    interval: 10s
    timeout: 5s
    failure_threshold: 3  # Mark unhealthy after 3 failures
    success_threshold: 2  # Mark healthy after 2 successes
    
    # Jitter to prevent thundering herd
    jitter: 2s
    
    # Exponential backoff for failed backends
    backoff:
      initial: 10s
      max: 300s
      multiplier: 2
  
  # Passive health monitoring
  passive:
    enabled: true
    window: 60s
    
    # Error rate thresholds
    degraded_threshold: 0.1  # 10% errors = degraded
    unhealthy_threshold: 0.5 # 50% errors = unhealthy
    
    # Latency thresholds
    latency_degraded: 500ms
    latency_unhealthy: 2000ms

circuit_breakers:
  enabled: true
  
  # Default configuration
  default:
    failure_threshold: 5
    success_threshold: 3
    timeout: 30s
    half_open_limit: 3
    
  # Per-backend overrides
  backends:
    "critical-mcp-1":
      failure_threshold: 10  # More tolerant for critical
      timeout: 60s

# Recovery settings
recovery:
  enabled: true
  
  strategies:
    - type: "connection_reset"
      max_attempts: 3
      
    - type: "restart"
      max_attempts: 2
      cooldown: 300s
      
    - type: "failover"
      enabled: true

# Alerting
alerting:
  enabled: true
  
  channels:
    - type: "slack"
      webhook: "${SLACK_WEBHOOK_URL}"
      channel: "#only1mcp-alerts"
      
    - type: "pagerduty"
      api_key: "${PAGERDUTY_API_KEY}"
      service_id: "P1234567"
  
  rules:
    - id: "all-backends-down"
      condition: "healthy_backends == 0"
      severity: "critical"
      title: "All MCP backends are down"
      
    - id: "high-error-rate"
      condition: "error_rate > 0.25"
      severity: "warning"
      title: "High error rate detected"
      
    - id: "discovery-failing"
      condition: "discovery_errors > 5"
      severity: "warning"
      title: "Service discovery failures"
```

---

## PERFORMANCE OPTIMIZATION

### Health Check Performance Tuning

```rust
//! Performance optimizations for health checking at scale

/// Optimized batch health checker
pub struct BatchHealthChecker {
    /// Connection pool for health checks
    pool: Arc<HealthCheckPool>,
    
    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,
    
    /// Cache for recent results
    cache: Arc<TtlCache<String, HealthResult>>,
}

impl BatchHealthChecker {
    /// Perform health checks in parallel batches
    pub async fn check_all_backends(
        &self,
        backend_ids: Vec<String>,
    ) -> HashMap<String, HealthResult> {
        let chunk_size = 50; // Check 50 backends in parallel
        let mut results = HashMap::new();
        
        for chunk in backend_ids.chunks(chunk_size) {
            let futures: Vec<_> = chunk
                .iter()
                .map(|id| self.check_single(id.clone()))
                .collect();
            
            let chunk_results = futures::future::join_all(futures).await;
            
            for (id, result) in chunk.iter().zip(chunk_results) {
                results.insert(id.to_string(), result);
            }
        }
        
        results
    }
    
    /// Check single backend with caching
    async fn check_single(&self, backend_id: String) -> HealthResult {
        // Check cache first
        if let Some(cached) = self.cache.get(&backend_id).await {
            if !cached.is_expired() {
                return cached.clone();
            }
        }
        
        // Acquire permit for rate limiting
        let _permit = self.semaphore.acquire().await.unwrap();
        
        // Get connection from pool
        let conn = self.pool.get_or_create(&backend_id).await?;
        
        // Perform health check
        let result = conn.check_health().await;
        
        // Cache result
        self.cache.insert(
            backend_id,
            result.clone(),
            Duration::from_secs(5)
        ).await;
        
        result
    }
}
```

---

## TESTING STRATEGIES

### Health Check Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};
    
    #[tokio::test]
    async fn test_health_check_success() {
        let _m = mock("GET", "/health")
            .with_status(200)
            .with_body(r#"{"status": "healthy"}"#)
            .create();
        
        let checker = HealthChecker::new(
            "test-backend".to_string(),
            format!("{}/health", server_url()),
            Duration::from_secs(5),
            Duration::from_secs(1),
            3,
            2,
        );
        
        let result = checker.perform_check().await;
        assert!(matches!(result, HealthCheckResult::Success { .. }));
    }
    
    #[tokio::test]
    async fn test_circuit_breaker_opens() {
        let breaker = CircuitBreaker::new(
            "test".to_string(),
            CircuitBreakerConfig {
                failure_threshold: 3,
                success_threshold: 2,
                timeout: Duration::from_secs(1),
                ..Default::default()
            }
        );
        
        // Record failures
        for _ in 0..3 {
            breaker.record_failure().await;
        }
        
        // Should be open
        assert!(!breaker.should_allow_request().await);
    }
    
    #[tokio::test]
    async fn test_mdns_discovery() {
        let config = MdnsConfig {
            service_type: "_test._tcp".to_string(),
            domain: "local".to_string(),
            timeout_ms: 1000,
            ..Default::default()
        };
        
        let discovery = MdnsDiscovery::new(config).await.unwrap();
        
        // Register test service
        discovery.register_self(8080).await.unwrap();
        
        // Should discover ourselves
        let services = discovery.browse_services().await.unwrap();
        assert!(!services.is_empty());
    }
}
```

---

## IMPLEMENTATION CHECKLIST

### Phase 1: Core Discovery (Week 1)
- [ ] Implement ServiceDiscovery trait
- [ ] Create mDNS discovery provider
- [ ] Build static service provider
- [ ] Integrate with ServerRegistry
- [ ] Add discovery metrics

### Phase 2: Health Checking (Week 2)
- [ ] Implement active health checker
- [ ] Build passive monitoring
- [ ] Create composite health scorer
- [ ] Add health check caching
- [ ] Implement exponential backoff

### Phase 3: Circuit Breakers (Week 3)
- [ ] Implement circuit breaker state machine
- [ ] Add failure detection algorithms
- [ ] Build recovery strategies
- [ ] Integrate with load balancer
- [ ] Add circuit breaker metrics

### Phase 4: Production Features (Week 4)
- [ ] Implement Consul integration
- [ ] Add alert management
- [ ] Build recovery orchestrator
- [ ] Create Grafana dashboards
- [ ] Production testing

### Performance Targets
- Discovery latency: <100ms
- Health check overhead: <5ms
- Circuit breaker decision: <1ms
- Recovery time: <10s
- False positive rate: <1%

### Testing Requirements
- Unit tests: 100% coverage for health logic
- Integration tests with mock services
- Chaos testing for failure scenarios
- Load testing with 100+ backends
- Network partition testing

---

## APPENDIX: PRODUCTION PATTERNS

### Pattern: Canary Health Checks

```rust
/// Use canary endpoints for gradual rollout validation
pub async fn canary_health_check(
    backend_id: &str,
    canary_percentage: f64,
) -> HealthResult {
    let mut rng = rand::thread_rng();
    
    if rng.gen::<f64>() < canary_percentage {
        // Check canary endpoint
        check_endpoint(&format!("{}/canary/health", backend_id)).await
    } else {
        // Check stable endpoint
        check_endpoint(&format!("{}/health", backend_id)).await
    }
}
```

### Pattern: Graceful Degradation

```rust
/// Degrade functionality instead of failing completely
pub async fn handle_degraded_health(
    backend_id: &str,
    health_state: HealthState,
) -> DegradationStrategy {
    match health_state {
        HealthState::Healthy => DegradationStrategy::None,
        HealthState::Degraded => DegradationStrategy::ReduceTraffic(0.5),
        HealthState::Unhealthy => DegradationStrategy::CircuitBreak,
        HealthState::Unknown => DegradationStrategy::ReduceTraffic(0.1),
    }
}
```

This completes the comprehensive specification for Only1MCP's backend server discovery and health checking system, providing zero-configuration local development with mDNS and production-grade reliability with Consul and sophisticated health monitoring.
