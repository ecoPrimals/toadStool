//! BYOB implementation tests

use super::*;
use chrono::Utc;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio;
use uuid::Uuid;

#[tokio::test]
async fn test_validate_deployment_request() {
    // Create a simple test runtime engine
    let mock_engine = create_test_runtime_engine();

    let executor = ByobComputeExecutor::new(mock_engine, ByobExecutorConfig::default());

    // Test valid deployment request
    let valid_request = create_test_deployment_request();
    assert!(executor.validate_deployment_request(&valid_request).is_ok());

    // Test invalid deployment - too many services
    let mut invalid_request = valid_request.clone();
    for i in 0..100 {
        invalid_request.services.insert(
            format!("service-{i}"),
            create_test_service_spec(&format!("service-{i}")),
        );
    }
    assert!(executor
        .validate_deployment_request(&invalid_request)
        .is_err());

    // Test invalid deployment - excessive resource requirements
    let mut resource_heavy_request = create_test_deployment_request();
    let mut heavy_service = create_test_service_spec("heavy-service");
    heavy_service.resources.cpu_cores = Some(1000.0); // Excessive CPU
    heavy_service.resources.memory_bytes = Some(1024 * 1024 * 1024 * 1024); // 1TB RAM
    resource_heavy_request
        .services
        .insert("heavy-service".to_string(), heavy_service);

    assert!(executor
        .validate_deployment_request(&resource_heavy_request)
        .is_err());

    // Test invalid deployment - empty services
    let mut empty_request = create_test_deployment_request();
    empty_request.services.clear();
    assert!(executor
        .validate_deployment_request(&empty_request)
        .is_err());

    // Test invalid deployment - service without image or command
    let mut no_image_no_cmd_request = create_test_deployment_request();
    let mut bad_service = create_test_service_spec("bad-service");
    bad_service.image = None;
    bad_service.command = None;
    no_image_no_cmd_request
        .services
        .insert("bad-service".to_string(), bad_service);
    assert!(executor
        .validate_deployment_request(&no_image_no_cmd_request)
        .is_err());
}

#[test]
fn test_byob_executor_creation() {
    // Test basic structure validation without mock dependencies
    let config = ByobExecutorConfig::default();

    // Verify default configuration
    assert_eq!(config.max_concurrent_deployments, 50);
    assert_eq!(config.default_network_subnet, "10.0.0.0/24");
    assert_eq!(config.resource_monitoring_interval, Duration::from_secs(30));
    assert_eq!(config.health_check_interval, Duration::from_secs(10));
    assert_eq!(config.deployment_timeout, Duration::from_secs(600));

    #[allow(deprecated)]
    let env_config = toadstool_config::env_config::EnvironmentConfig::from_env();
    #[allow(deprecated)]
    let expected_port = env_config.network.songbird_port;

    assert_eq!(config.default_host_port, expected_port);
    assert_eq!(
        config.web_service_ports,
        vec![80, 443, expected_port, 8443, 3000, 8000, 9000]
    );
}

#[test]
fn test_deployment_request_validation() {
    let deployment_request = create_test_deployment_request();

    // Test deployment request structure
    assert!(!deployment_request.deployment_id.is_nil());
    assert_eq!(deployment_request.team_id, "test-team");
    assert_eq!(deployment_request.services.len(), 2);

    // Test service configurations
    let web_service = deployment_request
        .services
        .get("web-service")
        .expect("web-service should exist in deployment request");
    assert_eq!(web_service.name, "web-service");
    assert!(web_service.image.is_some());
    assert!(!web_service.ports.is_empty());

    let api_service = deployment_request
        .services
        .get("api-service")
        .expect("api-service should exist in deployment request");
    assert_eq!(api_service.name, "api-service");
    assert!(api_service.image.is_some());
    assert!(!api_service.environment.is_empty());
}

#[test]
fn test_deployment_request_serialization_roundtrip() {
    let request = create_test_deployment_request();
    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("deployment_id"));
    assert!(json.contains("test-team"));
    let restored: ByobDeploymentRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.team_id, request.team_id);
    assert_eq!(restored.services.len(), request.services.len());
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
        created_at: Utc::now(),
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
                // EVOLVED: Health check will use service's actual network endpoint
                // Port will be discovered from deployment network configuration
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

