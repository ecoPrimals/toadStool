// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit tests for `universal::scheduler::execution` (`execute_*` paths).

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use crate::execution::{
    ExecutionOutput, ExecutionRequest, ExecutionResponse, ExecutionStatus, RuntimeConfig,
    RuntimeEngine, RuntimeType,
};
use crate::resources::RuntimeMetrics;
use crate::workload::WorkloadType;
use crate::{ToadStoolError, ToadStoolResult};

use crate::universal::UniversalScheduler;
use crate::universal::primal_provider_dispatch::UniversalPrimalProviderDispatch;
use crate::universal::registry::UniversalPrimalRegistry;
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

/// Mock runtime engine (same behavior as `scheduler::tests::SimpleMockRuntimeEngine`).
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
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
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
                status,
                payload,
                metadata,
                timestamp: std::time::SystemTime::now(),
            })
        }
    }

    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

/// Returns `Err` from `handle_primal_request` (native path uses `?`).
struct FailingNativePrimal {
    instance_id: String,
    context: PrimalContext,
}

impl UniversalPrimalProvider for FailingNativePrimal {
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
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }

    fn handle_primal_request(
        &self,
        _request: PrimalRequest,
    ) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        async { Err(ToadStoolError::execution("mock native primal failure")) }
    }

    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
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
        vec![PrimalCapability::ServerlessExecution {
            languages: vec!["rust".to_string()],
        }]
    }

    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
    }

    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }

    fn handle_primal_request(
        &self,
        request: PrimalRequest,
    ) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        let fail_route = self.fail_route;
        let status = self.status.clone();
        async move {
            if fail_route {
                return Err(ToadStoolError::execution("route handler failure"));
            }
            Ok(PrimalResponse {
                request_id: request.id,
                status,
                payload: serde_json::json!({"ok": true}),
                metadata: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            })
        }
    }

    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

#[tokio::test]
async fn execute_native_via_primal_success_with_stdout_stderr_exit() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-1".to_string(),
        context: ctx,
        status: ResponseStatus::Success,
        payload: serde_json::json!({
            "stdout": "out-line",
            "stderr": "err-line",
            "exit_code": 7
        }),
        metadata: HashMap::from([("k".to_string(), "v".to_string())]),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("echo", &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert_eq!(out.output.stdout.as_deref(), Some("out-line"));
    assert_eq!(out.output.stderr.as_deref(), Some("err-line"));
    assert_eq!(out.output.exit_code, Some(7));
    assert_eq!(out.output.metadata.get("k"), Some(&"v".to_string()));
    assert_eq!(out.runtime_used, RuntimeType::Native);
}

#[tokio::test]
async fn execute_native_via_primal_error_status() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-err".to_string(),
        context: ctx,
        status: ResponseStatus::Error {
            code: "E1".to_string(),
            message: "boom".to_string(),
        },
        payload: serde_json::json!({}),
        metadata: HashMap::new(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("/bin/true", &[], &HashMap::new())
        .await
        .unwrap();
    assert!(matches!(
        out.status,
        ExecutionStatus::Failed { ref error } if error == "boom"
    ));
}

#[tokio::test]
async fn execute_native_via_primal_timeout_status() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-to".to_string(),
        context: ctx,
        status: ResponseStatus::Timeout,
        payload: serde_json::json!({}),
        metadata: HashMap::new(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("/bin/true", &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::TimedOut);
}

