// SPDX-License-Identifier: AGPL-3.0-only
//! Cost module tests

#[cfg(test)]
#[expect(
    clippy::float_cmp,
    clippy::module_inception,
    reason = "test module; comparing exact literals"
)]
mod tests {
    use super::super::optimizer::CloudCostOptimizer;
    use super::super::pricing::{infer_pricing_tier, CloudCostModel, PricingTier};
    use super::super::types::{CostError, CostEstimate, CostLineItem, BYTES_PER_GB};
    use crate::cloud::types::{
        CloudCapabilities, ComputeType, CostConfig, NetworkingFeature, SecurityFeature, StorageType,
    };
    use crate::types::resources::{
        CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
    };
    use std::collections::HashMap;
    use toadstool::error::ToadStoolError;

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

    fn standard_requirements() -> crate::types::resources::ResourceRequirements {
        crate::types::resources::ResourceRequirements {
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

    #[test]
    fn test_cost_line_item_serde() {
        let item = CostLineItem {
            category: "cpu".to_string(),
            quantity: 4.0,
            unit: "core-hours".to_string(),
            unit_price: 0.08,
            total: 0.32,
        };
        let json = serde_json::to_string(&item).unwrap();
        let parsed: CostLineItem = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.category, "cpu");
        assert!((parsed.total - 0.32).abs() < 0.001);
    }

    #[test]
    fn test_cost_estimate_serde() {
        let est = CostEstimate {
            line_items: vec![CostLineItem {
                category: "cpu".to_string(),
                quantity: 4.0,
                unit: "core-hours".to_string(),
                unit_price: 0.08,
                total: 0.32,
            }],
            total_cost: 0.32,
            tier: "StandardCompute".to_string(),
            uses_spot: true,
            duration_hours: 1.0,
        };
        let json = serde_json::to_string(&est).unwrap();
        let parsed: CostEstimate = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.total_cost, 0.32);
        assert!(parsed.uses_spot);
    }

    #[test]
    fn test_cost_error_variants() {
        let _ = CostError::InvalidRequirement("bad".to_string());
        let _ = CostError::ModelNotFound("p1".to_string());
        let _ = CostError::InvalidDuration;
        let s = CostError::InvalidRequirement("x".to_string()).to_string();
        assert!(s.contains("Invalid"));
    }

    #[test]
    fn test_infer_pricing_tier_serverless() {
        let caps = CloudCapabilities {
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
        };
        let tier = infer_pricing_tier(&caps);
        assert_eq!(tier, PricingTier::Serverless);
    }

    #[test]
    fn test_infer_pricing_tier_high_memory() {
        let caps = CloudCapabilities {
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
        };
        let tier = infer_pricing_tier(&caps);
        assert_eq!(tier, PricingTier::HighMemoryCompute);
    }

    #[test]
    fn test_infer_pricing_tier_bare_metal() {
        let caps = CloudCapabilities {
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
        };
        let tier = infer_pricing_tier(&caps);
        assert_eq!(tier, PricingTier::BareMetalDedicated);
    }

    #[test]
    fn test_cloud_cost_model_new_digitalocean() {
        let model = CloudCostModel::new_digitalocean();
        assert!(model.cpu_rate > 0.0);
        assert!(model.memory_rate > 0.0);
    }

    #[test]
    fn test_cloud_cost_model_new_hetzner() {
        let model = CloudCostModel::new_hetzner();
        assert!(model.cpu_rate > 0.0);
        assert!(model.storage_rate < 0.2);
    }

    #[tokio::test]
    async fn test_estimate_cost_with_spot_pricing() {
        let cfg = CostConfig {
            budget_limit: None,
            cost_tracking_enabled: false,
            spot_instance_preference: 1.0,
        };
        let mut optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
        optimizer
            .add_provider_cost_model("p1", &empty_capabilities())
            .await
            .unwrap();
        let req = standard_requirements();
        let est = optimizer.estimate_cost("p1", &req, 1.0, 0.0).unwrap();
        assert!(est.uses_spot);
        assert!(est.total_cost > 0.0);
    }

    #[test]
    fn test_cost_error_from_toadstool_error() {
        let err: ToadStoolError = CostError::InvalidDuration.into();
        let s = err.to_string();
        assert!(!s.is_empty());
    }
}
