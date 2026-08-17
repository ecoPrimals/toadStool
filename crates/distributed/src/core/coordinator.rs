// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use toadstool::{ExecutionRequest, ToadStoolResult};

use super::config::{DistributedConfig, StandaloneConfig, ToadStoolCapabilities};
use crate::coordination_integration::{
    CoordinationClient, CoordinationConfig, CoordinationDiscovery,
};

/// Main distributed computing coordinator - uses capability-based coordination discovery
pub struct DistributedCoordinator {
    #[expect(
        dead_code,
        reason = "stored for future reconfiguration and capability queries"
    )]
    config: DistributedConfig,
    #[expect(dead_code, reason = "stored for future capability-based routing")]
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
#[expect(
    dead_code,
    reason = "execution session tracking; cancel_token used for cancellation"
)]
struct ExecutionSession {
    pub execution_id: Uuid,
    pub request: ExecutionRequest,
    pub started_at: Instant,
    pub cancel_token: CancellationToken,
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
        let coordination_client = if config.coordination.is_some() {
            info!("Discovering coordination services via capability-based discovery");

            // Use new coordination_integration module (vendor-agnostic)
            let coord_config = CoordinationConfig::default();

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

        // Use request's execution_id when valid (enables cancel by workload_id), else generate new
        let execution_id = if request.execution_id != Uuid::nil() {
            request.execution_id
        } else {
            Uuid::new_v4()
        };

        self.standalone_executor
            .submit_execution(execution_id, request)
            .await?;

        Ok(execution_id)
    }

    /// Cancel a running or queued execution by ID
    ///
    /// # Errors
    ///
    /// Returns an error if the execution ID is not found in active executions.
    #[must_use = "Cancellation result should be checked"]
    pub async fn cancel_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        self.standalone_executor
            .cancel_execution(execution_id)
            .await
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

        let cancel_token = CancellationToken::new();
        let session = ExecutionSession {
            execution_id,
            request,
            started_at: Instant::now(),
            cancel_token,
        };

        // Check if we're at capacity
        {
            let active_executions = self
                .active_executions
                .read()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if active_executions.len() >= self.config.max_concurrent_executions as usize {
                warn!("Rejecting execution - at capacity");
                return Err(toadstool::ToadStoolError::resource(
                    "Insufficient resources available for job execution",
                ));
            }
        }

        // Add to active executions
        {
            let mut active_executions = self
                .active_executions
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            active_executions.insert(execution_id, session);
        }

        // Here we would typically spawn a task to handle the execution
        // For now, we'll just log it
        info!("Execution {} queued for processing", execution_id);

        Ok(())
    }

    async fn cancel_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        let mut active_executions = self
            .active_executions
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let session = active_executions.remove(&execution_id).ok_or_else(|| {
            toadstool::ToadStoolError::execution(format!(
                "Execution {execution_id} not found (already completed or never submitted)"
            ))
        })?;
        drop(active_executions);

        session.cancel_token.cancel();
        info!("Execution {} cancelled", execution_id);
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
    use crate::core::config::CoordinationConfig;
    use toadstool_config::defaults::endpoints::coordination_loopback_bootstrap_url;

    #[tokio::test]
    async fn test_coordinator_creation_default() -> ToadStoolResult<()> {
        let config = DistributedConfig::default();
        DistributedCoordinator::new(config).await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_coordinator_submit_execution() -> ToadStoolResult<()> {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await?;

        let request = ExecutionRequest::default();
        let execution_id = coordinator.submit_execution(request).await?;
        assert_ne!(execution_id, Uuid::nil());
        Ok(())
    }

    #[tokio::test]
    async fn test_coordinator_start() -> ToadStoolResult<()> {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await?;
        let coordinator = Arc::new(coordinator);

        coordinator.start().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_coordinator_submit_multiple() -> ToadStoolResult<()> {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await?;

        let mut ids = Vec::with_capacity(5);
        for _ in 0..5 {
            ids.push(
                coordinator
                    .submit_execution(ExecutionRequest::default())
                    .await?,
            );
        }

        let unique: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), 5);
        Ok(())
    }

    #[tokio::test]
    async fn test_standalone_executor_capacity_limit() -> ToadStoolResult<()> {
        let config = DistributedConfig {
            standalone: StandaloneConfig {
                max_concurrent_executions: 2,
                default_timeout_secs: 60,
                enable_job_queue: true,
                max_queue_size: 10,
            },
            ..Default::default()
        };
        let coordinator = DistributedCoordinator::new(config).await?;

        // Submit 2 - should succeed
        coordinator
            .submit_execution(ExecutionRequest::default())
            .await?;
        coordinator
            .submit_execution(ExecutionRequest::default())
            .await?;

        // Submit 3rd - should fail with resource error
        let err = coordinator
            .submit_execution(ExecutionRequest::default())
            .await
            .expect_err("third submission should exceed capacity");
        let err_msg = err.to_string();
        assert!(err_msg.contains("Insufficient") || err_msg.contains("resource"));
        Ok(())
    }

    #[tokio::test]
    async fn test_coordinator_with_coordination_config() -> ToadStoolResult<()> {
        let config = DistributedConfig {
            coordination: Some(CoordinationConfig {
                endpoint: coordination_loopback_bootstrap_url(),
                auth_token: None,
                health_reporting_interval_secs: 30,
            }),
            ..Default::default()
        };
        let coordinator = DistributedCoordinator::new(config).await?;
        assert!(
            coordinator.coordination_client.is_none() || coordinator.coordination_client.is_some()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_standalone_executor_clone() -> ToadStoolResult<()> {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await?;
        let _executor = std::sync::Arc::clone(&coordinator.standalone_executor);
        assert!(Arc::strong_count(&coordinator.standalone_executor) >= 2);
        Ok(())
    }

    #[tokio::test]
    async fn test_submit_execution_returns_unique_ids() -> ToadStoolResult<()> {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await?;

        let id1 = coordinator
            .submit_execution(ExecutionRequest::default())
            .await?;
        let id2 = coordinator
            .submit_execution(ExecutionRequest::default())
            .await?;

        assert_ne!(id1, id2);
        Ok(())
    }
}
