// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive test coverage for BYOB executor

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::time::SystemTime;
use uuid::Uuid;

use toadstool::byob::{
    byob_impl::{ByobComputeExecutor, ByobExecutor},
    byob_types::*,
    config::ByobExecutorConfig,
};
use toadstool::{
    execution::{
        ExecutionOutput, ExecutionResponse, RuntimeCapabilities, RuntimeConfig, RuntimeType,
    },
    resources::RuntimeMetrics,
    workload::WorkloadType,
    ExecutionRequest, ExecutionStatus, RuntimeEngine, ToadStoolError, ToadStoolResult,
};

// Simple test runtime engine for testing
struct TestRuntimeEngine {
    should_fail: bool,
}

impl TestRuntimeEngine {
    fn new() -> Self {
        Self { should_fail: false }
    }

    fn new_failing() -> Self {
        Self { should_fail: true }
    }
}

impl RuntimeEngine for TestRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>> {
        let should_fail = self.should_fail;
        Box::pin(async move {
            if should_fail {
                return Err(ToadStoolError::runtime(
                    "Test execution failure".to_string(),
                ));
            }

            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput::default(),
                metrics: RuntimeMetrics::default(),
                duration: Duration::from_millis(100),
                runtime_used: RuntimeType::Native,
                warnings: vec![],
            })
        })
    }

    fn get_capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Native, WorkloadType::Container],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: std::collections::HashMap::new(),
            version: "test-1.0.0".to_string(),
        }
    }

    fn supports_workload(&self, _workload_type: &WorkloadType) -> bool {
        true
    }

    fn get_metrics(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_>> {
        Box::pin(async { Ok(RuntimeMetrics::default()) })
    }

    fn shutdown(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
}

// Helper function to create test deployment request
fn create_test_deployment_request() -> ByobDeploymentRequest {
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

// Helper function to create test service spec
fn create_test_service_spec(name: &str) -> ServiceSpec {
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
                format!(
                    "http://localhost:{}/health",
                    if name.contains("web") { 80 } else { 8080 }
                ),
            ],
            interval: 30,
            timeout: 5,
            retries: 3,
            start_period: 10,
        }),
        replicas: 1,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_success() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let request = create_test_deployment_request();
    let deployment_id = request.deployment_id;

    // Deploy biome
    let response = executor.deploy_biome(request).await;
    assert!(response.is_ok(), "Deployment should succeed");

    let response = response.unwrap();
    assert_eq!(response.deployment_id, deployment_id);
    assert!(matches!(response.status, DeploymentStatus::Running));
    assert!(
        !response.service_statuses.is_empty(),
        "Should have service statuses"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_with_service_failure() {
    let mock_engine = Arc::new(TestRuntimeEngine::new_failing());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let request = create_test_deployment_request();

    // Deploy biome should fail when service execution fails
    let response = executor.deploy_biome(request).await;
    assert!(
        response.is_err(),
        "Deployment should fail when service fails"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_resource_quota_exceeded() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Set CPU requirement higher than quota
    for service in request.services.values_mut() {
        service.resources.cpu_cores = Some(20.0);
    }

    let response = executor.deploy_biome(request).await;
    assert!(
        response.is_err(),
        "Deployment should fail when quota exceeded"
    );

    let err = response.unwrap_err();
    assert!(err.to_string().contains("CPU"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_memory_quota_exceeded() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Set memory requirement higher than quota
    for service in request.services.values_mut() {
        service.resources.memory_bytes = Some(10 * 1024 * 1024 * 1024); // 10GB
    }

    let response = executor.deploy_biome(request).await;
    assert!(
        response.is_err(),
        "Deployment should fail when memory quota exceeded"
    );

    let err = response.unwrap_err();
    assert!(err.to_string().contains("Memory"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_storage_quota_exceeded() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Set storage requirement higher than quota
    for service in request.services.values_mut() {
        service.resources.storage_bytes = Some(200 * 1024 * 1024 * 1024); // 200GB
    }

    let response = executor.deploy_biome(request).await;
    assert!(
        response.is_err(),
        "Deployment should fail when storage quota exceeded"
    );

    let err = response.unwrap_err();
    assert!(err.to_string().contains("Storage"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_gpu_quota_exceeded() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Set GPU requirement higher than quota
    for service in request.services.values_mut() {
        service.resources.gpu_count = Some(5);
    }

    let response = executor.deploy_biome(request).await;
    assert!(
        response.is_err(),
        "Deployment should fail when GPU quota exceeded"
    );

    let err = response.unwrap_err();
    assert!(err.to_string().contains("GPU"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_too_many_services() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Add too many services
    for i in 0..20 {
        request.services.insert(
            format!("service-{i}"),
            create_test_service_spec(&format!("service-{i}")),
        );
    }

    let response = executor.deploy_biome(request).await;
    assert!(
        response.is_err(),
        "Deployment should fail when too many services"
    );

    let err = response.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("Service count")
            || err_msg.contains("services")
            || err_msg.contains("quota"),
        "Expected error about service count, got: {err_msg}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deploy_biome_max_concurrent_deployments() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let config = ByobExecutorConfig {
        max_concurrent_deployments: 1,
        ..Default::default()
    };

    let executor = ByobComputeExecutor::new(mock_engine, config);

    // First deployment should succeed
    let request1 = create_test_deployment_request();
    let response1 = executor.deploy_biome(request1).await;
    assert!(response1.is_ok(), "First deployment should succeed");

    // Second deployment should fail due to limit
    let request2 = create_test_deployment_request();
    let response2 = executor.deploy_biome(request2).await;
    assert!(
        response2.is_err(),
        "Second deployment should fail due to limit"
    );

    let err = response2.unwrap_err();
    assert!(err.to_string().contains("Maximum concurrent deployments"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_deployment_status_success() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    // Deploy first
    let request = create_test_deployment_request();
    let deployment_id = request.deployment_id;
    executor
        .deploy_biome(request)
        .await
        .expect("Deploy should succeed");

    // Get status
    let status = executor.get_deployment_status(deployment_id).await;
    assert!(status.is_ok(), "Get status should succeed");

    let response = status.unwrap();
    assert_eq!(response.deployment_id, deployment_id);
    assert!(matches!(response.status, DeploymentStatus::Running));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_deployment_status_not_found() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let non_existent_id = Uuid::new_v4();
    let status = executor.get_deployment_status(non_existent_id).await;

    assert!(
        status.is_err(),
        "Get status should fail for non-existent deployment"
    );
    let err = status.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stop_deployment_success() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    // Deploy first
    let request = create_test_deployment_request();
    let deployment_id = request.deployment_id;
    executor
        .deploy_biome(request)
        .await
        .expect("Deploy should succeed");

    // Stop deployment
    let result = executor.stop_deployment(deployment_id).await;
    assert!(result.is_ok(), "Stop should succeed");

    // Verify status changed
    let status = executor.get_deployment_status(deployment_id).await.unwrap();
    assert!(matches!(status.status, DeploymentStatus::Stopped));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stop_deployment_not_found() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let non_existent_id = Uuid::new_v4();
    let result = executor.stop_deployment(non_existent_id).await;

    assert!(
        result.is_err(),
        "Stop should fail for non-existent deployment"
    );
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_deployments_empty() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let deployments = executor.list_deployments().await;
    assert!(deployments.is_ok(), "List should succeed");
    assert_eq!(deployments.unwrap().len(), 0, "Should have no deployments");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_deployments_multiple() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    // Deploy multiple biomes
    let request1 = create_test_deployment_request();
    let id1 = request1.deployment_id;
    executor
        .deploy_biome(request1)
        .await
        .expect("Deploy 1 should succeed");

    let request2 = create_test_deployment_request();
    let id2 = request2.deployment_id;
    executor
        .deploy_biome(request2)
        .await
        .expect("Deploy 2 should succeed");

    // List deployments
    let deployments = executor.list_deployments().await;
    assert!(deployments.is_ok(), "List should succeed");

    let list = deployments.unwrap();
    assert_eq!(list.len(), 2, "Should have 2 deployments");

    let ids: Vec<Uuid> = list.iter().map(|d| d.deployment_id).collect();
    assert!(ids.contains(&id1), "Should contain first deployment");
    assert!(ids.contains(&id2), "Should contain second deployment");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_resource_usage_success() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    // Deploy first
    let request = create_test_deployment_request();
    let deployment_id = request.deployment_id;
    executor
        .deploy_biome(request)
        .await
        .expect("Deploy should succeed");

    // Get resource usage
    let usage = executor.get_resource_usage(deployment_id).await;
    assert!(usage.is_ok(), "Get resource usage should succeed");

    let resource_usage = usage.unwrap();
    // Verify structure exists
    assert!(resource_usage.cpu_usage >= 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_resource_usage_not_found() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let non_existent_id = Uuid::new_v4();
    let usage = executor.get_resource_usage(non_existent_id).await;

    assert!(
        usage.is_err(),
        "Get resource usage should fail for non-existent deployment"
    );
    let err = usage.unwrap_err();
    assert!(err.to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deployment_with_native_workload() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Create service without image (native workload)
    let mut native_service = create_test_service_spec("native-service");
    native_service.image = None;
    request
        .services
        .insert("native-service".to_string(), native_service);

    let response = executor.deploy_biome(request).await;
    assert!(
        response.is_ok(),
        "Native workload deployment should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deployment_with_volumes() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Add service with multiple volumes
    let mut service = create_test_service_spec("volume-service");
    service.volumes = vec![
        VolumeMount {
            source: "/data".to_string(),
            target: "/app/data".to_string(),
            mount_type: "volume".to_string(),
            read_only: false,
        },
        VolumeMount {
            source: "/config".to_string(),
            target: "/app/config".to_string(),
            mount_type: "bind".to_string(),
            read_only: true,
        },
    ];
    request
        .services
        .insert("volume-service".to_string(), service);

    let response = executor.deploy_biome(request).await;
    assert!(response.is_ok(), "Volume deployment should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deployment_with_custom_ports() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Add service with custom ports
    let mut service = create_test_service_spec("custom-port-service");
    service.ports = vec![
        PortMapping {
            container_port: 3000,
            host_port: Some(3001),
            protocol: "tcp".to_string(),
        },
        PortMapping {
            container_port: 8080,
            host_port: None,
            protocol: "udp".to_string(),
        },
    ];
    request
        .services
        .insert("custom-port-service".to_string(), service);

    let response = executor.deploy_biome(request).await;
    assert!(response.is_ok(), "Custom port deployment should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deployment_with_gpu_resources() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let mut request = create_test_deployment_request();

    // Add service with GPU
    let mut service = create_test_service_spec("gpu-service");
    service.resources.gpu_count = Some(1);
    request.services.insert("gpu-service".to_string(), service);

    let response = executor.deploy_biome(request).await;
    assert!(response.is_ok(), "GPU deployment should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deployment_network_creation() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let request = create_test_deployment_request();
    let response = executor
        .deploy_biome(request)
        .await
        .expect("Deploy should succeed");

    // Verify network info exists
    assert!(!response.network_info.network_name.is_empty());
    assert!(!response.network_info.subnet_cidr.is_empty());
    assert!(!response.network_info.gateway_ip.is_empty());
    assert!(!response.network_info.service_endpoints.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_deployment_service_endpoints() {
    let mock_engine = Arc::new(TestRuntimeEngine::new());
    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    let request = create_test_deployment_request();
    let response = executor
        .deploy_biome(request)
        .await
        .expect("Deploy should succeed");

    // Verify service endpoints
    let endpoints = &response.network_info.service_endpoints;
    assert!(endpoints.contains_key("web-service"));
    assert!(endpoints.contains_key("api-service"));

    for endpoint in endpoints.values() {
        assert!(!endpoint.internal_ip.is_empty());
        assert!(!endpoint.ports.is_empty());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_config_defaults() {
    let config = ByobExecutorConfig::default();

    assert_eq!(config.max_concurrent_deployments, 50);
    assert_eq!(config.default_network_subnet, "10.0.0.0/24");
    assert_eq!(config.resource_monitoring_interval, Duration::from_secs(30));
    assert_eq!(config.health_check_interval, Duration::from_secs(10));
    assert_eq!(config.deployment_timeout, Duration::from_secs(600));
    assert!(!config.web_service_ports.is_empty());
}
