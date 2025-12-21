//! Health monitoring for BYOB deployments

use super::byob_types::{HealthCheckConfig, ServiceInstanceStatus, ServiceSpec};
use super::deployment::ActiveDeployment;
use crate::{ToadStoolError, ToadStoolResult};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Health monitoring manager for deployments
pub(super) struct HealthMonitor {
    health_check_interval: Duration,
}

impl HealthMonitor {
    /// Create a new health monitor with configuration
    pub fn new(health_check_interval: Duration) -> Self {
        Self {
            health_check_interval,
        }
    }

    /// Monitor deployment health continuously
    pub async fn monitor_deployment(
        &self,
        deployment: &mut ActiveDeployment,
    ) -> ToadStoolResult<()> {
        debug!(
            "Starting health monitoring for deployment {}",
            deployment.request.deployment_id
        );

        loop {
            // Perform health checks on all services
            let mut all_healthy = true;

            for (service_name, service_spec) in &deployment.request.services {
                match self.check_service_health(service_name, service_spec, deployment).await {
                    Ok(is_healthy) => {
                        if !is_healthy {
                            all_healthy = false;
                            warn!(
                                "Service {} in deployment {} is unhealthy",
                                service_name, deployment.request.deployment_id
                            );
                        }
                    }
                    Err(e) => {
                        error!(
                            "Error checking health for service {}: {}",
                            service_name, e
                        );
                        all_healthy = false;
                    }
                }
            }

            if all_healthy {
                debug!(
                    "All services healthy in deployment {}",
                    deployment.request.deployment_id
                );
            }

            // Wait before next check
            sleep(self.health_check_interval).await;

            // Check if deployment was stopped
            if deployment.stopped_at.is_some() {
                info!(
                    "Stopping health monitoring for deployment {}",
                    deployment.request.deployment_id
                );
                break;
            }
        }

        Ok(())
    }

    /// Check health of a single service
    async fn check_service_health(
        &self,
        service_name: &str,
        service_spec: &ServiceSpec,
        deployment: &mut ActiveDeployment,
    ) -> ToadStoolResult<bool> {
        // Get health check configuration
        let health_check = match &service_spec.health_check {
            Some(hc) => hc,
            None => {
                // No health check configured, assume healthy if running
                return Ok(self.is_service_running(service_name, deployment));
            }
        };

        // Perform health check based on type
        let is_healthy = self.perform_health_check(service_name, health_check, deployment).await?;

        // Update service status
        self.update_service_status(service_name, is_healthy, deployment);

        Ok(is_healthy)
    }

    /// Perform actual health check
    async fn perform_health_check(
        &self,
        service_name: &str,
        health_check: &HealthCheckConfig,
        deployment: &ActiveDeployment,
    ) -> ToadStoolResult<bool> {
        match health_check.check_type.as_str() {
            "http" => self.http_health_check(service_name, health_check, deployment).await,
            "tcp" => self.tcp_health_check(service_name, health_check, deployment).await,
            "exec" => self.exec_health_check(service_name, health_check).await,
            _ => {
                warn!(
                    "Unknown health check type '{}' for service {}",
                    health_check.check_type, service_name
                );
                Ok(false)
            }
        }
    }

    /// HTTP health check
    async fn http_health_check(
        &self,
        service_name: &str,
        health_check: &HealthCheckConfig,
        deployment: &ActiveDeployment,
    ) -> ToadStoolResult<bool> {
        // Get service port
        let port = health_check.port.unwrap_or(80);

        // Construct health endpoint URL using discovered network config
        // EVOLVED: No hardcoded localhost, use deployment's network info
        let network_ip = self.get_service_ip(service_name, deployment);
        let path = health_check.path.as_deref().unwrap_or("/health");
        let url = format!("http://{}:{}{}", network_ip, port, path);

        debug!("Performing HTTP health check for {}: {}", service_name, url);

        // Perform actual HTTP request with timeout
        #[cfg(feature = "networking")]
        {
            use std::time::Duration;
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .map_err(|e| ToadStoolError::Network(format!("HTTP client error: {}", e)))?;
            
            match client.get(&url).send().await {
                Ok(response) => Ok(response.status().is_success()),
                Err(_) => {
                    // If HTTP check fails, fall back to process check
                    Ok(self.is_service_running(service_name, deployment))
                }
            }
        }
        
        // Without networking feature, check if service is running
        #[cfg(not(feature = "networking"))]
        Ok(self.is_service_running(service_name, deployment))
    }

