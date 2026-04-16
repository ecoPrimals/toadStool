// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared mock providers and factory functions for universal scheduler tests.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use toadstool::resources::{
    CpuRequirements, MemoryRequirements, NetworkRequirements, ResourceRequirements,
    StorageRequirements,
};
use toadstool::universal::ResponseStatus;
use toadstool::universal::UniversalPrimalProvider;
use toadstool::universal::{
    JobPriority, NetworkLocation, PrimalCapability, PrimalContext, SecurityLevel, UniversalJob,
    UniversalJobType,
};
use toadstool::universal::{
    PrimalEndpoints, PrimalHealth, PrimalRequest, PrimalResponse, PrimalType,
};
use uuid::Uuid;

/// Succeeding mock provider for testing happy-path primal scheduling.
pub struct SucceedingMockProvider {
    pub instance_id: String,
    pub context: PrimalContext,
    pub primal_type: PrimalType,
}

impl UniversalPrimalProvider for SucceedingMockProvider {
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
        self.primal_type.clone()
    }
    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::NativeExecution {
            architectures: vec!["x86_64".to_string()],
        }]
    }
    fn health_check(&self) -> Pin<Box<dyn Future<Output = PrimalHealth> + Send + '_>> {
        Box::pin(async { PrimalHealth::Healthy })
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
    fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_>> {
        let primal_type = self.primal_type.clone();
        Box::pin(async move {
            Ok(PrimalResponse {
                request_id: request.id,
                status: ResponseStatus::Success,
                payload: serde_json::json!({
                    "stdout": format!("Primal '{}' executed successfully", primal_type.as_str()),
                    "exit_code": 0
                }),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            })
        })
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
    fn shutdown(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Minimal mock provider that always fails requests (for error path testing).
pub struct FailingMockProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

impl UniversalPrimalProvider for FailingMockProvider {
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
    fn health_check(&self) -> Pin<Box<dyn Future<Output = PrimalHealth> + Send + '_>> {
        Box::pin(async { PrimalHealth::Healthy })
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
    fn handle_primal_request(
        &self,
        _request: PrimalRequest,
    ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_>> {
        Box::pin(async {
            Err(toadstool::ToadStoolError::execution(
                "mock provider failure",
            ))
        })
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
    fn shutdown(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = toadstool::ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Create a standard test context with no external dependencies.
pub fn make_test_context() -> PrimalContext {
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

/// Create a richer test context for routing tests.
pub fn create_test_context() -> PrimalContext {
    PrimalContext {
        user_id: "test-user".to_string(),
        device_id: "test-device".to_string(),
        session_id: Uuid::new_v4().to_string(),
        network_location: NetworkLocation {
            ip_address: "127.0.0.1".to_string(),
            subnet: Some("192.168.1.0/24".to_string()),
            network_id: Some("test-network".to_string()),
            geo_location: Some("US-East".to_string()),
        },
        security_level: SecurityLevel::Standard,
        metadata: HashMap::new(),
    }
}

/// Create a native job with specified priority for schedule-focused tests.
pub fn create_test_native_job(priority: JobPriority) -> UniversalJob {
    UniversalJob {
        id: Uuid::new_v4(),
        job_type: UniversalJobType::Native {
            executable: "/bin/echo".to_string(),
            args: vec!["test".to_string()],
            env: HashMap::new(),
        },
        priority,
        resources: ResourceRequirements::default(),
        timeout: Some(Duration::from_secs(30)),
        created_at: std::time::SystemTime::now(),
        context: create_test_context(),
    }
}

/// Create an explicit resource specification for resource allocation tests.
pub fn create_resource_spec(
    min_cpu: f64,
    max_cpu: Option<f64>,
    min_mem_gb: u64,
    max_mem_gb: Option<u64>,
) -> ResourceRequirements {
    ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: min_cpu,
            max_cores: max_cpu,
            architecture: Some("x86_64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: min_mem_gb * 1024 * 1024 * 1024,
            max_bytes: max_mem_gb.map(|g| g * 1024 * 1024 * 1024),
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    }
}
