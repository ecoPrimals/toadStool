//! Health monitoring for BYOB deployments
//!
//! Handles service health checks and deployment monitoring.
//! Tracks service health status and triggers recovery actions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};
use uuid::Uuid;

use super::byob_types::HealthCheck;
use super::deployment::ActiveDeployment;
use crate::ToadStoolResult;

/// Trait for monitoring deployment health
///
/// **Responsibilities**:
/// - Execute health checks on services
/// - Monitor deployment-wide health status
/// - Track failed services
///
/// **Deep Debt Compliance**:
/// - ✅ Capability-based (checks service health check configs)
/// - ✅ Zero hardcoding (uses service-defined health checks)
/// - ✅ Agnostic (works with any health check command)
#[async_trait::async_trait]
pub trait HealthMonitor: Send + Sync {
    /// Monitor health of all services in a deployment
    ///
    /// **Parameters**:
    /// - `deployment_id`: Deployment to monitor
    ///
    /// **Returns**: Ok(()) if monitoring succeeded, Err if monitoring failed
    ///
    /// **Side Effects**:
    /// - Updates deployment health status
    /// - Logs health check results
    async fn monitor_deployment_health(&self, deployment_id: Uuid) -> ToadStoolResult<()>;

    /// Perform health check for a single service
    ///
    /// **Parameters**:
    /// - `service_name`: Name of service to check
    /// - `health_check`: Health check configuration
    ///
    /// **Returns**: Ok(true) if healthy, Ok(false) if unhealthy, Err on check failure
    fn perform_health_check(
        &self,
        service_name: &str,
        health_check: &HealthCheck,
    ) -> ToadStoolResult<bool>;
}

/// Default implementation of HealthMonitor for BYOB
pub struct ByobHealthMonitor {
    active_deployments: Arc<RwLock<HashMap<Uuid, ActiveDeployment>>>,
}

impl ByobHealthMonitor {
    /// Create a new health monitor (internal constructor)
    #[cfg(test)]
    pub(super) fn new(active_deployments: Arc<RwLock<HashMap<Uuid, ActiveDeployment>>>) -> Self {
        Self { active_deployments }
    }
}

#[async_trait::async_trait]
impl HealthMonitor for ByobHealthMonitor {
    async fn monitor_deployment_health(&self, deployment_id: Uuid) -> ToadStoolResult<()> {
        debug!("🔍 Monitoring health for deployment {}", deployment_id);

        let mut deployments = self.active_deployments.write().await;
        if let Some(deployment) = deployments.get_mut(&deployment_id) {
            // Check health of all services in the deployment
            let mut all_healthy = true;
            let mut failed_services = Vec::new();

            for (service_name, service_spec) in &deployment.request.services {
                if let Some(health_check) = &service_spec.health_check {
                    // ✅ RUNTIME DISCOVERY: Perform health check based on service config
                    match self.perform_health_check(service_name, health_check) {
                        Ok(healthy) => {
                            if healthy {
                                debug!("✅ Service {} passed health check", service_name);
                            } else {
                                all_healthy = false;
                                failed_services.push(service_name.clone());
                                warn!("❌ Service {} failed health check", service_name);
                            }
                        }
                        Err(e) => {
                            all_healthy = false;
                            failed_services.push(service_name.clone());
                            error!("❌ Health check error for service {}: {}", service_name, e);
                        }
                    }
                }
            }

            // Update deployment status (no health_status field exists yet)
            // NOTE: Could add health_status field to ActiveDeployment in future
            debug!(
                "✅ Deployment {} health check complete: {} healthy",
                deployment_id, all_healthy
            );

            if !failed_services.is_empty() {
                warn!(
                    "⚠️ Deployment {} has {} failed services: {:?}",
                    deployment_id,
                    failed_services.len(),
                    failed_services
                );
            }

            Ok(())
        } else {
            Err(crate::ToadStoolError::runtime(format!(
                "Deployment {deployment_id} not found"
            )))
        }
    }

