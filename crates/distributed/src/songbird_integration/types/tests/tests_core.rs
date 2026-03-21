// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::float_cmp)]
//! Core type tests

use super::super::protocols;
use super::super::*;
use super::make_test_job;
use crate::ResourceRequirements;
use std::collections::HashMap;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;
#[test]
fn test_connection_health_variants() {
    let _h = ConnectionHealth::Healthy;
    let _d = ConnectionHealth::Degraded;
    let _u = ConnectionHealth::Unhealthy;
    let _x = ConnectionHealth::Unknown;
}

#[test]
fn test_songbird_job_request_constructor() {
    let req = SongbirdJobRequest {
        job_id: Uuid::new_v4(),
        job_payload: bytes::Bytes::from(vec![1, 2, 3]),
        target_nodes: vec!["node1".to_string()],
        resource_requirements: ResourceRequirements::default(),
        priority: 5,
        constraints: vec!["gpu".to_string()],
    };
    assert_eq!(req.priority, 5);
    assert_eq!(req.target_nodes.len(), 1);
}

#[test]
fn test_songbird_job_request_serde_roundtrip() {
    let req = SongbirdJobRequest {
        job_id: Uuid::new_v4(),
        job_payload: bytes::Bytes::from(vec![1, 2, 3]),
        target_nodes: vec![],
        resource_requirements: ResourceRequirements::default(),
        priority: 1,
        constraints: vec![],
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: SongbirdJobRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.priority, req.priority);
}

#[test]
fn test_job_complexity_variants() {
    let _s = JobComplexity::Simple;
    let _m = JobComplexity::Moderate;
    let _c = JobComplexity::Complex;
    let _u = JobComplexity::UltraMassive;
}