// Simple test runtime engine for testing
struct TestRuntimeEngine;

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

fn create_test_runtime_engine() -> Arc<dyn RuntimeEngine> {
    Arc::new(TestRuntimeEngine)
}

/// Config with deterministic values for testing (avoids env-dependent songbird_port)
fn create_test_config(default_host_port: u16, web_service_ports: Vec<u16>) -> ByobExecutorConfig {
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

// ─── create_service_execution_request tests ──────────────────────────────

#[test]
fn test_create_service_execution_request_container_workload() {
    let config = create_test_config(9999, vec![80, 443, 8080]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "web-svc".to_string(),
        version: "1.0".to_string(),
        image: Some("nginx:latest".to_string()),
        command: Some(vec!["nginx".to_string()]),
        environment: HashMap::from([("FOO".to_string(), "bar".to_string())]),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(2.0),
            memory_bytes: Some(1024 * 1024 * 1024),
            storage_bytes: Some(2 * 1024 * 1024 * 1024),
            gpu_count: None,
        },
        ports: vec![PortMapping {
            container_port: 80,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![VolumeMount {
            source: "/data".to_string(),
            target: "/app/data".to_string(),
            mount_type: "volume".to_string(),
            read_only: true,
        }],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor
        .create_service_execution_request(&service, Uuid::new_v4())
        .unwrap();

    assert!(!req.execution_id.is_nil());
    match &req.workload {
        crate::WorkloadSpec::Container {
            image,
            command,
            env_vars,
            volumes,
            ports,
            ..
        } => {
            assert_eq!(image, "nginx:latest");
            assert_eq!(command, &Some(vec!["nginx".to_string()]));
            assert_eq!(env_vars.get("FOO"), Some(&"bar".to_string()));
            assert_eq!(volumes.len(), 1);
            assert_eq!(volumes[0].source.to_str(), Some("/data"));
            assert_eq!(volumes[0].target.to_str(), Some("/app/data"));
            assert!(matches!(
                volumes[0].mount_type,
                crate::workload::VolumeMountType::Volume
            ));
            assert!(volumes[0].read_only);
            assert_eq!(ports.len(), 1);
            assert_eq!(ports[0].container_port, 80);
            assert_eq!(ports[0].host_port, 9999); // default_host_port
            assert!(matches!(
                ports[0].protocol,
                crate::workload::PortProtocol::Tcp
            ));
        }
        _ => panic!("expected Container workload"),
    }
    assert_eq!(req.resources.cpu.min_cores, 2.0);
    assert_eq!(req.resources.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.resources.storage.min_bytes, 2 * 1024 * 1024 * 1024);
}

#[test]
fn test_create_service_execution_request_volume_mount_types() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let volumes = vec![
        VolumeMount {
            source: "/tmp".to_string(),
            target: "/tmp".to_string(),
            mount_type: "bind".to_string(),
            read_only: false,
        },
        VolumeMount {
            source: "myvol".to_string(),
            target: "/data".to_string(),
            mount_type: "volume".to_string(),
            read_only: true,
        },
    ];

    let service = ServiceSpec {
        name: "svc".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(1.0),
            memory_bytes: Some(512 * 1024 * 1024),
            storage_bytes: Some(1024 * 1024 * 1024),
            gpu_count: None,
        },
        ports: vec![],
        volumes,
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor
        .create_service_execution_request(&service, Uuid::new_v4())
        .unwrap();

    match &req.workload {
        crate::WorkloadSpec::Container { volumes: v, .. } => {
            assert!(matches!(
                v[0].mount_type,
                crate::workload::VolumeMountType::Bind
            ));
            assert!(matches!(
                v[1].mount_type,
                crate::workload::VolumeMountType::Volume
            ));
        }
        _ => panic!("expected Container"),
    }
}

#[test]
fn test_create_service_execution_request_udp_port() {
    let config = create_test_config(5353, vec![53]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "dns".to_string(),
        version: "1".to_string(),
        image: Some("dns:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(1.0),
            memory_bytes: Some(256 * 1024 * 1024),
            storage_bytes: Some(1024 * 1024 * 1024),
            gpu_count: None,
        },
        ports: vec![PortMapping {
            container_port: 53,
            host_port: Some(10053),
            protocol: "udp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor
        .create_service_execution_request(&service, Uuid::new_v4())
        .unwrap();

    match &req.workload {
        crate::WorkloadSpec::Container { ports, .. } => {
            assert!(matches!(
                ports[0].protocol,
                crate::workload::PortProtocol::Udp
            ));
            assert_eq!(ports[0].host_port, 10053);
            assert_eq!(ports[0].container_port, 53);
        }
        _ => panic!("expected Container"),
    }
}

#[test]
fn test_create_service_execution_request_native_workload() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "native-svc".to_string(),
        version: "1".to_string(),
        image: None,
        command: Some(vec!["/usr/bin/myapp".to_string()]),
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: None,
            memory_bytes: None,
            storage_bytes: None,
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor
        .create_service_execution_request(&service, Uuid::new_v4())
        .unwrap();

    match &req.workload {
        crate::WorkloadSpec::Native {
            executable: crate::ExecutableSource::File { path },
            ..
        } => {
            // When image is None, uses /bin/sh as fallback
            assert_eq!(path.to_str().unwrap(), "/bin/sh");
        }
        _ => panic!("expected Native workload"),
    }
    assert_eq!(req.resources.cpu.min_cores, 1.0);
    assert_eq!(req.resources.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.resources.storage.min_bytes, 10 * 1024 * 1024 * 1024);
}

#[test]
fn test_create_service_execution_request_native_with_image_path() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "native".to_string(),
        version: "1".to_string(),
        image: Some("/usr/local/bin/custom".to_string()),
        command: Some(vec!["run".to_string()]),
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor
        .create_service_execution_request(&service, Uuid::new_v4())
        .unwrap();

    // With image Some, creates Container workload (image branch)
    match &req.workload {
        crate::WorkloadSpec::Container { image, .. } => {
            assert_eq!(image, "/usr/local/bin/custom");
        }
        _ => panic!("expected Container when image is Some"),
    }
}

#[test]
fn test_create_service_execution_request_resource_defaults() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "minimal".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: None,
            memory_bytes: None,
            storage_bytes: None,
            gpu_count: None,
        },
        ports: vec![],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor
        .create_service_execution_request(&service, Uuid::new_v4())
        .unwrap();

    assert_eq!(req.resources.cpu.min_cores, 1.0);
    assert_eq!(req.resources.cpu.max_cores, None);
    assert_eq!(req.resources.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.resources.storage.min_bytes, 10 * 1024 * 1024 * 1024);
    assert!(req.resources.gpu.is_none());
}

#[test]
fn test_create_service_execution_request_with_gpu() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "gpu-svc".to_string(),
        version: "1".to_string(),
        image: Some("cuda:11".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements {
            cpu_cores: Some(8.0),
            memory_bytes: Some(32 * 1024 * 1024 * 1024),
            storage_bytes: Some(100 * 1024 * 1024 * 1024),
            gpu_count: Some(2),
        },
        ports: vec![],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let req = executor
        .create_service_execution_request(&service, Uuid::new_v4())
        .unwrap();

    let gpu = req.resources.gpu.expect("gpu requirements");
    assert_eq!(gpu.min_units, 2);
    assert_eq!(gpu.max_units, Some(2));
}

// ─── create_deployment_network tests ─────────────────────────────────────

#[test]
fn test_create_deployment_network_structure() {
    let config = create_test_config(8080, vec![80, 443]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let mut services = HashMap::new();
    services.insert("svc1".to_string(), create_test_service_spec("svc1"));

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "team-a".to_string(),
        deployment_name: "test".to_string(),
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
            network_name: "net1".to_string(),
            subnet_cidr: "192.168.1.0/24".to_string(),
            dns_config: None,
            load_balancer: None,
        },
        created_at: Utc::now(),
    };

    let network = executor.create_deployment_network(&request);

    assert_eq!(
        network.network_name,
        format!("byob-team-a-{}", request.deployment_id)
    );
    assert_eq!(network.subnet_cidr, "192.168.1.0/24");
    assert_eq!(network.gateway_ip, "10.0.0.1");
    assert_eq!(network.service_endpoints.len(), 1);
    assert!(network.service_endpoints.contains_key("svc1"));

    let ep = network.service_endpoints.get("svc1").unwrap();
    assert_eq!(ep.name, "svc1");
    assert!(ep.internal_ip.starts_with("10.0.0."));
}

