// SPDX-License-Identifier: AGPL-3.0-or-later
//! Load balancing across clouds
//!
//! This module contains the multi-cloud load balancer.

use toadstool::error::ToadStoolResult;

use super::types::LoadBalancerConfig;

/// Multi-cloud load balancer
pub struct MultiCloudLoadBalancer {
    pub(crate) _config: LoadBalancerConfig,
}

impl MultiCloudLoadBalancer {
    pub async fn new(config: LoadBalancerConfig) -> ToadStoolResult<Self> {
        Ok(Self { _config: config })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::LoadBalancingAlgorithm;
    use std::time::Duration;

    fn make_config() -> LoadBalancerConfig {
        LoadBalancerConfig {
            algorithm: LoadBalancingAlgorithm::RoundRobin,
            health_check_interval: Duration::from_secs(5),
            failover_timeout: Duration::from_secs(30),
        }
    }

    #[tokio::test]
    async fn test_new_load_balancer() {
        let lb = MultiCloudLoadBalancer::new(make_config()).await.unwrap();
        let _ = lb;
    }

    #[tokio::test]
    async fn test_new_load_balancer_least_connections() {
        let config = LoadBalancerConfig {
            algorithm: LoadBalancingAlgorithm::LeastConnections,
            health_check_interval: Duration::from_millis(500),
            failover_timeout: Duration::from_secs(10),
        };
        let lb = MultiCloudLoadBalancer::new(config).await.unwrap();
        let _ = lb;
    }
}