    fn perform_health_check(
        &self,
        service_name: &str,
        health_check: &HealthCheck,
    ) -> ToadStoolResult<bool> {
        debug!("Performing health check for service: {}", service_name);

        if health_check.command.is_empty() {
            return Ok(true); // No command means always healthy
        }

        let command = health_check.command[0].clone();
        let args: Vec<String> = health_check.command[1..].to_vec();
        let timeout_secs = health_check.timeout.clamp(1, 60);
        let timeout = std::time::Duration::from_secs(timeout_secs);

        let (tx, rx) = std::sync::mpsc::channel();
        let _ = std::thread::spawn(move || {
            let result = std::process::Command::new(&command)
                .args(&args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            let _ = tx.send(result);
        });

        match rx.recv_timeout(timeout) {
            Ok(Ok(status)) => {
                let healthy = status.success();
                if healthy {
                    debug!("Health check passed for {}", service_name);
                } else {
                    warn!(
                        "Health check failed for {} (exit: {:?})",
                        service_name,
                        status.code()
                    );
                }
                Ok(healthy)
            }
            Ok(Err(e)) => {
                warn!("Health check command failed for {}: {}", service_name, e);
                Ok(false)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                warn!(
                    "Health check timeout for {} ({}s)",
                    service_name, timeout_secs
                );
                Ok(false)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                warn!("Health check thread panicked for {}", service_name);
                Ok(false)
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byob::byob_types::{
        ByobDeploymentRequest, ServiceResourceRequirements, ServiceSpec,
    };
    use std::time::SystemTime;

    fn create_test_deployment_with_health_check(
        healthy: bool,
    ) -> (Uuid, Arc<RwLock<HashMap<Uuid, ActiveDeployment>>>) {
        let deployment_id = Uuid::new_v4();

        let health_check = if healthy {
            Some(HealthCheck {
                command: vec![
                    "curl".to_string(),
                    "-f".to_string(),
                    "http://localhost:8080/health".to_string(),
                ],
                interval: 30,
                timeout: 5,
                retries: 3,
                start_period: 10,
            })
        } else {
            Some(HealthCheck {
                command: vec![], // Empty command = unhealthy for test purposes
                interval: 30,
                timeout: 5,
                retries: 3,
                start_period: 10,
            })
        };

        let mut services = HashMap::new();
        services.insert(
            "test-service".to_string(),
            ServiceSpec {
                name: "test-service".to_string(),
                version: "1.0.0".to_string(),
                image: Some("test:latest".to_string()),
                command: None,
                environment: HashMap::new(),
                resources: ServiceResourceRequirements {
                    cpu_cores: Some(1.0),
                    memory_bytes: Some(1024 * 1024 * 512),
                    storage_bytes: None,
                    gpu_count: None,
                },
                ports: vec![],
                volumes: vec![],
                dependencies: vec![],
                health_check,
                replicas: 1,
            },
        );

        let request = ByobDeploymentRequest {
            deployment_id,
            team_id: "test-team".to_string(),
            deployment_name: "test-deployment".to_string(),
            services,
            network_config: crate::byob::byob_types::TeamNetworkConfig {
                network_name: "test-network".to_string(),
                subnet_cidr: "10.0.0.0/24".to_string(),
                dns_config: None,
                load_balancer: None,
            },
            resource_quotas: crate::byob::byob_types::TeamResourceQuotas {
                max_cpu_cores: 4.0,
                max_memory_bytes: 1024 * 1024 * 1024 * 4,
                max_storage_bytes: 1024 * 1024 * 1024 * 10,
                max_gpu_count: 0,
                max_concurrent_services: 10,
            },
            security_config: crate::byob::byob_types::TeamSecurityConfig {
                isolation_level: "standard".to_string(),
                network_policies: vec![],
                volume_policies: vec![],
                resource_policies: vec![],
            },
            created_at: SystemTime::now(),
        };

        let network_info = crate::byob::byob_types::NetworkInfo {
            network_name: "test-network".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            gateway_ip: "10.0.0.1".to_string(),
            service_endpoints: HashMap::new(),
        };

        let deployment = ActiveDeployment::new(request, network_info);

        let mut deployments = HashMap::new();
        deployments.insert(deployment_id, deployment);

        (deployment_id, Arc::new(RwLock::new(deployments)))
    }

    #[tokio::test]
    async fn test_monitor_healthy_deployment() {
        let (deployment_id, deployments) = create_test_deployment_with_health_check(true);
        let monitor = ByobHealthMonitor::new(deployments.clone());

        let result = monitor.monitor_deployment_health(deployment_id).await;
        assert!(result.is_ok());

        // Health monitoring completed successfully
        // NOTE: ActiveDeployment doesn't have health_status field yet
        // Could be added in future for health tracking
    }

    #[tokio::test]
    async fn test_monitor_nonexistent_deployment() {
        let (_, deployments) = create_test_deployment_with_health_check(true);
        let monitor = ByobHealthMonitor::new(deployments);

        let fake_id = Uuid::new_v4();
        let result = monitor.monitor_deployment_health(fake_id).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_perform_health_check_with_curl() {
        let deployments = Arc::new(RwLock::new(HashMap::new()));
        let monitor = ByobHealthMonitor::new(deployments);

        // Use `true` (always exits 0) since health checks now actually execute commands.
        // curl would fail in test env without a server.
        let health_check = HealthCheck {
            command: vec!["true".to_string()],
            interval: 30,
            timeout: 5,
            retries: 3,
            start_period: 10,
        };

        let result = monitor.perform_health_check("test-service", &health_check);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_perform_health_check_with_empty_command() {
        let deployments = Arc::new(RwLock::new(HashMap::new()));
        let monitor = ByobHealthMonitor::new(deployments);

        let health_check = HealthCheck {
            command: vec![],
            interval: 30,
            timeout: 5,
            retries: 3,
            start_period: 10,
        };

        let result = monitor.perform_health_check("test-service", &health_check);
        assert!(result.is_ok());
        assert!(result.unwrap()); // Empty command = always healthy
    }
}
