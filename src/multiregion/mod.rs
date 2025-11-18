//! Multi-region deployment support for Only1MCP
//!
//! Provides geographic load balancing, region affinity routing,
//! cross-region replication, and latency-aware routing for global deployments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{Error, Result};

/// Geographic region identifier
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct RegionId(pub String);

impl RegionId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for RegionId {
    fn from(s: String) -> Self {
        RegionId(s)
    }
}

impl From<&str> for RegionId {
    fn from(s: &str) -> Self {
        RegionId(s.to_string())
    }
}

/// Geographic region configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    /// Region identifier
    pub id: RegionId,

    /// Region display name
    pub name: String,

    /// Geographic location
    pub location: GeoLocation,

    /// MCP servers in this region
    pub servers: Vec<String>,

    /// Region health status
    pub healthy: bool,

    /// Average latency to this region (in milliseconds)
    pub avg_latency_ms: f64,

    /// Region capacity (max requests per second)
    pub capacity: u32,

    /// Current load (requests per second)
    pub current_load: u32,

    /// Replication peers (regions to replicate to)
    pub replication_peers: Vec<RegionId>,

    /// Region priority (higher = preferred)
    pub priority: u8,
}

impl Region {
    pub fn new(id: RegionId, name: String, location: GeoLocation) -> Self {
        Self {
            id,
            name,
            location,
            servers: Vec::new(),
            healthy: true,
            avg_latency_ms: 0.0,
            capacity: 1000,
            current_load: 0,
            replication_peers: Vec::new(),
            priority: 100,
        }
    }

    /// Check if region has capacity
    pub fn has_capacity(&self) -> bool {
        self.current_load < self.capacity
    }

    /// Get utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        (self.current_load as f64 / self.capacity as f64) * 100.0
    }
}

/// Geographic location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Latitude
    pub latitude: f64,

    /// Longitude
    pub longitude: f64,

    /// Country code (ISO 3166-1 alpha-2)
    pub country: String,

    /// City name
    pub city: String,

    /// Continent
    pub continent: Continent,
}

impl GeoLocation {
    /// Calculate distance to another location in kilometers (Haversine formula)
    pub fn distance_to(&self, other: &GeoLocation) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS_KM * c
    }
}

/// Continent identifiers
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum Continent {
    AF, // Africa
    AN, // Antarctica
    AS, // Asia
    EU, // Europe
    NA, // North America
    OC, // Oceania
    SA, // South America
}

/// Multi-region manager
pub struct MultiRegionManager {
    /// Registered regions
    regions: Arc<RwLock<HashMap<RegionId, Region>>>,

    /// Client IP to region mapping
    ip_region_map: Arc<RwLock<HashMap<IpAddr, RegionId>>>,

    /// GeoIP resolver
    geoip_resolver: Arc<GeoIPResolver>,

    /// Routing strategy
    routing_strategy: RegionRoutingStrategy,
}

impl MultiRegionManager {
    pub fn new(routing_strategy: RegionRoutingStrategy) -> Self {
        Self {
            regions: Arc::new(RwLock::new(HashMap::new())),
            ip_region_map: Arc::new(RwLock::new(HashMap::new())),
            geoip_resolver: Arc::new(GeoIPResolver::new()),
            routing_strategy,
        }
    }

    /// Register a region
    pub async fn register_region(&self, region: Region) -> Result<()> {
        let mut regions = self.regions.write().await;
        regions.insert(region.id.clone(), region);
        Ok(())
    }

    /// Get region by ID
    pub async fn get_region(&self, id: &RegionId) -> Option<Region> {
        let regions = self.regions.read().await;
        regions.get(id).cloned()
    }

    /// List all regions
    pub async fn list_regions(&self) -> Vec<Region> {
        let regions = self.regions.read().await;
        regions.values().cloned().collect()
    }

