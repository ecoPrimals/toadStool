// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for under-covered modules in toadstool-distributed (phase 2)
//!
//! Targets: coordination_integration/client/rpc, coordination (distribution,
//! connection, discovery, broadcasting, load_balancing), universal/scheduler,
//! universal/detection, network/distributor, cloud/orchestrator, cloud/compliance/validation,
//! cloud/cost/optimizer, primal_capabilities/workload

#![allow(clippy::pedantic)]
#![allow(deprecated)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use toadstool_distributed::cloud::{
    CheckResult, CloudCapabilities, CloudComplianceEnforcer, CloudCostOptimizer,
    CloudOrchestratorConfig, ComplianceCheck, ComplianceConfig, ComputeType, CostConfig,
    FederationConfig, HybridSchedulingStrategy, LoadBalancerConfig as CloudLoadBalancerConfig,
    LoadBalancingAlgorithm, NetworkingFeature, Region, SecurityFeature, StorageType,
};
use toadstool_distributed::coordination::{
    BroadcastConfig, ConnectionHealth, CoordinationBroadcaster, CoordinationConnection,
    CoordinationConnectionConfig, CoordinationLoadBalancer, CoordinationTransport,
    DistributionConfig, GrpcProtocolConfig, HttpProtocolConfig, JobAnalysis, JobComplexity,
    JobDistributionStrategy, LoadBalancerConfig as CoordinationLoadBalancerConfig,
    MassiveJobDistributor, MessageQueueProtocolConfig, ProtocolConfig,
};
use toadstool_distributed::network::{NetworkDistributor, NetworkDistributorConfig, NodeHealth};
use toadstool_distributed::primal_capabilities::workload::{
    WorkloadResourceRequirements, WorkloadStatus, WorkloadType,
};
use toadstool_distributed::primal_capabilities::{WorkloadExecutor, WorkloadRequest};
use toadstool_distributed::types::{
    CpuRequirements, MemoryRequirements, NetworkRequirements, ResourceRequirements,
    StorageRequirements,
};
use toadstool_distributed::universal::{
    NetworkEffectsConfig, OSLayerConfig, SchedulingAlgorithm, UniversalScheduler,
    UniversalSchedulerConfig,
};

// ============================================================================
// Coordination RPC Types (serialization, no live RPC)
// ============================================================================

#[test]
fn test_coordination_types_service_registration_serialization() {
    use toadstool_distributed::coordination_integration::types::ServiceRegistration;

    let reg = ServiceRegistration {
        service_id: "s155b-svc".to_string(),
        service_name: "Test Service".to_string(),
        version: "1.0".to_string(),
        capabilities: vec!["compute".to_string(), "storage".to_string()],
        endpoints: vec![],
        metadata: HashMap::new(),
        ttl_seconds: 60,
    };
    let json = serde_json::to_value(&reg).unwrap();
    assert!(json.get("service_id").is_some());
    assert_eq!(
        json.get("service_id").and_then(|v| v.as_str()),
        Some("s155b-svc")
    );

    let parsed: ServiceRegistration = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.service_id, reg.service_id);
    assert_eq!(parsed.ttl_seconds, 60);
}

#[test]
fn test_coordination_types_load_balancing_strategy_variants() {
    use toadstool_distributed::coordination_integration::types::LoadBalancingStrategy;

    let strategies = [
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::LeastResponseTime,
        LoadBalancingStrategy::Random,
    ];
    for s in &strategies {
        let json = serde_json::to_value(s).unwrap();
        let parsed: LoadBalancingStrategy = serde_json::from_value(json).unwrap();
        assert_eq!(format!("{s:?}"), format!("{parsed:?}"));
    }
}

// ============================================================================
// Coordination Distribution
// ============================================================================

#[tokio::test]
async fn test_massive_job_distributor_simple_complexity() {
    let config = DistributionConfig {
        max_subtasks: 50,
        splitting_strategies: HashMap::new(),
    };
    let distributor = MassiveJobDistributor::new(config).await.unwrap();
    let job = toadstool_distributed::UniversalJob {
        job_id: uuid::Uuid::new_v4(),
        job_type: None,
        execution_request: toadstool::ExecutionRequest::default(),
        target: toadstool_distributed::ExecutionTarget::Local,
        priority: toadstool_distributed::JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: toadstool_distributed::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };
    let analysis = JobAnalysis {
        complexity: JobComplexity::Simple,
        distribution_strategy: JobDistributionStrategy::SplitAndDistribute,
        estimated_subtasks: 1,
        resource_requirements: ResourceRequirements::default(),
        preferred_node_types: vec![],
    };
    let subtasks = distributor.split_job(&job, &analysis).await.unwrap();
    assert_eq!(subtasks.len(), 1);
}

