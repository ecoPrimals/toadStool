// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deployment state and lifecycle management

use std::collections::HashMap;
use std::time::Instant;
use uuid::Uuid;

use super::byob_types::{
    ByobDeploymentRequest, ByobDeploymentResponse, DeploymentStatus, NetworkInfo, NetworkUsage,
    ResourceUsage,
};

/// Active deployment tracking
#[derive(Debug)]
pub(super) struct ActiveDeployment {
    /// Deployment request
    pub request: ByobDeploymentRequest,
    /// Deployment status
    pub status: DeploymentStatus,
    /// Service execution IDs
    pub service_executions: HashMap<String, Uuid>,
    /// Resource usage tracking
    pub resource_usage: ResourceUsage,
    /// Network information
    pub network_info: NetworkInfo,
    /// Created timestamp (accessed via `elapsed()`)
    pub created_at: Instant,
    /// Updated timestamp
    pub updated_at: Instant,
}

impl ActiveDeployment {
    /// Create a new active deployment
    pub fn new(request: ByobDeploymentRequest, network_info: NetworkInfo) -> Self {
        Self {
            request,
            status: DeploymentStatus::Starting,
            service_executions: HashMap::new(),
            resource_usage: ResourceUsage {
                cpu_usage: 0.0,
                memory_usage: 0,
                storage_usage: 0,
                gpu_usage: 0,
                network_usage: NetworkUsage {
                    bytes_sent: 0,
                    bytes_received: 0,
                    packets_sent: 0,
                    packets_received: 0,
                },
            },
            network_info,
            created_at: Instant::now(),
            updated_at: Instant::now(),
        }
    }

    /// Update deployment status
    pub fn update_status(&mut self, status: DeploymentStatus) {
        self.status = status;
        self.updated_at = Instant::now();
    }

    /// Add a service execution
    /// ✅ ZERO-COPY: Use &str to avoid unnecessary String allocation at call site
    pub fn add_service_execution(&mut self, service_name: &str, execution_id: Uuid) {
        self.service_executions
            .insert(service_name.to_string(), execution_id);
        self.updated_at = Instant::now();
    }

    /// Remove a service execution
    pub fn remove_service_execution(&mut self, service_name: &str) -> Option<Uuid> {
        self.updated_at = Instant::now();
        self.service_executions.remove(service_name)
    }

    /// Update resource usage
    pub fn update_resource_usage(&mut self, resource_usage: ResourceUsage) {
        self.resource_usage = resource_usage;
        self.updated_at = Instant::now();
    }

    /// Get elapsed time since creation
    pub fn elapsed(&self) -> std::time::Duration {
        self.created_at.elapsed()
    }

    /// Check if deployment is active (Starting or Running)
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            DeploymentStatus::Starting | DeploymentStatus::Running
        )
    }

    /// Check if deployment has reached a terminal state
    pub fn is_completed(&self) -> bool {
        matches!(
            self.status,
            DeploymentStatus::Stopped
                | DeploymentStatus::Stopping
                | DeploymentStatus::Failed { .. }
        )
    }

    /// Get deployment response
    pub fn to_response(&self) -> ByobDeploymentResponse {
        // Convert service executions to service statuses
        let service_statuses: HashMap<String, _> = self
            .request
            .services
            .iter()
            .map(|(name, spec)| {
                let status = crate::byob::byob_types::ServiceStatus {
                    name: name.clone(),
                    state: "running".to_string(),
                    running_replicas: spec.replicas,
                    desired_replicas: spec.replicas,
                    health: "healthy".to_string(),
                    updated_at: std::time::SystemTime::now(),
                };
                (name.clone(), status)
            })
            .collect();

        ByobDeploymentResponse {
            deployment_id: self.request.deployment_id,
            status: self.status.clone(),
            service_statuses,
            resource_usage: self.resource_usage.clone(),
            network_info: self.network_info.clone(),
            created_at: self.request.created_at,
            updated_at: std::time::SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::byob_types::{TeamNetworkConfig, TeamResourceQuotas, TeamSecurityConfig};
    use super::*;
    use std::time::SystemTime;

    fn create_test_request() -> ByobDeploymentRequest {
        ByobDeploymentRequest {
            deployment_id: Uuid::new_v4(),
            team_id: "test-team".to_string(),
            deployment_name: "test-deployment".to_string(),
            services: HashMap::new(),
            resource_quotas: TeamResourceQuotas {
                max_cpu_cores: 4.0,
                max_memory_bytes: 8_589_934_592,
                max_storage_bytes: 107_374_182_400,
                max_gpu_count: 0,
                max_concurrent_services: 10,
            },
            security_config: TeamSecurityConfig {
                isolation_level: "standard".to_string(),
                network_policies: vec![],
                volume_policies: vec![],
                resource_policies: vec![],
            },
            network_config: TeamNetworkConfig {
                network_name: "test-network".to_string(),
                subnet_cidr: "10.0.0.0/24".to_string(),
                dns_config: None,
                load_balancer: None,
            },
            created_at: SystemTime::now(),
        }
    }

    fn create_test_network_info() -> NetworkInfo {
        NetworkInfo {
            network_name: "test-network".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            gateway_ip: "10.0.0.1".to_string(),
            service_endpoints: HashMap::new(),
        }
    }

    #[test]
    fn test_active_deployment_creation() {
        let request = create_test_request();
        let network_info = create_test_network_info();
        let deployment = ActiveDeployment::new(request, network_info);

        assert!(matches!(deployment.status, DeploymentStatus::Starting));
        assert!(deployment.service_executions.is_empty());
        assert!(deployment.is_active());
        assert!(!deployment.is_completed());
    }

    #[test]
    fn test_update_status() {
        let request = create_test_request();
        let network_info = create_test_network_info();
        let mut deployment = ActiveDeployment::new(request, network_info);

        deployment.update_status(DeploymentStatus::Running);
        assert!(matches!(deployment.status, DeploymentStatus::Running));
        assert!(deployment.is_active());
    }

    #[test]
    fn test_service_execution_management() {
        let request = create_test_request();
        let network_info = create_test_network_info();
        let mut deployment = ActiveDeployment::new(request, network_info);

        let execution_id = Uuid::new_v4();
        deployment.add_service_execution("service1", execution_id); // ✅ ZERO-COPY: No .to_string() needed

        assert_eq!(deployment.service_executions.len(), 1);
        assert_eq!(
            deployment.service_executions.get("service1"),
            Some(&execution_id)
        );

        let removed = deployment.remove_service_execution("service1");
        assert_eq!(removed, Some(execution_id));
        assert!(deployment.service_executions.is_empty());
    }
}
