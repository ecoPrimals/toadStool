// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared test helpers and mocks for cloud orchestrator tests

use crate::ResourceRequirements;
use crate::cloud::types::AvailabilityInfo;
use crate::cloud::{
    CloudOrchestratorConfig, ComplianceConfig, CostConfig, FederationConfig,
    HybridSchedulingStrategy, LoadBalancerConfig, LoadBalancingAlgorithm,
};
use crate::types::resources::{
    CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};
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
        federation_endpoint: None,
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

/// Re-export for orchestrator tests.
pub use crate::cloud::test_mocks::MockCloudProvider;

/// [`UniversalCloudOrchestrator`] with [`MockCloudProvider`] — use for type inference in tests.
pub use crate::cloud::TestUniversalOrchestrator;
