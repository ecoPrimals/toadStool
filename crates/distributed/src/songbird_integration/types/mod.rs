//! Type definitions for Songbird Integration

mod capabilities;
mod discovery;
mod node;
mod protocols;

pub use capabilities::{CapabilitySnapshot, CapabilityTracker};
pub use discovery::{
    AvailableCapacity, DiscoveryClient, LoadBalancingAdvice, NetworkCapacity, NetworkHealthMonitor,
    NetworkRequirements, NetworkStatus, NodeMetadata, NodeRegistration, NodeRegistry, NodeType,
    RegistrationResponse, ResourceReservation,
};
pub use node::{NodeCapabilities, NodeId};
pub use protocols::{
    GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig,
    SongbirdProtocol,
};

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use toadstool_common::auth::{AuthType, ServiceAuthConfig};
use toadstool_common::config_bases::ConnectionPoolConfig;
use tokio::sync::{broadcast, mpsc, RwLock};
use uuid::Uuid;

use crate::{ResourceRequirements, UniversalJob};

// ============================================================================
// Connection Types (required by discovery)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionHealth {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct SongbirdConnection {
    pub endpoints: Vec<String>,
    pub active_endpoint: String,
    pub auth_token: Option<String>,
    pub health_status: ConnectionHealth,
    pub protocol_config: ProtocolConfig,
    #[cfg(feature = "channels")]
    pub reply_channel: Option<Arc<tokio::sync::mpsc::UnboundedSender<SongbirdJobResponse>>>,
}

// ============================================================================
// Core Integration Types
// ============================================================================

pub struct ToadStoolSongbirdIntegration {
    pub(super) instance_id: String,
    pub(super) connection: SongbirdConnection,
    pub(super) local_capacity: Arc<LocalCapacityManager>,
    pub(super) workload_scheduler: Arc<crate::universal::UniversalScheduler>,
}

pub struct MassiveJobDistributor {
    pub(super) splitting_strategies: HashMap<crate::UniversalJobType, JobSplittingStrategy>,
    pub(super) distribution_algorithms: Vec<DistributionAlgorithm>,
    pub(super) load_estimator: LoadEstimator,
    pub(super) job_coordinator: JobCoordinator,
}

pub struct SongbirdNetworkDiscovery {
    pub(super) discovery_client: DiscoveryClient,
    pub(super) node_registry: RwLock<NodeRegistry>,
    pub(super) capability_tracker: CapabilityTracker,
    pub(super) health_monitor: NetworkHealthMonitor,
}

pub struct SongbirdLoadBalancer {
    pub(super) strategies: HashMap<String, LoadBalancingStrategy>,
    pub(super) capacity_tracker: NodeCapacityTracker,
    pub(super) performance_metrics: PerformanceMetrics,
    pub(super) feedback_sender: SongbirdFeedbackSender,
}

pub struct SongbirdBroadcaster {
    pub(super) channels: HashMap<String, BroadcastChannel>,
    pub(super) message_types: MessageTypeRegistry,
    pub(super) subscription_manager: SubscriptionManager,
}