#[tokio::test]
async fn execute_native_via_primal_service_unavailable_status() {
    let registry = Arc::new(UniversalPrimalRegistry::<NativePrimalTemplate>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(NativePrimalTemplate {
        instance_id: "native-su".to_string(),
        context: ctx,
        status: ResponseStatus::ServiceUnavailable,
        payload: serde_json::json!({}),
        metadata: HashMap::new(),
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("/bin/true", &[], &HashMap::new())
        .await
        .unwrap();
    assert!(matches!(
        out.status,
        ExecutionStatus::Failed { ref error } if error == "Service unavailable"
    ));
}

#[tokio::test]
async fn execute_native_primal_handler_error_propagates() {
    let registry = Arc::new(UniversalPrimalRegistry::<FailingNativePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(FailingNativePrimal {
        instance_id: "fail-native".to_string(),
        context: ctx,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let err = scheduler
        .execute_native("echo", &[], &HashMap::new())
        .await
        .unwrap_err();
    assert!(
        err.to_string().contains("mock native primal failure"),
        "unexpected: {err}"
    );
}

/// Provider without `NativeExecution` so `execute_native` falls through to the local engine.
struct OnlyWasmPrimal {
    instance_id: String,
    context: PrimalContext,
}

impl UniversalPrimalProvider for OnlyWasmPrimal {
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
        vec![PrimalCapability::WasmExecution { wasi_support: true }]
    }

    fn health_check(&self) -> impl Future<Output = PrimalHealth> + Send + '_ {
        async { PrimalHealth::Healthy }
    }

    fn endpoints(&self) -> PrimalEndpoints {
        PrimalEndpoints {
            primary: "http://localhost".to_string(),
            health: "http://localhost/health".to_string(),
            metrics: None,
            admin: None,
            events_endpoint: None,
            custom: HashMap::new(),
        }
    }

    fn handle_primal_request(
        &self,
        _request: PrimalRequest,
    ) -> impl Future<Output = ToadStoolResult<PrimalResponse>> + Send + '_ {
        async {
            unreachable!(
                "no native capability — scheduler should not route here for execute_native"
            )
        }
    }

    fn initialize(
        &mut self,
        _config: serde_json::Value,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn shutdown(&mut self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    fn can_serve_context(&self, _context: &PrimalContext) -> bool {
        true
    }
}

#[tokio::test]
async fn execute_native_uses_local_engine_when_no_native_primal() {
    let registry = Arc::new(UniversalPrimalRegistry::<OnlyWasmPrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(OnlyWasmPrimal {
        instance_id: "wasm-only".to_string(),
        context: ctx,
    });
    registry.register_primal(provider).await.unwrap();

    let mut engines = HashMap::new();
    engines.insert(RuntimeType::Native, Arc::new(MockRuntimeEngine));
    let scheduler =
        UniversalScheduler::<OnlyWasmPrimal, MockRuntimeEngine>::create_with_runtime_engines(
            registry, engines,
        )
        .await
        .unwrap();

    let out = scheduler
        .execute_native("ignored", &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert_eq!(out.runtime_used, RuntimeType::Native);
}

#[tokio::test]
async fn execute_wasm_no_engine_failed_response_and_warning() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_wasm(&[0, 97, 115, 109], &[], &HashMap::new())
        .await
        .unwrap();
    assert!(matches!(
        out.status,
        ExecutionStatus::Failed { ref error } if error.contains("No WASM execution capability")
    ));
    assert_eq!(out.output.exit_code, Some(126));
    assert!(
        out.warnings
            .iter()
            .any(|w| w.contains("register_runtime_engine")),
        "warnings: {:?}",
        out.warnings
    );
}

#[tokio::test]
async fn execute_wasm_with_registered_engine() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let mut engines = HashMap::new();
    engines.insert(RuntimeType::Wasm, Arc::new(MockRuntimeEngine));
    let scheduler = UniversalScheduler::<
        UniversalPrimalProviderDispatch,
        MockRuntimeEngine,
    >::create_with_runtime_engines(registry, engines)
        .await
        .unwrap();
    let out = scheduler
        .execute_wasm(&[0, 97, 115, 109], &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert_eq!(out.runtime_used, RuntimeType::Wasm);
}

#[tokio::test]
async fn execute_primal_success() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "worker-1".to_string(),
        context: ctx,
        primal_type: PrimalType::Custom("worker".to_string()),
        status: ResponseStatus::Success,
        fail_route: false,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_primal("worker", "run", &serde_json::json!({"x": 1}))
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert!(
        out.output
            .stdout
            .as_ref()
            .is_some_and(|s| s.contains("worker")),
        "stdout: {:?}",
        out.output.stdout
    );
}

#[tokio::test]
async fn execute_primal_response_error_timeout_unavailable() {
    for (status, needle) in [
        (
            ResponseStatus::Error {
                code: "E".to_string(),
                message: "e-msg".to_string(),
            },
            "e-msg",
        ),
        (ResponseStatus::Timeout, "timed out"),
        (ResponseStatus::ServiceUnavailable, "unavailable"),
    ] {
        let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
        let ctx = sample_context();
        let provider = Arc::new(TypedRoutePrimal {
            instance_id: format!("inst-{needle}"),
            context: ctx,
            primal_type: PrimalType::Custom("alpha".to_string()),
            status,
            fail_route: false,
        });
        registry.register_primal(provider).await.unwrap();

        let scheduler = UniversalScheduler::new(registry).await.unwrap();
        let out = scheduler
            .execute_primal("alpha", "go", &serde_json::json!({}))
            .await
            .unwrap();
        assert!(
            matches!(out.status, ExecutionStatus::Failed { .. }),
            "expected failed for {needle}: {:?}",
            out.status
        );
        let ExecutionStatus::Failed { error } = out.status else {
            unreachable!();
        };
        assert!(
            error.contains(needle),
            "error={error} expected needle={needle}"
        );
    }
}

#[tokio::test]
async fn execute_primal_route_handler_error_returns_failed_response() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "bad-route".to_string(),
        context: ctx,
        primal_type: PrimalType::Custom("beta".to_string()),
        status: ResponseStatus::Success,
        fail_route: true,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_primal("beta", "x", &serde_json::json!({}))
        .await
        .unwrap();
    assert!(matches!(out.status, ExecutionStatus::Failed { .. }));
    assert!(
        out.output
            .stderr
            .as_ref()
            .is_some_and(|s| s.contains("route handler failure")),
        "stderr: {:?}",
        out.output.stderr
    );
}

#[tokio::test]
async fn execute_primal_no_provider_lists_available_when_present() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "only-compute".to_string(),
        context: ctx,
        primal_type: PrimalType::Compute,
        status: ResponseStatus::Success,
        fail_route: false,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_primal("storage", "x", &serde_json::json!({}))
        .await
        .unwrap();
    let ExecutionStatus::Failed { error } = out.status else {
        panic!("expected failed");
    };
    assert!(
        error.contains("compute"),
        "error should list providers: {error}"
    );
}

#[tokio::test]
async fn execute_biome_os_success() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "biome-1".to_string(),
        context: ctx,
        primal_type: PrimalType::OS,
        status: ResponseStatus::Success,
        fail_route: false,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_biome_os(&serde_json::json!({"v": 1}), "team-a")
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert!(
        out.output
            .stdout
            .as_ref()
            .is_some_and(|s| s.contains("team-a")),
        "stdout: {:?}",
        out.output.stdout
    );
}

#[tokio::test]
async fn execute_biome_os_non_success_statuses() {
    for status in [
        ResponseStatus::Error {
            code: "E".to_string(),
            message: "bio-err".to_string(),
        },
        ResponseStatus::Timeout,
        ResponseStatus::ServiceUnavailable,
    ] {
        let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
        let ctx = sample_context();
        let provider = Arc::new(TypedRoutePrimal {
            instance_id: "biome-os-x".to_string(),
            context: ctx,
            primal_type: PrimalType::OS,
            status,
            fail_route: false,
        });
        registry.register_primal(provider).await.unwrap();

        let scheduler = UniversalScheduler::new(registry).await.unwrap();
        let out = scheduler
            .execute_biome_os(&serde_json::json!({}), "t1")
            .await
            .unwrap();
        assert!(matches!(out.status, ExecutionStatus::Failed { .. }));
    }
}

#[tokio::test]
async fn execute_biome_os_route_error() {
    let registry = Arc::new(UniversalPrimalRegistry::<TypedRoutePrimal>::new_typed());
    let ctx = sample_context();
    let provider = Arc::new(TypedRoutePrimal {
        instance_id: "biome-bad".to_string(),
        context: ctx,
        primal_type: PrimalType::OS,
        status: ResponseStatus::Success,
        fail_route: true,
    });
    registry.register_primal(provider).await.unwrap();

    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_biome_os(&serde_json::json!({}), "team-z")
        .await
        .unwrap();
    assert!(
        out.output
            .stderr
            .as_ref()
            .is_some_and(|s| s.contains("BiomeOS"))
    );
}

#[tokio::test]
async fn execute_biome_os_no_os_provider() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_biome_os(&serde_json::json!({}), "solo")
        .await
        .unwrap();
    assert!(matches!(out.status, ExecutionStatus::Failed { .. }));
    assert!(
        out.warnings.iter().any(|w| w.contains("BiomeOS primal")),
        "warnings: {:?}",
        out.warnings
    );
}

#[tokio::test]
async fn execute_native_direct_process_stderr_none_when_empty() {
    let registry = Arc::new(UniversalPrimalRegistry::<UniversalPrimalProviderDispatch>::new());
    let scheduler = UniversalScheduler::new(registry).await.unwrap();
    let out = scheduler
        .execute_native("true", &[], &HashMap::new())
        .await
        .unwrap();
    assert_eq!(out.status, ExecutionStatus::Success);
    assert!(
        out.output.stderr.is_none(),
        "empty stderr should be None: {:?}",
        out.output.stderr
    );
}
