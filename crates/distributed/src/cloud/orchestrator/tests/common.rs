// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared test helpers and mocks for cloud orchestrator tests

use std::future::Future;
use std::pin::Pin;
use std::time::SystemTime;
use uuid::Uuid;

use crate::cloud::types::AvailabilityInfo;
use crate::cloud::{
    CloudOrchestratorConfig, CloudProviderInterface, ComplianceConfig, CostConfig,
    FederationConfig, HybridSchedulingStrategy, LoadBalancerConfig, LoadBalancingAlgorithm,
};
use crate::types::resources::{
    CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};
use crate::{ResourceRequirements, UniversalJob};
use std::time::Duration;

pub fn make_orchestrator_config() -> CloudOrchestratorConfig {
    CloudOrchestratorConfig {
        scheduling_strategy: HybridSchedulingStrategy::Balanced {
            cost_weight: 0.33,
            performance_weight: 0.33,
            compliance_weight: 0.34,
        },
        cost_config: CostConfig {
            budget_limit: None,
            cost_tracking_enabled: true,
            spot_instance_preference: 0.5,
        },
        compliance_config: ComplianceConfig {
            required_certifications: vec![],
            allowed_regions: vec!["us-east-1".to_string()],
            data_sovereignty_requirements: vec![],
        },
        load_balancer_config: LoadBalancerConfig {
            algorithm: LoadBalancingAlgorithm::RoundRobin,
            health_check_interval: Duration::from_secs(10),
            failover_timeout: Duration::from_secs(30),
        },
        federation_config: FederationConfig {
            federation_id: "test-fed".to_string(),
            discovery_endpoints: vec![],
            trust_anchors: vec![],
        },
    }
}

pub fn make_availability(cpu: f64, memory_gb: f64, storage_gb: f64) -> AvailabilityInfo {
    AvailabilityInfo {
        cpu_cores: cpu,
        memory_gb,
        storage_gb,
        gpu_count: 0,
        regions: vec![],
        availability_zones: vec![],
    }
}

pub fn make_requirements(cpu: f64, memory_bytes: u64, storage_bytes: u64) -> ResourceRequirements {
    ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: cpu,
            max_cores: None,
        },
        memory: MemoryRequirements {
            min_bytes: memory_bytes,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: storage_bytes,
            max_bytes: None,
        },
        network: NetworkRequirements {
            bandwidth_mbps: None,
            latency_ms: None,
        },
        gpu: None,
    }
}

pub fn make_mock_capabilities() -> crate::cloud::types::CloudCapabilities {
    use crate::cloud::types::{
        ComplianceCertification, ComputeType, NetworkingFeature, Region, SecurityFeature,
        StorageType,
    };
    crate::cloud::types::CloudCapabilities {
        compute_types: vec![ComputeType::VM, ComputeType::Container],
        storage_types: vec![StorageType::BlockStorage, StorageType::ObjectStorage],
        networking_features: vec![NetworkingFeature::VPC, NetworkingFeature::LoadBalancer],
        security_features: vec![SecurityFeature::Encryption, SecurityFeature::Compliance],
        compliance_certifications: vec![
            ComplianceCertification::SOC2,
            ComplianceCertification::ISO27001,
        ],
        regions: vec![Region {
            name: "us-east-1".to_string(),
            location: "Virginia".to_string(),
            availability_zones: vec!["us-east-1a".to_string(), "us-east-1b".to_string()],
        }],
        max_cpu_cores: Some(256),
        max_memory_gb: Some(1024),
        gpu_support: true,
        kubernetes_support: true,
        serverless_support: false,
    }
}

pub fn make_mock_metadata(name: &str) -> crate::cloud::types::CloudProviderMetadata {
    crate::cloud::types::CloudProviderMetadata {
        name: name.to_string(),
        version: "1.0".to_string(),
        api_version: "v1".to_string(),
        supported_protocols: vec!["rest".to_string(), "grpc".to_string()],
        documentation_url: "https://example.com/docs".to_string(),
        support_contact: "support@example.com".to_string(),
    }
}

pub struct MockCloudProvider {
    pub name: String,
    pub availability: AvailabilityInfo,
    /// When set, returned from [`CloudProviderInterface::get_capabilities`] instead of [`make_mock_capabilities`].
    pub capabilities_override: Option<crate::cloud::types::CloudCapabilities>,
}

impl CloudProviderInterface for MockCloudProvider {
    fn deploy_job<'a>(
        &'a self,
        job: &'a UniversalJob,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobHandle>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(crate::cloud::types::CloudJobHandle {
                job_id: job.job_id,
                provider_job_id: format!("mock-{}", Uuid::new_v4()),
                provider_name: self.name.clone(),
                created_at: SystemTime::now(),
            })
        })
    }

    fn get_job_status<'a>(
        &'a self,
        _handle: &'a crate::cloud::types::CloudJobHandle,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobStatus>,
                > + Send
                + 'a,
        >,
    > {
        Box::pin(async move { Ok(crate::cloud::types::CloudJobStatus::Running) })
    }

    fn scale_job<'a>(
        &'a self,
        _handle: &'a crate::cloud::types::CloudJobHandle,
        _scale_config: crate::cloud::types::ScaleConfig,
    ) -> Pin<Box<dyn Future<Output = toadstool::error::ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn terminate_job<'a>(
        &'a self,
        _handle: &'a crate::cloud::types::CloudJobHandle,
    ) -> Pin<Box<dyn Future<Output = toadstool::error::ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }

    fn get_pricing<'a>(
        &'a self,
        _resource_spec: &'a crate::cloud::types::ResourceSpec,
    ) -> Pin<
        Box<
            dyn Future<Output = toadstool::error::ToadStoolResult<crate::cloud::types::PricingInfo>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            Ok(crate::cloud::types::PricingInfo {
                cpu_cost_per_hour: 0.1,
                memory_cost_per_gb_hour: 0.05,
                storage_cost_per_gb_month: 0.01,
                network_cost_per_gb: 0.02,
                total_estimated_cost: 10.0,
            })
        })
    }

    fn get_availability<'a>(
        &'a self,
        _region: Option<String>,
    ) -> Pin<
        Box<dyn Future<Output = toadstool::error::ToadStoolResult<AvailabilityInfo>> + Send + 'a>,
    > {
        let availability = self.availability.clone();
        Box::pin(async move { Ok(availability) })
    }

    fn validate_compliance<'a>(
        &'a self,
        _requirements: &'a crate::ResourceRequirements,
    ) -> Pin<Box<dyn Future<Output = toadstool::error::ToadStoolResult<bool>> + Send + 'a>> {
        Box::pin(async move { Ok(true) })
    }

    fn get_capabilities(&self) -> crate::cloud::types::CloudCapabilities {
        self.capabilities_override
            .clone()
            .unwrap_or_else(make_mock_capabilities)
    }

    fn get_metadata(&self) -> crate::cloud::types::CloudProviderMetadata {
        make_mock_metadata(&self.name)
    }
}
