// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC request handler and method router
//!
//! Routes JSON-RPC 2.0 requests to the appropriate executor or job queue.
//! Semantic method names are resolved through `SemanticMethodRegistry`
//! before dispatch, enabling both legacy `toadstool.*` names and the
//! standard `{domain}.{operation}` naming convention.

mod core;
mod dispatch;
mod hw_learn;
mod job;
mod resources;
mod silicon;
mod transport;
mod workload;

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use toadstool::semantic_methods::SemanticMethodRegistry;
use tracing::{debug, error, info};

use super::types::{JSONRPC_VERSION, JsonRpcError, JsonRpcRequest, JsonRpcResponse};

use dispatch::DispatchHandler;
pub use hw_learn::HwLearnHandler;
use job::JobHandler;
use resources::ResourceHandler;
use silicon::SiliconHandler;
use transport::TransportHandler;
use workload::WorkloadHandler;

use crate::glowplug_client::{self, SharedGlowPlugClient};

/// Pure Rust JSON-RPC Handler
///
/// Thin coordinator that delegates to specialized handlers.
/// Routes requests to appropriate methods. Supports both legacy `toadstool.*`
/// names and semantic `{domain}.{operation}` names via the registry.
pub struct JsonRpcHandler {
    version: Arc<str>,
    start_time: std::time::Instant,
    error_count: Arc<AtomicU64>,
    semantic_registry: SemanticMethodRegistry,
    dispatch: DispatchHandler,
    hw_learn: HwLearnHandler,
    job: JobHandler,
    workload: WorkloadHandler,
    resources: ResourceHandler,
    transport: TransportHandler,
    silicon: SiliconHandler,
    glowplug: SharedGlowPlugClient,
}

impl JsonRpcHandler {
    /// Create new handler with executor.
    ///
    /// Pass `error_count` to share the counter with other servers for unified monitoring.
    pub fn new(
        executor: Arc<dyn crate::tarpc_server::WorkloadExecutor + Send + Sync>,
        version: impl Into<Arc<str>>,
        error_count: Option<Arc<AtomicU64>>,
    ) -> Self {
        let local_gate_id = std::env::var("TOADSTOOL_GATE_ID")
            .or_else(|_| std::env::var("HOSTNAME"))
            .or_else(|_| toadstool_sysmon::system::hostname().ok_or(std::env::VarError::NotPresent))
            .unwrap_or_else(|_| String::from("local"));
        Self {
            version: version.into(),
            start_time: std::time::Instant::now(),
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
            semantic_registry: SemanticMethodRegistry::new(),
            dispatch: DispatchHandler::new(
                crate::visualization_client::create_visualization_client(),
            ),
            hw_learn: HwLearnHandler::new(),
            job: JobHandler::new(local_gate_id),
            workload: WorkloadHandler::new(executor),
            resources: ResourceHandler::new(),
            transport: TransportHandler::new(),
            silicon: SiliconHandler::new(),
            glowplug: glowplug_client::create_glowplug_client(),
        }
    }

