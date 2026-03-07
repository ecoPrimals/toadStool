// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::float_cmp)]
//! Additional type tests (serde, config, etc.)

use super::super::protocols;
use super::super::*;
use super::make_test_job;
use crate::ResourceRequirements;
use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;
use tokio::sync::mpsc;
use uuid::Uuid;
#[test]
fn test_connection_health_all_variants_debug() {
    for h in [
        ConnectionHealth::Healthy,
        ConnectionHealth::Degraded,
        ConnectionHealth::Unhealthy,
        ConnectionHealth::Unknown,
    ] {
        let s = format!("{:?}", h);
        assert!(!s.is_empty());
    }
}

#[test]
fn test_complexity_level_all_variants() {
    let _ = ComplexityLevel::Low;
    let _ = ComplexityLevel::Medium;
    let _ = ComplexityLevel::High;
    let _ = ComplexityLevel::Extreme;
}

#[test]
fn test_execution_metrics_constructor() {
    let m = ExecutionMetrics {
        start_time: SystemTime::now(),
        end_time: SystemTime::now(),
        cpu_usage: 0.5,
        memory_usage: 1024,
        network_io: 100,
        disk_io: 200,
    };
    assert_eq!(m.memory_usage, 1024);
}

#[test]
fn test_songbird_job_response_success_estimated_completion_none() {
    let resp = SongbirdJobResponse::Success {
        job_id: Uuid::new_v4(),
        status: "done".to_string(),
        message: "OK".to_string(),
        estimated_completion: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SongbirdJobResponse = serde_json::from_str(&json).unwrap();
    if let SongbirdJobResponse::Success {
        estimated_completion,
        ..
    } = parsed
    {
        assert!(estimated_completion.is_none());
    } else {
        panic!("expected Success");
    }
}

#[test]
fn test_job_distribution_strategy_hybrid_execution_serde() {
    let s = JobDistributionStrategy::HybridExecution;
    let json = serde_json::to_string(&s).unwrap();
    let _: JobDistributionStrategy = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_songbird_integration_config_constructor() {
    use protocols::{
        GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
        SongbirdProtocol,
    };
    use toadstool_common::auth::ServiceAuthConfig;
    use toadstool_common::config_bases::ConnectionPoolConfig;

    let config = SongbirdIntegrationConfig {
        connection_config: SongbirdConnectionConfig {
            endpoints: vec!["http://localhost:8080".to_string()],
            protocol_config: ProtocolConfig {
                protocol: SongbirdProtocol::HTTP,
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
            },
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        },
        distribution_config: DistributionConfig {
            max_subtasks: 8,
            splitting_strategies: HashMap::new(),
        },
        discovery_config: SongbirdDiscoveryConfig {
            discovery_interval: Duration::from_secs(60),
            node_timeout: Duration::from_secs(30),
        },
        load_balancer_config: LoadBalancerConfig {
            strategy: "round-robin".to_string(),
            feedback_interval: Duration::from_secs(5),
        },
        broadcast_config: BroadcastConfig {
            channels: vec![],
            message_retention: Duration::from_secs(60),
        },
        capacity_config: CapacityConfig {
            monitoring_interval: Duration::from_secs(10),
            resource_buffer: 0.0,
        },
        receiver_config: ReceiverConfig {
            max_concurrent_jobs: 4,
            job_timeout: Duration::from_secs(120),
        },
    };
    assert_eq!(config.receiver_config.max_concurrent_jobs, 4);
}

// ─── Serde round-trips for config structs without tests ────────────────

#[test]
fn test_songbird_connection_config_serde() {
    use protocols::{
        GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
        SongbirdProtocol,
    };
    use toadstool_common::auth::ServiceAuthConfig;
    use toadstool_common::config_bases::ConnectionPoolConfig;
    let config = SongbirdConnectionConfig {
        endpoints: vec!["http://a:8080".to_string()],
        protocol_config: ProtocolConfig {
            protocol: SongbirdProtocol::HTTP,
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
                queue_name: "q1".to_string(),
                exchange: "ex1".to_string(),
                routing_key: "rk1".to_string(),
            },
        },
        auth_config: ServiceAuthConfig::default(),
        pool: ConnectionPoolConfig::default(),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: SongbirdConnectionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.endpoints.len(), 1);
}

#[test]
fn test_songbird_discovery_config_serde() {
    let config = SongbirdDiscoveryConfig {
        discovery_interval: Duration::from_secs(30),
        node_timeout: Duration::from_secs(10),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: SongbirdDiscoveryConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.discovery_interval.as_secs(), 30);
}

#[test]
fn test_load_balancer_config_serde() {
    let config = LoadBalancerConfig {
        strategy: "least-loaded".to_string(),
        feedback_interval: Duration::from_secs(5),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: LoadBalancerConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.strategy, "least-loaded");
}

#[test]
fn test_broadcast_config_serde() {
    let config = BroadcastConfig {
        channels: vec!["events".to_string()],
        message_retention: Duration::from_secs(60),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: BroadcastConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.channels.len(), 1);
}

#[test]
fn test_capacity_config_serde() {
    let config = CapacityConfig {
        monitoring_interval: Duration::from_secs(20),
        resource_buffer: 0.2,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: CapacityConfig = serde_json::from_str(&json).unwrap();
    assert!((parsed.resource_buffer - 0.2).abs() < 0.001);
}

#[test]
fn test_receiver_config_serde() {
    let config = ReceiverConfig {
        max_concurrent_jobs: 16,
        job_timeout: Duration::from_secs(600),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ReceiverConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.max_concurrent_jobs, 16);
}

#[test]
fn test_songbird_integration_config_serde() {
    use protocols::{
        GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
        SongbirdProtocol,
    };
    use toadstool_common::auth::ServiceAuthConfig;
    use toadstool_common::config_bases::ConnectionPoolConfig;
    let config = SongbirdIntegrationConfig {
        connection_config: SongbirdConnectionConfig {
            endpoints: vec!["http://localhost:8080".to_string()],
            protocol_config: ProtocolConfig {
                protocol: SongbirdProtocol::HTTP,
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
            },
            auth_config: ServiceAuthConfig::default(),
            pool: ConnectionPoolConfig::default(),
        },
        distribution_config: DistributionConfig {
            max_subtasks: 8,
            splitting_strategies: HashMap::new(),
        },
        discovery_config: SongbirdDiscoveryConfig {
            discovery_interval: Duration::from_secs(60),
            node_timeout: Duration::from_secs(30),
        },
        load_balancer_config: LoadBalancerConfig {
            strategy: "round-robin".to_string(),
            feedback_interval: Duration::from_secs(5),
        },
        broadcast_config: BroadcastConfig {
            channels: vec![],
            message_retention: Duration::from_secs(60),
        },
        capacity_config: CapacityConfig {
            monitoring_interval: Duration::from_secs(10),
            resource_buffer: 0.0,
        },
        receiver_config: ReceiverConfig {
            max_concurrent_jobs: 4,
            job_timeout: Duration::from_secs(120),
        },
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: SongbirdIntegrationConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.receiver_config.max_concurrent_jobs, 4);
}

// ─── JobSplittingStrategy, LoadEstimator, JobCoordinator, CapacityInfo ───

#[tokio::test]
async fn test_job_splitting_strategy_split_job_max_subtasks_one() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::DataParallel,
        max_subtasks: 1,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_job_splitting_strategy_split_job_data_parallel() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::DataParallel,
        max_subtasks: 4,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert!(!result.is_empty());
    assert!(result.len() <= 4);
}

#[tokio::test]
async fn test_job_splitting_strategy_split_job_task_parallel() {
    let mut req = ResourceRequirements::default();
    req.cpu.min_cores = 4.0;
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::TaskParallel,
        max_subtasks: 3,
        min_subtask_size: 1,
    };
    let job = make_test_job(req);
    let result = strategy.split_job(&job).await;
    assert_eq!(result.len(), 3);
}

