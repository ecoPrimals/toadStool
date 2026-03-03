// SPDX-License-Identifier: AGPL-3.0-or-later
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use toadstool_common::constants::timeouts;
use uuid::Uuid;

use crate::network::{FaultToleranceManager, NetworkLoadBalancer, NetworkMetricsCollector};
use crate::types::{DistributedExecution, UniversalJob};
use toadstool::ToadStoolResult;

/// Configuration for network distributor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkDistributorConfig {
    /// Enable network distribution
    pub enabled: bool,
    /// Maximum concurrent distributions
    pub max_concurrent_distributions: u32,
    /// Distribution timeout (properly typed)
    #[serde(with = "humantime_serde")]
    pub distribution_timeout: Duration,
}

impl Default for NetworkDistributorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_distributions: 10,
            distribution_timeout: timeouts::WORKLOAD_EXECUTION_TIMEOUT,
        }
    }
}

/// Network distributor for distributed execution
pub struct NetworkDistributor {
    /// Configuration
    config: NetworkDistributorConfig,
    /// Load balancer — also serves as the live node registry
    load_balancer: Arc<NetworkLoadBalancer>,
    /// Fault tolerance manager
    _fault_tolerance: Arc<FaultToleranceManager>,
    /// Metrics collector
    _metrics: Arc<NetworkMetricsCollector>,
}

impl NetworkDistributor {
    /// Create a new network distributor
    #[must_use]
    pub fn new(config: NetworkDistributorConfig) -> Self {
        Self {
            config,
            load_balancer: Arc::new(NetworkLoadBalancer::new()),
            _fault_tolerance: Arc::new(FaultToleranceManager::new()),
            _metrics: Arc::new(NetworkMetricsCollector::new()),
        }
    }

    /// Register a discovered node so future `distribute_job` calls can reach it.
    ///
    /// Call this from Songbird capability discovery whenever a peer primal
    /// announces itself via mDNS-SD or the Songbird registry.
    pub async fn register_peer_node(&self, node_id: String, health: crate::network::NodeHealth) {
        self.load_balancer.register_node(node_id, health).await;
    }

    /// Deregister a node (e.g. on failed health probe).
    pub async fn deregister_peer_node(&self, node_id: &str) {
        self.load_balancer.deregister_node(node_id).await;
    }

    /// Distribute a job across the network.
    ///
    /// Selects the least-loaded healthy remote node from the load balancer.
    /// If no remote nodes are registered, falls back to a local-execution assignment
    /// so the caller always gets a valid `DistributedExecution` to work with.
    pub async fn distribute_job(&self, job: UniversalJob) -> ToadStoolResult<DistributedExecution> {
        if !self.config.enabled {
            tracing::debug!("Network distribution disabled; returning local-only execution.");
            return Ok(self.local_execution(job));
        }

        let target_node = match self.load_balancer.select_node().await {
            Some(node) => node,
            None => {
                tracing::debug!(
                    "No remote nodes registered; falling back to local-node execution."
                );
                return Ok(self.local_execution(job));
            }
        };

        let req = &job.resource_requirements;
        let assignment = crate::types::execution::NodeAssignment {
            node_id: target_node,
            resources: crate::types::resources::ResourceAllocation {
                cpu_cores: req.cpu.min_cores,
                memory_bytes: req.memory.min_bytes,
                storage_bytes: req.storage.min_bytes,
                network_bandwidth: 0,
                gpu_allocation: None,
                custom_resources: std::collections::HashMap::new(),
            },
            tasks: vec![job.job_id.to_string()],
        };

        Ok(DistributedExecution {
            execution_id: Uuid::new_v4(),
            distribution_time: SystemTime::now(),
            node_assignments: vec![assignment],
            resource_allocations: Vec::new(),
            status: crate::types::execution::DistributedExecutionStatus::Pending,
        })
    }

    /// Build a local-only execution plan (single self-assignment).
    fn local_execution(&self, job: UniversalJob) -> DistributedExecution {
        let req = &job.resource_requirements;
        DistributedExecution {
            execution_id: Uuid::new_v4(),
            distribution_time: SystemTime::now(),
            node_assignments: vec![crate::types::execution::NodeAssignment {
                node_id: env!("CARGO_PKG_NAME").to_string(),
                resources: crate::types::resources::ResourceAllocation {
                    cpu_cores: req.cpu.min_cores,
                    memory_bytes: req.memory.min_bytes,
                    storage_bytes: req.storage.min_bytes,
                    network_bandwidth: 0,
                    gpu_allocation: None,
                    custom_resources: std::collections::HashMap::new(),
                },
                tasks: vec![job.job_id.to_string()],
            }],
            resource_allocations: Vec::new(),
            status: crate::types::execution::DistributedExecutionStatus::Pending,
        }
    }

