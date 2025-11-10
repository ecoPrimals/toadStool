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
