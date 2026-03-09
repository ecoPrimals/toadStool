// SPDX-License-Identifier: AGPL-3.0-only
//! Execution backend tests — Native, WASM, Primal, BiomeOS paths via `schedule_job`.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use toadstool::execution::{ExecutionStatus, RuntimeType};
use toadstool::resources::ResourceRequirements;
use toadstool::universal::ResponseStatus;
use toadstool::universal::UniversalPrimalProvider;
use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalCapability, PrimalContext, PrimalEndpoints, PrimalHealth,
    PrimalRequest, PrimalResponse, PrimalType, SecurityLevel, UniversalJob, UniversalJobType,
    UniversalPrimalRegistry, UniversalScheduler,
};
use uuid::Uuid;

use super::helpers::create_test_context;

/// Mock provider that returns `ResponseStatus::Error` for testing error path
struct ErrorResponseMockProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for ErrorResponseMockProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "unix:///tmp/toadstool.sock".to_string(),
            health: "unix:///tmp/toadstool.sock".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Error {
                code: "E001".to_string(),
                message: "mock error".to_string(),
            },
            payload: serde_json::json!({}),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        })
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Mock provider that returns `ResponseStatus::Timeout`
struct TimeoutResponseMockProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for TimeoutResponseMockProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "unix:///tmp/toadstool.sock".to_string(),
            health: "unix:///tmp/toadstool.sock".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Timeout,
            payload: serde_json::json!({}),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        })
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Mock provider that returns `ResponseStatus::ServiceUnavailable`
struct ServiceUnavailableMockProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for ServiceUnavailableMockProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "unix:///tmp/toadstool.sock".to_string(),
            health: "unix:///tmp/toadstool.sock".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::ServiceUnavailable,
            payload: serde_json::json!({}),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        })
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Mock provider that returns `ResponseStatus::Success` with `stdout/stderr/exit_code`
struct SuccessWithOutputMockProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for SuccessWithOutputMockProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string(), "aarch64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "unix:///tmp/toadstool.sock".to_string(),
            health: "unix:///tmp/toadstool.sock".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Success,
            payload: serde_json::json!({
                "stdout": "primal output",
                "stderr": "primal stderr",
                "exit_code": 0
            }),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        })
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// BiomeOS primal type
struct BiomeOSMockProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for BiomeOSMockProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::OS
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "unix:///tmp/biomeos.sock".to_string(),
            health: "unix:///tmp/biomeos.sock".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Ok(PrimalResponse {
            request_id: request.id,
            status: ResponseStatus::Success,
            payload: serde_json::json!({"result": "biomeos_ok"}),
            metadata: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        })
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// BiomeOS provider that returns route error (Err)
struct BiomeOSErrorProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for BiomeOSErrorProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::OS
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "unix:///tmp/biomeos.sock".to_string(),
            health: "unix:///tmp/biomeos.sock".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        _request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Err(toadstool::ToadStoolError::execution("biomeos route failed"))
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Primal provider for type "compute" that returns route Err
struct PrimalRouteErrorProvider {
    instance_id: String,
    context: PrimalContext,
}

