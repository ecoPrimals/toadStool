// SPDX-License-Identifier: AGPL-3.0-or-later
//! Protocol-specific submission paths (HTTP, message queue) for Coordination jobs.

use std::time::SystemTime;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use tracing::{debug, warn};

use super::types::{
    CoordinationJobRequest, CoordinationJobResponse, CoordinationTransport, SubTask, SubTaskHandle,
    SubTaskStatus, ToadStoolCoordinationIntegration,
};

impl ToadStoolCoordinationIntegration {
    /// Submit subtask to Coordination for execution on specific nodes
    pub(super) async fn submit_subtask_to_coordination(
        &self,
        subtask: SubTask,
        target_nodes: Vec<String>,
    ) -> ToadStoolResult<SubTaskHandle> {
        debug!(
            "Submitting subtask {} to coordination service for nodes: {:?}",
            subtask.id, target_nodes
        );

        let coordination_request = CoordinationJobRequest {
            job_id: subtask.id,
            job_payload: subtask.payload.clone(),
            target_nodes: target_nodes.clone(),
            resource_requirements: subtask.resource_requirements.clone(),
            priority: subtask.priority,
            constraints: subtask.constraints.clone(),
        };

        // Submit to Coordination via appropriate protocol
        let response = match &self.connection.protocol_config.protocol {
            CoordinationTransport::HTTP => {
                self.submit_via_http(coordination_request, &self.connection.active_endpoint)
                    .await?
            }
            CoordinationTransport::MessageQueue => {
                self.submit_via_message_queue(coordination_request, "global")
                    .await?
            }
        };

        let job_id = match &response {
            CoordinationJobResponse::Success { job_id, .. } => *job_id,
            CoordinationJobResponse::Error { job_id, .. } => *job_id,
        };

        Ok(SubTaskHandle {
            subtask_id: subtask.id,
            coordination_job_id: job_id,
            target_nodes,
            submitted_at: SystemTime::now(),
            status: SubTaskStatus::Submitted,
        })
    }

    // Protocol-specific submission methods
    async fn submit_via_http(
        &self,
        _request: CoordinationJobRequest,
        endpoint: &str,
    ) -> ToadStoolResult<CoordinationJobResponse> {
        // PURE RUST: HTTP removed - use Unix sockets!
        warn!(
            "HTTP submission deprecated for endpoint: {} - use Unix socket RPC instead",
            endpoint
        );

        // Return error indicating this method is deprecated
        Err(ToadStoolError::not_supported(
            "HTTP job submission removed - use Unix socket RPC via the coordination service client instead. \
             External HTTP should go through the coordination service (Coordination) (Concentrated Gap architecture).",
        ))
    }

    async fn submit_via_message_queue(
        &self,
        _request: CoordinationJobRequest,
        queue_name: &str,
    ) -> ToadStoolResult<CoordinationJobResponse> {
        warn!(
            "Message queue submission not implemented for queue: {} — use JSON-RPC over Unix socket instead",
            queue_name
        );

        Err(ToadStoolError::not_supported(
            "Message queue job submission not implemented. Use JSON-RPC over Unix socket via \
             the coordination service client. Message queue integration (RabbitMQ/Kafka) is a \
             future scaling target (P3).",
        ))
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use uuid::Uuid;

    use toadstool::error::{SystemError, ToadStoolError};

    use super::super::types::{
        CapacityConfig, ConnectionHealth, CoordinationConnection, CoordinationTransport,
        HttpProtocolConfig, MessageQueueProtocolConfig, ProtocolConfig, SubTask,
        ToadStoolCoordinationIntegration,
    };
    use crate::universal::{UniversalScheduler, UniversalSchedulerConfig};
    use toadstool_common::constants::PRIMAL_NAME;
    use toadstool_common::constants::network::LOCALHOST_IPV4;

    fn connection_with_protocol(protocol: CoordinationTransport) -> CoordinationConnection {
        let endpoint = format!("http://{}:{}", LOCALHOST_IPV4, 50051_u16);
        CoordinationConnection {
            endpoints: vec![endpoint.clone()],
            active_endpoint: endpoint,
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
            protocol_config: ProtocolConfig {
                protocol,
                http: HttpProtocolConfig {
                    timeout_ms: 5000,
                    max_retries: 3,
                    headers: HashMap::new(),
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

    fn sample_subtask() -> SubTask {
        SubTask {
            id: Uuid::new_v4(),
            payload: Bytes::from_static(b"payload"),
            resource_requirements: crate::ResourceRequirements::default(),
            priority: 1,
            constraints: vec![],
        }
    }

    async fn integration_with_protocol(
        protocol: CoordinationTransport,
    ) -> ToadStoolCoordinationIntegration {
        let scheduler = Arc::new(
            UniversalScheduler::new(UniversalSchedulerConfig::default())
                .await
                .expect("scheduler"),
        );
        ToadStoolCoordinationIntegration::new(
            "transport-test".to_string(),
            connection_with_protocol(protocol),
            capacity_config(),
            scheduler,
        )
        .await
        .expect("integration")
    }

    #[tokio::test]
    async fn submit_subtask_http_returns_not_supported() {
        let integration = integration_with_protocol(CoordinationTransport::HTTP).await;
        let err = integration
            .submit_subtask_to_coordination(sample_subtask(), vec!["n1".to_string()])
            .await
            .expect_err("HTTP submission must be rejected");
        assert!(
            matches!(
                err,
                ToadStoolError::System(SystemError::NotSupported { .. })
            ),
            "expected NotSupported, got {err:?}"
        );
        assert!(
            err.to_string().contains("HTTP job submission removed"),
            "message should mention HTTP removal: {err}"
        );
    }

    #[tokio::test]
    async fn submit_subtask_message_queue_returns_not_supported() {
        let integration = integration_with_protocol(CoordinationTransport::MessageQueue).await;
        let err = integration
            .submit_subtask_to_coordination(sample_subtask(), vec!["n1".to_string()])
            .await
            .expect_err("message queue submission must be rejected");
        assert!(
            matches!(
                err,
                ToadStoolError::System(SystemError::NotSupported { .. })
            ),
            "expected NotSupported, got {err:?}"
        );
        assert!(
            err.to_string().contains("Message queue"),
            "message should mention message queue: {err}"
        );
    }

    #[tokio::test]
    async fn coordination_job_response_roundtrips_serde() {
        use super::super::types::CoordinationJobResponse;
        let resp = CoordinationJobResponse::Success {
            job_id: Uuid::new_v4(),
            status: "queued".to_owned(),
            message: "Job submitted via coordination".to_owned(),
            estimated_completion: Some(
                std::time::SystemTime::now() + std::time::Duration::from_secs(60),
            ),
        };
        let json = serde_json::to_string(&resp).expect("serde");
        let back: CoordinationJobResponse = serde_json::from_str(&json).expect("de");
        match back {
            CoordinationJobResponse::Success {
                status, message, ..
            } => {
                assert_eq!(status, "queued");
                assert!(message.contains("coordination"));
            }
            CoordinationJobResponse::Error { .. } => panic!("unexpected variant"),
        }
    }
}