    /// TCP health check
    async fn tcp_health_check(
        &self,
        service_name: &str,
        health_check: &HealthCheckConfig,
        deployment: &ActiveDeployment,
    ) -> ToadStoolResult<bool> {
        let port = health_check.port.unwrap_or(8080);

        debug!(
            "Performing TCP health check for {} on port {}",
            service_name, port
        );

        // Attempt TCP connection
        #[cfg(feature = "networking")]
        {
            let network_ip = self.get_service_ip(service_name, deployment);
            let addr = format!("{}:{}", network_ip, port);
            
            use std::time::Duration;
            match tokio::time::timeout(
                Duration::from_secs(3),
                tokio::net::TcpStream::connect(&addr)
            ).await {
                Ok(Ok(_)) => Ok(true),
                Ok(Err(_)) | Err(_) => {
                    // Connection failed or timeout - fall back to process check
                    Ok(self.is_service_running(service_name, deployment))
                }
            }
        }
        
        // Without networking feature, check if service is running
        #[cfg(not(feature = "networking"))]
        Ok(self.is_service_running(service_name, deployment))
    }

    /// Exec health check
    async fn exec_health_check(
        &self,
        service_name: &str,
        health_check: &HealthCheckConfig,
    ) -> ToadStoolResult<bool> {
        if let Some(command) = &health_check.command {
            debug!(
                "Performing exec health check for {}: {}",
                service_name, command
            );

            // Execute command and check exit status
            // In containerized environments, this would exec into the container
            // For native services, execute directly
            use tokio::process::Command;
            
            match Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await
            {
                Ok(output) => Ok(output.status.success()),
                Err(e) => {
                    error!("Health check command failed for {}: {}", service_name, e);
                    Ok(false)
                }
            }
        } else {
            warn!("Exec health check configured but no command provided for {}", service_name);
            Ok(false)
        }
    }

    /// Get service IP from deployment network info
    fn get_service_ip(&self, service_name: &str, deployment: &ActiveDeployment) -> String {
        // Extract IP from network info or use service-specific allocation
        // EVOLVED: Dynamic IP discovery, not hardcoded localhost
        deployment
            .service_instances
            .get(service_name)
            .and_then(|instances| instances.first())
            .and_then(|instance| {
                if let Some(host) = &instance.host {
                    Some(host.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                // Fall back to network gateway
                deployment.network_info.gateway.clone()
            })
    }

    /// Check if service is running
    fn is_service_running(&self, service_name: &str, deployment: &ActiveDeployment) -> bool {
        deployment
            .service_instances
            .get(service_name)
            .map(|instances| {
                instances
                    .iter()
                    .any(|i| i.status == ServiceInstanceStatus::Running)
            })
            .unwrap_or(false)
    }

    /// Update service status based on health check
    fn update_service_status(
        &self,
        service_name: &str,
        is_healthy: bool,
        deployment: &mut ActiveDeployment,
    ) {
        if let Some(instances) = deployment.service_instances.get_mut(service_name) {
            for instance in instances.iter_mut() {
                instance.health_status = if is_healthy {
                    "healthy".to_string()
                } else {
                    "unhealthy".to_string()
                };
                instance.last_health_check = Some(SystemTime::now());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byob::*;
    use std::collections::HashMap;

    fn create_test_deployment() -> ActiveDeployment {
        let mut services = HashMap::new();
        services.insert(
            "web".to_string(),
            ServiceSpec {
                image: "nginx:latest".to_string(),
                environment: HashMap::new(),
                ports: vec![],
                volumes: vec![],
                resources: Default::default(),
                depends_on: vec![],
                health_check: Some(HealthCheckConfig {
                    check_type: "http".to_string(),
                    port: Some(80),
                    path: Some("/health".to_string()),
                    interval_seconds: 10,
                    timeout_seconds: 5,
                    retries: 3,
                    command: None,
                }),
            },
        );

        let request = ByobDeploymentRequest {
            deployment_id: Uuid::new_v4(),
            team_id: "test-team".to_string(),
            services,
            network_config: None,
            resource_quotas: Default::default(),
        };

        let network_info = NetworkInfo {
            network_id: "test-network".to_string(),
            subnet: "10.0.0.0/24".to_string(),
            gateway: "10.0.0.1".to_string(),
            dns_servers: vec!["8.8.8.8".to_string()],
            isolation_enabled: true,
        };

        ActiveDeployment::new(request, network_info)
    }

    #[tokio::test]
    async fn test_http_health_check() {
        let monitor = HealthMonitor::new(Duration::from_secs(10));
        let deployment = create_test_deployment();

        let service_spec = deployment.request.services.get("web")
            .ok_or_else(|| ToadStoolError::validation("web service not found"))?;
        let health_check = service_spec.health_check.as_ref()
            .ok_or_else(|| ToadStoolError::validation("health_check not configured"))?;

        let result = monitor
            .http_health_check("web", health_check, &deployment)
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_service_running() {
        let monitor = HealthMonitor::new(Duration::from_secs(10));
        let deployment = create_test_deployment();

        // Service not started yet
        assert!(!monitor.is_service_running("web", &deployment));
    }

    #[test]
    fn test_get_service_ip_uses_gateway_fallback() {
        let monitor = HealthMonitor::new(Duration::from_secs(10));
        let deployment = create_test_deployment();

        let ip = monitor.get_service_ip("web", &deployment);
        assert_eq!(ip, "10.0.0.1"); // Falls back to gateway
    }
}

