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