#[test]
fn test_create_deployment_network_multiple_services() {
    let config = create_test_config(8080, vec![9999]); // port 9999 not in web-service
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let mut services = HashMap::new();
    services.insert("a".to_string(), create_test_service_spec("a"));
    services.insert("b".to_string(), create_test_service_spec("b"));

    let request = ByobDeploymentRequest {
        deployment_id: Uuid::new_v4(),
        team_id: "t".to_string(),
        deployment_name: "d".to_string(),
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
        created_at: Utc::now(),
    };

    let network = executor.create_deployment_network(&request);
    assert_eq!(network.service_endpoints.len(), 2);
    let ips: Vec<_> = network
        .service_endpoints
        .values()
        .map(|e| e.internal_ip.as_str())
        .collect();
    // Internal IPs are 10.0.0.(10+N) for N=0,1
    assert_eq!(ips.len(), 2);
    assert!(ips.iter().all(|ip| ip.starts_with("10.0.0.")));
}

// ─── allocate_external_ip tests ──────────────────────────────────────────

#[test]
fn test_allocate_external_ip_no_web_ports() {
    let config = create_test_config(8080, vec![80, 443]); // only 80, 443 are web
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "internal".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![PortMapping {
            container_port: 5432, // Postgres, not in web_service_ports
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let ip = executor.allocate_external_ip(&service, "team1");
    assert!(ip.is_none());
}

#[test]
fn test_allocate_external_ip_web_port_allocates() {
    let config = create_test_config(8080, vec![80, 443, 8080]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "web".to_string(),
        version: "1".to_string(),
        image: Some("nginx:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![PortMapping {
            container_port: 80,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let ip = executor.allocate_external_ip(&service, "team1");
    let ip = ip.expect("should allocate for port 80");
    assert!(
        ip.starts_with("198.51.100.")
            || ip.starts_with("203.0.113.")
            || ip.starts_with("192.0.2.")
            || ip.starts_with("203.0.114.")
    );
    assert!(ip.split('.').next_back().unwrap().parse::<u32>().unwrap() >= 50);
}

#[test]
fn test_allocate_external_ip_deterministic_by_team_and_service() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);

    let service = ServiceSpec {
        name: "svc".to_string(),
        version: "1".to_string(),
        image: Some("img:1".to_string()),
        command: None,
        environment: HashMap::new(),
        resources: ServiceResourceRequirements::default(),
        ports: vec![PortMapping {
            container_port: 80,
            host_port: None,
            protocol: "tcp".to_string(),
        }],
        volumes: vec![],
        dependencies: Vec::new(),
        health_check: None,
        replicas: 1,
    };

    let ip1 = executor.allocate_external_ip(&service, "a"); // len 1 -> 1
    let ip2 = executor.allocate_external_ip(&service, "ab"); // len 2 -> 2
    let ip3 = executor.allocate_external_ip(&service, "abc"); // len 3 -> 3
    let ip4 = executor.allocate_external_ip(&service, "abcd"); // len 4 -> 0

    assert!(ip1.is_some());
    assert!(ip2.is_some());
    assert!(ip3.is_some());
    assert!(ip4.is_some());
    // Same service name + team gives same IP
    let ip1b = executor.allocate_external_ip(&service, "a");
    assert_eq!(ip1, ip1b);
}

// ─── perform_health_check tests ──────────────────────────────────────────

#[test]
fn test_perform_health_check_empty_command() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());
    let health = HealthCheck {
        command: vec![],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };
    let result = executor.perform_health_check("svc", &health).unwrap();
    assert!(result);
}

