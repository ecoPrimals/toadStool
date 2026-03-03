// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based pricing tiers and cost models

use crate::cloud::types::{CloudCapabilities, ComputeType};

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
pub(crate) fn infer_pricing_tier(capabilities: &CloudCapabilities) -> PricingTier {
    if capabilities.gpu_support {
        PricingTier::GpuAccelerated
    } else if capabilities.serverless_support {
        PricingTier::Serverless
    } else if capabilities.max_memory_gb.is_some_and(|g| g >= 64) {
        PricingTier::HighMemoryCompute
    } else if capabilities
        .compute_types
        .iter()
        .any(|t| matches!(t, ComputeType::BareMetalC))
    {
        PricingTier::BareMetalDedicated
    } else {
        PricingTier::StandardCompute
    }
}

// ─── CloudCostModel ─────────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cloud::types::{
        CloudCapabilities, ComputeType, NetworkingFeature, SecurityFeature, StorageType,
    };

    fn caps_gpu() -> CloudCapabilities {
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

    fn caps_serverless() -> CloudCapabilities {
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
            serverless_support: true,
        }
    }

    fn caps_high_memory() -> CloudCapabilities {
        CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption],
            compliance_certifications: vec![],
            regions: vec![],
            max_cpu_cores: None,
            max_memory_gb: Some(128),
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        }
    }

    fn caps_bare_metal() -> CloudCapabilities {
        CloudCapabilities {
            compute_types: vec![ComputeType::VM, ComputeType::BareMetalC],
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

    #[test]
    fn test_pricing_tier_variants_cpu_cost() {
        assert_eq!(PricingTier::StandardCompute.cpu_cost_per_core_hour(), 0.08);
        assert_eq!(
            PricingTier::HighMemoryCompute.cpu_cost_per_core_hour(),
            0.12
        );
        assert_eq!(PricingTier::GpuAccelerated.cpu_cost_per_core_hour(), 0.15);
        assert_eq!(
            PricingTier::BareMetalDedicated.cpu_cost_per_core_hour(),
            0.25
        );
        assert!(PricingTier::Serverless.cpu_cost_per_core_hour() < 0.01);
        assert_eq!(PricingTier::EdgeLocal.cpu_cost_per_core_hour(), 0.01);
    }

    #[test]
    fn test_pricing_tier_variants_gpu_cost() {
        assert_eq!(PricingTier::GpuAccelerated.gpu_cost_per_gpu_hour(), 2.50);
        assert_eq!(
            PricingTier::BareMetalDedicated.gpu_cost_per_gpu_hour(),
            3.00
        );
        assert_eq!(PricingTier::StandardCompute.gpu_cost_per_gpu_hour(), 0.0);
    }

    #[test]
    fn test_infer_pricing_tier_gpu() {
        assert_eq!(infer_pricing_tier(&caps_gpu()), PricingTier::GpuAccelerated);
    }

    #[test]
    fn test_infer_pricing_tier_serverless() {
        assert_eq!(
            infer_pricing_tier(&caps_serverless()),
            PricingTier::Serverless
        );
    }

    #[test]
    fn test_infer_pricing_tier_high_memory() {
        assert_eq!(
            infer_pricing_tier(&caps_high_memory()),
            PricingTier::HighMemoryCompute
        );
    }

    #[test]
    fn test_infer_pricing_tier_bare_metal() {
        assert_eq!(
            infer_pricing_tier(&caps_bare_metal()),
            PricingTier::BareMetalDedicated
        );
    }

    #[test]
    fn test_infer_pricing_tier_standard_default() {
        let caps = CloudCapabilities {
            compute_types: vec![ComputeType::VM],
            storage_types: vec![StorageType::BlockStorage],
            networking_features: vec![NetworkingFeature::VPC],
            security_features: vec![SecurityFeature::Encryption],
            compliance_certifications: vec![],
            regions: vec![],
            max_cpu_cores: None,
            max_memory_gb: Some(32),
            gpu_support: false,
            kubernetes_support: false,
            serverless_support: false,
        };
        assert_eq!(infer_pricing_tier(&caps), PricingTier::StandardCompute);
    }

    #[test]
    fn test_cloud_cost_model_construction() {
        let standard = CloudCostModel::standard_compute();
        assert!(standard.cpu_rate > 0.0);
        assert!(standard.memory_rate > 0.0);
        assert!(standard.storage_rate > 0.0);
        assert!(standard.network_rate > 0.0);
    }

    #[test]
    fn test_cloud_cost_model_all_tier_constructors() {
        let models = [
            CloudCostModel::standard_compute(),
            CloudCostModel::high_memory(),
            CloudCostModel::gpu_accelerated(),
            CloudCostModel::bare_metal_dedicated(),
            CloudCostModel::serverless(),
            CloudCostModel::edge_local(),
        ];
        for m in models {
            assert!(m.cpu_rate >= 0.0);
            assert!(m.memory_rate >= 0.0);
            assert!(m.storage_rate >= 0.0);
            assert!(m.network_rate >= 0.0);
        }
    }

    #[test]
    fn test_price_calculation_accuracy_standard() {
        let model = CloudCostModel::standard_compute();
        let cpu_cost = 4.0 * 1.0 * model.cpu_rate;
        assert!((cpu_cost - 0.32).abs() < 0.001);
    }

    #[test]
    fn test_price_calculation_accuracy_gpu_tier() {
        let tier = PricingTier::GpuAccelerated;
        let gpu_cost = 2.0 * 24.0 * tier.gpu_cost_per_gpu_hour();
        assert!((gpu_cost - 120.0).abs() < 0.001);
    }

    #[test]
    fn test_cloud_cost_model_legacy_providers() {
        let aws = CloudCostModel::new_aws();
        let azure = CloudCostModel::new_azure();
        let gcp = CloudCostModel::new_gcp();
        assert!(azure.cpu_rate < aws.cpu_rate);
        assert!(gcp.cpu_rate < aws.cpu_rate);
    }

    #[test]
    fn test_edge_local_zero_storage_network() {
        let model = CloudCostModel::edge_local();
        assert_eq!(model.storage_rate, 0.0);
        assert_eq!(model.network_rate, 0.0);
    }
}