    /// Expose the load balancer so Songbird integration can register discovered nodes.
    #[must_use]
    pub fn load_balancer(&self) -> Arc<NetworkLoadBalancer> {
        Arc::clone(&self.load_balancer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = NetworkDistributorConfig::default();

        assert!(config.enabled);
        assert_eq!(config.max_concurrent_distributions, 10);
        assert_eq!(config.distribution_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_config_serialization() {
        let config = NetworkDistributorConfig {
            enabled: false,
            max_concurrent_distributions: 20,
            distribution_timeout: Duration::from_secs(600),
        };

        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: NetworkDistributorConfig =
            serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(
            deserialized.max_concurrent_distributions,
            config.max_concurrent_distributions
        );
        assert_eq!(
            deserialized.distribution_timeout,
            config.distribution_timeout
        );
    }

    #[test]
    fn test_config_clone() {
        let config = NetworkDistributorConfig::default();
        let cloned = config.clone();

        assert_eq!(cloned.enabled, config.enabled);
        assert_eq!(
            cloned.max_concurrent_distributions,
            config.max_concurrent_distributions
        );
    }

    #[test]
    fn test_distributor_creation() {
        let config = NetworkDistributorConfig::default();
        let distributor = NetworkDistributor::new(config);
        assert!(distributor.config.enabled);
    }

    #[test]
    fn test_distributor_with_custom_config() {
        let config = NetworkDistributorConfig {
            enabled: true,
            max_concurrent_distributions: 50,
            distribution_timeout: Duration::from_secs(120),
        };

        let distributor = NetworkDistributor::new(config);
        assert_eq!(distributor.config.max_concurrent_distributions, 50);
        assert_eq!(
            distributor.config.distribution_timeout,
            Duration::from_secs(120)
        );
    }

    #[tokio::test]
    async fn test_distribute_job_no_remote_nodes_falls_back_to_local() {
        use crate::types::{
            DistributedRetryConfig, ExecutionTarget, JobPriority, ResourceRequirements,
        };
        use std::time::SystemTime;
        use toadstool::ExecutionRequest;

        let distributor = NetworkDistributor::new(NetworkDistributorConfig::default());

        let job = UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: None,
            execution_request: ExecutionRequest::default(),
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        };

        let execution = distributor.distribute_job(job).await.unwrap();
        // Without registered nodes the local fallback assigns to self.
        assert_eq!(execution.node_assignments.len(), 1);
        assert_eq!(
            execution.node_assignments[0].node_id,
            env!("CARGO_PKG_NAME")
        );
        assert!(execution.resource_allocations.is_empty());
    }

    #[tokio::test]
    async fn test_distribute_job_routes_to_registered_node() {
        use crate::network::NodeHealth;
        use crate::types::{
            DistributedRetryConfig, ExecutionTarget, JobPriority, ResourceRequirements,
        };
        use std::time::SystemTime;
        use toadstool::ExecutionRequest;

        let distributor = NetworkDistributor::new(NetworkDistributorConfig::default());
        distributor
            .register_peer_node(
                "peer-a".to_string(),
                NodeHealth {
                    healthy: true,
                    cpu_usage: 30.0,
                    memory_usage: 40.0,
                    response_time_ms: 10,
                },
            )
            .await;

        let job = UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: None,
            execution_request: ExecutionRequest::default(),
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        };

        let execution = distributor.distribute_job(job).await.unwrap();
        assert_eq!(execution.node_assignments.len(), 1);
        assert_eq!(execution.node_assignments[0].node_id, "peer-a");
    }

    #[tokio::test]
    async fn test_distribute_job_creates_unique_ids() {
        use crate::types::{
            DistributedRetryConfig, ExecutionTarget, JobPriority, ResourceRequirements,
        };
        use std::time::SystemTime;
        use toadstool::ExecutionRequest;

        let distributor = NetworkDistributor::new(NetworkDistributorConfig::default());

        let make_job = || UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: None,
            execution_request: ExecutionRequest::default(),
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        };

        let exec1 = distributor.distribute_job(make_job()).await.unwrap();
        let exec2 = distributor.distribute_job(make_job()).await.unwrap();

        assert_ne!(exec1.execution_id, exec2.execution_id);
    }

    #[test]
    fn test_config_custom_values() {
        let config = NetworkDistributorConfig {
            enabled: false,
            max_concurrent_distributions: 100,
            distribution_timeout: Duration::from_secs(1800),
        };

        assert!(!config.enabled);
        assert_eq!(config.max_concurrent_distributions, 100);
        assert_eq!(config.distribution_timeout, Duration::from_secs(1800));
    }

    #[test]
    fn test_config_debug() {
        let config = NetworkDistributorConfig::default();
        let debug_str = format!("{:?}", config);

        assert!(debug_str.contains("NetworkDistributorConfig"));
    }

    #[tokio::test]
    async fn test_distribute_job_disabled_falls_back_to_local() {
        use crate::types::{
            DistributedRetryConfig, ExecutionTarget, JobPriority, ResourceRequirements,
        };
        use std::time::SystemTime;
        use toadstool::ExecutionRequest;

        let config = NetworkDistributorConfig {
            enabled: false,
            ..NetworkDistributorConfig::default()
        };
        let distributor = NetworkDistributor::new(config);

        let job = UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: None,
            execution_request: ExecutionRequest::default(),
            target: ExecutionTarget::Local,
            priority: JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: ResourceRequirements::default(),
            retry_config: DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        };

        let execution = distributor.distribute_job(job).await.unwrap();
        assert_eq!(execution.node_assignments.len(), 1);
        assert_eq!(
            execution.node_assignments[0].node_id,
            env!("CARGO_PKG_NAME")
        );
    }

    #[tokio::test]
    async fn test_deregister_peer_node() {
        use crate::network::NodeHealth;

        let distributor = NetworkDistributor::new(NetworkDistributorConfig::default());
        distributor
            .register_peer_node(
                "tmp-node".into(),
                NodeHealth {
                    healthy: true,
                    cpu_usage: 20.0,
                    memory_usage: 30.0,
                    response_time_ms: 50,
                },
            )
            .await;

        let snapshot = distributor.load_balancer().node_health_snapshot().await;
        assert_eq!(snapshot.len(), 1);

        distributor.deregister_peer_node("tmp-node").await;
        let snapshot = distributor.load_balancer().node_health_snapshot().await;
        assert!(snapshot.is_empty());
    }

    #[tokio::test]
    async fn test_load_balancer_accessor() {
        let distributor = NetworkDistributor::new(NetworkDistributorConfig::default());
        let lb = distributor.load_balancer();
        assert!(lb.select_node().await.is_none());
    }
}