#[test]
fn test_perform_health_check_http_commands() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());

    for cmd in ["curl", "wget", "http"] {
        let health = HealthCheck {
            command: vec![
                cmd.to_string(),
                "-f".to_string(),
                "http://localhost/".to_string(),
            ],
            interval: 30,
            timeout: 5,
            retries: 3,
            start_period: 10,
        };
        let result = executor.perform_health_check("svc", &health).unwrap();
        assert!(result, "health check for {} should pass", cmd);
    }
}

#[test]
fn test_perform_health_check_ping() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());
    let health = HealthCheck {
        command: vec!["ping".to_string(), "-c".to_string(), "1".to_string()],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };
    let result = executor.perform_health_check("svc", &health).unwrap();
    assert!(result);
}

#[test]
fn test_perform_health_check_custom_command() {
    let executor =
        ByobComputeExecutor::new(create_test_runtime_engine(), ByobExecutorConfig::default());
    let health = HealthCheck {
        command: vec!["custom-script".to_string(), "arg1".to_string()],
        interval: 30,
        timeout: 5,
        retries: 3,
        start_period: 10,
    };
    let result = executor.perform_health_check("svc", &health).unwrap();
    assert!(result);
}

// ─── create_byob_executor factory ────────────────────────────────────────

#[test]
fn test_create_byob_executor_returns_arc_of_executor() {
    let engine = create_test_runtime_engine();
    let executor = create_byob_executor(engine);
    // Should be usable as ByobExecutor (trait object)
    assert!(std::sync::Arc::strong_count(&executor) >= 1);
}

