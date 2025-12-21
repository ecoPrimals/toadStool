//! Request validation for BYOB deployments

use super::byob_types::{ByobDeploymentRequest, ServiceSpec};
use crate::{ToadStoolError, ToadStoolResult};

/// Validates deployment requests for resource quotas and constraints
pub(super) struct DeploymentValidator;

impl DeploymentValidator {
    /// Validate deployment request against resource quotas
    pub fn validate(request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
        Self::validate_resource_quotas(request)?;
        Self::validate_services(request)?;
        Self::validate_network_config(request)?;
        Ok(())
    }

    /// Validate resource quotas
    fn validate_resource_quotas(request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
        let totals = Self::calculate_resource_totals(request);

        if totals.cpu > request.resource_quotas.max_cpu_cores {
            return Err(ToadStoolError::resource(format!(
                "CPU requirement {:.2} exceeds team quota {:.2}",
                totals.cpu, request.resource_quotas.max_cpu_cores
            )));
        }

        if totals.memory > request.resource_quotas.max_memory_bytes {
            return Err(ToadStoolError::resource(format!(
                "Memory requirement {} exceeds team quota {}",
                totals.memory, request.resource_quotas.max_memory_bytes
            )));
        }

        if totals.storage > request.resource_quotas.max_storage_bytes {
            return Err(ToadStoolError::resource(format!(
                "Storage requirement {} exceeds team quota {}",
                totals.storage, request.resource_quotas.max_storage_bytes
            )));
        }

        if totals.gpu > request.resource_quotas.max_gpu_count {
            return Err(ToadStoolError::resource(format!(
                "GPU requirement {} exceeds team quota {}",
                totals.gpu, request.resource_quotas.max_gpu_count
            )));
        }

        Ok(())
    }

    /// Validate service specifications
    fn validate_services(request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
        if request.services.is_empty() {
            return Err(ToadStoolError::validation(
                "Deployment must contain at least one service",
            ));
        }

        for (service_name, service_spec) in &request.services {
            Self::validate_service_spec(service_name, service_spec)?;
        }

        Ok(())
    }

    /// Validate individual service specification
    fn validate_service_spec(name: &str, spec: &ServiceSpec) -> ToadStoolResult<()> {
        if spec.image.is_empty() {
            return Err(ToadStoolError::validation(format!(
                "Service '{name}' has empty image specification"
            )));
        }

        // Validate port specifications
        for port_mapping in &spec.ports {
            if port_mapping.container_port == 0 {
                return Err(ToadStoolError::validation(format!(
                    "Service '{name}' has invalid container port 0"
                )));
            }
        }

        Ok(())
    }

    /// Validate network configuration
    fn validate_network_config(request: &ByobDeploymentRequest) -> ToadStoolResult<()> {
        // Validate network isolation settings
        if let Some(network_config) = &request.network_config {
            if network_config.isolation_level.is_empty() {
                return Err(ToadStoolError::validation(
                    "Network isolation level must be specified",
                ));
            }
        }

        Ok(())
    }

    /// Calculate total resource requirements
    fn calculate_resource_totals(request: &ByobDeploymentRequest) -> ResourceTotals {
        let mut totals = ResourceTotals::default();

        for service_spec in request.services.values() {
            totals.cpu += service_spec.resources.cpu_cores.unwrap_or(0.0);
            totals.memory += service_spec.resources.memory_bytes.unwrap_or(0);
            totals.storage += service_spec.resources.storage_bytes.unwrap_or(0);
            totals.gpu += service_spec.resources.gpu_count.unwrap_or(0);
        }

        totals
    }
}

/// Resource totals for validation
#[derive(Default)]
struct ResourceTotals {
    cpu: f64,
    memory: u64,
    storage: u64,
    gpu: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byob::*;

    #[test]
    fn test_validate_empty_services() {
        let mut request = create_test_request();
        request.services.clear();

        let result = DeploymentValidator::validate(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_resource_quotas_exceeded() {
        let mut request = create_test_request();
        request.resource_quotas.max_cpu_cores = 0.5;

        let result = DeploymentValidator::validate(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_request() {
        let request = create_test_request();
        let result = DeploymentValidator::validate(&request);
        assert!(result.is_ok());
    }

    fn create_test_request() -> ByobDeploymentRequest {
        use std::collections::HashMap;
        use uuid::Uuid;

        let mut services = HashMap::new();
        services.insert(
            "test-service".to_string(),
            ServiceSpec {
                image: "test:latest".to_string(),
                environment: HashMap::new(),
                ports: vec![],
                volumes: vec![],
                resources: ResourceRequirements {
                    cpu_cores: Some(1.0),
                    memory_bytes: Some(1024 * 1024 * 1024),
                    storage_bytes: Some(1024 * 1024 * 1024),
                    gpu_count: Some(0),
                },
                depends_on: vec![],
                health_check: None,
            },
        );

        ByobDeploymentRequest {
            deployment_id: Uuid::new_v4(),
            team_id: "test-team".to_string(),
            services,
            network_config: None,
            resource_quotas: ResourceQuotas {
                max_cpu_cores: 10.0,
                max_memory_bytes: 10 * 1024 * 1024 * 1024,
                max_storage_bytes: 100 * 1024 * 1024 * 1024,
                max_gpu_count: 2,
            },
        }
    }
}