#[tokio::test]
async fn test_job_splitting_strategy_split_job_map_reduce() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::MapReduce,
        max_subtasks: 2,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert!(!result.is_empty());
}

#[tokio::test]
async fn test_load_estimator_estimate_load() {
    let estimator = LoadEstimator::default();
    let job = make_test_job(ResourceRequirements::default());
    let load = estimator.estimate_load(&job).await;
    assert!(load.cpu_load >= 0.0 && load.cpu_load <= 1.0);
    assert!(load.memory_load >= 0.0 && load.memory_load <= 1.0);
    assert!(load.network_load >= 0.0 && load.network_load <= 1.0);
}

#[tokio::test]
async fn test_job_coordinator_coordinate() {
    let coord = JobCoordinator::default();
    let plan = DistributionPlan {
        plan_id: Uuid::new_v4(),
        job_id: Uuid::new_v4(),
        subtasks: vec![
            SubTaskPlan {
                subtask_id: Uuid::new_v4(),
                target_nodes: vec!["n1".to_string()],
                resource_allocation: ResourceRequirements::default(),
                dependencies: vec![],
            },
            SubTaskPlan {
                subtask_id: Uuid::new_v4(),
                target_nodes: vec!["n2".to_string()],
                resource_allocation: ResourceRequirements::default(),
                dependencies: vec![],
            },
        ],
        coordination_strategy: CoordinationStrategy::Parallel,
    };
    let job = coord.coordinate(&plan).await;
    assert_eq!(job.subtask_count, 2);
}