    /// Select the best region for a client
    pub async fn select_region(&self, client_ip: IpAddr) -> Result<RegionId> {
        // Check cached mapping first
        {
            let ip_map = self.ip_region_map.read().await;
            if let Some(region_id) = ip_map.get(&client_ip) {
                debug!("Using cached region mapping for {}: {:?}", client_ip, region_id);
                return Ok(region_id.clone());
            }
        }

        // Resolve client location
        let client_location = self.geoip_resolver.resolve(client_ip).await?;

        // Get all healthy regions with capacity
        let regions = self.regions.read().await;
        let available_regions: Vec<&Region> = regions
            .values()
            .filter(|r| r.healthy && r.has_capacity())
            .collect();

        if available_regions.is_empty() {
            return Err(Error::Internal(
                "No healthy regions with capacity available".to_string(),
            ));
        }

        // Select region based on strategy
        let selected_region = match self.routing_strategy {
            RegionRoutingStrategy::Nearest => {
                self.select_nearest_region(&client_location, &available_regions)
            }
            RegionRoutingStrategy::LeastLatency => {
                self.select_least_latency_region(&available_regions)
            }
            RegionRoutingStrategy::LeastLoad => {
                self.select_least_loaded_region(&available_regions)
            }
            RegionRoutingStrategy::Weighted => {
                self.select_weighted_region(&client_location, &available_regions)
            }
        };

        // Cache the mapping
        let mut ip_map = self.ip_region_map.write().await;
        ip_map.insert(client_ip, selected_region.clone());

        info!(
            "Selected region {:?} for client {} (strategy: {:?})",
            selected_region, client_ip, self.routing_strategy
        );

        Ok(selected_region)
    }

    /// Select nearest region by geographic distance
    fn select_nearest_region(
        &self,
        client_location: &GeoLocation,
        regions: &[&Region],
    ) -> RegionId {
        let mut nearest = None;
        let mut min_distance = f64::MAX;

        for region in regions {
            let distance = client_location.distance_to(&region.location);
            if distance < min_distance {
                min_distance = distance;
                nearest = Some(region.id.clone());
            }
        }

        nearest.unwrap_or_else(|| regions[0].id.clone())
    }

    /// Select region with least latency
    fn select_least_latency_region(&self, regions: &[&Region]) -> RegionId {
        regions
            .iter()
            .min_by(|a, b| {
                a.avg_latency_ms
                    .partial_cmp(&b.avg_latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|r| r.id.clone())
            .unwrap_or_else(|| regions[0].id.clone())
    }

    /// Select least loaded region
    fn select_least_loaded_region(&self, regions: &[&Region]) -> RegionId {
        regions
            .iter()
            .min_by(|a, b| a.utilization().partial_cmp(&b.utilization()).unwrap())
            .map(|r| r.id.clone())
            .unwrap_or_else(|| regions[0].id.clone())
    }

    /// Select region using weighted strategy (distance + latency + load)
    fn select_weighted_region(
        &self,
        client_location: &GeoLocation,
        regions: &[&Region],
    ) -> RegionId {
        let mut best_region = None;
        let mut best_score = f64::MAX;

        for region in regions {
            // Normalize distance to 0-1 range (max 20000 km)
            let distance = client_location.distance_to(&region.location);
            let distance_score = (distance / 20000.0).min(1.0);

            // Normalize latency to 0-1 range (max 1000ms)
            let latency_score = (region.avg_latency_ms / 1000.0).min(1.0);

            // Utilization as 0-1
            let load_score = region.utilization() / 100.0;

            // Priority factor (higher priority = lower score)
            let priority_score = 1.0 - (region.priority as f64 / 255.0);

            // Weighted combination (lower is better)
            let score = 0.4 * distance_score
                + 0.3 * latency_score
                + 0.2 * load_score
                + 0.1 * priority_score;

            if score < best_score {
                best_score = score;
                best_region = Some(region.id.clone());
            }
        }

        best_region.unwrap_or_else(|| regions[0].id.clone())
    }

    /// Update region health status
    pub async fn update_region_health(&self, region_id: &RegionId, healthy: bool) {
        let mut regions = self.regions.write().await;
        if let Some(region) = regions.get_mut(region_id) {
            region.healthy = healthy;
            if !healthy {
                warn!("Region {:?} marked as unhealthy", region_id);
            }
        }
    }

    /// Update region latency
    pub async fn update_region_latency(&self, region_id: &RegionId, latency_ms: f64) {
        let mut regions = self.regions.write().await;
        if let Some(region) = regions.get_mut(region_id) {
            // Exponential moving average
            let alpha = 0.3;
            region.avg_latency_ms = alpha * latency_ms + (1.0 - alpha) * region.avg_latency_ms;
        }
    }

    /// Update region load
    pub async fn update_region_load(&self, region_id: &RegionId, load: u32) {
        let mut regions = self.regions.write().await;
        if let Some(region) = regions.get_mut(region_id) {
            region.current_load = load;
        }
    }

    /// Clear IP region mapping cache
    pub async fn clear_cache(&self) {
        let mut ip_map = self.ip_region_map.write().await;
        ip_map.clear();
        debug!("Cleared IP-to-region mapping cache");
    }
}

/// Region routing strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionRoutingStrategy {
    /// Route to nearest region by geographic distance
    Nearest,

    /// Route to region with lowest latency
    LeastLatency,

    /// Route to least loaded region
    LeastLoad,

    /// Weighted combination of distance, latency, and load
    Weighted,
}

/// GeoIP resolver (simplified implementation)
pub struct GeoIPResolver {
    /// IP to location cache
    cache: Arc<RwLock<HashMap<IpAddr, GeoLocation>>>,

