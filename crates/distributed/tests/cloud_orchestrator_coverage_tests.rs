// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic)]
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for cloud orchestrator (cloud/orchestrator/mod.rs) - coverage target 90%
//!
//! Tests `deploy_universal_job`, `register_provider`, deployment strategies,
//! burst distribution, and multi-cloud paths.

use std::time::SystemTime;
use toadstool::ExecutionRequest;
use uuid::Uuid;

use std::time::Duration;
use super::common::{MockCloudProvider, TestUniversalOrchestrator};
use crate::cloud::types::AvailabilityInfo;
use crate::cloud::{
    CloudOrchestratorConfig, ComplianceConfig, CostConfig, FederationConfig,
    HybridSchedulingStrategy, LoadBalancerConfig, LoadBalancingAlgorithm,
};
use crate::types::resources::{
    CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
};
use crate::{ResourceRequirements, UniversalJob, UniversalJobType};

fn make_orchestrator_config() -> CloudOrchestratorConfig {
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

fn make_availability(cpu: f64, memory_gb: f64, storage_gb: f64) -> AvailabilityInfo {
    AvailabilityInfo {
        cpu_cores: cpu,
        memory_gb,
        storage_gb,
        gpu_count: 0,
        regions: vec![],
        availability_zones: vec![],
    }
}

fn make_requirements(cpu: f64, memory_bytes: u64, storage_bytes: u64) -> ResourceRequirements {
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

fn make_job(job_type: UniversalJobType) -> UniversalJob {
    UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: Some(job_type),
        execution_request: ExecutionRequest::default(),
        target: crate::ExecutionTarget::Local,
        priority: crate::JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    }
}

// ============================================================================
// Deploy with HybridCloudBurst - primary has capacity
// ============================================================================

#[tokio::test]
async fn test_deploy_burst_primary_has_capacity() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    let primary = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "aws".to_string(),
        availability: make_availability(32.0, 64.0, 500.0),
    };
    let burst = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "gcp".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    };
    orch.register_provider("aws".to_string(), primary)
        .await
        .unwrap();
    orch.register_provider("gcp".to_string(), burst)
        .await
        .unwrap();

    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Deploy with multiple providers - MultiCloud path
// ============================================================================

#[tokio::test]
async fn test_deploy_multi_cloud_with_multiple_allowed_regions() {
    let mut config = make_orchestrator_config();
    config.compliance_config.allowed_regions = vec![
        "us-east-1".to_string(),
        "eu-west-1".to_string(),
        "ap-south-1".to_string(),
    ];
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    let p1 = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    };
    let p2 = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "gcp".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    };
    orch.register_provider("aws".to_string(), p1).await.unwrap();
    orch.register_provider("gcp".to_string(), p2).await.unwrap();

    let job = make_job(UniversalJobType::RemoteToadStool {
        endpoint: "http://remote:8080".to_string(),
    });
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Job with None job_type - default path
// ============================================================================

#[tokio::test]
async fn test_deploy_job_with_none_type() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    let mock = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    };
    orch.register_provider("aws".to_string(), mock)
        .await
        .unwrap();

    let job = UniversalJob {
        job_id: Uuid::new_v4(),
        job_type: None,
        execution_request: ExecutionRequest::default(),
        target: crate::ExecutionTarget::Local,
        priority: crate::JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: make_requirements(
            4.0,
            8 * 1024 * 1024 * 1024,
            50 * 1024 * 1024 * 1024,
        ),
        retry_config: crate::types::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };

    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Provider availability failure - mark_provider_unavailable path
// ============================================================================

#[tokio::test]
async fn test_deploy_with_provider_availability_failure() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    let failing = MockCloudProvider {
        capabilities_override: None,
        fail_availability: true,
        name: "failing".to_string(),
        availability: make_availability(1.0, 1.0, 1.0),
    };
    orch.register_provider("failing".to_string(), failing)
        .await
        .unwrap();

    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_err());
}

// ─── Additional coverage via deploy path (can_handle / capacity checked internally) ─

/// Deploy with generous resources: exercises `can_handle_full_job` returning true.
#[tokio::test]
async fn test_deploy_generous_resource_match() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    let mock = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "generous".to_string(),
        availability: make_availability(64.0, 256.0, 2000.0),
    };
    orch.register_provider("generous".to_string(), mock)
        .await
        .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let _result = orch.deploy_universal_job(&job).await;
}

/// Deploy with insufficient resources: exercises error path.
#[tokio::test]
async fn test_deploy_insufficient_cpu_provider() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    let mock = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "small".to_string(),
        availability: make_availability(0.5, 1.0, 5.0),
    };
    orch.register_provider("small".to_string(), mock)
        .await
        .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let _result = orch.deploy_universal_job(&job).await;
}

#[tokio::test]
async fn test_deploy_ecosystem_tool_job_type() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    let mock = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "aws".to_string(),
        availability: make_availability(16.0, 32.0, 200.0),
    };
    orch.register_provider("aws".to_string(), mock)
        .await
        .unwrap();

    let job = make_job(UniversalJobType::EcosystemTool {
        tool_name: "biome".to_string(),
        endpoint: "http://biome:8080".to_string(),
    });
    let _result = orch.deploy_universal_job(&job).await;
}

#[tokio::test]
async fn test_deploy_storage_intensive_job() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    let mock = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "storage".to_string(),
        availability: make_availability(4.0, 8.0, 500.0),
    };
    orch.register_provider("storage".to_string(), mock)
        .await
        .unwrap();

    let job = make_job(UniversalJobType::StorageIntensive);
    let _result = orch.deploy_universal_job(&job).await;
}

