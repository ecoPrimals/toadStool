// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Deployment Coordinator
//!
//! Coordinates code deployment to edge devices with support for OTA updates, rollbacks, and deployment strategies.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use toadstool::error::ToadStoolResult;

use crate::EdgeRuntimeConfig;
use crate::platforms::EdgeDevice;

/// Deployment Coordinator
pub struct DeploymentCoordinator {
    #[expect(
        dead_code,
        reason = "stored from init; will be used for rollback policy + health checks"
    )]
    config: EdgeRuntimeConfig,
    active_deployments: Arc<RwLock<HashMap<Uuid, DeploymentInfo>>>,
}

/// Deployment Information
#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    pub id: Uuid,
    pub device_id: Uuid,
    pub status: DeploymentStatus,
    pub strategy: DeploymentStrategy,
    pub version: String,
    pub started_at: std::time::SystemTime,
    pub completed_at: Option<std::time::SystemTime>,
    pub error: Option<String>,
}

/// Deployment Status
#[derive(Debug, Clone, PartialEq)]
pub enum DeploymentStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

/// Deployment Strategy
#[derive(Debug, Clone)]
pub enum DeploymentStrategy {
    Immediate,
    Scheduled(std::time::SystemTime),
    Staged,
    Canary,
}

impl DeploymentCoordinator {
    /// Create a new deployment coordinator
    pub async fn new(config: &EdgeRuntimeConfig) -> ToadStoolResult<Self> {
        info!("Initializing deployment coordinator");

        Ok(Self {
            config: config.clone(),
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Deploy code to device
    pub async fn deploy_to_device(
        &self,
        device: &dyn EdgeDevice,
        code: &[u8],
    ) -> ToadStoolResult<String> {
        let deployment_id = Uuid::new_v4();
        let device_id = device.get_id();

        info!(
            "Starting deployment {} to device {}",
            deployment_id, device_id
        );

        // Create deployment info
        let deployment_info = DeploymentInfo {
            id: deployment_id,
            device_id,
            status: DeploymentStatus::InProgress,
            strategy: DeploymentStrategy::Immediate,
            version: "1.0.0".to_string(),
            started_at: std::time::SystemTime::now(),
            completed_at: None,
            error: None,
        };

        // Store deployment info
        {
            let mut deployments = self.active_deployments.write().await;
            deployments.insert(deployment_id, deployment_info);
        }

        // Perform deployment
        let result = device.deploy(code).await;

        // Update deployment status
        {
            let mut deployments = self.active_deployments.write().await;
            if let Some(deployment) = deployments.get_mut(&deployment_id) {
                match result {
                    Ok(_) => {
                        deployment.status = DeploymentStatus::Completed;
                        deployment.completed_at = Some(std::time::SystemTime::now());
                    }
                    Err(ref e) => {
                        deployment.status = DeploymentStatus::Failed;
                        deployment.error = Some(e.to_string());
                    }
                }
            }
        }

        result
    }

    /// Get deployment status
    pub async fn get_deployment_status(&self, deployment_id: Uuid) -> Option<DeploymentInfo> {
        let deployments = self.active_deployments.read().await;
        deployments.get(&deployment_id).cloned()
    }

    /// Get active deployments
    pub async fn get_active_deployments(&self) -> Vec<DeploymentInfo> {
        let deployments = self.active_deployments.read().await;
        deployments.values().cloned().collect()
    }
}
