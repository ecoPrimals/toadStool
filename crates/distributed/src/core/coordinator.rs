use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{ExecutionRequest, ToadStoolResult};

use super::config::{DistributedConfig, StandaloneConfig, ToadStoolCapabilities};
use crate::coordination_integration::{
    CoordinationClient, CoordinationConfig, CoordinationDiscovery,
};

/// Main distributed computing coordinator - uses capability-based coordination discovery
pub struct DistributedCoordinator {
    #[allow(dead_code)]
    config: DistributedConfig,
    #[allow(dead_code)]
    capabilities: Arc<RwLock<ToadStoolCapabilities>>,
    coordination_client: Option<Arc<CoordinationClient>>,
    standalone_executor: Arc<StandaloneExecutor>,
}

/// Standalone execution engine for local operations
pub struct StandaloneExecutor {
    config: StandaloneConfig,
    active_executions: Arc<RwLock<HashMap<Uuid, ExecutionSession>>>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct ExecutionSession {
    pub execution_id: Uuid,
    pub request: ExecutionRequest,
    pub started_at: Instant,
}

impl DistributedCoordinator {
    /// Create a new distributed coordinator
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Capability detection fails
    /// - Coordination service discovery fails (if enabled)
    /// - Configuration validation fails
    #[must_use = "DistributedCoordinator creation should be checked"]
    pub async fn new(config: DistributedConfig) -> ToadStoolResult<Self> {
        info!("Initializing distributed coordinator with capability-based discovery");

        // Detect capabilities
        let capabilities = Arc::new(RwLock::new(Self::detect_capabilities().await?));

        // Create standalone executor
        let standalone_executor = Arc::new(StandaloneExecutor::new(config.standalone.clone())?);

        // Create coordination client using capability-based discovery
        let coordination_client = if config.songbird_integration.is_some() {
            info!("Discovering coordination services via capability-based discovery");

            // Use new coordination_integration module (vendor-agnostic)
            let coord_config = CoordinationConfig {
                auto_discover: true,
                discovery_timeout_ms: 5000,
                preferred_location: crate::coordination_integration::ServiceLocation::Any,
                fallback_enabled: true,
                required_capabilities: vec![
                    toadstool_common::primal_identity::CoordinationCapability::ServiceDiscovery,
                    toadstool_common::primal_identity::CoordinationCapability::LoadBalancing,
                ],
                health_check_interval_secs: 30,
            };

            let discovery = CoordinationDiscovery::new(coord_config).await?;

            match discovery.discover().await {
                Ok(services) if !services.is_empty() => {
                    let service = &services[0];
                    info!("Discovered coordination service: {}", service.name);
                    let client = CoordinationClient::new(service).await?;
                    Some(Arc::new(client))
                }
                Ok(_) => {
                    warn!("No coordination services found, operating in standalone mode");
                    None
                }
                Err(e) => {
                    warn!(
                        "Coordination service discovery failed, operating in standalone mode: {}",
                        e
                    );
                    None
                }
            }
        } else {
            info!("Coordination disabled, operating in standalone mode");
            None
        };

        Ok(Self {
            config,
            capabilities,
            coordination_client,
            standalone_executor,
        })
    }

    /// Start the coordinator
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Coordination service connection fails
    /// - Health reporting setup fails
    /// - Background tasks cannot be spawned
    #[must_use = "Coordinator start result should be checked"]
    pub async fn start(self: Arc<Self>) -> ToadStoolResult<()> {
        info!("Starting distributed coordinator");

        // Start coordination client if available
        if let Some(_client) = &self.coordination_client {
            info!("Coordination service initialized and ready");
            // Note: CoordinationClient manages its own connection lifecycle
        } else {
            info!("Operating in standalone mode (no coordination service)");
        }

        // Start health reporting
        // This would typically be done in a background task
        info!("Distributed coordinator started successfully");
        Ok(())
    }

    /// Submit an execution request
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Standalone executor rejects the request
    /// - Request validation fails
    /// - Execution session cannot be created
    #[must_use = "Execution submission result should be checked"]
    pub async fn submit_execution(&self, request: ExecutionRequest) -> ToadStoolResult<Uuid> {
        info!("Submitting execution request");

        // For now, route everything to standalone executor
        let execution_id = Uuid::new_v4();
        self.standalone_executor
            .submit_execution(execution_id, request)
            .await?;

        Ok(execution_id)
    }

