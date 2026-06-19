// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core Coordination integration implementation — constructor, receiver wiring, and tests.

use std::sync::Arc;

use toadstool::error::ToadStoolResult;

use crate::universal::UniversalScheduler;

use super::types::{
    CapacityConfig, CoordinationConnection, JobReceiver, ReceiverConfig,
    ToadStoolCoordinationIntegration,
};

impl ToadStoolCoordinationIntegration {
    /// Construct integration with Coordination connection, capacity config, and scheduler.
    pub async fn new(
        instance_id: String,
        connection: CoordinationConnection,
        capacity_config: CapacityConfig,
        scheduler: Arc<UniversalScheduler>,
    ) -> ToadStoolResult<Self> {
        let local_capacity =
            Arc::new(super::types::LocalCapacityManager::new(capacity_config).await?);

        Ok(Self {
            instance_id,
            connection,
            local_capacity,
            workload_scheduler: scheduler,
        })
    }
}

impl JobReceiver {
    /// Create a job receiver with the given config and Coordination connection.
    pub async fn new(
        _config: ReceiverConfig,
        _connection: Arc<CoordinationConnection>,
    ) -> ToadStoolResult<Self> {
        let (_tx, receiver) = tokio::sync::mpsc::channel(100);
        Ok(Self { receiver })
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{
        CapacityConfig, ConnectionHealth, CoordinationConnection, CoordinationTransport,
        HttpProtocolConfig, LocalCapacityManager, MessageQueueProtocolConfig, ProtocolConfig,
        ReceiverConfig, ToadStoolCoordinationIntegration,
    };
    use super::*;
    use crate::UniversalJob;
    use crate::{ExecutionTarget, UniversalScheduler, UniversalSchedulerConfig};
    use std::time::{Duration, SystemTime};
    use toadstool_common::constants::PRIMAL_NAME;
    use toadstool_common::constants::network::LOCALHOST_IPV4;
    use uuid::Uuid;

    fn test_connection() -> CoordinationConnection {
        let endpoint = format!("http://{}:{}", LOCALHOST_IPV4, 50051_u16);
        CoordinationConnection {
            endpoints: vec![endpoint.clone()],
            active_endpoint: endpoint,
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
            protocol_config: ProtocolConfig {
                protocol: CoordinationTransport::HTTP,
                http: HttpProtocolConfig {
                    timeout_ms: 5000,
                    max_retries: 3,
                    headers: std::collections::HashMap::new(),
                },
                message_queue: MessageQueueProtocolConfig {
                    queue_name: "jobs".to_string(),
                    exchange: PRIMAL_NAME.to_string(),
                    routing_key: "compute".to_string(),
                },
            },
            #[cfg(feature = "channels")]
            reply_channel: None,
        }
    }

    fn capacity_config() -> CapacityConfig {
        CapacityConfig {
            monitoring_interval: Duration::from_secs(60),
            resource_buffer: 0.1,
        }
    }

    fn simple_job() -> UniversalJob {
        UniversalJob {
            job_id: Uuid::new_v4(),
            job_type: Some(crate::UniversalJobType::Local),
            execution_request: toadstool::ExecutionRequest::default(),
            target: ExecutionTarget::Local,
            priority: toadstool::JobPriority::Normal,
            dependencies: vec![],
            resource_requirements: crate::ResourceRequirements::default(),
            retry_config: crate::DistributedRetryConfig::default(),
            created_at: SystemTime::now(),
        }
    }

    #[tokio::test]
    async fn test_local_capacity_manager_new() {
        let cap = LocalCapacityManager::new(capacity_config()).await.unwrap();
        let avail = cap.get_available_capacity().await.unwrap();
        assert!(avail.cpu_cores > 0.0);
        assert!(avail.memory_bytes > 0);
        assert!(avail.storage_bytes > 0);
    }

    #[tokio::test]
    async fn test_local_capacity_manager_can_accept_job() {
        let cap = LocalCapacityManager::new(capacity_config()).await.unwrap();
        let req = crate::ResourceRequirements::default();
        let _can = cap.can_accept_job(&req).await.unwrap();
    }

    #[tokio::test]
    async fn test_local_capacity_manager_reserve_and_release() {
        let cap = LocalCapacityManager::new(capacity_config()).await.unwrap();
        let req = crate::ResourceRequirements::default();
        let reservation = cap.reserve_resources(&req).await.unwrap();
        assert!(!reservation.reservation_id.is_nil());
        cap.release_reservation(reservation).await.unwrap();
    }

    #[tokio::test]
    async fn test_local_capacity_manager_get_capabilities() {
        let cap = LocalCapacityManager::new(capacity_config()).await.unwrap();
        let node_caps = cap.get_current_capabilities().await.unwrap();
        assert!(node_caps.cpu_cores > 0.0);
        assert!(
            node_caps
                .software_capabilities
                .contains(&"rust".to_string())
        );
    }

    #[tokio::test]
    async fn test_job_receiver_new() {
        let config = ReceiverConfig {
            max_concurrent_jobs: 10,
            job_timeout: Duration::from_secs(60),
        };
        let conn = Arc::new(test_connection());
        let mut receiver = JobReceiver::new(config, conn).await.unwrap();
        assert!(receiver.receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_integration_new() {
        let config = UniversalSchedulerConfig::default();
        let scheduler = Arc::new(UniversalScheduler::new(config).await.unwrap());
        let _integration = ToadStoolCoordinationIntegration::new(
            "test-instance".to_string(),
            test_connection(),
            capacity_config(),
            scheduler,
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_submit_job_local_only() {
        let config = UniversalSchedulerConfig::default();
        let scheduler = Arc::new(UniversalScheduler::new(config).await.unwrap());
        let integration = ToadStoolCoordinationIntegration::new(
            "test".to_string(),
            test_connection(),
            capacity_config(),
            scheduler,
        )
        .await
        .unwrap();
        let job = simple_job();
        let handles = integration.submit_job(job).await.unwrap();
        assert!(handles.is_empty());
    }

    #[tokio::test]
    async fn test_submit_job_http_returns_error() {
        let config = UniversalSchedulerConfig::default();
        let scheduler = Arc::new(UniversalScheduler::new(config).await.unwrap());
        let integration = ToadStoolCoordinationIntegration::new(
            "test".to_string(),
            test_connection(),
            capacity_config(),
            scheduler,
        )
        .await
        .unwrap();
        let mut job = simple_job();
        job.resource_requirements.cpu.min_cores = 8.0;
        job.resource_requirements.memory.min_bytes = 32 * 1024 * 1024 * 1024;
        let result = integration.submit_job(job).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("HTTP job submission removed")
        );
    }

    #[test]
    fn test_capacity_config_serialization() {
        let config = capacity_config();
        let json = serde_json::to_value(&config).unwrap();
        assert!(json.get("monitoring_interval").is_some());
    }
}
