//! Cost optimization across clouds
//!
//! This module contains the cost optimizer and related cost management functionality.

use std::collections::HashMap;
use toadstool::error::ToadStoolResult;

use super::types::CloudCapabilities;
use super::types::{CostConfig, CostModel};

/// Cloud cost optimizer
pub struct CloudCostOptimizer {
    pub(crate) _config: CostConfig,
    pub(crate) _cost_models: HashMap<String, CostModel>,
}

impl CloudCostOptimizer {
    pub async fn new(config: CostConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            _config: config,
            _cost_models: HashMap::new(),
        })
    }

    pub async fn add_provider_cost_model(
        &mut self,
        name: &str,
        _capabilities: &CloudCapabilities,
    ) -> ToadStoolResult<()> {
        // Simple cost model for now
        let cost_model = CostModel {
            cpu_cost_per_core_hour: 0.10,
            memory_cost_per_gb_hour: 0.02,
            storage_cost_per_gb_month: 0.10,
            network_cost_per_gb: 0.05,
        };

        self._cost_models.insert(name.to_string(), cost_model);
        Ok(())
    }
}

/// Cloud cost model implementations
#[derive(Debug, Clone)]
pub struct CloudCostModel {
    pub cpu_rate: f64,
    pub memory_rate: f64,
    pub storage_rate: f64,
    pub network_rate: f64,
}

impl CloudCostModel {
    pub fn new_aws() -> Self {
        Self {
            cpu_rate: 0.10,
            memory_rate: 0.02,
            storage_rate: 0.10,
            network_rate: 0.05,
        }
    }

    pub fn new_azure() -> Self {
        Self {
            cpu_rate: 0.09,
            memory_rate: 0.018,
            storage_rate: 0.08,
            network_rate: 0.04,
        }
    }

    pub fn new_gcp() -> Self {
        Self {
            cpu_rate: 0.08,
            memory_rate: 0.015,
            storage_rate: 0.04,
            network_rate: 0.03,
        }
    }

    pub fn new_digitalocean() -> Self {
        Self {
            cpu_rate: 0.06,
            memory_rate: 0.012,
            storage_rate: 0.02,
            network_rate: 0.02,
        }
    }

    pub fn new_hetzner() -> Self {
        Self {
            cpu_rate: 0.04,
            memory_rate: 0.008,
            storage_rate: 0.01,
            network_rate: 0.01,
        }
    }

    pub fn new_localhost() -> Self {
        Self {
            cpu_rate: 0.01,
            memory_rate: 0.002,
            storage_rate: 0.0,
            network_rate: 0.0,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::types::{
        CloudCapabilities, ComputeType, NetworkingFeature, SecurityFeature, StorageType,
    };

    fn empty_capabilities() -> CloudCapabilities {
        CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption],
            compliance_certifications: vec![],
            regions: vec![],
            max_cpu_cores: None,
            max_memory_gb: None,
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        }
    }

    #[tokio::test]
    async fn test_new_optimizer() {
        let cfg = CostConfig {
            budget_limit: Some(100.0),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.5,
        };
        let optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        assert!(optimizer._cost_models.is_empty());
    }

    #[tokio::test]
    async fn test_add_provider_cost_model() {
        let cfg = CostConfig {
            budget_limit: None,
            cost_tracking_enabled: false,
            spot_instance_preference: 0.0,
        };
        let mut optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        let caps = empty_capabilities();
        optimizer
            .add_provider_cost_model("aws", &caps)
            .await
            .unwrap();
        assert!(optimizer._cost_models.contains_key("aws"));
    }

    #[test]
    fn test_cost_model_rates_aws() {
        let model = CloudCostModel::new_aws();
        assert!(model.cpu_rate > 0.0);
        assert!(model.memory_rate > 0.0);
        assert!(model.storage_rate > 0.0);
        assert!(model.network_rate > 0.0);
    }

    #[test]
    fn test_cost_model_rates_azure_lower_than_aws() {
        let aws = CloudCostModel::new_aws();
        let azure = CloudCostModel::new_azure();
        // Azure advertised slightly lower than AWS in the model
        assert!(azure.cpu_rate < aws.cpu_rate);
    }

    #[test]
    fn test_cost_model_rates_gcp_lower_than_azure() {
        let azure = CloudCostModel::new_azure();
        let gcp = CloudCostModel::new_gcp();
        assert!(gcp.cpu_rate < azure.cpu_rate);
    }

    #[test]
    fn test_cost_model_localhost_zero_storage_and_network() {
        let local = CloudCostModel::new_localhost();
        assert_eq!(local.storage_rate, 0.0);
        assert_eq!(local.network_rate, 0.0);
        assert!(local.cpu_rate > 0.0);
    }

    #[test]
    fn test_cost_model_hetzner_lowest_cpu() {
        let models = [
            CloudCostModel::new_aws(),
            CloudCostModel::new_azure(),
            CloudCostModel::new_gcp(),
            CloudCostModel::new_digitalocean(),
            CloudCostModel::new_hetzner(),
        ];
        let min_cpu = models
            .iter()
            .map(|m| m.cpu_rate)
            .fold(f64::INFINITY, f64::min);
        assert_eq!(CloudCostModel::new_hetzner().cpu_rate, min_cpu);
    }
}