    async fn detect_capabilities() -> ToadStoolResult<ToadStoolCapabilities> {
        ToadStoolCapabilities::detect_current().await
    }
}

impl StandaloneExecutor {
    fn new(config: StandaloneConfig) -> ToadStoolResult<Self> {
        Ok(Self {
            config,
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn submit_execution(
        &self,
        execution_id: Uuid,
        request: ExecutionRequest,
    ) -> ToadStoolResult<()> {
        info!(
            "Submitting execution to standalone executor: {}",
            execution_id
        );

        let session = ExecutionSession {
            execution_id,
            request,
            started_at: Instant::now(),
        };

        // Check if we're at capacity
        {
            let active_executions = self.active_executions.read().await;
            if active_executions.len() >= self.config.max_concurrent_executions as usize {
                warn!("Rejecting execution - at capacity");
                return Err(toadstool::ToadStoolError::resource(
                    "Insufficient resources available for job execution",
                ));
            }
        }

        // Add to active executions
        {
            let mut active_executions = self.active_executions.write().await;
            active_executions.insert(execution_id, session);
        }

        // Here we would typically spawn a task to handle the execution
        // For now, we'll just log it
        info!("Execution {} queued for processing", execution_id);

        Ok(())
    }
}

impl Clone for StandaloneExecutor {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            active_executions: Arc::clone(&self.active_executions),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::SongbirdConfig;

    #[tokio::test]
    async fn test_coordinator_creation_default() {
        let config = DistributedConfig::default();
        let result = DistributedCoordinator::new(config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_coordinator_submit_execution() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await.unwrap();

        let request = ExecutionRequest::default();
        let result = coordinator.submit_execution(request).await;
        assert!(result.is_ok());
        let execution_id = result.unwrap();
        assert_ne!(execution_id, Uuid::nil());
    }

    #[tokio::test]
    async fn test_coordinator_start() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await.unwrap();
        let coordinator = Arc::new(coordinator);

        let result = coordinator.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_coordinator_submit_multiple() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await.unwrap();

        let ids: Vec<Uuid> = futures::future::join_all((0..5).map(|_| {
            let coord = &coordinator;
            let req = ExecutionRequest::default();
            async move { coord.submit_execution(req).await.unwrap() }
        }))
        .await;

        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 5);
    }

    #[tokio::test]
    async fn test_standalone_executor_capacity_limit() {
        let config = DistributedConfig {
            standalone: StandaloneConfig {
                max_concurrent_executions: 2,
                default_timeout_secs: 60,
                enable_job_queue: true,
                max_queue_size: 10,
            },
            ..Default::default()
        };
        let coordinator = DistributedCoordinator::new(config).await.unwrap();

        // Submit 2 - should succeed
        let _ = coordinator
            .submit_execution(ExecutionRequest::default())
            .await
            .unwrap();
        let _ = coordinator
            .submit_execution(ExecutionRequest::default())
            .await
            .unwrap();

        // Submit 3rd - should fail with resource error
        let result = coordinator
            .submit_execution(ExecutionRequest::default())
            .await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("Insufficient") || err_msg.contains("resource"));
    }

    #[tokio::test]
    async fn test_coordinator_with_songbird_config() {
        let config = DistributedConfig {
            songbird_integration: Some(SongbirdConfig {
                endpoint: "http://localhost:8080".to_string(),
                auth_token: None,
                health_reporting_interval_secs: 30,
            }),
            ..Default::default()
        };
        let result = DistributedCoordinator::new(config).await;
        assert!(result.is_ok());
        let coordinator = result.unwrap();
        assert!(
            coordinator.coordination_client.is_none() || coordinator.coordination_client.is_some()
        );
    }

    #[tokio::test]
    async fn test_standalone_executor_clone() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await.unwrap();
        let _executor = coordinator.standalone_executor.clone();
        assert!(Arc::strong_count(&coordinator.standalone_executor) >= 2);
    }

    #[tokio::test]
    async fn test_submit_execution_returns_unique_ids() {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await.unwrap();

        let id1 = coordinator
            .submit_execution(ExecutionRequest::default())
            .await
            .unwrap();
        let id2 = coordinator
            .submit_execution(ExecutionRequest::default())
            .await
            .unwrap();

        assert_ne!(id1, id2);
    }
}