#[tokio::test]
async fn test_register_provider_multiple() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    orch.register_provider(
        "p1".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "p1".to_string(),
            availability: make_availability(8.0, 16.0, 100.0),
        },
    )
    .await
    .unwrap();
    orch.register_provider(
        "p2".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "p2".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();

    let job = make_job(UniversalJobType::ComputeIntensive);
    let _result = orch.deploy_universal_job(&job).await;
}

#[tokio::test]
async fn test_deploy_remote_toadstool_job() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();

    let mock = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "remote".to_string(),
        availability: make_availability(8.0, 16.0, 100.0),
    };
    orch.register_provider("remote".to_string(), mock)
        .await
        .unwrap();

    let job = make_job(UniversalJobType::RemoteToadStool {
        endpoint: "http://remote:8080".to_string(),
    });
    let _result = orch.deploy_universal_job(&job).await;
}

/// Deploy with vastly over-provisioned resources: exercises capacity >> 1.0 path.
#[tokio::test]
async fn test_deploy_over_provisioned_resources() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    let mock = MockCloudProvider {
        capabilities_override: None,
        fail_availability: false,
        name: "mega".to_string(),
        availability: make_availability(100.0, 200.0, 1000.0),
    };
    orch.register_provider("mega".to_string(), mock)
        .await
        .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let _result = orch.deploy_universal_job(&job).await;
}

// ============================================================================
// Provider not found - scheduler returns "aws" but we register "gcp" only
// ============================================================================

/// When all registered providers fail availability, deploy returns an error
/// because no compliant providers remain after filtering.
#[tokio::test]
async fn test_deploy_all_providers_unavailable() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    let failing = MockCloudProvider {
        capabilities_override: None,
        fail_availability: true,
        name: "only-provider".to_string(),
        availability: make_availability(1.0, 1.0, 1.0),
    };
    orch.register_provider("only-provider".to_string(), failing)
        .await
        .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("No compliant providers"),
        "expected no-compliant-providers error, got: {err_msg}",
    );
}

// ============================================================================
// Cost config and optimization coverage
// ============================================================================

#[tokio::test]
async fn test_orchestrator_with_budget_limit() {
    let mut config = make_orchestrator_config();
    config.cost_config.budget_limit = Some(1000.0);
    config.cost_config.spot_instance_preference = 0.8;
    let orch = TestUniversalOrchestrator::new(config).await;
    assert!(orch.is_ok());
}

#[tokio::test]
async fn test_orchestrator_cost_optimized_strategy() {
    let mut config = make_orchestrator_config();
    config.scheduling_strategy = HybridSchedulingStrategy::CostOptimized;
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_orchestrator_performance_optimized_strategy() {
    let mut config = make_orchestrator_config();
    config.scheduling_strategy = HybridSchedulingStrategy::PerformanceOptimized;
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_orchestrator_compliance_first_strategy() {
    let mut config = make_orchestrator_config();
    config.scheduling_strategy = HybridSchedulingStrategy::ComplianceFirst;
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_orchestrator_geographic_affinity_strategy() {
    let mut config = make_orchestrator_config();
    config.scheduling_strategy = HybridSchedulingStrategy::GeographicAffinity {
        preferred_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
    };
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_orchestrator_latency_sensitive_strategy() {
    let mut config = make_orchestrator_config();
    config.scheduling_strategy = HybridSchedulingStrategy::LatencySensitive {
        max_latency_ms: 50,
        target_regions: vec!["us-east-1".to_string()],
    };
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_orchestrator_sustainability_strategy() {
    let mut config = make_orchestrator_config();
    config.scheduling_strategy = HybridSchedulingStrategy::SustainabilityFocused {
        renewable_energy_preference: 0.9,
    };
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::ComputeIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Federation config coverage
// ============================================================================

#[tokio::test]
async fn test_orchestrator_with_federation_config() {
    let mut config = make_orchestrator_config();
    config.federation_config.discovery_endpoints = vec!["https://fed.example.com".to_string()];
    config.federation_config.trust_anchors = vec!["anchor-1".to_string()];
    let orch = TestUniversalOrchestrator::new(config).await;
    assert!(orch.is_ok());
}

// ============================================================================
// Local job type - split_job_for_multi_cloud error path (when MultiCloud selected)
// Note: Current scheduler returns single provider, so Local jobs deploy to single cloud.
// This test verifies Local job deploys (succeeds to single cloud when aws registered).
// ============================================================================

#[tokio::test]
async fn test_deploy_local_job_single_cloud_succeeds() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::Local);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Network-intensive job type
// ============================================================================

#[tokio::test]
async fn test_deploy_network_intensive_job() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::NetworkIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Memory-intensive job type
// ============================================================================

#[tokio::test]
async fn test_deploy_memory_intensive_job() {
    let config = make_orchestrator_config();
    let mut orch = TestUniversalOrchestrator::new(config).await.unwrap();
    orch.register_provider(
        "aws".to_string(),
        MockCloudProvider {
            capabilities_override: None,
            fail_availability: false,
            name: "aws".to_string(),
            availability: make_availability(16.0, 32.0, 200.0),
        },
    )
    .await
    .unwrap();
    let job = make_job(UniversalJobType::MemoryIntensive);
    let result = orch.deploy_universal_job(&job).await;
    assert!(result.is_ok());
}
