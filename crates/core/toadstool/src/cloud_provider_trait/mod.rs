// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

//! Cloud Provider Trait
//!
//! This module defines the trait for cloud compute providers, enabling
//! vendor-agnostic cloud integration.
//!
//! # Philosophy
//!
//! **Discovery Over Hardcoding**: Providers are discovered at runtime and
//! implement a common trait. No hardcoded vendor logic in the core.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::cloud_provider_trait::{CloudProvider, CloudCapabilities};
//!
//! # async fn example(provider: Box<dyn CloudProvider>) -> Result<(), Box<dyn std::error::Error>> {
//! // Query provider capabilities
//! let caps = provider.capabilities().await?;
//! println!("Provider: {}", caps.name);
//! println!("Regions: {:?}", caps.available_regions);
//! println!("Has GPU: {}", caps.supports_gpu);
//!
//! // Deploy workload
//! let instance_id = provider.deploy_workload("my-workload", &caps.available_regions[0]).await?;
//! println!("Deployed: {}", instance_id);
//! # Ok(())
//! # }
//! ```

mod provider;
mod registry;
mod types;

pub use provider::CloudProvider;
pub use registry::CloudProviderRegistry;
pub use types::{
    CloudCapabilities, CloudError, CostEstimate, GpuType, WorkloadHealth, WorkloadLocation,
    WorkloadSpec,
};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::collections::HashMap;

    // Mock provider for testing
    struct MockProvider {
        name: String,
    }

    // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
    #[async_trait]
    impl CloudProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn capabilities(&self) -> Result<CloudCapabilities, CloudError> {
            Ok(CloudCapabilities {
                name: self.name.clone(),
                available_regions: vec!["us-west-1".to_string()],
                supports_gpu: true,
                gpu_types: vec!["V100".to_string()],
                max_memory_gb: 256.0,
                max_cpu_cores: 64,
                supports_spot_instances: true,
                supports_autoscaling: true,
                custom: HashMap::new(),
            })
        }

        async fn deploy_workload(
            &self,
            workload_id: &str,
            _region: &str,
        ) -> Result<String, CloudError> {
            Ok(format!("instance-{workload_id}"))
        }

        async fn migrate_workload(
            &self,
            workload_id: &str,
            _source: WorkloadLocation,
            _target_region: &str,
        ) -> Result<String, CloudError> {
            Ok(format!("migrated-{workload_id}"))
        }

        async fn check_health(&self, _instance_id: &str) -> Result<WorkloadHealth, CloudError> {
            Ok(WorkloadHealth::Healthy)
        }

        async fn terminate_workload(&self, _instance_id: &str) -> Result<(), CloudError> {
            Ok(())
        }

        async fn estimate_cost(
            &self,
            _workload_spec: &WorkloadSpec,
            _region: &str,
        ) -> Result<CostEstimate, CloudError> {
            Ok(CostEstimate {
                cost_per_hour: 5.0,
                estimated_total_cost: Some(10.0),
                breakdown: HashMap::new(),
            })
        }

        async fn available_gpu_types(&self, _region: &str) -> Result<Vec<GpuType>, CloudError> {
            Ok(vec![GpuType {
                name: "V100".to_string(),
                memory_gb: 16.0,
                compute_capability: Some("7.0".to_string()),
                cost_per_hour: 3.0,
                available_regions: vec!["us-west-1".to_string()],
            }])
        }
    }

    #[tokio::test]
    async fn test_mock_provider() {
        let provider = MockProvider {
            name: "TestCloud".to_string(),
        };

        assert_eq!(provider.name(), "TestCloud");

        let caps = provider.capabilities().await.unwrap();
        assert_eq!(caps.name, "TestCloud");
        assert!(caps.supports_gpu);
    }

    #[tokio::test]
    async fn test_provider_registry() {
        let mut registry = CloudProviderRegistry::new();

        let provider = Box::new(MockProvider {
            name: "TestCloud".to_string(),
        });

        registry.register(provider);

        assert!(registry.has_provider("TestCloud"));
        assert_eq!(registry.available_providers().len(), 1);

        let retrieved = registry.get("TestCloud");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "TestCloud");
    }

    #[test]
    fn test_workload_health() {
        let healthy = WorkloadHealth::Healthy;
        assert_eq!(healthy, WorkloadHealth::Healthy);

        let degraded = WorkloadHealth::Degraded {
            reason: "high latency".to_string(),
        };
        assert!(matches!(degraded, WorkloadHealth::Degraded { .. }));
    }

    #[test]
    fn test_cloud_error() {
        let err = CloudError::ProviderUnavailable("AWS".to_string());
        assert!(err.to_string().contains("AWS"));
    }

    #[test]
    fn test_cloud_error_all_variants() {
        assert!(
            CloudError::RegionUnsupported("eu-west".to_string())
                .to_string()
                .contains("eu-west")
        );
        assert!(
            CloudError::InsufficientCapacity("no GPU".to_string())
                .to_string()
                .contains("no GPU")
        );
        assert!(
            CloudError::DeploymentFailed("timeout".to_string())
                .to_string()
                .contains("timeout")
        );
        assert!(
            CloudError::MigrationFailed("network".to_string())
                .to_string()
                .contains("network")
        );
        assert!(
            CloudError::AuthenticationFailed("invalid key".to_string())
                .to_string()
                .contains("invalid key")
        );
        assert!(
            CloudError::NetworkError("refused".to_string())
                .to_string()
                .contains("refused")
        );
        assert!(
            CloudError::InvalidConfiguration("missing".to_string())
                .to_string()
                .contains("missing")
        );
        assert!(
            CloudError::Unknown("mystery".to_string())
                .to_string()
                .contains("mystery")
        );
    }

    #[test]
    fn test_workload_location_cloud() {
        let loc = WorkloadLocation::Cloud {
            provider: "AWS".to_string(),
            region: "us-east-1".to_string(),
            instance_id: "i-abc123".to_string(),
        };
        match &loc {
            WorkloadLocation::Cloud {
                provider,
                region,
                instance_id,
            } => {
                assert_eq!(provider, "AWS");
                assert_eq!(region, "us-east-1");
                assert_eq!(instance_id, "i-abc123");
            }
            _ => unreachable!("Expected Cloud variant"),
        }
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparison intended in this context"
    )] // literals just assigned in test
    fn test_workload_spec_construction() {
        let spec = WorkloadSpec {
            id: "wl-1".to_string(),
            memory_gb: 8.0,
            cpu_cores: 4,
            requires_gpu: true,
            preferred_gpu_type: Some("A100".to_string()),
            estimated_runtime_hours: Some(2.5),
            custom: HashMap::new(),
        };
        assert_eq!(spec.id, "wl-1");
        assert_eq!(spec.memory_gb, 8.0);
        assert!(spec.requires_gpu);
        assert_eq!(spec.preferred_gpu_type.as_deref(), Some("A100"));
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparison intended in this context"
    )] // literals just assigned in test
    fn test_cost_estimate_construction() {
        let mut breakdown = HashMap::new();
        breakdown.insert("compute".to_string(), 5.0);
        let est = CostEstimate {
            cost_per_hour: 5.0,
            estimated_total_cost: Some(50.0),
            breakdown,
        };
        assert_eq!(est.cost_per_hour, 5.0);
        assert_eq!(est.estimated_total_cost, Some(50.0));
        assert_eq!(est.breakdown.get("compute"), Some(&5.0));
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparison intended in this context"
    )] // literal just assigned in test
    fn test_gpu_type_construction() {
        let gpu = GpuType {
            name: "A100".to_string(),
            memory_gb: 40.0,
            compute_capability: Some("8.0".to_string()),
            cost_per_hour: 3.5,
            available_regions: vec!["us-west-1".to_string()],
        };
        assert_eq!(gpu.name, "A100");
        assert_eq!(gpu.memory_gb, 40.0);
    }

    #[test]
    fn test_cloud_provider_registry_default() {
        let registry = CloudProviderRegistry::default();
        assert!(registry.available_providers().is_empty());
    }

    #[test]
    fn test_workload_health_unhealthy() {
        let health = WorkloadHealth::Unhealthy {
            reason: "crash".to_string(),
        };
        assert!(matches!(health, WorkloadHealth::Unhealthy { .. }));
    }

    #[test]
    fn test_workload_health_unknown() {
        let health = WorkloadHealth::Unknown;
        assert_eq!(health, WorkloadHealth::Unknown);
    }
}
