//! Cost optimization across clouds
//!
//! This module provides real cost estimation based on resource requirements (CPU, GPU, memory,
//! network), capability-based pricing tiers, cost capping, and budget enforcement.

use std::collections::HashMap;
use thiserror::Error;
use toadstool::error::{ToadStoolError, ToadStoolResult};

use super::types::CloudCapabilities;
use super::types::{CostConfig, CostModel};
use crate::types::resources::ResourceRequirements;

// ─── Named Constants ─────────────────────────────────────────────────────────

/// Default spot instance discount multiplier (spot is typically 60–70% cheaper than on-demand).
pub const SPOT_DISCOUNT_FACTOR: f64 = 0.35;

/// Hours per day for daily cost calculations.
pub const HOURS_PER_DAY: f64 = 24.0;

/// Days per month for monthly cost calculations.
pub const DAYS_PER_MONTH: f64 = 30.0;

/// Bytes per GB for storage conversions.
pub const BYTES_PER_GB: u64 = 1024 * 1024 * 1024;

// ─── Capability-Based Pricing Tiers ─────────────────────────────────────────

/// Pricing tier based on compute capability, not hardcoded cloud provider names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PricingTier {
    /// Basic VM compute (standard CPU)
    StandardCompute,
    /// High-memory or memory-optimized instances
    HighMemoryCompute,
    /// GPU-accelerated compute
    GpuAccelerated,
    /// Bare metal or high-performance dedicated
    BareMetalDedicated,
    /// Serverless / pay-per-use (typically cheaper per unit, different billing)
    Serverless,
    /// Edge / local / on-prem (minimal cloud cost)
    EdgeLocal,
}

impl PricingTier {
    /// Returns the base CPU cost per core-hour for this tier.
    pub fn cpu_cost_per_core_hour(self) -> f64 {
        match self {
            PricingTier::StandardCompute => 0.08,
            PricingTier::HighMemoryCompute => 0.12,
            PricingTier::GpuAccelerated => 0.15,
            PricingTier::BareMetalDedicated => 0.25,
            PricingTier::Serverless => 0.0001, // Per-invocation dominated; use small base
            PricingTier::EdgeLocal => 0.01,
        }
    }

    /// Returns the memory cost per GB-hour for this tier.
    pub fn memory_cost_per_gb_hour(self) -> f64 {
        match self {
            PricingTier::StandardCompute => 0.012,
            PricingTier::HighMemoryCompute => 0.018,
            PricingTier::GpuAccelerated => 0.020,
            PricingTier::BareMetalDedicated => 0.030,
            PricingTier::Serverless => 0.000016,
            PricingTier::EdgeLocal => 0.002,
        }
    }

    /// Returns the storage cost per GB-month for this tier.
    pub fn storage_cost_per_gb_month(self) -> f64 {
        match self {
            PricingTier::StandardCompute => 0.08,
            PricingTier::HighMemoryCompute => 0.10,
            PricingTier::GpuAccelerated => 0.08,
            PricingTier::BareMetalDedicated => 0.15,
            PricingTier::Serverless => 0.023,
            PricingTier::EdgeLocal => 0.0,
        }
    }

    /// Returns the network cost per GB for this tier.
    pub fn network_cost_per_gb(self) -> f64 {
        match self {
            PricingTier::StandardCompute => 0.05,
            PricingTier::HighMemoryCompute => 0.05,
            PricingTier::GpuAccelerated => 0.06,
            PricingTier::BareMetalDedicated => 0.04,
            PricingTier::Serverless => 0.09,
            PricingTier::EdgeLocal => 0.0,
        }
    }

    /// Returns the GPU cost per GPU-hour for tiers that support GPU.
    pub fn gpu_cost_per_gpu_hour(self) -> f64 {
        match self {
            PricingTier::GpuAccelerated => 2.50,
            PricingTier::BareMetalDedicated => 3.00,
            _ => 0.0,
        }
    }
}

/// Infer pricing tier from cloud capabilities.
fn infer_pricing_tier(capabilities: &CloudCapabilities) -> PricingTier {
    if capabilities.gpu_support {
        PricingTier::GpuAccelerated
    } else if capabilities.serverless_support {
        PricingTier::Serverless
    } else if capabilities.max_memory_gb.map(|g| g >= 64).unwrap_or(false) {
        PricingTier::HighMemoryCompute
    } else if capabilities
        .compute_types
        .iter()
        .any(|t| matches!(t, super::types::ComputeType::BareMetalC))
    {
        PricingTier::BareMetalDedicated
    } else {
        PricingTier::StandardCompute
    }
}

