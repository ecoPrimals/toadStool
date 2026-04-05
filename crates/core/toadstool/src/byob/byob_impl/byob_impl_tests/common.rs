// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared test helpers for BYOB implementation tests

use super::super::*;
use crate::byob::{
    DnsConfig, PortMapping, ServiceResourceRequirements, TeamNetworkConfig, TeamResourceQuotas,
    TeamSecurityConfig, VolumeMount,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

/// Helper function to create test deployment request
pub fn create_test_deployment_request() -> ByobDeploymentRequest {
    let mut services = HashMap::new();
    services.insert(
        "web-service".to_string(),
        create_test_service_spec("web-service"),
    );
    services.insert(
        "api-service".to_string(),
        create_test_service_spec("api-service"),
    );

    ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "test-team".to_string(),
        deployment_name: "test-deployment".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 8 * 1024 * 1024 * 1024, // 8GB
            max_storage_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            max_gpu_count: 2,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec!["default".to_string()],
            volume_policies: vec!["read-write".to_string()],
            resource_policies: vec!["standard".to_string()],
        },
        network_config: TeamNetworkConfig {
            network_name: "test-network".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: Some(DnsConfig {
                servers: vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()],
                search_domains: vec!["local".to_string()],
            }),
            load_balancer: None,
        },
        created_at: SystemTime::now(),
    }
}

/// Helper function to create test service spec
pub fn create_test_service_spec(name: &str) -> ServiceSpec {
    ServiceSpec {
        name: name.to_string(),
        version: "1.0.0".to_string(),
        image: Some(format!("test/{name}:latest")),
        command: Some(vec!["./start.sh".to_string()]),
        environment: HashMap::from([
            ("ENV".to_string(), "test".to_string()),
            ("SERVICE_NAME".to_string(), name.to_string()),
        ]),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(1.0),
            memory_bytes: Some(512 * 1024 * 1024),   // 512MB
            storage_bytes: Some(1024 * 1024 * 1024), // 1GB
            gpu_count: None,
        },
        ports: vec![PortMapping {
            container_port: if name.contains("web") { 80 } else { 8080 },
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![VolumeMount {
            source: "/tmp/test".to_string(),
            target: "/app/data".to_string(),
            mount_type: "bind".to_string(),
            read_only: false,
        }],
        dependencies: Vec::new(),
        health_check: Some(HealthCheck {
            command: vec![
                "curl".to_string(),
                "-f".to_string(),
                format!("http://{{SERVICE_IP}}:{{SERVICE_PORT}}/health"),
            ],
            interval: 30,
            timeout: 5,
            retries: 3,
            start_period: 10,
        }),
        replicas: 1,
    }
}

/// Simple test runtime engine for testing
pub struct TestRuntimeEngine;

impl RuntimeEngine for TestRuntimeEngine {
    fn initialize(
        &mut self,
        _config: crate::execution::RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<
        Box<dyn Future<Output = ToadStoolResult<crate::execution::ExecutionResponse>> + Send + '_>,
    > {
        Box::pin(async move {
            Ok(crate::execution::ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: crate::execution::ExecutionOutput::default(),
                metrics: crate::resources::RuntimeMetrics::default(),
                duration: Duration::from_millis(100),
                runtime_used: crate::execution::RuntimeType::Native,
                warnings: vec![],
            })
        })
    }

    fn get_capabilities(&self) -> crate::execution::RuntimeCapabilities {
        crate::execution::RuntimeCapabilities {
            supported_workloads: vec![crate::workload::WorkloadType::Native],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: std::collections::HashMap::new(),
            version: "test-1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, _workload_type: &crate::workload::WorkloadType) -> bool {
        true
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<crate::resources::RuntimeMetrics>> + Send + '_>>
    {
        Box::pin(async { Ok(crate::resources::RuntimeMetrics::default()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

pub fn create_test_runtime_engine() -> Arc<dyn RuntimeEngine> {
    Arc::new(TestRuntimeEngine)
}

/// Config with deterministic values for testing (avoids env-dependent `coordination_port`)
pub fn create_test_config(
    default_host_port: u16,
    web_service_ports: Vec<u16>,
) -> ByobExecutorConfig {
    ByobExecutorConfig {
        max_concurrent_deployments: 10,
        default_network_subnet: "10.0.0.0/24".to_string(),
        resource_monitoring_interval: Duration::from_secs(30),
        health_check_interval: Duration::from_secs(10),
        deployment_timeout: Duration::from_secs(600),
        default_host_port,
        web_service_ports,
        graceful_shutdown_timeout_secs: 30,
    }
}