#[test]
fn test_capacity_info_from_system() {
    let info = CapacityInfo::from_system();
    assert!(info.cpu_cores > 0.0);
    assert!(info.memory_bytes > 0);
    let _ = info.storage_bytes;
}

#[tokio::test]
async fn test_job_receiver_receive_none_when_empty() {
    let (tx, rx) = mpsc::channel::<SongbirdJobMessage>(1);
    drop(tx);
    let mut receiver = JobReceiver { receiver: rx };
    let result = receiver.receive().await;
    assert!(result.is_none());
}

#[test]
fn test_splitting_strategy_type_variants() {
    let _ = SplittingStrategyType::DataParallel;
    let _ = SplittingStrategyType::TaskParallel;
    let _ = SplittingStrategyType::Pipeline;
    let _ = SplittingStrategyType::MapReduce;
    let _ = SplittingStrategyType::Custom("custom".to_string());
}

#[tokio::test]
async fn test_job_splitting_strategy_custom_falls_back_to_task_parallel() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::Custom("custom".to_string()),
        max_subtasks: 2,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert_eq!(result.len(), 2);
}

#[tokio::test]
async fn test_job_splitting_strategy_pipeline_falls_back_to_task_parallel() {
    let strategy = JobSplittingStrategy {
        strategy_type: SplittingStrategyType::Pipeline,
        max_subtasks: 2,
        min_subtask_size: 1,
    };
    let job = make_test_job(ResourceRequirements::default());
    let result = strategy.split_job(&job).await;
    assert_eq!(result.len(), 2);
}

#[test]
fn test_node_type_all_variants_serde() {
    for nt in [
        NodeType::ToadStool,
        NodeType::NestGate,
        NodeType::BearDog,
        NodeType::Songbird,
        NodeType::Custom("my-type".to_string()),
    ] {
        let json = serde_json::to_string(&nt).unwrap();
        let _: NodeType = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_node_registration_serde() {
    use super::node::NodeCapabilities;
    let reg = NodeRegistration {
        node_id: "node-1".to_string(),
        node_type: NodeType::ToadStool,
        capabilities: NodeCapabilities {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            storage_gb: 100.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        },
        endpoints: vec!["http://localhost:8080".to_string()],
        protocols: vec!["http".to_string()],
        metadata: NodeMetadata {
            version: "1.0".to_string(),
            build_info: "test".to_string(),
            capabilities: NodeCapabilities {
                cpu_cores: 4.0,
                memory_gb: 8.0,
                storage_gb: 100.0,
                gpu_count: 0,
                specialized_hardware: vec![],
                software_capabilities: vec![],
            },
        },
    };
    let json = serde_json::to_string(&reg).unwrap();
    let parsed: NodeRegistration = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.node_id, reg.node_id);
}

#[test]
fn test_node_metadata_serde() {
    use super::node::NodeCapabilities;
    let meta = NodeMetadata {
        version: "2.0".to_string(),
        build_info: "release".to_string(),
        capabilities: NodeCapabilities {
            cpu_cores: 8.0,
            memory_gb: 16.0,
            storage_gb: 200.0,
            gpu_count: 1,
            specialized_hardware: vec!["cuda".to_string()],
            software_capabilities: vec!["wasm".to_string()],
        },
    };
    let json = serde_json::to_string(&meta).unwrap();
    let parsed: NodeMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.version, "2.0");
}