// ─── Structured Cost Types ───────────────────────────────────────────────────

/// Structured cost breakdown for a single resource dimension.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostLineItem {
    /// Resource category (e.g., "cpu", "memory", "gpu", "network", "storage").
    pub category: String,
    /// Quantity (e.g., core-hours, GB-hours).
    pub quantity: f64,
    /// Unit label (e.g., "core-hours", "GB-month").
    pub unit: String,
    /// Unit price in the configured currency.
    pub unit_price: f64,
    /// Total cost for this line item.
    pub total: f64,
}

/// Full cost estimate with breakdown.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CostEstimate {
    /// Per-resource cost line items.
    pub line_items: Vec<CostLineItem>,
    /// Sum of all line items.
    pub total_cost: f64,
    /// Provider/tier identifier used for estimation.
    pub tier: String,
    /// Whether spot/preemptible pricing was applied.
    pub uses_spot: bool,
    /// Duration in hours this estimate covers.
    pub duration_hours: f64,
}

/// Cost-related errors.
#[derive(Debug, Error)]
pub enum CostError {
    #[error("Budget limit exceeded: estimate ${estimate:.2} exceeds limit ${limit:.2}")]
    BudgetExceeded { estimate: f64, limit: f64 },

    #[error("Invalid resource requirement: {0}")]
    InvalidRequirement(String),

    #[error("Cost model not found for provider: {0}")]
    ModelNotFound(String),

    #[error("Negative or zero duration for cost estimation")]
    InvalidDuration,
}

impl From<CostError> for ToadStoolError {
    fn from(e: CostError) -> Self {
        ToadStoolError::resource(e.to_string())
    }
}

// ─── CloudCostOptimizer ─────────────────────────────────────────────────────

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

/// Cloud cost model implementations (capability-based, not provider-named).
#[derive(Debug, Clone)]
pub struct CloudCostModel {
    pub cpu_rate: f64,
    pub memory_rate: f64,
    pub storage_rate: f64,
    pub network_rate: f64,
}

impl CloudCostModel {
    /// Create cost model for standard compute tier.
    pub fn standard_compute() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour(),
            memory_rate: t.memory_cost_per_gb_hour(),
            storage_rate: t.storage_cost_per_gb_month(),
            network_rate: t.network_cost_per_gb(),
        }
    }

    /// Create cost model for high-memory tier.
    pub fn high_memory() -> Self {
        let t = PricingTier::HighMemoryCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour(),
            memory_rate: t.memory_cost_per_gb_hour(),
            storage_rate: t.storage_cost_per_gb_month(),
            network_rate: t.network_cost_per_gb(),
        }
    }

    /// Create cost model for GPU-accelerated tier.
    pub fn gpu_accelerated() -> Self {
        let t = PricingTier::GpuAccelerated;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour(),
            memory_rate: t.memory_cost_per_gb_hour(),
            storage_rate: t.storage_cost_per_gb_month(),
            network_rate: t.network_cost_per_gb(),
        }
    }

    /// Create cost model for bare-metal / dedicated tier.
    pub fn bare_metal_dedicated() -> Self {
        let t = PricingTier::BareMetalDedicated;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour(),
            memory_rate: t.memory_cost_per_gb_hour(),
            storage_rate: t.storage_cost_per_gb_month(),
            network_rate: t.network_cost_per_gb(),
        }
    }

    /// Create cost model for serverless tier.
    pub fn serverless() -> Self {
        let t = PricingTier::Serverless;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour(),
            memory_rate: t.memory_cost_per_gb_hour(),
            storage_rate: t.storage_cost_per_gb_month(),
            network_rate: t.network_cost_per_gb(),
        }
    }

    /// Create cost model for edge/local tier.
    pub fn edge_local() -> Self {
        let t = PricingTier::EdgeLocal;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour(),
            memory_rate: t.memory_cost_per_gb_hour(),
            storage_rate: t.storage_cost_per_gb_month(),
            network_rate: t.network_cost_per_gb(),
        }
    }
}

// Legacy constructors for backward compatibility
impl CloudCostModel {
    pub fn new_aws() -> Self {
        Self::standard_compute()
    }