    /// Default locations for well-known IP ranges
    default_locations: HashMap<String, GeoLocation>,
}

impl GeoIPResolver {
    pub fn new() -> Self {
        let mut default_locations = HashMap::new();

        // Add some default locations for common IP ranges
        // In production, use a real GeoIP database like MaxMind

        default_locations.insert(
            "default".to_string(),
            GeoLocation {
                latitude: 37.7749,
                longitude: -122.4194,
                country: "US".to_string(),
                city: "San Francisco".to_string(),
                continent: Continent::NA,
            },
        );

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            default_locations,
        }
    }

    /// Resolve IP address to geographic location
    pub async fn resolve(&self, ip: IpAddr) -> Result<GeoLocation> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(location) = cache.get(&ip) {
                return Ok(location.clone());
            }
        }

        // In production, query GeoIP database
        // For now, return default location
        let location = self
            .default_locations
            .get("default")
            .ok_or_else(|| Error::Internal("GeoIP lookup failed".to_string()))?
            .clone();

        // Cache result
        let mut cache = self.cache.write().await;
        cache.insert(ip, location.clone());

        Ok(location)
    }

    /// Add custom IP to location mapping
    pub async fn add_mapping(&self, ip: IpAddr, location: GeoLocation) {
        let mut cache = self.cache.write().await;
        cache.insert(ip, location);
    }
}

impl Default for GeoIPResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Cross-region replication manager
pub struct ReplicationManager {
    /// Regions being managed
    regions: Arc<RwLock<HashMap<RegionId, Region>>>,

    /// Replication lag tracking
    replication_lag: Arc<RwLock<HashMap<(RegionId, RegionId), Duration>>>,
}

impl ReplicationManager {
    pub fn new(regions: Arc<RwLock<HashMap<RegionId, Region>>>) -> Self {
        Self {
            regions,
            replication_lag: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Replicate data from source to target region
    pub async fn replicate(
        &self,
        source: &RegionId,
        target: &RegionId,
        data: Vec<u8>,
    ) -> Result<()> {
        debug!(
            "Replicating {} bytes from {:?} to {:?}",
            data.len(),
            source,
            target
        );

        // In production, implement actual data transfer
        // For now, just track that replication occurred

        // Simulate replication delay
        tokio::time::sleep(Duration::from_millis(10)).await;

        Ok(())
    }

    /// Update replication lag between regions
    pub async fn update_lag(&self, source: RegionId, target: RegionId, lag: Duration) {
        let mut lags = self.replication_lag.write().await;
        lags.insert((source, target), lag);
    }

    /// Get replication lag between regions
    pub async fn get_lag(&self, source: &RegionId, target: &RegionId) -> Option<Duration> {
        let lags = self.replication_lag.read().await;
        lags.get(&(source.clone(), target.clone())).copied()
    }

    /// Get replication health for a region
    pub async fn get_replication_health(&self, region: &RegionId) -> ReplicationHealth {
        let regions = self.regions.read().await;
        let lags = self.replication_lag.read().await;

        if let Some(region_config) = regions.get(region) {
            let mut peer_lags = Vec::new();

            for peer in &region_config.replication_peers {
                if let Some(lag) = lags.get(&(region.clone(), peer.clone())) {
                    peer_lags.push((peer.clone(), *lag));
                }
            }

            // Calculate max lag
            let max_lag = peer_lags
                .iter()
                .map(|(_, lag)| *lag)
                .max()
                .unwrap_or(Duration::from_secs(0));

            let healthy = max_lag < Duration::from_secs(60); // Unhealthy if lag > 60s

            ReplicationHealth {
                region: region.clone(),
                peer_lags,
                max_lag,
                healthy,
            }
        } else {
            ReplicationHealth {
                region: region.clone(),
                peer_lags: Vec::new(),
                max_lag: Duration::from_secs(0),
                healthy: false,
            }
        }
    }
}

/// Replication health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationHealth {
    pub region: RegionId,
    pub peer_lags: Vec<(RegionId, Duration)>,
    pub max_lag: Duration,
    pub healthy: bool,
}

/// Region failover manager
pub struct FailoverManager {
    /// Primary region for each client
    primary_regions: Arc<RwLock<HashMap<IpAddr, RegionId>>>,

