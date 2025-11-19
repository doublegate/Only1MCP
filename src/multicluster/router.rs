/// Multi-Cluster Router Implementation

use super::{ClusterInfo, RequestMetadata, RoutingStrategy};
use crate::error::{Error, Result};

pub struct MultiClusterRouter {
    clusters: Vec<ClusterInfo>,
    strategy: RoutingStrategy,
}

impl MultiClusterRouter {
    pub fn new(clusters: Vec<ClusterInfo>, strategy: RoutingStrategy) -> Self {
        Self { clusters, strategy }
    }

    /// Select the appropriate cluster for a request
    pub async fn select_cluster(&self, metadata: &RequestMetadata) -> Result<String> {
        match &self.strategy {
            RoutingStrategy::Geographic => self.select_by_geography(metadata),
            RoutingStrategy::LoadBased => self.select_by_load().await,
            RoutingStrategy::CostOptimized => self.select_by_cost(),
            RoutingStrategy::Weighted => self.select_by_weight(),
            RoutingStrategy::Failover { primary, secondary } => {
                self.select_with_failover(primary, secondary)
            }
        }
    }

    /// Select cluster based on geographic proximity
    fn select_by_geography(&self, metadata: &RequestMetadata) -> Result<String> {
        if let Some(ref client_region) = metadata.client_region {
            // Find cluster in same region
            if let Some(cluster) = self
                .clusters
                .iter()
                .find(|c| c.region == *client_region && is_healthy(c))
            {
                return Ok(cluster.id.clone());
            }

            // Find nearest region (simplified - would use actual geographic distance)
            if let Some(cluster) = self.clusters.iter().find(|c| is_healthy(c)) {
                return Ok(cluster.id.clone());
            }
        }

        // Fallback to first healthy cluster
        self.clusters
            .iter()
            .find(|c| is_healthy(c))
            .map(|c| c.id.clone())
            .ok_or_else(|| Error::Config("No healthy clusters available".to_string()))
    }

    /// Select cluster based on available capacity
    async fn select_by_load(&self) -> Result<String> {
        let cluster = self
            .clusters
            .iter()
            .filter(|c| is_healthy(c))
            .max_by_key(|c| c.capacity.available_cpu_cores)
            .ok_or_else(|| Error::Config("No healthy clusters available".to_string()))?;

        Ok(cluster.id.clone())
    }

    /// Select cheapest cluster (simplified cost model)
    fn select_by_cost(&self) -> Result<String> {
        // In a real implementation, this would factor in:
        // - Compute costs per region
        // - Data transfer costs
        // - Storage costs

        // For now, use weighted selection as a proxy
        self.select_by_weight()
    }

    /// Select cluster based on configured weights
    fn select_by_weight(&self) -> Result<String> {
        let total_weight: u32 = self
            .clusters
            .iter()
            .filter(|c| is_healthy(c))
            .map(|c| c.weight)
            .sum();

        if total_weight == 0 {
            return Err(Error::Config("No healthy clusters with weight > 0".to_string()));
        }

        // Simple weighted random selection (in production, use better RNG)
        let mut rng = total_weight / 2; // Simplified
        for cluster in self.clusters.iter().filter(|c| is_healthy(c)) {
            if rng < cluster.weight {
                return Ok(cluster.id.clone());
            }
            rng -= cluster.weight;
        }

        // Fallback
        self.clusters
            .iter()
            .find(|c| is_healthy(c))
            .map(|c| c.id.clone())
            .ok_or_else(|| Error::Config("No healthy clusters available".to_string()))
    }

    /// Select with primary-secondary failover
    fn select_with_failover(&self, primary: &str, secondary: &[String]) -> Result<String> {
        // Check if primary is healthy
        if let Some(cluster) = self.clusters.iter().find(|c| c.id == primary && is_healthy(c)) {
            return Ok(cluster.id.clone());
        }

        // Try secondaries in order
        for secondary_id in secondary {
            if let Some(cluster) = self
                .clusters
                .iter()
                .find(|c| c.id == *secondary_id && is_healthy(c))
            {
                tracing::warn!(
                    "Primary cluster {} unhealthy, failing over to {}",
                    primary,
                    secondary_id
                );
                return Ok(cluster.id.clone());
            }
        }

        Err(Error::Config(format!(
            "All clusters in failover chain unhealthy: primary={}, secondaries={:?}",
            primary, secondary
        )))
    }

    /// Get all clusters
    pub fn get_clusters(&self) -> Vec<ClusterInfo> {
        self.clusters.clone()
    }
}

/// Check if cluster is healthy
fn is_healthy(cluster: &ClusterInfo) -> bool {
    cluster.status == super::ClusterStatus::Healthy
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multicluster::{ClusterCapacity, ClusterStatus, RequestPriority};

    fn create_test_cluster(id: &str, region: &str, weight: u32) -> ClusterInfo {
        ClusterInfo {
            id: id.to_string(),
            name: format!("{} Cluster", id),
            region: region.to_string(),
            endpoint: format!("https://{}.example.com", id),
            weight,
            capacity: ClusterCapacity {
                total_nodes: 10,
                available_nodes: 8,
                total_cpu_cores: 40,
                available_cpu_cores: 32,
                total_memory_gb: 160,
                available_memory_gb: 128,
            },
            status: ClusterStatus::Healthy,
        }
    }

    #[tokio::test]
    async fn test_geographic_routing() {
        let clusters = vec![
            create_test_cluster("us-east", "us-east-1", 100),
            create_test_cluster("us-west", "us-west-1", 100),
            create_test_cluster("eu-west", "eu-west-1", 100),
        ];

        let router = MultiClusterRouter::new(clusters, RoutingStrategy::Geographic);

        let metadata = RequestMetadata {
            client_ip: None,
            client_region: Some("us-east-1".to_string()),
            user_id: None,
            priority: RequestPriority::Normal,
        };

        let selected = router.select_cluster(&metadata).await.unwrap();
        assert_eq!(selected, "us-east");
    }

    #[tokio::test]
    async fn test_load_based_routing() {
        let mut clusters = vec![
            create_test_cluster("cluster1", "us-east-1", 100),
            create_test_cluster("cluster2", "us-west-1", 100),
        ];

        // Make cluster2 have more available capacity
        clusters[1].capacity.available_cpu_cores = 64;

        let router = MultiClusterRouter::new(clusters, RoutingStrategy::LoadBased);

        let metadata = RequestMetadata {
            client_ip: None,
            client_region: None,
            user_id: None,
            priority: RequestPriority::Normal,
        };

        let selected = router.select_cluster(&metadata).await.unwrap();
        assert_eq!(selected, "cluster2");
    }

    #[tokio::test]
    async fn test_failover_routing() {
        let mut clusters = vec![
            create_test_cluster("primary", "us-east-1", 100),
            create_test_cluster("secondary1", "us-west-1", 100),
            create_test_cluster("secondary2", "eu-west-1", 100),
        ];

        // Mark primary as unavailable
        clusters[0].status = ClusterStatus::Unavailable;

        let router = MultiClusterRouter::new(
            clusters,
            RoutingStrategy::Failover {
                primary: "primary".to_string(),
                secondary: vec!["secondary1".to_string(), "secondary2".to_string()],
            },
        );

        let metadata = RequestMetadata {
            client_ip: None,
            client_region: None,
            user_id: None,
            priority: RequestPriority::Normal,
        };

        let selected = router.select_cluster(&metadata).await.unwrap();
        assert_eq!(selected, "secondary1");
    }
}