// ─── DeploymentStatus enum variants ───────────────────────────────────────

#[test]
fn test_deployment_status_variants() {
    let starting = DeploymentStatus::Starting;
    let running = DeploymentStatus::Running;
    let stopping = DeploymentStatus::Stopping;
    let stopped = DeploymentStatus::Stopped;
    let failed = DeploymentStatus::Failed {
        error: "test error".to_string(),
    };

    assert!(matches!(starting, DeploymentStatus::Starting));
    assert!(matches!(running, DeploymentStatus::Running));
    assert!(matches!(stopping, DeploymentStatus::Stopping));
    assert!(matches!(stopped, DeploymentStatus::Stopped));
    assert!(matches!(
        failed,
        DeploymentStatus::Failed { error: ref e } if e == "test error"
    ));
}

#[test]
fn test_deployment_status_serialization() {
    let status = DeploymentStatus::Failed {
        error: "deployment failed".to_string(),
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("Failed"));
    assert!(json.contains("deployment failed"));

    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();
    match deserialized {
        DeploymentStatus::Failed { error } => assert_eq!(error, "deployment failed"),
        _ => panic!("expected Failed variant"),
    }
}

#[test]
fn test_deployment_status_running_serialization() {
    let status = DeploymentStatus::Running;
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("Running") || json == "\"Running\"");
    let deserialized: DeploymentStatus = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, DeploymentStatus::Running));
}

// ─── ResourceUsage and NetworkUsage ──────────────────────────────────────

#[test]
fn test_resource_usage_default_like() {
    let usage = ResourceUsage {
        cpu_usage: 1.5,
        memory_usage: 512 * 1024 * 1024,
        storage_usage: 10 * 1024 * 1024 * 1024,
        gpu_usage: 0,
        network_usage: NetworkUsage {
            bytes_sent: 1000,
            bytes_received: 500,
            packets_sent: 10,
            packets_received: 5,
        },
    };
    assert_eq!(usage.cpu_usage, 1.5);
    assert_eq!(usage.network_usage.bytes_sent, 1000);
}

#[test]
fn test_byob_executor_new_initializes_empty_deployments() {
    let engine = create_test_runtime_engine();
    let config = ByobExecutorConfig::default();
    let executor = ByobComputeExecutor::new(engine, config);
    // Can't easily assert on private fields, but deploy+list should work
    // We verify constructor doesn't panic and we can use it
    let request = create_test_deployment_request();
    assert!(executor.validate_deployment_request(&request).is_ok());
}