#[tokio::test]
async fn test_massive_job_distributor_splitting_strategy_from_string() {
    use toadstool_distributed::coordination::{JobSplittingStrategy, SplittingStrategyType};

    let s = JobSplittingStrategy::from_string("map_reduce");
    assert!(matches!(s.strategy_type, SplittingStrategyType::MapReduce));
    assert_eq!(s.max_subtasks, 100);
}

// ============================================================================
// Coordination Connection
// ============================================================================

fn make_protocol_config(protocol: CoordinationTransport) -> ProtocolConfig {
    ProtocolConfig {
        protocol,
        http: HttpProtocolConfig {
            timeout_ms: 5000,
            max_retries: 3,
            headers: HashMap::new(),
        },
        grpc: GrpcProtocolConfig {
            timeout_ms: 5000,
            max_message_size: 1024 * 1024,
            compression: false,
        },
        message_queue: MessageQueueProtocolConfig {
            queue_name: "default".to_string(),
            exchange: "default".to_string(),
            routing_key: "default".to_string(),
        },
    }
}

fn make_connection_config(
    endpoints: Vec<String>,
    protocol: CoordinationTransport,
) -> CoordinationConnectionConfig {
    CoordinationConnectionConfig {
        endpoints,
        protocol_config: make_protocol_config(protocol),
        auth_config: toadstool_common::auth::ServiceAuthConfig::default(),
        pool: toadstool_common::config_bases::ConnectionPoolConfig::default(),
    }
}

#[tokio::test]
async fn test_coordination_connection_empty_endpoints_fails() {
    let config = make_connection_config(vec![], CoordinationTransport::GRPC);
    let result: Result<_, _> = CoordinationConnection::new(config).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("No coordination endpoints")
    );
}

#[tokio::test]
async fn test_coordination_connection_grpc_http_succeeds() {
    let config = make_connection_config(
        vec!["http://localhost:9999".to_string()],
        CoordinationTransport::GRPC,
    );
    let conn = CoordinationConnection::new(config).await.unwrap();
    assert_eq!(conn.active_endpoint, "http://localhost:9999");
}

// ============================================================================
// Coordination Broadcasting
// ============================================================================

#[tokio::test]
async fn test_coordination_broadcaster_new() {
    let config = BroadcastConfig {
        channels: vec!["test-channel".to_string()],
        message_retention: Duration::from_secs(60),
    };
    let conn = Arc::new(
        CoordinationConnection::new(make_connection_config(
            vec!["http://localhost:1".to_string()],
            CoordinationTransport::GRPC,
        ))
        .await
        .unwrap(),
    );
    let broadcaster = CoordinationBroadcaster::new(config, conn).await.unwrap();
    let _ = broadcaster;
}

// ============================================================================
// Coordination Load Balancing
// ============================================================================

#[tokio::test]
async fn test_coordination_load_balancer_new_and_request_advice() {
    let config = CoordinationLoadBalancerConfig {
        strategy: "least-loaded".to_string(),
        feedback_interval: Duration::from_secs(5),
    };
    let conn = Arc::new(
        CoordinationConnection::new(make_connection_config(
            vec!["http://localhost:1".to_string()],
            CoordinationTransport::GRPC,
        ))
        .await
        .unwrap(),
    );
    let lb = CoordinationLoadBalancer::new(config, conn).await.unwrap();
    let advice = lb
        .request_advice(&ResourceRequirements::default())
        .await
        .unwrap();
    assert!(!advice.recommended_nodes.is_empty());
    assert!(advice.reasoning.contains("localhost") || advice.reasoning.contains("No capacity"));
}

// ============================================================================
// Universal Scheduler
// ============================================================================

#[test]
fn test_universal_scheduler_config_defaults() {
    let config = UniversalSchedulerConfig::default();
    assert!(!config.scheduling_algorithms.is_empty());
    assert!(config.network_effects.enabled);
    assert_eq!(config.network_effects.fault_tolerance.max_retries, 3);
}