    /// Failover history
    failover_history: Arc<RwLock<Vec<FailoverEvent>>>,
}

impl FailoverManager {
    pub fn new() -> Self {
        Self {
            primary_regions: Arc::new(RwLock::new(HashMap::new())),
            failover_history: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Trigger failover from one region to another
    pub async fn failover(
        &self,
        client_ip: IpAddr,
        from_region: RegionId,
        to_region: RegionId,
        reason: String,
    ) -> Result<()> {
        info!(
            "Failing over client {} from {:?} to {:?}: {}",
            client_ip, from_region, to_region, reason
        );

        // Update primary region
        let mut primaries = self.primary_regions.write().await;
        primaries.insert(client_ip, to_region.clone());

        // Record failover event
        let event = FailoverEvent {
            client_ip,
            from_region,
            to_region,
            reason,
            timestamp: chrono::Utc::now(),
        };

        let mut history = self.failover_history.write().await;
        history.push(event);

        // Keep only recent history (last 1000 events)
        if history.len() > 1000 {
            history.remove(0);
        }

        Ok(())
    }

    /// Get failover history
    pub async fn get_history(&self, limit: usize) -> Vec<FailoverEvent> {
        let history = self.failover_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }
}

impl Default for FailoverManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Failover event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverEvent {
    pub client_ip: IpAddr,
    pub from_region: RegionId,
    pub to_region: RegionId,
    pub reason: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_geo_location_distance() {
        let san_francisco = GeoLocation {
            latitude: 37.7749,
            longitude: -122.4194,
            country: "US".to_string(),
            city: "San Francisco".to_string(),
            continent: Continent::NA,
        };

        let new_york = GeoLocation {
            latitude: 40.7128,
            longitude: -74.0060,
            country: "US".to_string(),
            city: "New York".to_string(),
            continent: Continent::NA,
        };

        let distance = san_francisco.distance_to(&new_york);
        assert!(distance > 4000.0 && distance < 5000.0); // ~4,130 km
    }

    #[tokio::test]
    async fn test_region_registration() {
        let manager = MultiRegionManager::new(RegionRoutingStrategy::Nearest);

        let region = Region::new(
            RegionId::new("us-west-1"),
            "US West 1".to_string(),
            GeoLocation {
                latitude: 37.7749,
                longitude: -122.4194,
                country: "US".to_string(),
                city: "San Francisco".to_string(),
                continent: Continent::NA,
            },
        );

        manager.register_region(region).await.unwrap();

        let retrieved = manager.get_region(&RegionId::new("us-west-1")).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "US West 1");
    }

    #[tokio::test]
    async fn test_region_selection() {
        let manager = MultiRegionManager::new(RegionRoutingStrategy::LeastLoad);

        // Add regions
        let mut region1 = Region::new(
            RegionId::new("us-west"),
            "US West".to_string(),
            GeoLocation {
                latitude: 37.7749,
                longitude: -122.4194,
                country: "US".to_string(),
                city: "San Francisco".to_string(),
                continent: Continent::NA,
            },
        );
        region1.current_load = 100;
        region1.capacity = 1000;

        let mut region2 = Region::new(
            RegionId::new("us-east"),
            "US East".to_string(),
            GeoLocation {
                latitude: 40.7128,
                longitude: -74.0060,
                country: "US".to_string(),
                city: "New York".to_string(),
                continent: Continent::NA,
            },
        );
        region2.current_load = 500;
        region2.capacity = 1000;

        manager.register_region(region1).await.unwrap();
        manager.register_region(region2).await.unwrap();

        // Should select us-west (lower load)
        let client_ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        let selected = manager.select_region(client_ip).await.unwrap();

        assert_eq!(selected.as_str(), "us-west");
    }

    #[test]
    fn test_region_utilization() {
        let mut region = Region::new(
            RegionId::new("test"),
            "Test".to_string(),
            GeoLocation {
                latitude: 0.0,
                longitude: 0.0,
                country: "US".to_string(),
                city: "Test".to_string(),
                continent: Continent::NA,
            },
        );

        region.capacity = 1000;
        region.current_load = 750;

        assert_eq!(region.utilization(), 75.0);
        assert!(region.has_capacity());
    }
}
