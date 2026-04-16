// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mock primal providers and shared context for execution path tests.

use std::collections::HashMap;
use std::future::Future;
use toadstool::universal::ResponseStatus;
use toadstool::universal::UniversalPrimalProvider;
use toadstool::universal::{
    NetworkLocation, PrimalCapability, PrimalContext, PrimalEndpoints, PrimalHealth, PrimalRequest,
    PrimalResponse, PrimalType, SecurityLevel,
};
use uuid::Uuid;

/// Mock provider that returns `ResponseStatus::Error` for testing error path
pub struct ErrorResponseMockProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

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
    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
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
    ) -> impl Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_ {
        async move {
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
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn shutdown(&mut self) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Mock provider that returns `ResponseStatus::Timeout`
pub struct TimeoutResponseMockProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

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
    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
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
    ) -> impl Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_ {
        async move {
            Ok(PrimalResponse {
                request_id: request.id,
                status: ResponseStatus::Timeout,
                payload: serde_json::json!({}),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            })
        }
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn shutdown(&mut self) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Mock provider that returns `ResponseStatus::ServiceUnavailable`
pub struct ServiceUnavailableMockProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

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
    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
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
    ) -> impl Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_ {
        async move {
            Ok(PrimalResponse {
                request_id: request.id,
                status: ResponseStatus::ServiceUnavailable,
                payload: serde_json::json!({}),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            })
        }
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn shutdown(&mut self) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Mock provider that returns `ResponseStatus::Success` with `stdout/stderr/exit_code`
pub struct SuccessWithOutputMockProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

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
    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
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
    ) -> impl Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_ {
        async move {
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
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn shutdown(&mut self) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// BiomeOS primal type
pub struct BiomeOSMockProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

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
    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
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
    fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> impl Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_ {
        async move {
            Ok(PrimalResponse {
                request_id: request.id,
                status: ResponseStatus::Success,
                payload: serde_json::json!({"result": "biomeos_ok"}),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            })
        }
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn shutdown(&mut self) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// BiomeOS provider that returns route error (Err)
pub struct BiomeOSErrorProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

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
    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
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
    fn handle_primal_request(
        &self,
        _request: PrimalRequest,
    ) -> impl Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_ {
        async { Err(toadstool::ToadStoolError::execution("biomeos route failed")) }
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn shutdown(&mut self) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Primal provider for type "compute" that returns route Err
pub struct PrimalRouteErrorProvider {
    pub instance_id: String,
    pub context: PrimalContext,
}

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
    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
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
    ) -> impl Future<Output = toadstool::ToadStoolResult<PrimalResponse>> + Send + '_ {
        async { Err(toadstool::ToadStoolError::execution("primal route failed")) }
    }
    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn shutdown(&mut self) -> impl Future<Output = toadstool::ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

pub fn test_ctx() -> PrimalContext {
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