    /// Handle a JSON-RPC request (main entry point).
    ///
    /// Pattern: parse → validate → resolve → route → execute → respond
    pub async fn handle_request(&self, request: &JsonRpcRequest<'_>) -> JsonRpcResponse {
        if request.jsonrpc != JSONRPC_VERSION {
            self.error_count.fetch_add(1, Ordering::Relaxed);
            return JsonRpcResponse {
                jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                result: None,
                error: Some(JsonRpcError::invalid_request(
                    "Invalid JSON-RPC version (must be '2.0')",
                )),
                id: request.id.clone().unwrap_or(serde_json::Value::Null),
            };
        }

        let id = request.id.clone().unwrap_or(serde_json::Value::Null);

        info!(method = %request.method.as_ref(), "JSON-RPC request");

        match self
            .handle_method(request.method.as_ref(), request.params.as_ref())
            .await
        {
            Ok(result) => JsonRpcResponse {
                jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                result: Some(result),
                error: None,
                id,
            },
            Err(err) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                error!(
                    method = %request.method.as_ref(),
                    error = %err.message,
                    "JSON-RPC error",
                );
                JsonRpcResponse {
                    jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                    result: None,
                    error: Some(err),
                    id,
                }
            }
        }
    }

    /// Route a method name to its handler.
    ///
    /// Resolution order:
    /// 1. Direct literal match (backward-compatible `toadstool.*` and `compute.*` names).
    /// 2. Semantic registry lookup: `{domain}.{operation}` → implementation name → handler.
    async fn handle_method(
        &self,
        method: &str,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match method {
            "toadstool.submit_workload" => return self.workload.submit_workload(params).await,
            "toadstool.query_status" => return self.job.query_status(params).await,
            "toadstool.cancel_workload" => return self.workload.cancel_workload(params).await,
            "toadstool.list_workloads" => return self.job.list_workloads(params).await,
            "toadstool.query_capabilities" => return self.workload.query_capabilities().await,
            // Wire Standard L1/L2: triad shapes differ; full payload only for check + legacy name.
            "toadstool.health" | "health.check" => {
                return core::health(&self.version, self.start_time, &self.error_count).await;
            }
            "health.liveness" => return core::health_liveness().await,
            "health.readiness" => return core::health_readiness(self.version.as_ref()).await,
            "identity.get" => {
                return core::identity_get(&self.version, &self.semantic_registry).await;
            }
            "toadstool.version" => return core::version_info(&self.version).await,

            "toadstool.resources.estimate"
            | "toadstool.ai.local_inference"
            | "resources.estimate"
            | "ai.local_inference" => return self.resources.resources_estimate(params).await,
            "toadstool.resources.validate_availability"
            | "toadstool.ai.local_execute"
            | "resources.validate_availability"
            | "ai.local_execute" => {
                return self.resources.resources_validate_availability(params).await;
            }
            "toadstool.resources.suggest_optimizations" | "resources.suggest_optimizations" => {
                return self.resources.resources_suggest_optimizations(params).await;
            }

            "compute.health" => {
                return core::health(&self.version, self.start_time, &self.error_count).await;
            }
            "compute.version" => return core::version_info(&self.version).await,
            "capabilities.list" | "capability.list" | "primal.capabilities" => {
                return core::capabilities_list(&self.semantic_registry, &self.version).await;
            }
            "compute.capabilities" => return self.workload.query_capabilities().await,
            "compute.discover_capabilities" => {
                return core::discover_capabilities(&self.semantic_registry, &self.version).await;
            }

            "compute.submit" => return self.job.compute_submit(params).await,
            "compute.status" => return self.job.compute_status(params).await,
            "compute.result" => return self.job.compute_result(params).await,
            "compute.cancel" => return self.job.compute_cancel(params).await,
            "compute.list" => return self.job.compute_list(params).await,

            "compute.dispatch.submit" => return self.dispatch.dispatch_submit(params).await,
            "compute.dispatch.status" => return self.dispatch.dispatch_status(params).await,
            "compute.dispatch.result" => return self.dispatch.dispatch_result(params).await,
            "compute.dispatch.forward" => return self.dispatch.dispatch_forward(params).await,
            "compute.dispatch.capabilities" => {
                return self.dispatch.dispatch_capabilities(params).await;
            }

            "gpu.query_info" | "gpu.info" => return core::gpu_info().await,
            "gpu.query_memory" | "gpu.memory" => return core::gpu_memory().await,
            "gpu.query_telemetry" | "gpu.telemetry" => {
                return self.hw_learn.gpu_telemetry(params).await;
            }

            "gate.update" => return self.job.gate_update(params).await,
            "gate.remove" => return self.job.gate_remove(params).await,
            "gate.list" => return self.job.gate_list().await,
            "gate.route" => return self.job.gate_route(params).await,

            "transport.discover" => return Ok(TransportHandler::transport_discover(params)),
            "transport.list" => return self.transport.transport_list().await,
            "transport.route" => return self.transport.transport_route(params).await,
            "transport.open" => return self.transport.transport_open(params).await,
            "transport.stream" => return self.transport.transport_stream(params).await,
            "transport.status" => return self.transport.transport_status(params).await,

            // Hardware learning domain — biomeOS v2.30 compute.hardware.* capabilities
            "compute.hardware.observe" => return self.hw_learn.hw_learn_observe(params).await,
            "compute.hardware.distill" => return self.hw_learn.hw_learn_distill(params).await,
            "compute.hardware.apply" => return self.hw_learn.hw_learn_apply(params).await,
            "compute.hardware.share_recipe" => {
                return self.hw_learn.hw_learn_share_recipe(params).await;
            }
            "compute.hardware.auto_init" => return self.hw_learn.hw_learn_auto_init(params).await,
            "compute.hardware.auto_init_all" => {
                return self.hw_learn.hw_learn_auto_init_all(params).await;
            }
            "compute.hardware.status" => return self.hw_learn.hw_learn_status(params).await,
            "compute.hardware.vfio_devices" => {
                return self.hw_learn.hw_learn_vfio_devices(params).await;
            }

            "shader.dispatch" => return self.dispatch.shader_dispatch(params).await,

            "ember.list" => return Ok(self.ember_list()),
            "ember.status" => return Ok(self.ember_status()),

            "compute.performance_surface.report" => {
                return self.silicon.report(params).await;
            }
            "compute.performance_surface.query" => {
                return self.silicon.query(params).await;
            }
            "compute.performance_surface.list" => return self.silicon.list().await,
            "compute.route.multi_unit" => {
                return self.silicon.route_multi_unit(params).await;
            }

            "provenance.query" | "provenance.get" | "toadstool.provenance" => {
                return Self::toadstool_provenance().await;
            }

            _ => {}
        }

        if let Some(impl_name) = self.semantic_registry.resolve(method) {
            debug!("Semantic resolve: {} → {}", method, impl_name);
            return self.dispatch_by_impl_name(impl_name, params).await;
        }

        Err(JsonRpcError::method_not_found(method))
    }

    async fn dispatch_by_impl_name(
        &self,
        impl_name: &str,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match impl_name {
            "execute_workload" | "submit_workload" => self.workload.submit_workload(params).await,
            "get_workload_status" | "query_status" => self.job.query_status(params).await,
            "cancel_workload" => self.workload.cancel_workload(params).await,
            "list_workloads" => self.job.list_workloads(params).await,
            "query_capabilities" => self.workload.query_capabilities().await,
            "check_health" => core::health(&self.version, self.start_time, &self.error_count).await,
            "dispatch_submit" => self.dispatch.dispatch_submit(params).await,
            "dispatch_status" => self.dispatch.dispatch_status(params).await,
            "dispatch_result" => self.dispatch.dispatch_result(params).await,
            "dispatch_capabilities" => self.dispatch.dispatch_capabilities(params).await,
            "shader_dispatch" => self.dispatch.shader_dispatch(params).await,
            "toadstool_provenance" => Self::toadstool_provenance().await,
            "gpu_info" => core::gpu_info().await,
            "gpu_memory" => core::gpu_memory().await,
            "gpu_telemetry" => self.hw_learn.gpu_telemetry(params).await,
            "hw_learn_observe" => self.hw_learn.hw_learn_observe(params).await,
            "hw_learn_distill" => self.hw_learn.hw_learn_distill(params).await,
            "hw_learn_apply" => self.hw_learn.hw_learn_apply(params).await,
            "hw_learn_share_recipe" => self.hw_learn.hw_learn_share_recipe(params).await,
            "hw_learn_status" => self.hw_learn.hw_learn_status(params).await,
            "hw_learn_auto_init" => self.hw_learn.hw_learn_auto_init(params).await,
            "hw_learn_auto_init_all" => self.hw_learn.hw_learn_auto_init_all(params).await,
            "hw_learn_vfio_devices" => self.hw_learn.hw_learn_vfio_devices(params).await,
            "performance_surface_report" => self.silicon.report(params).await,
            "performance_surface_query" => self.silicon.query(params).await,
            "performance_surface_list" => self.silicon.list().await,
            "route_multi_unit" => self.silicon.route_multi_unit(params).await,
            _ => Err(JsonRpcError::method_not_found(impl_name)),
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Provenance domain — cross-spring evolution introspection
    // ═══════════════════════════════════════════════════════════

    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    async fn toadstool_provenance() -> Result<serde_json::Value, JsonRpcError> {
        Ok(toadstool::cross_spring_provenance::provenance_json())
    }

    // ═══════════════════════════════════════════════════════════
    // Ember domain — toadStool-native GPU device management
    // ═══════════════════════════════════════════════════════════

    fn ember_list(&self) -> serde_json::Value {
        let list = self.glowplug.list_devices();
        serde_json::to_value(list).unwrap_or_else(|_| serde_json::json!({"devices": []}))
    }

    fn ember_status(&self) -> serde_json::Value {
        let status = self.glowplug.status();
        serde_json::to_value(status).unwrap_or_else(|_| serde_json::json!({"available": false}))
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::sync::Arc;

    use super::JsonRpcHandler;
    use crate::pure_jsonrpc::types::{JsonRpcError, JsonRpcRequest};

    fn test_handler() -> JsonRpcHandler {
        let executor = Arc::new(crate::tarpc_server::StandaloneExecutor::new());
        JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None)
    }

    fn mk_request(
        method: &str,
        params: Option<serde_json::Value>,
        id: i32,
    ) -> JsonRpcRequest<'static> {
        JsonRpcRequest {
            jsonrpc: Cow::Borrowed("2.0"),
            method: Cow::Owned(method.to_string()),
            params,
            id: Some(serde_json::json!(id)),
        }
    }

    #[tokio::test]
    async fn test_health_returns_valid_status() {
        let handler = test_handler();
        let request = mk_request("toadstool.health", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["healthy"].as_bool().unwrap());
        assert!(result["version"].as_str().is_some());
        assert!(result["uptime_secs"].as_u64().is_some());
        assert!(result["error_count"].as_u64().is_some());
    }

    #[tokio::test]
    async fn test_health_triad_liveness_readiness_check() {
        let handler = test_handler();

        let live = handler
            .handle_request(&mk_request("health.liveness", None, 10))
            .await;
        assert!(live.error.is_none());
        let r = live.result.expect("liveness");
        assert_eq!(r["status"], "alive");
        assert!(r.get("healthy").is_none(), "liveness must be minimal");

        let ready = handler
            .handle_request(&mk_request("health.readiness", None, 11))
            .await;
        assert!(ready.error.is_none());
        let r = ready.result.expect("readiness");
        assert_eq!(r["status"], "ready");
        assert_eq!(r["version"], "test-1.0.0");

        let check = handler
            .handle_request(&mk_request("health.check", None, 12))
            .await;
        assert!(check.error.is_none());
        let r = check.result.expect("check");
        assert!(r["healthy"].as_bool().unwrap());
        assert_eq!(r["status"], "alive");
    }

    #[tokio::test]
    async fn test_version_info_returns_expected_fields() {
        let handler = test_handler();
        let request = mk_request("toadstool.version", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["version"], "test-1.0.0");
        assert_eq!(result["protocol"], "JSON-RPC 2.0");
        assert_eq!(result["service"], "ToadStool Compute");
        assert!(result["implementation"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_handle_method_returns_method_not_found_for_unknown() {
        let handler = test_handler();
        let request = mk_request("unknown.nonexistent.method", None, 99);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::METHOD_NOT_FOUND);
        assert!(err.message.contains("unknown.nonexistent.method"));
    }

    #[tokio::test]
    async fn test_discover_capabilities_includes_shader_methods() {
        let handler = test_handler();
        let request = mk_request("compute.discover_capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let methods = result["methods"].as_array().expect("methods is array");
        let has_shader_dispatch = methods
            .iter()
            .any(|m| m.as_str() == Some("shader.dispatch"));
        assert!(
            has_shader_dispatch,
            "methods should include shader.dispatch"
        );
    }

    #[tokio::test]
    async fn test_shader_dispatch_routes_and_returns_domain() {
        let handler = test_handler();
        let params = serde_json::json!({
            "binary": [0xDE, 0xAD, 0xBE, 0xEF],
            "bdf": "0000:03:00.0",
            "dispatch_mode": "passthrough",
        });
        let request = mk_request("shader.dispatch", Some(params), 1);
        let response = handler.handle_request(&request).await;
        assert!(
            response.error.is_none(),
            "shader.dispatch should route without error"
        );
        let result = response.result.expect("result present");
        assert_eq!(result["domain"], "shader.dispatch");
        assert!(result["job_id"].as_str().is_some());
        assert_eq!(result["binary_size"], 4);
    }
}