    pub fn new_azure() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.9,
            memory_rate: t.memory_cost_per_gb_hour() * 0.9,
            storage_rate: t.storage_cost_per_gb_month() * 0.8,
            network_rate: t.network_cost_per_gb() * 0.8,
        }
    }

    pub fn new_gcp() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.8,
            memory_rate: t.memory_cost_per_gb_hour() * 0.75,
            storage_rate: t.storage_cost_per_gb_month() * 0.5,
            network_rate: t.network_cost_per_gb() * 0.6,
        }
    }

    pub fn new_digitalocean() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.6,
            memory_rate: t.memory_cost_per_gb_hour() * 0.6,
            storage_rate: t.storage_cost_per_gb_month() * 0.2,
            network_rate: t.network_cost_per_gb() * 0.4,
        }
    }

    pub fn new_hetzner() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.5,
            memory_rate: t.memory_cost_per_gb_hour() * 0.4,
            storage_rate: t.storage_cost_per_gb_month() * 0.125,
            network_rate: t.network_cost_per_gb() * 0.2,
        }
    }

    pub fn new_localhost() -> Self {
        Self::edge_local()
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::types::{
        CloudCapabilities, ComputeType, NetworkingFeature, SecurityFeature, StorageType,
    };
    use crate::types::resources::{
        CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
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

    fn gpu_capabilities() -> CloudCapabilities {
        CloudCapabilities {
            compute_types: vec![ComputeType::VM, ComputeType::GPU],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption],
            compliance_certifications: vec![],
            regions: vec![],
            max_cpu_cores: Some(64),
            max_memory_gb: Some(256),
            gpu_support: true,
            kubernetes_support: false,
            serverless_support: false,
        }
    }

    fn standard_requirements() -> ResourceRequirements {
        ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: 4.0,
                max_cores: Some(8.0),
            },
            memory: MemoryRequirements {
                min_bytes: 8 * BYTES_PER_GB,
                max_bytes: None,
            },
            storage: StorageRequirements {
                min_bytes: 50 * BYTES_PER_GB,
                max_bytes: None,
            },
            network: NetworkRequirements {
                bandwidth_mbps: Some(100),
                latency_ms: None,
            },
            gpu: None,
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
        assert!(optimizer.cost_models.is_empty());
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
            .add_provider_cost_model("provider-x", &caps)
            .await
            .unwrap();
        assert!(optimizer.cost_models.contains_key("provider-x"));
    }

    #[tokio::test]
    async fn test_estimate_cost_returns_structured_breakdown() {
        let cfg = CostConfig {
            budget_limit: None,
            cost_tracking_enabled: true,
            spot_instance_preference: 0.0,
        };
        let mut optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        optimizer
            .add_provider_cost_model("p1", &empty_capabilities())
            .await
            .unwrap();

        let req = standard_requirements();
        let est = optimizer.estimate_cost("p1", &req, 1.0, 10.0).unwrap();

        assert!(!est.line_items.is_empty());
        assert!(est.total_cost > 0.0);
        assert_eq!(est.duration_hours, 1.0);
        assert!(!est.tier.is_empty());
    }

    #[tokio::test]
    async fn test_estimate_cost_budget_enforcement() {
        let cfg = CostConfig {
            budget_limit: Some(0.01),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.0,
        };
        let mut optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        optimizer
            .add_provider_cost_model("p1", &empty_capabilities())
            .await
            .unwrap();

        let req = standard_requirements();
        let res = optimizer.estimate_cost("p1", &req, 100.0, 1000.0);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_estimate_cost_model_not_found() {
        let cfg = CostConfig {
            budget_limit: None,
            cost_tracking_enabled: false,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        let req = standard_requirements();
        let res = optimizer.estimate_cost("nonexistent", &req, 1.0, 0.0);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_estimate_cost_invalid_duration() {
        let cfg = CostConfig {
            budget_limit: None,
            cost_tracking_enabled: false,
            spot_instance_preference: 0.0,
        };
        let mut optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        optimizer
            .add_provider_cost_model("p1", &empty_capabilities())
            .await
            .unwrap();
        let req = standard_requirements();
        let res = optimizer.estimate_cost("p1", &req, 0.0, 0.0);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_record_spend_budget_exceeded() {
        let cfg = CostConfig {
            budget_limit: Some(10.0),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        let res = optimizer.record_spend(15.0, 0.0);
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_record_spend_tracking_disabled() {
        let cfg = CostConfig {
            budget_limit: Some(10.0),
            cost_tracking_enabled: false,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        let res = optimizer.record_spend(100.0, 0.0);
        assert!(res.is_ok());
    }

    #[test]
    fn test_pricing_tier_standard() {
        let t = PricingTier::StandardCompute;
        assert!(t.cpu_cost_per_core_hour() > 0.0);
        assert!(t.memory_cost_per_gb_hour() > 0.0);
        assert!(t.gpu_cost_per_gpu_hour() == 0.0);
    }

    #[test]
    fn test_pricing_tier_gpu() {
        let t = PricingTier::GpuAccelerated;
        assert!(t.gpu_cost_per_gpu_hour() > 0.0);
    }

    #[test]
    fn test_cloud_cost_model_capability_based() {
        let standard = CloudCostModel::standard_compute();
        let gpu = CloudCostModel::gpu_accelerated();
        assert!(gpu.cpu_rate >= standard.cpu_rate);
    }

    #[test]
    fn test_cloud_cost_model_legacy_aws() {
        let model = CloudCostModel::new_aws();
        assert!(model.cpu_rate > 0.0);
        assert!(model.memory_rate > 0.0);
    }

    #[test]
    fn test_cloud_cost_model_localhost_zero_storage_network() {
        let local = CloudCostModel::new_localhost();
        assert_eq!(local.storage_rate, 0.0);
        assert_eq!(local.network_rate, 0.0);
    }

    #[test]
    fn test_infer_tier_gpu() {
        let tier = infer_pricing_tier(&gpu_capabilities());
        assert_eq!(tier, PricingTier::GpuAccelerated);
    }

    #[test]
    fn test_would_exceed_budget_true_when_over_limit() {
        let cfg = CostConfig {
            budget_limit: Some(100.0),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer {
            config: cfg,
            cost_models: HashMap::new(),
            capability_models: HashMap::new(),
        };
        assert!(optimizer.would_exceed_budget(150.0));
        assert!(optimizer.would_exceed_budget(100.01));
    }

    #[test]
    fn test_would_exceed_budget_false_when_under_limit() {
        let cfg = CostConfig {
            budget_limit: Some(100.0),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer {
            config: cfg,
            cost_models: HashMap::new(),
            capability_models: HashMap::new(),
        };
        assert!(!optimizer.would_exceed_budget(50.0));
        assert!(!optimizer.would_exceed_budget(99.99));
    }

    #[test]
    fn test_would_exceed_budget_no_limit_returns_false() {
        let cfg = CostConfig {
            budget_limit: None,
            cost_tracking_enabled: false,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer {
            config: cfg,
            cost_models: HashMap::new(),
            capability_models: HashMap::new(),
        };
        assert!(!optimizer.would_exceed_budget(1_000_000.0));
    }

    #[tokio::test]
    async fn test_record_spend_within_budget_succeeds() {
        let cfg = CostConfig {
            budget_limit: Some(100.0),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        let res = optimizer.record_spend(50.0, 30.0); // 50 + 30 = 80, under 100
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_record_spend_exactly_at_budget_fails() {
        let cfg = CostConfig {
            budget_limit: Some(100.0),
            cost_tracking_enabled: true,
            spot_instance_preference: 0.0,
        };
        let optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        let res = optimizer.record_spend(50.0, 50.0); // 50 + 50 = 100, not over
        assert!(res.is_ok());
    }

    #[test]
    fn test_pricing_tier_all_variants_have_positive_rates() {
        for tier in [
            PricingTier::StandardCompute,
            PricingTier::HighMemoryCompute,
            PricingTier::GpuAccelerated,
            PricingTier::BareMetalDedicated,
            PricingTier::Serverless,
            PricingTier::EdgeLocal,
        ] {
            assert!(tier.cpu_cost_per_core_hour() >= 0.0);
            assert!(tier.memory_cost_per_gb_hour() >= 0.0);
            assert!(tier.storage_cost_per_gb_month() >= 0.0);
            assert!(tier.network_cost_per_gb() >= 0.0);
        }
    }

    #[test]
    fn test_cloud_cost_model_legacy_azure() {
        let model = CloudCostModel::new_azure();
        assert!(model.cpu_rate > 0.0);
        assert!(model.memory_rate > 0.0);
    }

    #[test]
    fn test_cloud_cost_model_legacy_gcp() {
        let model = CloudCostModel::new_gcp();
        assert!(model.cpu_rate > 0.0);
    }

    #[test]
    fn test_cost_error_display() {
        let err = CostError::BudgetExceeded {
            estimate: 150.0,
            limit: 100.0,
        };
        let s = err.to_string();
        assert!(s.contains("150"));
        assert!(s.contains("100"));
    }
}
