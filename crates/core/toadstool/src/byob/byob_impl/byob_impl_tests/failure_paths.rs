// SPDX-License-Identifier: AGPL-3.0-only
//! Error-path coverage for BYOB service execution and deployment.

use super::super::*;
use super::common::*;
use crate::byob::{
    PortMapping, ServiceResourceRequirements, TeamNetworkConfig, TeamResourceQuotas,
    TeamSecurityConfig,
};
use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

struct NonSuccessRuntimeEngine;

impl RuntimeEngine for NonSuccessRuntimeEngine {
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
                status: ExecutionStatus::Failed {
                    error: Cow::Borrowed("simulated failure"),
                },
                output: crate::execution::ExecutionOutput::default(),
                metrics: crate::resources::RuntimeMetrics::default(),
                duration: Duration::from_millis(1),
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
            version: "test".to_string(),
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

struct ErringRuntimeEngine;

impl RuntimeEngine for ErringRuntimeEngine {
    fn initialize(
        &mut self,
        _config: crate::execution::RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn execute(
        &self,
        _request: ExecutionRequest,
    ) -> Pin<
        Box<dyn Future<Output = ToadStoolResult<crate::execution::ExecutionResponse>> + Send + '_>,
    > {
        Box::pin(async move {
            Err(ToadStoolError::runtime(
                "engine refused execute".to_string(),
            ))
        })
    }

    fn get_capabilities(&self) -> crate::execution::RuntimeCapabilities {
        crate::execution::RuntimeCapabilities {
            supported_workloads: vec![crate::workload::WorkloadType::Native],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: std::collections::HashMap::new(),
            version: "test".to_string(),
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

fn minimal_valid_request() -> ByobDeploymentRequest {
    let mut services = HashMap::new();
    services.insert(
        "only".to_string(),
        ServiceSpec {
            name: "only".to_string(),
            version: "1".to_string(),
            image: Some("alpine:latest".to_string()),
            command: None,
            environment: HashMap::new(),
            resources: ServiceResourceRequirements {
                cpu_cores: Some(0.5),
                memory_bytes: Some(128 * 1024 * 1024),
                storage_bytes: Some(1024 * 1024 * 1024),
                gpu_count: None,
            },
            ports: vec![],
            volumes: vec![],
            dependencies: vec![],
            health_check: None,
            replicas: 1,
        },
    );

    ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-x".to_string(),
        deployment_name: "fail-test".to_string(),
        services,
        resource_quotas: TeamResourceQuotas {
            max_cpu_cores: 10.0,
            max_memory_bytes: 8 * 1024 * 1024 * 1024,
            max_storage_bytes: 100 * 1024 * 1024 * 1024,
            max_gpu_count: 2,
            max_concurrent_services: 10,
        },
        security_config: TeamSecurityConfig {
            isolation_level: "standard".to_string(),
            network_policies: vec![],
            volume_policies: vec![],
            resource_policies: vec![],
        },
        network_config: TeamNetworkConfig {
            network_name: "n".to_string(),
            subnet_cidr: "10.0.0.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: SystemTime::now(),
    }
}

#[tokio::test]
async fn test_deploy_biome_execute_returns_non_success_status() {
    let executor = ByobComputeExecutor::new(
        Arc::new(NonSuccessRuntimeEngine) as Arc<dyn RuntimeEngine>,
        create_test_config(8080, vec![80]),
    );
    let err = executor
        .deploy_biome(minimal_valid_request())
        .await
        .unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("failed to start") || msg.contains("Service"),
        "unexpected error: {msg}"
    );
}

#[tokio::test]
async fn test_deploy_biome_execute_returns_error() {
    let executor = ByobComputeExecutor::new(
        Arc::new(ErringRuntimeEngine) as Arc<dyn RuntimeEngine>,
        create_test_config(8080, vec![80]),
    );
    let err = executor
        .deploy_biome(minimal_valid_request())
        .await
        .unwrap_err();
    assert!(err.to_string().contains("refused execute"));
}

#[tokio::test]
async fn test_get_resource_usage_network_tiers_web_and_database() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));

    let mut req = minimal_valid_request();
    req.deployment_id = Uuid::new_v4();
    req.services.clear();
    req.services.insert(
        "web".to_string(),
        ServiceSpec {
            name: "web".to_string(),
            version: "1".to_string(),
            image: Some("corp/web:latest".to_string()),
            command: None,
            environment: HashMap::new(),
            resources: ServiceResourceRequirements {
                cpu_cores: Some(1.0),
                memory_bytes: Some(1024 * 1024 * 1024),
                storage_bytes: Some(10 * 1024 * 1024 * 1024),
                gpu_count: None,
            },
            ports: vec![],
            volumes: vec![],
            dependencies: vec![],
            health_check: None,
            replicas: 1,
        },
    );
    req.services.insert(
        "db".to_string(),
        ServiceSpec {
            name: "db".to_string(),
            version: "1".to_string(),
            image: Some("postgres:database".to_string()),
            command: None,
            environment: HashMap::new(),
            resources: ServiceResourceRequirements {
                cpu_cores: Some(1.0),
                memory_bytes: Some(512 * 1024 * 1024),
                storage_bytes: Some(10 * 1024 * 1024 * 1024),
                gpu_count: None,
            },
            ports: vec![],
            volumes: vec![],
            dependencies: vec![],
            health_check: None,
            replicas: 1,
        },
    );

    executor.deploy_biome(req.clone()).await.unwrap();
    let usage = executor
        .get_resource_usage(req.deployment_id)
        .await
        .unwrap();
    assert!(usage.network_usage.bytes_sent > 1024 * 1024);
    assert!(usage.network_usage.bytes_received > 0);
}

#[tokio::test]
async fn test_list_deployments_includes_stopped_deployment() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let mut req = minimal_valid_request();
    req.deployment_id = Uuid::new_v4();
    executor.deploy_biome(req.clone()).await.unwrap();
    executor.stop_deployment(req.deployment_id).await.unwrap();
    let list = executor.list_deployments().await.unwrap();
    assert_eq!(list.len(), 1);
    assert!(matches!(list[0].status, DeploymentStatus::Stopped));
}

#[test]
fn test_create_service_execution_request_host_port_default_and_udp_branch() {
    let config = create_test_config(7777, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);
    let service = ServiceSpec {
        name: "p".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![PortMapping {
            container_port: 443,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![],
        dependencies: vec![],
        health_check: None,
        replicas: 1,
    };
    let req = executor.create_service_execution_request(&service, Uuid::new_v4());
    match req.workload {
        crate::WorkloadSpec::Container { ports, .. } => {
            assert_eq!(ports[0].host_port, 7777);
        }
        _ => panic!("expected container"),
    }
}
