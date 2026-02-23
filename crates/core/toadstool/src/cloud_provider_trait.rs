// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

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

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cloud provider trait
///
/// All cloud providers (AWS, GCP, Azure, etc.) implement this trait.
#[async_trait]
pub trait CloudProvider: Send + Sync {
    /// Get provider name (e.g., "AWS", "GCP", "Azure")
    fn name(&self) -> &str;

    /// Get provider capabilities
    async fn capabilities(&self) -> Result<CloudCapabilities, CloudError>;

    /// Deploy a workload to this provider
    ///
    /// Returns instance/deployment ID
    async fn deploy_workload(&self, workload_id: &str, region: &str) -> Result<String, CloudError>;

    /// Migrate workload from another location
    async fn migrate_workload(
        &self,
        workload_id: &str,
        source: WorkloadLocation,
        target_region: &str,
    ) -> Result<String, CloudError>;

    /// Check workload health
    async fn check_health(&self, instance_id: &str) -> Result<WorkloadHealth, CloudError>;

    /// Terminate workload
    async fn terminate_workload(&self, instance_id: &str) -> Result<(), CloudError>;

    /// Estimate cost for workload
    async fn estimate_cost(
        &self,
        workload_spec: &WorkloadSpec,
        region: &str,
    ) -> Result<CostEstimate, CloudError>;

    /// Get available GPU types
    async fn available_gpu_types(&self, region: &str) -> Result<Vec<GpuType>, CloudError>;
}

/// Cloud provider capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCapabilities {
    /// Provider name
    pub name: String,

    /// Available regions
    pub available_regions: Vec<String>,

    /// Supports GPU instances
    pub supports_gpu: bool,

    /// Available GPU types
    pub gpu_types: Vec<String>,

    /// Maximum memory per instance (GB)
    pub max_memory_gb: f64,

    /// Maximum CPU cores per instance
    pub max_cpu_cores: usize,

    /// Supports spot/preemptible instances
    pub supports_spot_instances: bool,

    /// Supports auto-scaling
    pub supports_autoscaling: bool,

    /// Custom capabilities (provider-specific)
    pub custom: HashMap<String, String>,
}

/// Workload location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadLocation {
    /// Local (bare metal, VM, container)
    Local { hostname: String },

    /// Cloud provider
    Cloud {
        provider: String,
        region: String,
        instance_id: String,
    },
}

/// Workload health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WorkloadHealth {
    /// Healthy and running
    Healthy,

    /// Degraded performance
    Degraded { reason: String },

    /// Unhealthy/failing
    Unhealthy { reason: String },

    /// Unknown status
    Unknown,
}

/// Workload specification for deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadSpec {
    /// Workload ID
    pub id: String,

    /// Required memory (GB)
    pub memory_gb: f64,

    /// Required CPU cores
    pub cpu_cores: usize,

    /// GPU required
    pub requires_gpu: bool,

    /// Preferred GPU type
    pub preferred_gpu_type: Option<String>,

    /// Estimated runtime (hours)
    pub estimated_runtime_hours: Option<f64>,

    /// Custom requirements
    pub custom: HashMap<String, String>,
}

/// Cost estimate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    /// Estimated cost per hour (USD)
    pub cost_per_hour: f64,

    /// Estimated total cost (USD)
    pub estimated_total_cost: Option<f64>,

    /// Breakdown by resource
    pub breakdown: HashMap<String, f64>,
}

/// GPU type information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuType {
    /// GPU model name
    pub name: String,

    /// Memory (GB)
    pub memory_gb: f64,

    /// Compute capability
    pub compute_capability: Option<String>,

    /// Cost per hour (USD)
    pub cost_per_hour: f64,

    /// Available in regions
    pub available_regions: Vec<String>,
}

/// Cloud provider error
#[derive(Debug, Clone, thiserror::Error)]
pub enum CloudError {
    /// Provider not available
    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),

    /// Region not supported
    #[error("Region not supported: {0}")]
    RegionUnsupported(String),

    /// Insufficient quota/capacity
    #[error("Insufficient capacity: {0}")]
    InsufficientCapacity(String),

    /// Deployment failed
    #[error("Deployment failed: {0}")]
    DeploymentFailed(String),

    /// Migration failed
    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),

    /// Unknown error
    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Cloud provider registry
///
/// Maintains a registry of available cloud providers.
pub struct CloudProviderRegistry {
    providers: HashMap<String, Box<dyn CloudProvider>>,
}

impl CloudProviderRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider
    pub fn register(&mut self, provider: Box<dyn CloudProvider>) {
        let name = provider.name().to_string();
        self.providers.insert(name, provider);
    }

    /// Get provider by name
    pub fn get(&self, name: &str) -> Option<&dyn CloudProvider> {
        self.providers.get(name).map(|p| p.as_ref())
    }

    /// Get all provider names
    pub fn available_providers(&self) -> Vec<String> {
        self.providers.keys().cloned().collect()
    }

    /// Check if provider is available
    pub fn has_provider(&self, name: &str) -> bool {
        self.providers.contains_key(name)
    }
}

impl Default for CloudProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock provider for testing
    struct MockProvider {
        name: String,
    }

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
            Ok(format!("instance-{}", workload_id))
        }

        async fn migrate_workload(
            &self,
            workload_id: &str,
            _source: WorkloadLocation,
            _target_region: &str,
        ) -> Result<String, CloudError> {
            Ok(format!("migrated-{}", workload_id))
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
        assert!(CloudError::RegionUnsupported("eu-west".to_string())
            .to_string()
            .contains("eu-west"));
        assert!(CloudError::InsufficientCapacity("no GPU".to_string())
            .to_string()
            .contains("no GPU"));
        assert!(CloudError::DeploymentFailed("timeout".to_string())
            .to_string()
            .contains("timeout"));
        assert!(CloudError::MigrationFailed("network".to_string())
            .to_string()
            .contains("network"));
        assert!(CloudError::AuthenticationFailed("invalid key".to_string())
            .to_string()
            .contains("invalid key"));
        assert!(CloudError::NetworkError("refused".to_string())
            .to_string()
            .contains("refused"));
        assert!(CloudError::InvalidConfiguration("missing".to_string())
            .to_string()
            .contains("missing"));
        assert!(CloudError::Unknown("mystery".to_string())
            .to_string()
            .contains("mystery"));
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
            _ => panic!("Expected Cloud variant"),
        }
    }

    #[test]
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
