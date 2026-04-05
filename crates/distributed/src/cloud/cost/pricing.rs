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
    pub const fn cpu_cost_per_core_hour(self) -> f64 {
        match self {
            Self::StandardCompute => 0.08,
            Self::HighMemoryCompute => 0.12,
            Self::GpuAccelerated => 0.15,
            Self::BareMetalDedicated => 0.25,
            Self::Serverless => 0.0001, // Per-invocation dominated; use small base
            Self::EdgeLocal => 0.01,
        }
    }

    /// Returns the memory cost per GB-hour for this tier.
    pub const fn memory_cost_per_gb_hour(self) -> f64 {
        match self {
            Self::StandardCompute => 0.012,
            Self::HighMemoryCompute => 0.018,
            Self::GpuAccelerated => 0.020,
            Self::BareMetalDedicated => 0.030,
            Self::Serverless => 0.000016,
            Self::EdgeLocal => 0.002,
        }
    }

    /// Returns the storage cost per GB-month for this tier.
    pub const fn storage_cost_per_gb_month(self) -> f64 {
        match self {
            Self::StandardCompute => 0.08,
            Self::HighMemoryCompute => 0.10,
            Self::GpuAccelerated => 0.08,
            Self::BareMetalDedicated => 0.15,
            Self::Serverless => 0.023,
            Self::EdgeLocal => 0.0,
        }
    }

    /// Returns the network cost per GB for this tier.
    pub const fn network_cost_per_gb(self) -> f64 {
        match self {
            Self::StandardCompute => 0.05,
            Self::HighMemoryCompute => 0.05,
            Self::GpuAccelerated => 0.06,
            Self::BareMetalDedicated => 0.04,
            Self::Serverless => 0.09,
            Self::EdgeLocal => 0.0,
        }
    }

    /// Returns the GPU cost per GPU-hour for tiers that support GPU.
    pub const fn gpu_cost_per_gpu_hour(self) -> f64 {
        match self {
            Self::GpuAccelerated => 2.50,
            Self::BareMetalDedicated => 3.00,
            _ => 0.0,
        }
    }
}

/// Selects a [`PricingTier`] from advertised [`CloudCapabilities`].
pub fn infer_pricing_tier(capabilities: &CloudCapabilities) -> PricingTier {
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
    /// CPU cost per core-hour (currency units).
    pub cpu_rate: f64,
    /// Memory cost per GB-hour.
    pub memory_rate: f64,
    /// Storage cost per GB-month.
    pub storage_rate: f64,
    /// Network cost per GB transferred.
    pub network_rate: f64,
}

impl CloudCostModel {
    /// Create cost model for standard compute tier.
    pub const fn standard_compute() -> Self {
        Self {
            cpu_rate: PricingTier::StandardCompute.cpu_cost_per_core_hour(),
            memory_rate: PricingTier::StandardCompute.memory_cost_per_gb_hour(),
            storage_rate: PricingTier::StandardCompute.storage_cost_per_gb_month(),
            network_rate: PricingTier::StandardCompute.network_cost_per_gb(),
        }
    }

    /// Create cost model for high-memory tier.
    pub const fn high_memory() -> Self {
        Self {
            cpu_rate: PricingTier::HighMemoryCompute.cpu_cost_per_core_hour(),
            memory_rate: PricingTier::HighMemoryCompute.memory_cost_per_gb_hour(),
            storage_rate: PricingTier::HighMemoryCompute.storage_cost_per_gb_month(),
            network_rate: PricingTier::HighMemoryCompute.network_cost_per_gb(),
        }
    }

    /// Create cost model for GPU-accelerated tier.
    pub const fn gpu_accelerated() -> Self {
        Self {
            cpu_rate: PricingTier::GpuAccelerated.cpu_cost_per_core_hour(),
            memory_rate: PricingTier::GpuAccelerated.memory_cost_per_gb_hour(),
            storage_rate: PricingTier::GpuAccelerated.storage_cost_per_gb_month(),
            network_rate: PricingTier::GpuAccelerated.network_cost_per_gb(),
        }
    }

    /// Create cost model for bare-metal / dedicated tier.
    pub const fn bare_metal_dedicated() -> Self {
        Self {
            cpu_rate: PricingTier::BareMetalDedicated.cpu_cost_per_core_hour(),
            memory_rate: PricingTier::BareMetalDedicated.memory_cost_per_gb_hour(),
            storage_rate: PricingTier::BareMetalDedicated.storage_cost_per_gb_month(),
            network_rate: PricingTier::BareMetalDedicated.network_cost_per_gb(),
        }
    }

    /// Create cost model for serverless tier.
    pub const fn serverless() -> Self {
        Self {
            cpu_rate: PricingTier::Serverless.cpu_cost_per_core_hour(),
            memory_rate: PricingTier::Serverless.memory_cost_per_gb_hour(),
            storage_rate: PricingTier::Serverless.storage_cost_per_gb_month(),
            network_rate: PricingTier::Serverless.network_cost_per_gb(),
        }
    }

    /// Create cost model for edge/local tier.
    pub const fn edge_local() -> Self {
        Self {
            cpu_rate: PricingTier::EdgeLocal.cpu_cost_per_core_hour(),
            memory_rate: PricingTier::EdgeLocal.memory_cost_per_gb_hour(),
            storage_rate: PricingTier::EdgeLocal.storage_cost_per_gb_month(),
            network_rate: PricingTier::EdgeLocal.network_cost_per_gb(),
        }
    }
}

// Legacy constructors for backward compatibility
impl CloudCostModel {
    /// Legacy alias for [`Self::standard_compute`] (AWS-style baseline).
    pub const fn new_aws() -> Self {
        Self::standard_compute()
    }

    /// Approximate Azure-relative rates vs standard compute.
    pub fn new_azure() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.9,
            memory_rate: t.memory_cost_per_gb_hour() * 0.9,
            storage_rate: t.storage_cost_per_gb_month() * 0.8,
            network_rate: t.network_cost_per_gb() * 0.8,
        }
    }

    /// Approximate GCP-relative rates vs standard compute.
    pub fn new_gcp() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.8,
            memory_rate: t.memory_cost_per_gb_hour() * 0.75,
            storage_rate: t.storage_cost_per_gb_month() * 0.5,
            network_rate: t.network_cost_per_gb() * 0.6,
        }
    }

    /// Approximate DigitalOcean-relative rates vs standard compute.
    pub fn new_digitalocean() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.6,
            memory_rate: t.memory_cost_per_gb_hour() * 0.6,
            storage_rate: t.storage_cost_per_gb_month() * 0.2,
            network_rate: t.network_cost_per_gb() * 0.4,
        }
    }

    /// Approximate Hetzner-relative rates vs standard compute.
    pub fn new_hetzner() -> Self {
        let t = PricingTier::StandardCompute;
        Self {
            cpu_rate: t.cpu_cost_per_core_hour() * 0.5,
            memory_rate: t.memory_cost_per_gb_hour() * 0.4,
            storage_rate: t.storage_cost_per_gb_month() * 0.125,
            network_rate: t.network_cost_per_gb() * 0.2,
        }
    }

    /// Legacy alias for [`Self::edge_local`] (local/minimal cloud cost).
    pub const fn new_localhost() -> Self {
        Self::edge_local()
    }
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
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