#[test]
fn test_complexity_level_serde() {
    let level = ComplexityLevel::Extreme;
    let json = serde_json::to_string(&level).unwrap();
    let _: ComplexityLevel = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_job_distribution_strategy_serde() {
    let s = JobDistributionStrategy::LoadBalanced;
    let json = serde_json::to_string(&s).unwrap();
    let _: JobDistributionStrategy = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_sub_task_constructor() {
    let task = SubTask {
        id: Uuid::new_v4(),
        payload: bytes::Bytes::new(),
        resource_requirements: ResourceRequirements::default(),
        priority: 1,
        constraints: vec![],
    };
    assert_eq!(task.priority, 1);
}

#[test]
fn test_sub_task_handle_constructor() {
    let handle = SubTaskHandle {
        subtask_id: Uuid::new_v4(),
        songbird_job_id: Uuid::new_v4(),
        target_nodes: vec![],
        submitted_at: SystemTime::now(),
        status: SubTaskStatus::Submitted,
    };
    assert!(matches!(handle.status, SubTaskStatus::Submitted));
}

#[test]
fn test_completion_strategy_variants() {
    let _a = CompletionStrategy::WaitForAll;
    let _b = CompletionStrategy::WaitForMajority;
    let _c = CompletionStrategy::WaitForAny;
    let _d = CompletionStrategy::Custom("custom".to_string());
}

#[test]
fn test_capacity_info_can_handle_job_sufficient() {
    let info = CapacityInfo {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
    };
    let job = make_test_job(ResourceRequirements::default());
    assert!(info.can_handle_job(&job));
}

#[test]
fn test_capacity_info_can_handle_job_insufficient_cpu() {
    let info = CapacityInfo {
        cpu_cores: 0.5,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
    };
    let job = make_test_job(ResourceRequirements::default());
    assert!(!info.can_handle_job(&job));
}

#[test]
fn test_node_capacity_tracker_new_and_update() {
    let tracker = NodeCapacityTracker::new();
    tracker.update(&"node1".to_string(), 0.5);
    let snapshot = tracker.snapshot();
    assert_eq!(snapshot.get("node1").copied(), Some(0.5));
}

#[test]
fn test_node_capacity_tracker_least_loaded() {
    let tracker = NodeCapacityTracker::new();
    tracker.update(&"node1".to_string(), 0.8);
    tracker.update(&"node2".to_string(), 0.2);
    assert_eq!(tracker.least_loaded(), Some("node2".to_string()));
}

#[test]
fn test_node_capacity_tracker_default() {
    let tracker = NodeCapacityTracker::default();
    assert!(tracker.least_loaded().is_none());
}

#[test]
fn test_performance_metrics_record_and_rates() {
    let metrics = PerformanceMetrics::new();
    metrics.record(100, false);
    metrics.record(200, true);
    assert_eq!(metrics.request_count(), 2);
    assert!((metrics.error_rate() - 0.5).abs() < 0.01);
    assert!((metrics.mean_latency_ms() - 150.0).abs() < 0.01);
}

#[test]
fn test_performance_metrics_error_rate_zero_when_no_requests() {
    let metrics = PerformanceMetrics::new();
    assert_eq!(metrics.error_rate(), 0.0);
}

#[test]
fn test_songbird_feedback_sender_send() {
    let (sender, _receiver) = SongbirdFeedbackSender::new();
    let sent = sender.send(SongbirdFeedback::LoadUpdate {
        node_id: "n1".to_string(),
        load: 0.5,
    });
    assert!(sent);
}

#[test]
fn test_broadcast_channel_new_and_name() {
    let ch = BroadcastChannel::new("test-channel");
    assert_eq!(ch.name(), "test-channel");
}

#[test]
fn test_broadcast_channel_publish_and_subscribe() {
    let ch = BroadcastChannel::new("events");
    let _sub = ch.subscribe();
    let msg = SongbirdBroadcastMessage::HealthUpdate {
        node_id: "n1".to_string(),
        health_status: "ok".to_string(),
        timestamp: SystemTime::now(),
    };
    let result = ch.publish(msg);
    assert!(result.is_ok());
}

#[test]
fn test_message_type_registry_register_and_is_known() {
    let registry = MessageTypeRegistry::new();
    registry.register("job-complete");
    assert!(registry.is_known("job-complete"));
    assert!(!registry.is_known("unknown"));
}

#[test]
fn test_subscription_manager_subscribe_creates_channel() {
    let mgr = SubscriptionManager::new();
    let _rx = mgr.subscribe("test");
}

#[test]
fn test_load_estimator_default() {
    let est = LoadEstimator::default();
    assert_eq!(est.estimation_model, "linear");
}

#[test]
fn test_job_coordinator_default() {
    let coord = JobCoordinator::default();
    assert_eq!(coord.coordination_strategy, "parallel");
}

#[test]
fn test_job_coordinator_with_strategy() {
    let coord = JobCoordinator::with_strategy("sequential");
    assert_eq!(coord.coordination_strategy, "sequential");
}

#[test]
fn test_execution_metrics_serde() {
    let m = ExecutionMetrics {
        start_time: SystemTime::now(),
        end_time: SystemTime::now(),
        cpu_usage: 0.5,
        memory_usage: 1024,
        network_io: 0,
        disk_io: 0,
    };
    let json = serde_json::to_string(&m).unwrap();
    let _: ExecutionMetrics = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_authentication_config_constructor() {
    let config = AuthenticationConfig {
        auth_type: toadstool_common::auth::AuthType::ApiKey,
        api_key: Some("key".to_string()),
        token: None,
        username: None,
        password: None,
    };
    assert!(config.api_key.is_some());
}

// ─── Additional serde round-trips ─────────────────────────────────────────────

#[test]
fn test_connection_health_debug_eq() {
    let health = format!("{:?}", ConnectionHealth::Healthy);
    assert!(health.contains("Healthy"));
    assert_eq!(ConnectionHealth::Healthy, ConnectionHealth::Healthy);
    assert_ne!(ConnectionHealth::Healthy, ConnectionHealth::Unhealthy);
}

#[test]
fn test_songbird_job_response_success_serde() {
    let resp = SongbirdJobResponse::Success {
        job_id: Uuid::new_v4(),
        status: "done".to_string(),
        message: "OK".to_string(),
        estimated_completion: Some(SystemTime::now()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SongbirdJobResponse = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, SongbirdJobResponse::Success { .. }));
}

#[test]
fn test_songbird_job_response_error_serde() {
    let resp = SongbirdJobResponse::Error {
        job_id: Uuid::new_v4(),
        error: "failed".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SongbirdJobResponse = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, SongbirdJobResponse::Error { .. }));
}

#[test]
fn test_job_result_serde() {
    let result = JobResult {
        job_id: Uuid::new_v4(),
        status: "completed".to_string(),
        output: vec![1, 2, 3],
        metrics: ExecutionMetrics {
            start_time: SystemTime::now(),
            end_time: SystemTime::now(),
            cpu_usage: 0.5,
            memory_usage: 1024,
            network_io: 0,
            disk_io: 0,
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: JobResult = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.status, result.status);
    assert_eq!(parsed.output, result.output);
}

#[test]
fn test_job_analysis_serde() {
    let analysis = JobAnalysis {
        complexity: JobComplexity::Complex,
        distribution_strategy: JobDistributionStrategy::LoadBalanced,
        estimated_subtasks: 10,
        resource_requirements: ResourceRequirements::default(),
        preferred_node_types: vec!["compute".to_string()],
    };
    let json = serde_json::to_string(&analysis).unwrap();
    let parsed: JobAnalysis = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.estimated_subtasks, 10);
    assert!(matches!(parsed.complexity, JobComplexity::Complex));
}

#[test]
fn test_intensity_level_serde() {
    for level in [
        IntensityLevel::Low,
        IntensityLevel::Medium,
        IntensityLevel::High,
        IntensityLevel::Extreme,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let _: IntensityLevel = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_job_complexity_all_variants_serde() {
    for c in [
        JobComplexity::Simple,
        JobComplexity::Moderate,
        JobComplexity::Complex,
        JobComplexity::UltraMassive,
    ] {
        let json = serde_json::to_string(&c).unwrap();
        let _: JobComplexity = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_job_distribution_strategy_all_variants_serde() {
    for s in [
        JobDistributionStrategy::LocalOnly,
        JobDistributionStrategy::SplitAndDistribute,
        JobDistributionStrategy::ReplicateAcrossNodes,
        JobDistributionStrategy::SongbirdEcosystem,
        JobDistributionStrategy::LoadBalanced,
        JobDistributionStrategy::MassiveDistribution,
    ] {
        let json = serde_json::to_string(&s).unwrap();
        let _: JobDistributionStrategy = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_sub_task_serde() {
    let task = SubTask {
        id: Uuid::new_v4(),
        payload: bytes::Bytes::from(vec![1, 2, 3]),
        resource_requirements: ResourceRequirements::default(),
        priority: 3,
        constraints: vec!["gpu".to_string()],
    };
    let json = serde_json::to_string(&task).unwrap();
    let parsed: SubTask = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.priority, 3);
    assert_eq!(parsed.constraints, task.constraints);
}

#[test]
fn test_sub_task_handle_serde() {
    let handle = SubTaskHandle {
        subtask_id: Uuid::new_v4(),
        songbird_job_id: Uuid::new_v4(),
        target_nodes: vec!["n1".to_string()],
        submitted_at: SystemTime::now(),
        status: SubTaskStatus::Running,
    };
    let json = serde_json::to_string(&handle).unwrap();
    let parsed: SubTaskHandle = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed.status, SubTaskStatus::Running));
}

#[test]
fn test_sub_task_status_all_variants_serde() {
    for s in [
        SubTaskStatus::Submitted,
        SubTaskStatus::Running,
        SubTaskStatus::Completed,
        SubTaskStatus::Failed,
    ] {
        let json = serde_json::to_string(&s).unwrap();
        let _: SubTaskStatus = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_coordination_job_serde() {
    let job = CoordinationJob {
        job_id: Uuid::new_v4(),
        original_job_id: Uuid::new_v4(),
        subtask_count: 5,
        completion_strategy: CompletionStrategy::WaitForMajority,
    };
    let json = serde_json::to_string(&job).unwrap();
    let parsed: CoordinationJob = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.subtask_count, 5);
}

#[test]
fn test_distribution_plan_serde() {
    let plan = DistributionPlan {
        plan_id: Uuid::new_v4(),
        job_id: Uuid::new_v4(),
        subtasks: vec![],
        coordination_strategy: CoordinationStrategy::Parallel,
    };
    let json = serde_json::to_string(&plan).unwrap();
    let parsed: DistributionPlan = serde_json::from_str(&json).unwrap();
    assert!(matches!(
        parsed.coordination_strategy,
        CoordinationStrategy::Parallel
    ));
}

#[test]
fn test_sub_task_plan_serde() {
    let plan = SubTaskPlan {
        subtask_id: Uuid::new_v4(),
        target_nodes: vec!["n1".to_string()],
        resource_allocation: ResourceRequirements::default(),
        dependencies: vec![Uuid::new_v4()],
    };
    let json = serde_json::to_string(&plan).unwrap();
    let _: SubTaskPlan = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_coordination_strategy_all_variants_serde() {
    for s in [
        CoordinationStrategy::Sequential,
        CoordinationStrategy::Parallel,
        CoordinationStrategy::Pipeline,
        CoordinationStrategy::MapReduce,
    ] {
        let json = serde_json::to_string(&s).unwrap();
        let _: CoordinationStrategy = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_massive_job_result_local_serde() {
    let result = MassiveJobResult::Local {
        result: JobResult {
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
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    let _: MassiveJobResult = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_authentication_config_serde() {
    let config = AuthenticationConfig {
        auth_type: toadstool_common::auth::AuthType::Bearer,
        api_key: None,
        token: Some("bearer-token".to_string()),
        username: None,
        password: None,
    };
    let json = serde_json::to_string(&config).unwrap();
    let _: AuthenticationConfig = serde_json::from_str(&json).unwrap();
}

#[test]
fn test_songbird_broadcast_message_serde() {
    let msg = SongbirdBroadcastMessage::CapabilityUpdate {
        node_id: "n1".to_string(),
        capabilities: NodeCapabilities {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            storage_gb: 100.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        },
        timestamp: SystemTime::now(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: SongbirdBroadcastMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.channel_name(), "capability-updates");
}

#[test]
fn test_songbird_feedback_variants() {
    let load = SongbirdFeedback::LoadUpdate {
        node_id: "n1".to_string(),
        load: 0.5,
    };
    assert!(matches!(load, SongbirdFeedback::LoadUpdate { .. }));
    let err = SongbirdFeedback::ErrorReport {
        node_id: "n1".to_string(),
        error: "fail".to_string(),
    };
    assert!(matches!(err, SongbirdFeedback::ErrorReport { .. }));
    let cap = SongbirdFeedback::CapacityAvailable {
        node_id: "n1".to_string(),
    };
    assert!(matches!(cap, SongbirdFeedback::CapacityAvailable { .. }));
}

#[test]
fn test_message_type_registry_known_types() {
    let registry = MessageTypeRegistry::new();
    registry.register("a");
    registry.register("b");
    let known = registry.known_types();
    assert_eq!(known.len(), 2);
    assert!(known.iter().any(|s| s == "a"));
}

#[test]
fn test_subscription_manager_publish_no_channel() {
    let mgr = SubscriptionManager::new();
    let msg = SongbirdBroadcastMessage::HealthUpdate {
        node_id: "n1".to_string(),
        health_status: "ok".to_string(),
        timestamp: SystemTime::now(),
    };
    let n = mgr.publish("nonexistent", msg);
    assert_eq!(n, 0);
}

#[test]
fn test_subscription_manager_close_channel() {
    let mgr = SubscriptionManager::new();
    let _sub = mgr.subscribe("ch");
    mgr.close_channel("ch");
    let msg = SongbirdBroadcastMessage::HealthUpdate {
        node_id: "n1".to_string(),
        health_status: "ok".to_string(),
        timestamp: SystemTime::now(),
    };
    let n = mgr.publish("ch", msg);
    assert_eq!(n, 0);
}

#[test]
fn test_node_capacity_tracker_clamp_load() {
    let tracker = NodeCapacityTracker::new();
    tracker.update(&"n1".to_string(), 1.5);
    let snap = tracker.snapshot();
    assert_eq!(snap.get("n1").copied(), Some(1.0));
}

#[test]
fn test_songbird_feedback_sender_default() {
    let _sender = SongbirdFeedbackSender::default();
}

// ─── Additional coverage: config structs, connection, more serde ───────────

#[test]
fn test_songbird_connection_constructor() {
    use protocols::{
        GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, SongbirdProtocol,
    };
    let conn = SongbirdConnection {
        endpoints: vec!["http://a".to_string(), "http://b".to_string()],
        active_endpoint: "http://a".to_string(),
        auth_token: Some("token".to_string()),
        health_status: ConnectionHealth::Healthy,
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
        #[cfg(feature = "channels")]
        reply_channel: None,
    };
    assert_eq!(conn.endpoints.len(), 2);
    assert_eq!(conn.health_status, ConnectionHealth::Healthy);
}

#[test]
fn test_songbird_broadcast_message_health_update_serde() {
    let msg = SongbirdBroadcastMessage::HealthUpdate {
        node_id: "n1".to_string(),
        health_status: "ok".to_string(),
        timestamp: SystemTime::now(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: SongbirdBroadcastMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.channel_name(), "health-updates");
}

#[test]
fn test_songbird_broadcast_message_custom_message_serde() {
    let msg = SongbirdBroadcastMessage::CustomMessage {
        message_type: "job-complete".to_string(),
        payload: serde_json::json!({"job_id": "abc"}),
        timestamp: SystemTime::now(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let parsed: SongbirdBroadcastMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.channel_name(), "job-complete");
}

#[test]
fn test_completion_strategy_custom_serde() {
    let s = CompletionStrategy::Custom("my-strategy".to_string());
    let json = serde_json::to_string(&s).unwrap();
    let parsed: CompletionStrategy = serde_json::from_str(&json).unwrap();
    match &parsed {
        CompletionStrategy::Custom(v) => assert_eq!(v, "my-strategy"),
        _ => panic!("expected Custom"),
    }
}

#[test]
fn test_completion_strategy_all_variants_serde() {
    for s in [
        CompletionStrategy::WaitForAll,
        CompletionStrategy::WaitForMajority,
        CompletionStrategy::WaitForAny,
        CompletionStrategy::Custom("x".to_string()),
    ] {
        let json = serde_json::to_string(&s).unwrap();
        let _: CompletionStrategy = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_massive_job_result_distributed_serde() {
    let result = MassiveJobResult::Distributed {
        original_job_id: Uuid::new_v4(),
        subtask_handles: vec![],
        coordination_job: CoordinationJob {
            job_id: Uuid::new_v4(),
            original_job_id: Uuid::new_v4(),
            subtask_count: 0,
            completion_strategy: CompletionStrategy::WaitForAll,
        },
        distribution_plan: DistributionPlan {
            plan_id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            subtasks: vec![],
            coordination_strategy: CoordinationStrategy::Parallel,
        },
    };
    let json = serde_json::to_string(&result).unwrap();
    let parsed: MassiveJobResult = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed, MassiveJobResult::Distributed { .. }));
}

#[test]
fn test_distribution_config_constructor() {
    let mut strategies = HashMap::new();
    strategies.insert("compute".to_string(), "split".to_string());
    let config = DistributionConfig {
        max_subtasks: 16,
        splitting_strategies: strategies,
    };
    assert_eq!(config.max_subtasks, 16);
}

#[test]
fn test_distribution_config_serde() {
    let mut strategies = HashMap::new();
    strategies.insert("cpu".to_string(), "parallel".to_string());
    let config = DistributionConfig {
        max_subtasks: 8,
        splitting_strategies: strategies,
    };
    let json = serde_json::to_string(&config).unwrap();
    let parsed: DistributionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.max_subtasks, 8);
}

#[test]
fn test_load_balancer_config_constructor() {
    let config = LoadBalancerConfig {
        strategy: "least-loaded".to_string(),
        feedback_interval: Duration::from_secs(5),
    };
    assert_eq!(config.strategy, "least-loaded");
}

#[test]
fn test_broadcast_config_constructor() {
    let config = BroadcastConfig {
        channels: vec!["events".to_string(), "alerts".to_string()],
        message_retention: Duration::from_secs(60),
    };
    assert_eq!(config.channels.len(), 2);
}

#[test]
fn test_capacity_config_constructor() {
    let config = CapacityConfig {
        monitoring_interval: Duration::from_secs(30),
        resource_buffer: 0.1,
    };
    assert!((config.resource_buffer - 0.1).abs() < 0.001);
}

#[test]
fn test_receiver_config_constructor() {
    let config = ReceiverConfig {
        max_concurrent_jobs: 10,
        job_timeout: Duration::from_secs(300),
    };
    assert_eq!(config.max_concurrent_jobs, 10);
}
