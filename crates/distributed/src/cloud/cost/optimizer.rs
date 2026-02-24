//! Cloud cost optimizer with estimation and budget enforcement

use std::collections::HashMap;
use toadstool::error::ToadStoolResult;

use super::pricing::{infer_pricing_tier, PricingTier};
use super::types::{CostError, CostEstimate, CostLineItem};
use super::types::{BYTES_PER_GB, DAYS_PER_MONTH, HOURS_PER_DAY, SPOT_DISCOUNT_FACTOR};
use crate::cloud::types::{CloudCapabilities, CostConfig, CostModel};
use crate::types::resources::ResourceRequirements;

/// Cloud cost optimizer with real estimation, capability-based pricing, and budget enforcement.
pub struct CloudCostOptimizer {
    pub(crate) config: CostConfig,
    pub(crate) cost_models: HashMap<String, CostModel>,
    pub(crate) capability_models: HashMap<String, CloudCapabilities>,
}

impl CloudCostOptimizer {
    pub async fn new(config: CostConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            config,
            cost_models: HashMap::new(),
            capability_models: HashMap::new(),
        })
    }

    /// Add a provider's cost model. Uses capability-based tier when capabilities are provided.
    pub async fn add_provider_cost_model(
        &mut self,
        name: &str,
        capabilities: &CloudCapabilities,
    ) -> ToadStoolResult<()> {
        let tier = infer_pricing_tier(capabilities);
        let cost_model = CostModel {
            cpu_cost_per_core_hour: tier.cpu_cost_per_core_hour(),
            memory_cost_per_gb_hour: tier.memory_cost_per_gb_hour(),
            storage_cost_per_gb_month: tier.storage_cost_per_gb_month(),
            network_cost_per_gb: tier.network_cost_per_gb(),
        };
        self.cost_models.insert(name.to_string(), cost_model);
        self.capability_models
            .insert(name.to_string(), capabilities.clone());
        Ok(())
    }

    /// Estimate cost for given resource requirements and duration.
    pub fn estimate_cost(
        &self,
        provider: &str,
        requirements: &ResourceRequirements,
        duration_hours: f64,
        network_gb: f64,
    ) -> ToadStoolResult<CostEstimate> {
        if duration_hours <= 0.0 {
            return Err(CostError::InvalidDuration.into());
        }

        let model = self
            .cost_models
            .get(provider)
            .ok_or_else(|| CostError::ModelNotFound(provider.to_string()))?;

        let capabilities = self.capability_models.get(provider);
        let tier = capabilities
            .map(infer_pricing_tier)
            .unwrap_or(PricingTier::StandardCompute);

        let spot_factor = 1.0 - (self.config.spot_instance_preference * SPOT_DISCOUNT_FACTOR);

        let cpu_cores = requirements.cpu.min_cores;
        let memory_gb = requirements.memory.min_bytes as f64 / BYTES_PER_GB as f64;
        let storage_gb = requirements.storage.min_bytes as f64 / BYTES_PER_GB as f64;

        let cpu_cost = cpu_cores * duration_hours * model.cpu_cost_per_core_hour * spot_factor;
        let memory_cost = memory_gb * duration_hours * model.memory_cost_per_gb_hour * spot_factor;
        // Storage is billed per GB-month; prorate by duration
        let storage_cost = storage_gb * model.storage_cost_per_gb_month * duration_hours
            / (HOURS_PER_DAY * DAYS_PER_MONTH);
        let network_cost = network_gb * model.network_cost_per_gb;

        let gpu_count = requirements.gpu.as_ref().map(|_| 1.0).unwrap_or(0.0);
        let gpu_cost = if gpu_count > 0.0 {
            gpu_count * duration_hours * tier.gpu_cost_per_gpu_hour() * spot_factor
        } else {
            0.0
        };

        let mut line_items = vec![
            CostLineItem {
                category: "cpu".to_string(),
                quantity: cpu_cores * duration_hours,
                unit: "core-hours".to_string(),
                unit_price: model.cpu_cost_per_core_hour * spot_factor,
                total: cpu_cost,
            },
            CostLineItem {
                category: "memory".to_string(),
                quantity: memory_gb * duration_hours,
                unit: "GB-hours".to_string(),
                unit_price: model.memory_cost_per_gb_hour * spot_factor,
                total: memory_cost,
            },
            CostLineItem {
                category: "storage".to_string(),
                quantity: storage_gb,
                unit: "GB-month".to_string(),
                unit_price: model.storage_cost_per_gb_month,
                total: storage_cost,
            },
            CostLineItem {
                category: "network".to_string(),
                quantity: network_gb,
                unit: "GB".to_string(),
                unit_price: model.network_cost_per_gb,
                total: network_cost,
            },
        ];

        if gpu_cost > 0.0 {
            line_items.push(CostLineItem {
                category: "gpu".to_string(),
                quantity: gpu_count * duration_hours,
                unit: "GPU-hours".to_string(),
                unit_price: tier.gpu_cost_per_gpu_hour() * spot_factor,
                total: gpu_cost,
            });
        }

        let total_cost = line_items.iter().map(|i| i.total).sum();

        if let Some(limit) = self.config.budget_limit {
            if total_cost > limit {
                return Err(CostError::BudgetExceeded {
                    estimate: total_cost,
                    limit,
                }
                .into());
            }
        }

        Ok(CostEstimate {
            line_items,
            total_cost,
            tier: format!("{tier:?}"),
            uses_spot: self.config.spot_instance_preference > 0.0,
            duration_hours,
        })
    }

    /// Record spend against budget (for tracking). Returns error if budget would be exceeded.
    pub fn record_spend(&self, amount: f64, current_spend: f64) -> ToadStoolResult<()> {
        if !self.config.cost_tracking_enabled {
            return Ok(());
        }
        if let Some(limit) = self.config.budget_limit {
            let new_total = current_spend + amount;
            if new_total > limit {
                return Err(CostError::BudgetExceeded {
                    estimate: new_total,
                    limit,
                }
                .into());
            }
        }
        Ok(())
    }

    /// Check whether an estimate would exceed the configured budget.
    pub fn would_exceed_budget(&self, estimate: f64) -> bool {
        self.config
            .budget_limit
            .map(|limit| estimate > limit)
            .unwrap_or(false)
    }
}