// ─── ByobExecutor trait integration tests ─────────────────────────────────

#[tokio::test]
async fn test_deploy_biome_success() {
    let engine = create_test_runtime_engine();
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(engine, config);
    let request = create_test_deployment_request();
    let response = executor.deploy_biome(request.clone()).await.unwrap();
    assert_eq!(response.deployment_id, request.deployment_id);
    assert!(matches!(response.status, DeploymentStatus::Running));
}

#[tokio::test]
async fn test_deploy_biome_validation_fails() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let mut bad_request = create_test_deployment_request();
    bad_request.services.clear();
    let result = executor.deploy_biome(bad_request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_deployment_status_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request.clone()).await.unwrap();
    let status = executor
        .get_deployment_status(request.deployment_id)
        .await
        .unwrap();
    assert_eq!(status.deployment_id, request.deployment_id);
}

#[tokio::test]
async fn test_get_deployment_status_not_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let result = executor.get_deployment_status(Uuid::new_v4()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_stop_deployment_not_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let result = executor.stop_deployment(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_stop_deployment_success() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request.clone()).await.unwrap();
    let result = executor.stop_deployment(request.deployment_id).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_deployments_empty() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let list = executor.list_deployments().await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn test_list_deployments_with_deployments() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request).await.unwrap();
    let list = executor.list_deployments().await.unwrap();
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn test_get_resource_usage_not_found() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, ByobExecutorConfig::default());
    let result = executor.get_resource_usage(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_resource_usage_after_deploy() {
    let engine = create_test_runtime_engine();
    let executor = ByobComputeExecutor::new(engine, create_test_config(8080, vec![80]));
    let request = create_test_deployment_request();
    executor.deploy_biome(request.clone()).await.unwrap();
    let usage = executor
        .get_resource_usage(request.deployment_id)
        .await
        .unwrap();
    assert!(usage.cpu_usage >= 0.0);
    assert!(usage.memory_usage >= 0);
}

#[tokio::test]
async fn test_deploy_biome_max_concurrent_limit() {
    let engine = create_test_runtime_engine();
    let config = ByobExecutorConfig {
        max_concurrent_deployments: 1,
        default_network_subnet: "10.0.0.0/24".to_string(),
        resource_monitoring_interval: Duration::from_secs(30),
        health_check_interval: Duration::from_secs(10),
        deployment_timeout: Duration::from_secs(600),
        default_host_port: 8080,
        web_service_ports: vec![80],
        graceful_shutdown_timeout_secs: 30,
    };
    let executor = ByobComputeExecutor::new(engine, config);
    let req1 = create_test_deployment_request();
    executor.deploy_biome(req1).await.unwrap();
    let mut req2 = create_test_deployment_request();
    req2.deployment_id = Uuid::new_v4();
    let result = executor.deploy_biome(req2).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("concurrent"));
}

#[tokio::test]
async fn test_network_info_service_endpoints() {
    let config = create_test_config(8080, vec![80]);
    let executor = ByobComputeExecutor::new(create_test_runtime_engine(), config);
    let request = create_test_deployment_request();
    let network = executor.create_deployment_network(&request);
    for (name, ep) in &network.service_endpoints {
        assert_eq!(ep.name, *name);
        assert!(ep.internal_ip.starts_with("10.0.0."));
        assert!(!ep.ports.is_empty());
    }
}

#[test]
fn test_service_endpoint_structure() {
    use super::super::byob_types::{PortMapping, ServiceEndpoint};
    let ep = ServiceEndpoint {
        name: "test-svc".to_string(),
        internal_ip: "10.0.0.10".to_string(),
        external_ip: Some("203.0.113.50".to_string()),
        ports: vec![PortMapping {
            container_port: 80,
            host_port: Some(8080),
            protocol: "tcp".to_string(),
        }],
    };
    assert_eq!(ep.name, "test-svc");
    assert_eq!(ep.internal_ip, "10.0.0.10");
    assert_eq!(ep.external_ip.as_deref(), Some("203.0.113.50"));
}