#[test]
fn test_scheduling_algorithm_variants() {
    let _ = SchedulingAlgorithm::FirstComeFirstServe;
    let _ = SchedulingAlgorithm::Priority;
    let _ = SchedulingAlgorithm::RoundRobin;
    let _ = SchedulingAlgorithm::ShortestJobFirst;
    let _ = SchedulingAlgorithm::ResourceAware;
    let _ = SchedulingAlgorithm::NetworkAware;
    let _ = SchedulingAlgorithm::EnergyOptimized;
}

#[test]
fn test_network_effects_config_defaults() {
    let config = NetworkEffectsConfig::default();
    assert!(config.enabled);
    assert_eq!(config.load_balancing.health_check_interval_ms, 5000);
    assert_eq!(config.fault_tolerance.circuit_breaker_threshold, 5);
}

#[test]
fn test_os_layer_config_default() {
    let config = OSLayerConfig::default();
    assert!(!config.virtual_filesystem_enabled);
    assert!(!config.process_virtualization_enabled);
}

#[tokio::test]
async fn test_universal_scheduler_creation() {
    let config = UniversalSchedulerConfig::default();
    let scheduler = UniversalScheduler::new(config).await;
    assert!(scheduler.is_ok());
}

#[tokio::test]
async fn test_universal_scheduler_schedule_job_local() {
    let config = UniversalSchedulerConfig::default();
    let scheduler = UniversalScheduler::new(config).await.unwrap();
    let job = toadstool_distributed::UniversalJob {
        job_id: uuid::Uuid::new_v4(),
        job_type: None,
        execution_request: toadstool::ExecutionRequest::default(),
        target: toadstool_distributed::ExecutionTarget::Local,
        priority: toadstool_distributed::JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: toadstool_distributed::DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };
    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
}

// ============================================================================
// Universal Detection
// ============================================================================

#[tokio::test]
async fn test_universal_substrate_capabilities_detect_all() {
    use toadstool_distributed::universal::substrate::UniversalSubstrateCapabilities;

    let caps = UniversalSubstrateCapabilities::detect_all().await;
    assert!(caps.is_ok());
    let caps = caps.unwrap();
    assert!(!caps.traditional_platforms.is_empty());
    assert!(!caps.operating_systems.is_empty());
}

// ============================================================================
// Network Distributor
// ============================================================================

#[test]
fn test_network_distributor_config_default() {
    let config = NetworkDistributorConfig::default();
    assert!(config.enabled);
    assert_eq!(config.max_concurrent_distributions, 10);
}

#[tokio::test]
async fn test_network_distributor_distribute_job_no_nodes_local_fallback() {
    use toadstool_distributed::types::{DistributedRetryConfig, ExecutionTarget, JobPriority};

    let distributor = NetworkDistributor::new(NetworkDistributorConfig::default());
    let job = toadstool_distributed::UniversalJob {
        job_id: uuid::Uuid::new_v4(),
        job_type: None,
        execution_request: toadstool::ExecutionRequest::default(),
        target: ExecutionTarget::Local,
        priority: JobPriority::Normal,
        dependencies: vec![],
        resource_requirements: ResourceRequirements::default(),
        retry_config: DistributedRetryConfig::default(),
        created_at: SystemTime::now(),
    };
    let execution = distributor.distribute_job(job).await.unwrap();
    assert_eq!(execution.node_assignments.len(), 1);
}

#[tokio::test]
async fn test_network_distributor_register_and_deregister_peer() {
    let distributor = NetworkDistributor::new(NetworkDistributorConfig::default());
    distributor
        .register_peer_node(
            "peer-s155b".to_string(),
            NodeHealth {
                healthy: true,
                cpu_usage: 0.3,
                memory_usage: 0.4,
                response_time_ms: 50,
            },
        )
        .await;
    let snapshot = distributor.load_balancer().node_health_snapshot().await;
    assert_eq!(snapshot.len(), 1);
    distributor.deregister_peer_node("peer-s155b").await;
    let snapshot = distributor.load_balancer().node_health_snapshot().await;
    assert!(snapshot.is_empty());
}

// ============================================================================
// Cloud Orchestrator
// ============================================================================