#[test]
fn test_network_requirements_serde() {
    let req = NetworkRequirements {
        bandwidth_mbps: Some(1000),
        latency_ms: Some(50),
        reliability_percent: Some(99.9),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: NetworkRequirements = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.bandwidth_mbps, Some(1000));
}

#[test]
fn test_load_metric_constructor() {
    let metric = LoadMetric {
        cpu_load: 0.5,
        memory_load: 0.3,
        network_load: 0.1,
    };
    assert!((metric.cpu_load - 0.5).abs() < 0.001);
}

#[test]
fn test_job_result_empty_output_serde() {
    let result = JobResult {
        job_id: Uuid::new_v4(),
        status: "ok".to_string(),
        output: vec![],
        metrics: ExecutionMetrics {
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            cpu_usage: 0.0,
            memory_usage: 0,
            network_io: 0,
            disk_io: 0,
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: JobResult = serde_json::from_str(&json).unwrap();
    assert!(parsed.output.is_empty());
}

#[test]
fn test_complexity_level_all_variants_serde() {
    for level in [
        ComplexityLevel::Low,
        ComplexityLevel::Medium,
        ComplexityLevel::High,
        ComplexityLevel::Extreme,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let _: ComplexityLevel = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_authentication_config_all_auth_types() {
    use toadstool_common::auth::AuthType;
    for auth_type in [
        AuthType::None,
        AuthType::ApiKey,
        AuthType::Bearer,
        AuthType::Basic,
        AuthType::OAuth2,
    ] {
        let config = AuthenticationConfig {
            auth_type: auth_type.clone(),
            api_key: None,
            token: None,
            username: None,
            password: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: AuthenticationConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(
            format!("{:?}", parsed.auth_type),
            format!("{:?}", auth_type)
        );
    }
}

#[test]
fn test_sub_task_plan_empty_dependencies() {
    let plan = SubTaskPlan {
        subtask_id: Uuid::new_v4(),
        target_nodes: vec![],
        resource_allocation: ResourceRequirements::default(),
        dependencies: vec![],
    };
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: SubTaskPlan = serde_json::from_str(&json).unwrap();
    assert!(parsed.dependencies.is_empty());
}

#[test]
fn test_songbird_job_request_full_roundtrip() {
    let mut req = ResourceRequirements::default();
    req.cpu.min_cores = 2.0;
    let job_req = SongbirdJobRequest {
        job_id: Uuid::new_v4(),
        job_payload: vec![1, 2, 3, 4, 5],
        target_nodes: vec!["n1".to_string(), "n2".to_string()],
        resource_requirements: req.clone(),
        priority: 10,
        constraints: vec!["gpu".to_string(), "fast".to_string()],
    };
    let json = serde_json::to_string(&job_req).unwrap();
    let parsed: SongbirdJobRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.target_nodes.len(), 2);
    assert_eq!(parsed.constraints.len(), 2);
}

#[test]
fn test_distribution_plan_with_subtasks_serde() {
    let plan = DistributionPlan {
        plan_id: Uuid::new_v4(),
        job_id: Uuid::new_v4(),
        subtasks: vec![
            SubTaskPlan {
                subtask_id: Uuid::new_v4(),
                target_nodes: vec!["n1".to_string()],
                resource_allocation: ResourceRequirements::default(),
                dependencies: vec![],
            },
            SubTaskPlan {
                subtask_id: Uuid::new_v4(),
                target_nodes: vec!["n2".to_string()],
                resource_allocation: ResourceRequirements::default(),
                dependencies: vec![Uuid::new_v4()],
            },
        ],
        coordination_strategy: CoordinationStrategy::MapReduce,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: DistributionPlan = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.subtasks.len(), 2);
}

#[test]
fn test_songbird_job_request_empty_payload_serde() {
    let req = SongbirdJobRequest {
        job_id: Uuid::new_v4(),
        job_payload: vec![],
        target_nodes: vec![],
        resource_requirements: ResourceRequirements::default(),
        priority: 0,
        constraints: vec![],
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: SongbirdJobRequest = serde_json::from_str(&json).unwrap();
    assert!(parsed.job_payload.is_empty());
}

#[test]
fn test_job_analysis_empty_preferred_node_types_serde() {
    let analysis = JobAnalysis {
        complexity: JobComplexity::Simple,
        distribution_strategy: JobDistributionStrategy::LocalOnly,
        estimated_subtasks: 1,
        resource_requirements: ResourceRequirements::default(),
        preferred_node_types: vec![],
    };
    let json = serde_json::to_string(&analysis).unwrap();
    let parsed: JobAnalysis = serde_json::from_str(&json).unwrap();
    assert!(parsed.preferred_node_types.is_empty());
}

#[test]
fn test_sub_task_handle_empty_target_nodes_serde() {
    let handle = SubTaskHandle {
        subtask_id: Uuid::new_v4(),
        songbird_job_id: Uuid::new_v4(),
        target_nodes: vec![],
        submitted_at: SystemTime::now(),
        status: SubTaskStatus::Failed,
    };
    let json = serde_json::to_string(&handle).unwrap();
    let parsed: SubTaskHandle = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed.status, SubTaskStatus::Failed));
}

#[test]
fn test_coordination_job_wait_for_any_serde() {
    let job = CoordinationJob {
        job_id: Uuid::new_v4(),
        original_job_id: Uuid::new_v4(),
        subtask_count: 3,
        completion_strategy: CompletionStrategy::WaitForAny,
    };
    let json = serde_json::to_string(&job).unwrap();
    let parsed: CoordinationJob = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        parsed.completion_strategy,
        CompletionStrategy::WaitForAny
    ));
}

#[test]
fn test_distribution_plan_empty_subtasks_serde() {
    let plan = DistributionPlan {
        plan_id: Uuid::new_v4(),
        job_id: Uuid::new_v4(),
        subtasks: vec![],
        coordination_strategy: CoordinationStrategy::Sequential,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: DistributionPlan = serde_json::from_str(&json).unwrap();
    assert!(parsed.subtasks.is_empty());
}

#[test]
fn test_authentication_config_basic_serde() {
    let config = AuthenticationConfig {
        auth_type: toadstool_common::auth::AuthType::Basic,
        api_key: None,
        token: None,
        username: Some("user".to_string()),
        password: Some("pass".to_string()),
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: AuthenticationConfig = serde_json::from_str(&json).unwrap();
    assert!(parsed.username.is_some());
}

#[test]
fn test_capacity_info_insufficient_memory() {
    let mut req = ResourceRequirements::default();
    req.memory.min_bytes = 32 * 1024 * 1024 * 1024;
    let info = CapacityInfo {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
    };
    let job = make_test_job(req);
    assert!(!info.can_handle_job(&job));
}

#[test]
fn test_capacity_info_insufficient_storage() {
    let mut req = ResourceRequirements::default();
    req.storage.min_bytes = 200 * 1024 * 1024 * 1024;
    let info = CapacityInfo {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
    };
    let job = make_test_job(req);
    assert!(!info.can_handle_job(&job));
}

#[test]
fn test_performance_metrics_default() {
    let metrics = PerformanceMetrics::default();
    assert_eq!(metrics.request_count(), 0);
    assert_eq!(metrics.mean_latency_ms(), 0.0);
}

#[test]
fn test_message_type_registry_default() {
    let registry = MessageTypeRegistry::default();
    assert!(!registry.is_known("unknown"));
}

#[test]
fn test_broadcast_channel_empty_name() {
    let ch = BroadcastChannel::new("");
    assert_eq!(ch.name(), "");
}

#[test]
fn test_songbird_broadcast_message_all_channel_names() {
    assert_eq!(
        SongbirdBroadcastMessage::CapabilityUpdate {
            node_id: "n".to_string(),
            capabilities: NodeCapabilities {
                cpu_cores: 1.0,
                memory_gb: 1.0,
                storage_gb: 1.0,
                gpu_count: 0,
                specialized_hardware: vec![],
                software_capabilities: vec![],
            },
            timestamp: SystemTime::now(),
        }
        .channel_name(),
        "capability-updates"
    );
    assert_eq!(
        SongbirdBroadcastMessage::HealthUpdate {
            node_id: "n".to_string(),
            health_status: "ok".to_string(),
            timestamp: SystemTime::now(),
        }
        .channel_name(),
        "health-updates"
    );
    assert_eq!(
        SongbirdBroadcastMessage::CustomMessage {
            message_type: "custom".to_string(),
            payload: serde_json::Value::Null,
            timestamp: SystemTime::now(),
        }
        .channel_name(),
        "custom"
    );
}

#[test]
fn test_execution_metrics_zero_values_serde() {
    let m = ExecutionMetrics {
        start_time: SystemTime::now(),
        end_time: SystemTime::now(),
        cpu_usage: 0.0,
        memory_usage: 0,
        network_io: 0,
        disk_io: 0,
    };
    let json = serde_json::to_string(&m).unwrap();
    let _: ExecutionMetrics = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_connection_health_partial_eq() {
    assert_eq!(ConnectionHealth::Degraded, ConnectionHealth::Degraded);
    assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Degraded);
}

#[test]
fn test_universal_job_processor_constructor() {
    let p = UniversalJobProcessor {
        processor_id: "proc-1".to_string(),
    };
    assert_eq!(p.processor_id, "proc-1");
}

// ─── Re-export path tests (types/mod.rs coverage) ────────────────────────────

#[test]
fn test_load_balancing_advice_constructor() {
    use std::collections::HashMap;
    let advice = LoadBalancingAdvice {
        recommended_nodes: vec!["n1".to_string(), "n2".to_string()],
        load_distribution: {
            let mut m = HashMap::new();
            m.insert("n1".to_string(), 0.6);
            m.insert("n2".to_string(), 0.4);
            m
        },
        reasoning: "Load balanced".to_string(),
    };
    assert_eq!(advice.recommended_nodes.len(), 2);
    assert!(advice.reasoning.contains("Load balanced"));
}

#[test]
fn test_resource_reservation_constructor() {
    let res = ResourceReservation {
        reservation_id: uuid::Uuid::new_v4(),
        resources: ResourceRequirements::default(),
    };
    let _ = res.reservation_id;
    assert!(res.resources.cpu.min_cores >= 0.0);
}

#[test]
fn test_network_status_constructor() {
    let status = NetworkStatus {
        total_nodes: 5,
        active_nodes: 4,
        total_capacity: NodeCapabilities {
            cpu_cores: 16.0,
            memory_gb: 32.0,
            storage_gb: 200.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        },
        current_utilization: 0.75,
    };
    assert_eq!(status.total_nodes, 5);
    assert_eq!(status.active_nodes, 4);
    assert!((status.current_utilization - 0.75).abs() < 0.01);
}

#[test]
fn test_registration_response_constructor() {
    let resp = RegistrationResponse {
        node_id: "node-x".to_string(),
        status: "ok".to_string(),
        assigned_channels: vec!["ch1".to_string()],
    };
    assert_eq!(resp.node_id, "node-x");
    assert_eq!(resp.assigned_channels.len(), 1);
}

#[test]
fn test_types_reexport_load_estimator() {
    let est = LoadEstimator::default();
    assert_eq!(est.estimation_model, "linear");
}

#[test]
fn test_types_reexport_distribution_algorithm() {
    let algo = DistributionAlgorithm::RoundRobin;
    assert!(matches!(algo, DistributionAlgorithm::RoundRobin));
}

#[test]
fn test_types_reexport_load_balancing_strategy() {
    let strategy: LoadBalancingStrategy = "least-loaded".to_string();
    assert_eq!(strategy, "least-loaded");
}