#[async_trait]
impl UniversalPrimalProvider for PrimalRouteErrorProvider {
    fn primal_id(&self) -> &str {
        &self.instance_id
    }
    fn instance_id(&self) -> &str {
        &self.instance_id
    }
    fn context(&self) -> &PrimalContext {
        &self.context
    }
    fn primal_type(&self) -> PrimalType {
        PrimalType::Compute
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }]
    }
    async fn health_check(&self) -> PrimalHealth {
        PrimalHealth::Healthy
    }
    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "unix:///tmp/toadstool.sock".to_string(),
            health: "unix:///tmp/toadstool.sock".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }
    async fn handle_primal_request(
        &self,
        _request: PrimalRequest,
    ) -> toadstool::ToadStoolResult<PrimalResponse> {
        Err(toadstool::ToadStoolError::execution("primal route failed"))
    }
    async fn initialize(&mut self, _config: serde_json::Value) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    async fn shutdown(&mut self) -> toadstool::ToadStoolResult<()> {
        Ok(())
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

fn test_ctx() -> PrimalContext {
    PrimalContext {
        user_id: "test".to_string(),
        device_id: "test-device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: None,
            network_id: None,
            geo_location: None,
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(SuccessWithOutputMockProvider {
        instance_id: "native-provider-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.runtime_used, RuntimeType::Native);
    assert!(matches!(response.status, ExecutionStatus::Success));
    assert_eq!(response.output.stdout.as_deref(), Some("primal output"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider_error_response() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(ErrorResponseMockProvider {
        instance_id: "error-provider".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["x".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: _ }
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider_timeout_response() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(TimeoutResponseMockProvider {
        instance_id: "timeout-provider".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["x".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.status, ExecutionStatus::TimedOut);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_via_primal_provider_service_unavailable() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(ServiceUnavailableMockProvider {
        instance_id: "unavail-provider".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["x".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("Service unavailable")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_spawn_failure() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/nonexistent/executable/that/does/not/exist".to_string(),
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("Failed to spawn")
    ));
    assert_eq!(response.output.exit_code, Some(127));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_wasm_no_engine_returns_failed() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Wasm {
            module: vec![0x00, 0x61, 0x73, 0x6d],
            args: vec![],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("No WASM execution capability")
    ));
    assert_eq!(response.runtime_used, RuntimeType::Wasm);
    assert!(!response.warnings.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_with_provider_success() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(SuccessWithOutputMockProvider {
        instance_id: "compute-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "compute".to_string(),
            endpoint: "execute".to_string(),
            payload: serde_json::json!({"task": "test"}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, ExecutionStatus::Success));
    assert!(
        response
            .output
            .stdout
            .as_ref()
            .is_some_and(|s| s.to_lowercase().contains("primal")),
        "stdout: {:?}",
        response.output.stdout
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_route_failure() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(PrimalRouteErrorProvider {
        instance_id: "compute-err".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "compute".to_string(),
            endpoint: "execute".to_string(),
            payload: serde_json::json!({}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("execution failed")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_no_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "nonexistent".to_string(),
            endpoint: "run".to_string(),
            payload: serde_json::json!({}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("No primal provider")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_primal_no_provider_with_available_list() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(SuccessWithOutputMockProvider {
        instance_id: "other-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Primal {
            primal_type: "different-type".to_string(),
            endpoint: "run".to_string(),
            payload: serde_json::json!({}),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("No primal provider")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_biome_os_with_provider_success() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(BiomeOSMockProvider {
        instance_id: "biomeos-1".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({"version": "1"}),
            team_id: "team-42".to_string(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(response.status, ExecutionStatus::Success));
    assert!(response
        .output
        .stdout
        .as_ref()
        .is_some_and(|s| s.contains("BiomeOS")));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_biome_os_route_failure() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let provider = Arc::new(BiomeOSErrorProvider {
        instance_id: "biomeos-err".to_string(),
        context: test_ctx(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({}),
            team_id: "team-1".to_string(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("execution failed")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_biome_os_no_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::BiomeOS {
            biome_manifest: serde_json::json!({}),
            team_id: "team-1".to_string(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("BiomeOS integration not available")
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_native_process_failure_exit_code() {
    let registry = Arc::new(UniversalPrimalRegistry::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let job = UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/sh".to_string(),
            args: vec!["-c".to_string(), "exit 42".to_string()],
            env: HashMap::new(),
        },
        priority: JobPriority::Normal,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    };

    let result = scheduler.schedule_job(job).await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(matches!(
        response.status,
        ExecutionStatus::Failed { error: ref e } if e.contains("42")
    ));
}

#[test]
fn test_discover_self_ip_via_env_toadstool_bind_address() {
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("192.168.1.1:8080"), || {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = runtime.block_on(async {
            let registry = Arc::new(UniversalPrimalRegistry::new());
            let provider = Arc::new(SuccessWithOutputMockProvider {
                instance_id: "p1".to_string(),
                context: test_ctx(),
            });
            registry.register_primal(provider).await.unwrap();
            let scheduler = UniversalScheduler::new(registry).await.unwrap();
            let job = UniversalJob {
                id: Uuid::new_v4(),
                job_type: UniversalJobType::Native {
                    executable: "/bin/echo".to_string(),
                    args: vec!["x".to_string()],
                    env: HashMap::new(),
                },
                priority: JobPriority::Normal,
                resources: ResourceRequirements::default(),
                timeout: Some(Duration::from_secs(30)),
                created_at: std::time::SystemTime::now(),
                context: create_test_context(),
            };
            scheduler.schedule_job(job).await
        });
        assert!(result.is_ok());
    });
}
