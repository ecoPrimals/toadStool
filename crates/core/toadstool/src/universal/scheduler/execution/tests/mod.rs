// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for `universal::scheduler::execution` — split by execution backend.

mod native;
mod wasm;
mod primal;
mod biome_os;

use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;

use crate::execution::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeConfig,
    RuntimeEngine, RuntimeType,
};
use crate::resources::RuntimeMetrics;
use crate::workload::WorkloadType;
use crate::{ToadStoolError, ToadStoolResult};

use crate::universal::requests::{PrimalEndpoints, PrimalRequest, PrimalResponse, ResponseStatus};
use crate::universal::traits::UniversalPrimalProvider;
use crate::universal::types::{
    NetworkLocation, PrimalCapability, PrimalContext, PrimalHealth, PrimalType, SecurityLevel,
};

fn sample_context() -> PrimalContext {
    PrimalContext {
        user_id: "u1".to_string(),
        device_id: "d1".to_string(),
        session_id: "s1".to_string(),
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

struct MockRuntimeEngine;

impl RuntimeEngine for MockRuntimeEngine {
    fn initialize(
        &mut self,
        _config: RuntimeConfig,
    ) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn execute(
        &self,
        request: ExecutionRequest,
    ) -> impl std::future::Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_ {
        async move {
            let runtime_used = request.runtime_hint.unwrap_or(RuntimeType::Native);
            Ok(ExecutionResponse {
                execution_id: request.execution_id,
                status: ExecutionStatus::Success,
                output: ExecutionOutput::default(),
                metrics: RuntimeMetrics::default(),
                duration: Duration::from_millis(10),
                runtime_used,
                warnings: vec![],
            })
        }
    }

    fn get_capabilities(&self) -> crate::RuntimeCapabilities {
        crate::RuntimeCapabilities {
            supported_workloads: vec![WorkloadType::Native, WorkloadType::Wasm],
            max_concurrent_executions: Some(10),
            supported_architectures: vec!["x86_64".to_string()],
            platform_features: HashMap::new(),
            version: "1.0-test".to_string(),
        }
    }

    fn supports_workload(&self, _: &WorkloadType) -> bool {
        true
    }

    fn get_metrics(
        &self,
    ) -> impl std::future::Future<Output = ToadStoolResult<RuntimeMetrics>> + Send + '_ {
        async { Ok(RuntimeMetrics::default()) }
    }

    fn shutdown(&mut self) -> impl std::future::Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }
}

/// Native-capable primal; `handle_primal_request` builds response from template.
struct NativePrimalTemplate {
    instance_id: String,
    context: PrimalContext,
    status: ResponseStatus,
    payload: serde_json::Value,
    metadata: HashMap<String, String>,
}

impl UniversalPrimalProvider for NativePrimalTemplate {
    fn primal_id(&self) -> &str { &self.instance_id }
    fn instance_id(&self) -> &str { &self.instance_id }
    fn context(&self) -> &PrimalContext { &self.context }
    fn primal_type(&self) -> PrimalType { PrimalType::Compute }

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
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None, admin: None, events_endpoint: None, custom: HashMap::new(),
        }
    }

    fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        let status = self.status.clone();
        let payload = self.payload.clone();
        let metadata = self.metadata.clone();
        async move {
            Ok(PrimalResponse {
                request_id: request.id,
                status, payload, metadata,
                timestamp: std::time::SystemTime::now(),
            })
        }
    }

    fn initialize(&mut self, _config: serde_json::Value) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool { true }
}

/// Returns `Err` from `handle_primal_request` (native path uses `?`).
struct FailingNativePrimal {
    instance_id: String,
    context: PrimalContext,
}

impl UniversalPrimalProvider for FailingNativePrimal {
    fn primal_id(&self) -> &str { &self.instance_id }
    fn instance_id(&self) -> &str { &self.instance_id }
    fn context(&self) -> &PrimalContext { &self.context }
    fn primal_type(&self) -> PrimalType { PrimalType::Compute }

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
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None, admin: None, events_endpoint: None, custom: HashMap::new(),
        }
    }

    fn handle_primal_request(&self, _request: PrimalRequest) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        async { Err(ToadStoolError::execution("mock native primal failure")) }
    }

    fn initialize(&mut self, _config: serde_json::Value) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool { true }
}

/// Primal with fixed type for `execute_primal` routing.
struct TypedRoutePrimal {
    instance_id: String,
    context: PrimalContext,
    primal_type: PrimalType,
    status: ResponseStatus,
    fail_route: bool,
}

impl UniversalPrimalProvider for TypedRoutePrimal {
    fn primal_id(&self) -> &str { &self.instance_id }
    fn instance_id(&self) -> &str { &self.instance_id }
    fn context(&self) -> &PrimalContext { &self.context }
    fn primal_type(&self) -> PrimalType { self.primal_type.clone() }

    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::ServerlessExecution { languages: vec!["rust".to_string()] }]
    }

    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
    }

    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None, admin: None, events_endpoint: None, custom: HashMap::new(),
        }
    }

    fn handle_primal_request(&self, request: PrimalRequest) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        let fail_route = self.fail_route;
        let status = self.status.clone();
        async move {
            if fail_route {
                return Err(ToadStoolError::execution("route handler failure"));
            }
            Ok(PrimalResponse {
                request_id: request.id, status,
                payload: serde_json::json!({"ok": true}),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            })
        }
    }

    fn initialize(&mut self, _config: serde_json::Value) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool { true }
}

/// Provider without `NativeExecution` so `execute_native` falls through to local engine.
struct OnlyWasmPrimal {
    instance_id: String,
    context: PrimalContext,
}

impl UniversalPrimalProvider for OnlyWasmPrimal {
    fn primal_id(&self) -> &str { &self.instance_id }
    fn instance_id(&self) -> &str { &self.instance_id }
    fn context(&self) -> &PrimalContext { &self.context }
    fn primal_type(&self) -> PrimalType { PrimalType::Compute }

    fn capabilities(&self) -> Vec<PrimalCapability> {
        vec![PrimalCapability::WasmExecution { wasi_support: true }]
    }

    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
    }

    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None, admin: None, events_endpoint: None, custom: HashMap::new(),
        }
    }

    fn handle_primal_request(&self, _request: PrimalRequest) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        async { unreachable!("no native capability — scheduler should not route here for execute_native") }
    }

    fn initialize(&mut self, _config: serde_json::Value) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ { async { Ok(()) } }
    fn can_serve_context(&self, _context: &PrimalContext) -> bool { true }
}