// ============================================================================
// Job Request/Response Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdJobRequest {
    pub job_id: Uuid,
    pub job_payload: Vec<u8>,
    pub target_nodes: Vec<String>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobAnalysis {
    pub complexity: JobComplexity,
    pub distribution_strategy: JobDistributionStrategy,
    pub estimated_subtasks: usize,
    pub resource_requirements: ResourceRequirements,
    pub preferred_node_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdJobResponse {
    Success {
        job_id: Uuid,
        status: String,
        message: String,
        estimated_completion: Option<DateTime<Utc>>,
    },
    Error {
        job_id: Uuid,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub job_id: Uuid,
    pub status: String,
    pub output: Vec<u8>,
    pub metrics: ExecutionMetrics,
}

// ============================================================================
// Job Complexity and Distribution
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobComplexity {
    Simple,
    Moderate,
    Complex,
    UltraMassive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplexityLevel {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntensityLevel {
    Low,
    Medium,
    High,
    Extreme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobDistributionStrategy {
    LocalOnly,
    SplitAndDistribute,
    ReplicateAcrossNodes,
    HybridExecution,
    SongbirdEcosystem,
    LoadBalanced,
    MassiveDistribution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MassiveJobResult {
    Local {
        result: JobResult,
    },
    Distributed {
        original_job_id: Uuid,
        subtask_handles: Vec<SubTaskHandle>,
        coordination_job: CoordinationJob,
        distribution_plan: DistributionPlan,
    },
}

// ============================================================================
// Sub-task and Coordination Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTask {
    pub id: Uuid,
    pub payload: Vec<u8>,
    pub resource_requirements: ResourceRequirements,
    pub priority: u8,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskHandle {
    pub subtask_id: Uuid,
    pub songbird_job_id: Uuid,
    pub target_nodes: Vec<String>,
    pub submitted_at: DateTime<Utc>,
    pub status: SubTaskStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubTaskStatus {
    Submitted,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinationJob {
    pub job_id: Uuid,
    pub original_job_id: Uuid,
    pub subtask_count: usize,
    pub completion_strategy: CompletionStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionStrategy {
    WaitForAll,
    WaitForMajority,
    WaitForAny,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPlan {
    pub plan_id: Uuid,
    pub job_id: Uuid,
    pub subtasks: Vec<SubTaskPlan>,
    pub coordination_strategy: CoordinationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubTaskPlan {
    pub subtask_id: Uuid,
    pub target_nodes: Vec<String>,
    pub resource_allocation: ResourceRequirements,
    pub dependencies: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinationStrategy {
    Sequential,
    Parallel,
    Pipeline,
    MapReduce,
}

// ============================================================================
// Authentication and Config
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationConfig {
    pub auth_type: AuthType,
    pub api_key: Option<String>,
    pub token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

pub type AuthConfig = ServiceAuthConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdIntegrationConfig {
    pub connection_config: SongbirdConnectionConfig,
    pub distribution_config: DistributionConfig,
    pub discovery_config: SongbirdDiscoveryConfig,
    pub load_balancer_config: LoadBalancerConfig,
    pub broadcast_config: BroadcastConfig,
    pub capacity_config: CapacityConfig,
    pub receiver_config: ReceiverConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdConnectionConfig {
    pub endpoints: Vec<String>,
    pub protocol_config: ProtocolConfig,
    pub auth_config: AuthConfig,
    #[serde(flatten)]
    pub pool: ConnectionPoolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    pub max_subtasks: usize,
    pub splitting_strategies: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongbirdDiscoveryConfig {
    pub discovery_interval: Duration,
    pub node_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    pub strategy: String,
    pub feedback_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastConfig {
    pub channels: Vec<String>,
    pub message_retention: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityConfig {
    pub monitoring_interval: Duration,
    pub resource_buffer: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiverConfig {
    pub max_concurrent_jobs: usize,
    pub job_timeout: Duration,
}

// ============================================================================
// Message Types
// ============================================================================

pub enum SongbirdJobMessage {
    ExecuteJob {
        job: Box<UniversalJob>,
        reply_channel: mpsc::Sender<SongbirdJobResponse>,
    },
    CancelJob {
        job_id: Uuid,
    },
    StatusUpdate {
        job_id: Uuid,
        status: SubTaskStatus,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SongbirdBroadcastMessage {
    CapabilityUpdate {
        node_id: NodeId,
        capabilities: NodeCapabilities,
        timestamp: DateTime<Utc>,
    },
    HealthUpdate {
        node_id: NodeId,
        health_status: String,
        timestamp: DateTime<Utc>,
    },
    CustomMessage {
        message_type: String,
        payload: serde_json::Value,
        timestamp: DateTime<Utc>,
    },
}

impl SongbirdBroadcastMessage {
    /// Derive a routing channel name from the message variant.
    ///
    /// Used by `SongbirdBroadcaster::broadcast()` to route to the correct channel.
    pub fn channel_name(&self) -> &str {
        match self {
            Self::CapabilityUpdate { .. } => "capability-updates",
            Self::HealthUpdate { .. } => "health-updates",
            Self::CustomMessage { message_type, .. } => message_type.as_str(),
        }
    }
}

#[cfg(test)]
mod channel_name_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DistributedRetryConfig, ExecutionTarget, ResourceRequirements, UniversalJobType};
    use chrono::Utc;

    fn make_test_job(resource_requirements: ResourceRequirements) -> UniversalJob {
        UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(UniversalJobType::Local),
            execution_request: toadstool::ExecutionRequest::default(),
            target: ExecutionTarget::Local,
            priority: toadstool::JobPriority::Normal,
            dependencies: vec![],
            resource_requirements,
            retry_config: DistributedRetryConfig::default(),
            created_at: Utc::now(),
        }
    }

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
            job_payload: vec![1, 2, 3],
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
            job_payload: vec![1, 2, 3],
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
            payload: vec![],
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
            submitted_at: Utc::now(),
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
            timestamp: Utc::now(),
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
            start_time: Utc::now(),
            end_time: Utc::now(),
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
            estimated_completion: Some(Utc::now()),
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
                start_time: Utc::now(),
                end_time: Utc::now(),
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
            payload: vec![1, 2, 3],
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
            submitted_at: Utc::now(),
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
                    start_time: Utc::now(),
                    end_time: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
        };
        assert_eq!(conn.endpoints.len(), 2);
        assert_eq!(conn.health_status, ConnectionHealth::Healthy);
    }

    #[test]
    fn test_songbird_broadcast_message_health_update_serde() {
        let msg = SongbirdBroadcastMessage::HealthUpdate {
            node_id: "n1".to_string(),
            health_status: "ok".to_string(),
            timestamp: Utc::now(),
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
            timestamp: Utc::now(),
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
            start_time: Utc::now(),
            end_time: Utc::now(),
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

    // ─── Additional serde round-trips for discovery, node, protocol types ────────

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
        use node::NodeCapabilities;
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
        use node::NodeCapabilities;
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
                start_time: Utc::now(),
                end_time: Utc::now(),
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

    // ─── Additional serde roundtrips for maximum coverage ─────────────────────

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
            submitted_at: Utc::now(),
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
                timestamp: Utc::now(),
            }
            .channel_name(),
            "capability-updates"
        );
        assert_eq!(
            SongbirdBroadcastMessage::HealthUpdate {
                node_id: "n".to_string(),
                health_status: "ok".to_string(),
                timestamp: Utc::now(),
            }
            .channel_name(),
            "health-updates"
        );
        assert_eq!(
            SongbirdBroadcastMessage::CustomMessage {
                message_type: "custom".to_string(),
                payload: serde_json::Value::Null,
                timestamp: Utc::now(),
            }
            .channel_name(),
            "custom"
        );
    }

    #[test]
    fn test_execution_metrics_zero_values_serde() {
        let m = ExecutionMetrics {
            start_time: Utc::now(),
            end_time: Utc::now(),
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
}

// ============================================================================
// Capacity Management
// ============================================================================

pub struct LocalCapacityManager {
    pub(super) available_capacity: Arc<RwLock<CapacityInfo>>,
}

#[derive(Debug, Clone)]
pub struct CapacityInfo {
    pub cpu_cores: f64,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
}

impl CapacityInfo {
    #[must_use]
    pub fn can_handle_job(&self, job: &UniversalJob) -> bool {
        let requirements = &job.resource_requirements;
        requirements.cpu.min_cores <= self.cpu_cores
            && requirements.memory.min_bytes <= self.memory_bytes
            && requirements.storage.min_bytes <= self.storage_bytes
    }

    #[must_use]
    pub fn from_system() -> Self {
        use sysinfo::Disks;
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(1.0);
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let memory_bytes = sys.available_memory();
        let disks = Disks::new_with_refreshed_list();
        let storage_bytes: u64 = disks
            .iter()
            .filter(|disk| {
                let fs = disk.file_system().to_string_lossy();
                !fs.contains("tmpfs")
                    && !fs.contains("devtmpfs")
                    && !fs.contains("squashfs")
                    && !fs.contains("overlay")
            })
            .map(sysinfo::Disk::available_space)
            .sum();
        Self {
            cpu_cores,
            memory_bytes,
            storage_bytes,
        }
    }
}

// ============================================================================
// Metrics and Distribution Support
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub cpu_usage: f64,
    pub memory_usage: u64,
    pub network_io: u64,
    pub disk_io: u64,
}

pub struct LoadMetric {
    pub cpu_load: f64,
    pub memory_load: f64,
    pub network_load: f64,
}

pub struct JobSplittingStrategy {
    pub strategy_type: SplittingStrategyType,
    pub max_subtasks: usize,
    pub min_subtask_size: usize,
}

impl JobSplittingStrategy {
    pub async fn split_job(&self, job: &UniversalJob) -> Vec<SubTask> {
        if self.max_subtasks <= 1 {
            return vec![];
        }
        let cpu_cores = job.resource_requirements.cpu.min_cores as usize;
        let num_subtasks = std::cmp::min(self.max_subtasks, cpu_cores.max(2));
        match &self.strategy_type {
            SplittingStrategyType::DataParallel => {
                self.split_data_parallel(job, num_subtasks).await
            }
            SplittingStrategyType::TaskParallel => {
                self.split_task_parallel(job, num_subtasks).await
            }
            SplittingStrategyType::MapReduce => self.split_map_reduce(job, num_subtasks).await,
            _ => self.split_task_parallel(job, num_subtasks).await,
        }
    }

    async fn split_data_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        let mut subtasks = Vec::with_capacity(num_subtasks);
        let per_task_cpu = (job.resource_requirements.cpu.min_cores / num_subtasks as f64).max(0.5);
        let per_task_memory = job.resource_requirements.memory.min_bytes / num_subtasks as u64;
        for i in 0..num_subtasks {
            subtasks.push(SubTask {
                id: Uuid::new_v4(),
                payload: vec![],
                resource_requirements: ResourceRequirements {
                    cpu: crate::types::resources::CpuRequirements {
                        min_cores: per_task_cpu,
                        max_cores: job
                            .resource_requirements
                            .cpu
                            .max_cores
                            .map(|c| c / num_subtasks as f64),
                    },
                    memory: crate::types::resources::MemoryRequirements {
                        min_bytes: per_task_memory,
                        max_bytes: job
                            .resource_requirements
                            .memory
                            .max_bytes
                            .map(|m| m / num_subtasks as u64),
                    },
                    storage: job.resource_requirements.storage.clone(),
                    network: job.resource_requirements.network.clone(),
                    gpu: job.resource_requirements.gpu.clone(),
                },
                priority: job.priority as u8,
                constraints: vec![format!("chunk_{}_of_{}", i, num_subtasks)],
            });
        }
        subtasks
    }

    async fn split_task_parallel(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        (0..num_subtasks)
            .map(|i| SubTask {
                id: Uuid::new_v4(),
                payload: vec![],
                resource_requirements: job.resource_requirements.clone(),
                priority: job.priority as u8,
                constraints: vec![format!("task_{}_of_{}", i, num_subtasks)],
            })
            .collect()
    }

    async fn split_map_reduce(&self, job: &UniversalJob, num_subtasks: usize) -> Vec<SubTask> {
        self.split_data_parallel(job, num_subtasks).await
    }
}

pub enum SplittingStrategyType {
    DataParallel,
    TaskParallel,
    Pipeline,
    MapReduce,
    Custom(String),
}

pub type DistributionAlgorithm = crate::common::distribution::DistributionAlgorithm;

pub struct LoadEstimator {
    pub estimation_model: String,
}

impl Default for LoadEstimator {
    fn default() -> Self {
        Self {
            estimation_model: "linear".to_string(),
        }
    }
}

impl LoadEstimator {
    pub async fn estimate_load(&self, job: &UniversalJob) -> LoadMetric {
        let cpu_cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(4.0);
        let mut sys = sysinfo::System::new_all();
        sys.refresh_memory();
        let total_memory = sys.total_memory() as f64;
        let cpu_load = (job.resource_requirements.cpu.min_cores / cpu_cores).min(1.0);
        let memory_load =
            (job.resource_requirements.memory.min_bytes as f64 / total_memory).min(1.0);
        let network_load = if let Some(bandwidth) = job.resource_requirements.network.bandwidth_mbps
        {
            (bandwidth as f64 / 1000.0).min(1.0)
        } else {
            match &job.job_type {
                Some(crate::types::jobs::UniversalJobType::Local) => 0.1,
                Some(crate::types::jobs::UniversalJobType::Native) => 0.1,
                Some(crate::types::jobs::UniversalJobType::RemoteToadStool { .. }) => 0.3,
                Some(crate::types::jobs::UniversalJobType::EcosystemTool { .. }) => 0.2,
                Some(crate::types::jobs::UniversalJobType::RecursiveHosting { .. }) => 0.4,
                Some(crate::types::jobs::UniversalJobType::NetworkIntensive) => 0.8,
                Some(crate::types::jobs::UniversalJobType::DataProcessing) => 0.4,
                Some(crate::types::jobs::UniversalJobType::MachineLearning) => 0.3,
                Some(_) => 0.2,
                None => 0.2,
            }
        };
        LoadMetric {
            cpu_load,
            memory_load,
            network_load,
        }
    }
}

pub struct JobCoordinator {
    pub coordination_strategy: String,
}

impl Default for JobCoordinator {
    fn default() -> Self {
        Self {
            coordination_strategy: "parallel".to_string(),
        }
    }
}

impl JobCoordinator {
    pub async fn coordinate(&self, plan: &DistributionPlan) -> CoordinationJob {
        let completion_strategy = match plan.coordination_strategy {
            CoordinationStrategy::Sequential => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Parallel => CompletionStrategy::WaitForAll,
            CoordinationStrategy::Pipeline => CompletionStrategy::WaitForAll,
            CoordinationStrategy::MapReduce => CompletionStrategy::WaitForAll,
        };
        CoordinationJob {
            job_id: Uuid::new_v4(),
            original_job_id: plan.job_id,
            subtask_count: plan.subtasks.len(),
            completion_strategy,
        }
    }

    #[must_use]
    pub fn with_strategy(strategy: &str) -> Self {
        Self {
            coordination_strategy: strategy.to_string(),
        }
    }
}

// ============================================================================
// Load Balancing Types
// ============================================================================

/// Named strategy handle — the string identifies which algorithm to apply
/// (e.g. "round-robin", "least-loaded", "capability-aware").
pub type LoadBalancingStrategy = String;

/// Per-node load tracking.
///
/// Records the most-recently observed load fraction (0.0–1.0) and when it
/// was updated. Used by `SongbirdLoadBalancer::request_advice` to select
/// the least-loaded eligible node.
pub struct NodeCapacityTracker {
    /// `node_id → (load_fraction, updated_at)`
    inner: Mutex<HashMap<NodeId, (f64, Instant)>>,
}

impl NodeCapacityTracker {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// Record a load observation for the given node (0.0 = idle, 1.0 = saturated).
    pub fn update(&self, node_id: &NodeId, load: f64) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.insert(node_id.clone(), (load.clamp(0.0, 1.0), Instant::now()));
        }
    }

    /// Return the node with the lowest tracked load, or `None` if no data.
    pub fn least_loaded(&self) -> Option<NodeId> {
        self.inner.lock().ok().and_then(|guard| {
            guard
                .iter()
                .min_by(|a, b| {
                    a.1 .0
                        .partial_cmp(&b.1 .0)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(id, _)| id.clone())
        })
    }

    /// Current load snapshot: `node_id → load_fraction`.
    pub fn snapshot(&self) -> HashMap<NodeId, f64> {
        self.inner
            .lock()
            .ok()
            .map(|g| g.iter().map(|(k, (v, _))| (k.clone(), *v)).collect())
            .unwrap_or_default()
    }
}

impl Default for NodeCapacityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Lightweight in-process performance counters for a Songbird connection.
///
/// Tracks request count, error count, and cumulative latency so callers
/// can compute p50/p95 approximations or derive error rates.
pub struct PerformanceMetrics {
    inner: Mutex<PerformanceCounters>,
}

#[derive(Default)]
struct PerformanceCounters {
    requests: u64,
    errors: u64,
    total_latency_ms: u64,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PerformanceCounters::default()),
        }
    }

    /// Record a completed request with its latency and whether it failed.
    pub fn record(&self, latency_ms: u64, is_error: bool) {
        if let Ok(mut g) = self.inner.lock() {
            g.requests += 1;
            g.total_latency_ms += latency_ms;
            if is_error {
                g.errors += 1;
            }
        }
    }

    /// Error rate in [0.0, 1.0]; 0.0 when no requests recorded.
    pub fn error_rate(&self) -> f64 {
        self.inner
            .lock()
            .ok()
            .map(|g| {
                if g.requests == 0 {
                    0.0
                } else {
                    g.errors as f64 / g.requests as f64
                }
            })
            .unwrap_or(0.0)
    }

    /// Mean latency in milliseconds; 0.0 when no requests recorded.
    pub fn mean_latency_ms(&self) -> f64 {
        self.inner
            .lock()
            .ok()
            .map(|g| {
                if g.requests == 0 {
                    0.0
                } else {
                    g.total_latency_ms as f64 / g.requests as f64
                }
            })
            .unwrap_or(0.0)
    }

    pub fn request_count(&self) -> u64 {
        self.inner.lock().ok().map(|g| g.requests).unwrap_or(0)
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Feedback messages sent back to Songbird about local node state.
#[derive(Debug, Clone)]
pub enum SongbirdFeedback {
    LoadUpdate { node_id: NodeId, load: f64 },
    ErrorReport { node_id: NodeId, error: String },
    CapacityAvailable { node_id: NodeId },
}

/// Sends feedback events to Songbird's coordination loop.
///
/// Backed by an unbounded mpsc channel. Callers `send()` feedback; a
/// background task (or the Songbird client) drains `SongbirdFeedbackReceiver`.
pub struct SongbirdFeedbackSender {
    tx: mpsc::UnboundedSender<SongbirdFeedback>,
}

pub struct SongbirdFeedbackReceiver {
    pub rx: mpsc::UnboundedReceiver<SongbirdFeedback>,
}

impl SongbirdFeedbackSender {
    pub fn new() -> (Self, SongbirdFeedbackReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Self { tx }, SongbirdFeedbackReceiver { rx })
    }

    /// Emit a feedback event. Returns `false` if the receiver has been dropped.
    pub fn send(&self, feedback: SongbirdFeedback) -> bool {
        self.tx.send(feedback).is_ok()
    }
}

impl Default for SongbirdFeedbackSender {
    fn default() -> Self {
        Self::new().0
    }
}

// ============================================================================
// Broadcasting Types
// ============================================================================

/// A named pub/sub broadcast channel backed by `tokio::sync::broadcast`.
///
/// `BroadcastChannel::subscribe()` returns a `broadcast::Receiver` that
/// receives all future messages sent on this channel.
pub struct BroadcastChannel {
    name: String,
    tx: broadcast::Sender<SongbirdBroadcastMessage>,
}

impl BroadcastChannel {
    pub fn new(name: impl Into<String>) -> Self {
        let (tx, _) = broadcast::channel(64);
        Self {
            name: name.into(),
            tx,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Publish a message to all current subscribers.
    ///
    /// Returns `Ok(n)` where `n` is the number of active receivers.
    /// Returns `Err(broadcast::error::SendError)` only when there are no
    /// subscribers (harmless — callers can ignore or log).
    pub fn publish(
        &self,
        msg: SongbirdBroadcastMessage,
    ) -> Result<usize, broadcast::error::SendError<SongbirdBroadcastMessage>> {
        self.tx.send(msg)
    }

    /// Subscribe to this channel.
    pub fn subscribe(&self) -> broadcast::Receiver<SongbirdBroadcastMessage> {
        self.tx.subscribe()
    }
}

/// Registry of known message type names for routing / validation.
///
/// Prevents typos in channel names and provides a single source of truth
/// for which message types are in use.
pub struct MessageTypeRegistry {
    types: Mutex<HashSet<String>>,
}

impl MessageTypeRegistry {
    pub fn new() -> Self {
        Self {
            types: Mutex::new(HashSet::new()),
        }
    }

    /// Register a message type; idempotent.
    pub fn register(&self, type_name: impl Into<String>) {
        if let Ok(mut g) = self.types.lock() {
            g.insert(type_name.into());
        }
    }

    /// `true` if the type was previously registered.
    pub fn is_known(&self, type_name: &str) -> bool {
        self.types
            .lock()
            .ok()
            .is_some_and(|g| g.contains(type_name))
    }

    pub fn known_types(&self) -> Vec<String> {
        self.types
            .lock()
            .ok()
            .map(|g| g.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for MessageTypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Maps channel names to their broadcast channels.
///
/// Callers call `get_or_create(name)` to obtain a subscriber handle.
/// The channel is created on first access; subsequent calls return a
/// new `broadcast::Receiver` from the same sender.
pub struct SubscriptionManager {
    channels: Mutex<HashMap<String, broadcast::Sender<SongbirdBroadcastMessage>>>,
}

impl SubscriptionManager {
    pub fn new() -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to a named channel, creating it if it does not yet exist.
    pub fn subscribe(&self, channel: &str) -> broadcast::Receiver<SongbirdBroadcastMessage> {
        let mut guard = self
            .channels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard
            .entry(channel.to_string())
            .or_insert_with(|| broadcast::channel(64).0)
            .subscribe()
    }

    /// Publish a message to a named channel.
    ///
    /// Returns `0` if the channel does not exist or has no subscribers.
    pub fn publish(&self, channel: &str, msg: SongbirdBroadcastMessage) -> usize {
        self.channels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(channel)
            .and_then(|tx| tx.send(msg).ok())
            .unwrap_or(0)
    }

    /// Unsubscribe by dropping all senders for a channel (channel closes).
    pub fn close_channel(&self, channel: &str) {
        self.channels
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(channel);
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Job Receiver
// ============================================================================

pub struct JobReceiver {
    pub receiver: mpsc::Receiver<SongbirdJobMessage>,
}

impl JobReceiver {
    pub async fn receive(&mut self) -> Option<SongbirdJobMessage> {
        self.receiver.recv().await
    }
}

pub struct UniversalJobProcessor {
    pub processor_id: String,
}