fn make_orchestrator_config() -> CloudOrchestratorConfig {
    CloudOrchestratorConfig {
        scheduling_strategy: HybridSchedulingStrategy::Balanced {
            cost_weight: 0.33,
            performance_weight: 0.33,
            compliance_weight: 0.34,
        },
        cost_config: CostConfig {
            budget_limit: None,
            cost_tracking_enabled: false,
            spot_instance_preference: 0.0,
        },
        compliance_config: ComplianceConfig {
            required_certifications: vec![],
            allowed_regions: vec![],
            data_sovereignty_requirements: vec![],
        },
        load_balancer_config: CloudLoadBalancerConfig {
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

// ============================================================================
// Cloud Compliance Validation
// ============================================================================

#[test]
fn test_check_result_variants() {
    let _ = CheckResult::Pass;
    let fail = CheckResult::Fail {
        reason: "test".to_string(),
    };
    assert!(matches!(fail, CheckResult::Fail { .. }));
}

#[test]
fn test_compliance_check_serialization() {
    let check = ComplianceCheck {
        check_name: "encryption".to_string(),
        result: CheckResult::Pass,
    };
    let json = serde_json::to_value(&check).unwrap();
    assert_eq!(
        json.get("check_name").and_then(|v| v.as_str()),
        Some("encryption")
    );
}

#[tokio::test]
async fn test_cloud_compliance_enforcer_creation() {
    let config = ComplianceConfig {
        required_certifications: vec![],
        allowed_regions: vec![],
        data_sovereignty_requirements: vec![],
    };
    let enforcer = CloudComplianceEnforcer::new(config).await.unwrap();
    let _ = enforcer;
}

#[tokio::test]
async fn test_cloud_compliance_enforcer_report_for_provider_not_registered() {
    let config = ComplianceConfig {
        required_certifications: vec![],
        allowed_regions: vec![],
        data_sovereignty_requirements: vec![],
    };
    let enforcer = CloudComplianceEnforcer::new(config).await.unwrap();
    let result = enforcer.report_for_provider("nonexistent");
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cloud_compliance_enforcer_add_provider_and_report() {
    let config = ComplianceConfig {
        required_certifications: vec![],
        allowed_regions: vec![],
        data_sovereignty_requirements: vec![],
    };
    let mut enforcer = CloudComplianceEnforcer::new(config).await.unwrap();
    let caps = CloudCapabilities {
        compute_types: vec![ComputeType::VM],
        storage_types: vec![StorageType::BlockStorage],
        networking_features: vec![NetworkingFeature::VPC],
        security_features: vec![SecurityFeature::Encryption],
        compliance_certifications: vec![],
        regions: vec![Region {
            name: "us-east-1".to_string(),
            location: "N. Virginia".to_string(),
            availability_zones: vec![],
        }],
        max_cpu_cores: Some(64),
        max_memory_gb: Some(256),
        gpu_support: false,
        kubernetes_support: false,
        serverless_support: false,
    };
    enforcer
        .add_provider_compliance("provider-a", &caps)
        .await
        .unwrap();
    let report = enforcer.report_for_provider("provider-a").unwrap();
    assert_eq!(report.provider_name, "provider-a");
    assert!(!report.checks.is_empty());
}

#[tokio::test]
async fn test_cloud_compliance_enforcer_with_security_tier() {
    use toadstool_distributed::cloud::SecurityTier;

    let config = ComplianceConfig {
        required_certifications: vec![],
        allowed_regions: vec![],
        data_sovereignty_requirements: vec![],
    };
    let enforcer = CloudComplianceEnforcer::new(config)
        .await
        .unwrap()
        .with_security_tier(SecurityTier::High);
    let _ = enforcer;
}

// ============================================================================
// Cloud Cost Optimizer
// ============================================================================

#[tokio::test]
async fn test_cloud_cost_optimizer_estimate_cost() {
    let cfg = CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0,
    };
    let mut optimizer = CloudCostOptimizer::new(cfg).await.unwrap();
    let caps = CloudCapabilities {
        compute_types: vec![ComputeType::VM],
        storage_types: vec![StorageType::BlockStorage],
        networking_features: vec![NetworkingFeature::VPC],
        security_features: vec![SecurityFeature::Encryption],
        compliance_certifications: vec![],
        regions: vec![],
        max_cpu_cores: Some(64),
        max_memory_gb: Some(256),
        gpu_support: false,
        kubernetes_support: false,
        serverless_support: false,
    };
    optimizer
        .add_provider_cost_model("p1", &caps)
        .await
        .unwrap();
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            max_cores: None,
        },
        memory: MemoryRequirements {
            min_bytes: 4 * 1024 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: None,
        },
        network: NetworkRequirements {
            bandwidth_mbps: None,
            latency_ms: None,
        },
        gpu: None,
    };
    let est = optimizer.estimate_cost("p1", &req, 1.0, 0.0).unwrap();
    assert!(est.total_cost > 0.0);
    assert_eq!(est.duration_hours, 1.0);
}

#[tokio::test]
async fn test_cloud_cost_optimizer_invalid_duration_fails() {
    let mut optimizer = CloudCostOptimizer::new(CostConfig {
        budget_limit: None,
        cost_tracking_enabled: false,
        spot_instance_preference: 0.0,
    })
    .await
    .unwrap();
    let caps = CloudCapabilities {
        compute_types: vec![ComputeType::VM],
        storage_types: vec![StorageType::BlockStorage],
        networking_features: vec![],
        security_features: vec![],
        compliance_certifications: vec![],
        regions: vec![],
        max_cpu_cores: None,
        max_memory_gb: None,
        gpu_support: false,
        kubernetes_support: false,
        serverless_support: false,
    };
    optimizer.add_provider_cost_model("p", &caps).await.unwrap();
    let req = ResourceRequirements::default();
    let result = optimizer.estimate_cost("p", &req, 0.0, 0.0);
    assert!(result.is_err());
}

// ============================================================================
// Primal Capabilities Workload
// ============================================================================

#[test]
fn test_workload_executor_creation() {
    let executor = WorkloadExecutor::new();
    let _ = executor;
    let default = WorkloadExecutor::default();
    let _ = default;
}

#[test]
fn test_workload_request_serialization() {
    let request = WorkloadRequest {
        request_id: "req-1".to_string(),
        from_primal: "coordination".to_string(),
        required_capability: "compute".to_string(),
        workload_type: WorkloadType::Native {
            executable: "python".to_string(),
            args: vec!["script.py".to_string()],
        },
        resource_requirements: WorkloadResourceRequirements {
            cpu_cores: Some(4),
            memory_mb: Some(8192),
            gpu_required: false,
            gpu_memory_mb: None,
        },
        environment: HashMap::new(),
        timeout_seconds: Some(3600),
        priority: "normal".to_string(),
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: WorkloadRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.request_id, "req-1");
}

#[test]
fn test_workload_status_variants() {
    let _ = WorkloadStatus::Accepted;
    let _ = WorkloadStatus::Running;
    let _ = WorkloadStatus::Completed;
    let _ = WorkloadStatus::Failed;
    let _ = WorkloadStatus::TimedOut;
}

#[tokio::test]
async fn test_workload_executor_execute() {
    use toadstool_distributed::error::DistributedError;

    let executor = WorkloadExecutor::new();
    let request = WorkloadRequest {
        request_id: "exec-1".to_string(),
        from_primal: "coordination".to_string(),
        required_capability: "compute".to_string(),
        workload_type: WorkloadType::Native {
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        },
        resource_requirements: WorkloadResourceRequirements {
            cpu_cores: Some(1),
            memory_mb: Some(512),
            gpu_required: false,
            gpu_memory_mb: None,
        },
        environment: HashMap::new(),
        timeout_seconds: None,
        priority: "normal".to_string(),
    };
    // Current implementation returns WorkloadConversionRequiresScheduler until scheduler integration
    let result = executor.execute(request).await;
    assert!(
        result.is_err(),
        "execute returns Err until scheduler integration"
    );
    assert!(matches!(
        result.unwrap_err(),
        DistributedError::WorkloadConversionRequiresScheduler
    ));
}

// ============================================================================
// Error Paths
// ============================================================================

#[tokio::test]
async fn test_coordination_connection_invalid_endpoint_degraded() {
    let config = make_connection_config(
        vec!["invalid".to_string(), "also-invalid".to_string()],
        CoordinationTransport::GRPC,
    );
    let conn = CoordinationConnection::new(config).await.unwrap();
    assert_eq!(conn.health_status, ConnectionHealth::Degraded);
}
