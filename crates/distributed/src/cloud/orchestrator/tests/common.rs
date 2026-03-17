// SPDX-License-Identifier: AGPL-3.0-only
//! Shared test helpers and mocks for cloud orchestrator tests

use async_trait::async_trait;
use std::time::SystemTime;
use uuid::Uuid;

use crate::cloud::types::AvailabilityInfo;
use crate::cloud::{
    CloudOrchestratorConfig, CloudProviderInterface, ComplianceConfig, CostConfig, FederationConfig,
    HybridSchedulingStrategy, LoadBalancerConfig, LoadBalancingAlgorithm,
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
}

#[async_trait]
impl CloudProviderInterface for MockCloudProvider {
    async fn deploy_job(
        &self,
        job: &UniversalJob,
    ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobHandle> {
        Ok(crate::cloud::types::CloudJobHandle {
            job_id: job.job_id,
            provider_job_id: format!("mock-{}", Uuid::new_v4()),
            provider_name: self.name.clone(),
            created_at: SystemTime::now(),
        })
    }

    async fn get_job_status(
        &self,
        _handle: &crate::cloud::types::CloudJobHandle,
    ) -> toadstool::error::ToadStoolResult<crate::cloud::types::CloudJobStatus> {
        Ok(crate::cloud::types::CloudJobStatus::Running)
    }

    async fn scale_job(
        &self,
        _handle: &crate::cloud::types::CloudJobHandle,
        _scale_config: crate::cloud::types::ScaleConfig,
    ) -> toadstool::error::ToadStoolResult<()> {
        Ok(())
    }

    async fn terminate_job(
        &self,
        _handle: &crate::cloud::types::CloudJobHandle,
    ) -> toadstool::error::ToadStoolResult<()> {
        Ok(())
    }

    async fn get_pricing(
        &self,
        _resource_spec: &crate::cloud::types::ResourceSpec,
    ) -> toadstool::error::ToadStoolResult<crate::cloud::types::PricingInfo> {
        Ok(crate::cloud::types::PricingInfo {
            cpu_cost_per_hour: 0.1,
            memory_cost_per_gb_hour: 0.05,
            storage_cost_per_gb_month: 0.01,
            network_cost_per_gb: 0.02,
            total_estimated_cost: 10.0,
        })
    }

    async fn get_availability(
        &self,
        _region: Option<String>,
    ) -> toadstool::error::ToadStoolResult<AvailabilityInfo> {
        Ok(self.availability.clone())
    }

    async fn validate_compliance(
        &self,
        _requirements: &crate::ResourceRequirements,
    ) -> toadstool::error::ToadStoolResult<bool> {
        Ok(true)
    }

    fn get_capabilities(&self) -> crate::cloud::types::CloudCapabilities {
        make_mock_capabilities()
    }

    fn get_metadata(&self) -> crate::cloud::types::CloudProviderMetadata {
        make_mock_metadata(&self.name)
    }
}
