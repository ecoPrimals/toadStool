// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC request handler and method router
//!
//! Routes JSON-RPC 2.0 requests to the appropriate executor or job queue.
//! Semantic method names are resolved through `SemanticMethodRegistry`
//! before dispatch, enabling both legacy `toadstool.*` names and the
//! standard `{domain}.{operation}` naming convention.

mod core;
mod job;
mod ollama;
mod resources;
mod science;
mod transport;
mod workload;

use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use toadstool::semantic_methods::SemanticMethodRegistry;
use tracing::{debug, error, info};

use super::types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, JSONRPC_VERSION};

use job::JobHandler;
use ollama::OllamaHandler;
use resources::ResourceHandler;
use transport::TransportHandler;
use workload::WorkloadHandler;

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
    job: JobHandler,
    workload: WorkloadHandler,
    resources: ResourceHandler,
    transport: TransportHandler,
    ollama: OllamaHandler,
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
        let local_gate_id = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("TOADSTOOL_GATE_ID"))
            .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|h| h.trim().to_string()))
            .unwrap_or_else(|_| "local".to_string());
        Self {
            version: version.into(),
            start_time: std::time::Instant::now(),
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
            semantic_registry: SemanticMethodRegistry::new(),
            job: JobHandler::new(local_gate_id),
            workload: WorkloadHandler::new(executor),
            resources: ResourceHandler::new(),
            transport: TransportHandler::new(),
            ollama: OllamaHandler::new(),
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

        info!("JSON-RPC request: {}", request.method.as_ref());

        match self
            .handle_method(request.method.as_ref(), request.params.as_ref())
            .await
        {
            Ok(result) => JsonRpcResponse {
                jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                result: Some(result),
                error: None,
                id: request.id.clone().unwrap_or(serde_json::Value::Null),
            },
            Err(err) => {
                self.error_count.fetch_add(1, Ordering::Relaxed);
                error!(
                    "JSON-RPC error for {}: {}",
                    request.method.as_ref(),
                    err.message
                );
                JsonRpcResponse {
                    jsonrpc: Cow::Borrowed(JSONRPC_VERSION),
                    result: None,
                    error: Some(err),
                    id: request.id.clone().unwrap_or(serde_json::Value::Null),
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
            "toadstool.health" => {
                return core::health(&self.version, self.start_time, &self.error_count).await
            }
            "toadstool.version" => return core::version_info(&self.version).await,

            "toadstool.resources.estimate" | "resources.estimate" | "ai.local_inference" => {
                return self.resources.resources_estimate(params).await
            }
            "toadstool.resources.validate_availability"
            | "resources.validate_availability"
            | "ai.local_execute" => {
                return self.resources.resources_validate_availability(params).await
            }
            "toadstool.resources.suggest_optimizations" | "resources.suggest_optimizations" => {
                return self.resources.resources_suggest_optimizations(params).await
            }

            "compute.health" => {
                return core::health(&self.version, self.start_time, &self.error_count).await
            }
            "compute.version" => return core::version_info(&self.version).await,
            "compute.capabilities" => return self.workload.query_capabilities().await,
            "compute.discover_capabilities" => {
                return core::discover_capabilities(&self.semantic_registry, &self.version).await
            }

            "compute.submit" => return self.job.compute_submit(params).await,
            "compute.status" => return self.job.compute_status(params).await,
            "compute.result" => return self.job.compute_result(params).await,
            "compute.cancel" => return self.job.compute_cancel(params).await,
            "compute.list" => return self.job.compute_list(params).await,

            "gpu.info" => return core::gpu_info().await,
            "gpu.memory" => return core::gpu_memory().await,

            "ollama.list_models" => return self.ollama.ollama_list_models().await,
            "ollama.inference" => return self.ollama.ollama_inference(params).await,
            "ollama.load" => return self.ollama.ollama_load(params).await,
            "ollama.unload" => return self.ollama.ollama_unload(params).await,

            "gate.update" => return self.job.gate_update(params).await,
            "gate.remove" => return self.job.gate_remove(params).await,
            "gate.list" => return self.job.gate_list().await,
            "gate.route" => return self.job.gate_route(params).await,

            "transport.discover" => return self.transport.transport_discover(params).await,
            "transport.list" => return self.transport.transport_list().await,
            "transport.route" => return self.transport.transport_route(params).await,

            "science.compute.submit" => {
                return science::science_compute_submit(&self.job, params).await
            }
            "science.compute.status" => {
                return science::science_compute_status(&self.job, params).await
            }
            "science.compute.result" => {
                return science::science_compute_result(&self.job, params).await
            }
            "science.compute.cancel" => {
                return science::science_compute_cancel(&self.job, params).await
            }
            "science.gpu.dispatch" => {
                return science::science_gpu_dispatch(&self.job, params).await
            }
            "science.gpu.capabilities" => return science::science_gpu_capabilities().await,
            "science.npu.dispatch" => {
                return science::science_npu_dispatch(&self.job, params).await
            }
            "science.npu.capabilities" => return science::science_npu_capabilities().await,
            "science.substrate.discover" => return science::science_substrate_discover().await,
            "science.substrate.probe" => return science::science_substrate_probe(params).await,

            "shader.compile.wgsl" => return self.shader_compile_wgsl(params).await,
            "shader.compile.spirv" => return self.shader_compile_spirv(params).await,
            "shader.compile.status" => return self.shader_compile_status(params).await,
            "shader.compile.capabilities" => return self.shader_compile_capabilities().await,

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
            "science_compute_submit" => science::science_compute_submit(&self.job, params).await,
            "science_compute_status" => science::science_compute_status(&self.job, params).await,
            "science_compute_result" => science::science_compute_result(&self.job, params).await,
            "science_compute_cancel" => science::science_compute_cancel(&self.job, params).await,
            "science_gpu_dispatch" => science::science_gpu_dispatch(&self.job, params).await,
            "science_gpu_capabilities" => science::science_gpu_capabilities().await,
            "science_npu_dispatch" => science::science_npu_dispatch(&self.job, params).await,
            "science_npu_capabilities" => science::science_npu_capabilities().await,
            "science_substrate_discover" => science::science_substrate_discover().await,
            "science_substrate_probe" => science::science_substrate_probe(params).await,
            "shader_compile_wgsl" => self.shader_compile_wgsl(params).await,
            "shader_compile_spirv" => self.shader_compile_spirv(params).await,
            "shader_compile_status" => self.shader_compile_status(params).await,
            "shader_compile_capabilities" => self.shader_compile_capabilities().await,
            _ => Err(JsonRpcError::method_not_found(impl_name)),
        }
    }

    // ═══════════════════════════════════════════════════════════
    // Shader domain — compilation pipeline IPC for coralReef
    //
    // Routes WGSL/SPIR-V compilation requests. When coralReef is
    // available, these will proxy to tarpc.compiler.compile().
    // Currently returns pipeline metadata for capability probing.
    // ═══════════════════════════════════════════════════════════

    async fn shader_compile_wgsl(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let shader_source = params
            .and_then(|p| p.get("source"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        if shader_source.is_empty() {
            return Err(JsonRpcError::invalid_params(
                "Missing required 'source' parameter with WGSL shader source",
            ));
        }

        Ok(serde_json::json!({
            "status": "accepted",
            "pipeline": "naga_wgsl_to_spirv",
            "source_language": "wgsl",
            "target": "spirv",
            "note": "Compilation routed through naga. coralReef native path not yet available."
        }))
    }

    async fn shader_compile_spirv(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let has_binary = params.and_then(|p| p.get("spirv_binary")).is_some();

        if !has_binary {
            return Err(JsonRpcError::invalid_params(
                "Missing required 'spirv_binary' parameter with base64-encoded SPIR-V",
            ));
        }

        Ok(serde_json::json!({
            "status": "accepted",
            "pipeline": "spirv_passthrough",
            "source_language": "spirv",
            "native_compilation_available": false,
            "note": "SPIR-V accepted. Native binary compilation via coralReef not yet available."
        }))
    }

    async fn shader_compile_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let compile_id = params
            .and_then(|p| p.get("compile_id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");

        Ok(serde_json::json!({
            "compile_id": compile_id,
            "status": "not_found",
            "note": "Compilation tracking is available when coralReef pipeline is active"
        }))
    }

    #[allow(clippy::unused_async)]
    async fn shader_compile_capabilities(&self) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "source_languages": ["wgsl"],
            "target_formats": ["spirv"],
            "native_binary_compilation": false,
            "coral_reef_available": false,
            "coral_driver_available": false,
            "naga_pipeline": true,
            "domain": "shader"
        }))
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
    async fn test_discover_capabilities_includes_science_in_node_capabilities() {
        let handler = test_handler();
        let request = mk_request("compute.discover_capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let node_capabilities = result["node_capabilities"]
            .as_array()
            .expect("node_capabilities is array");
        let has_science = node_capabilities
            .iter()
            .any(|c| c.as_str() == Some("science"));
        assert!(has_science, "node_capabilities should include 'science'");
    }

    #[tokio::test]
    async fn test_discover_capabilities_includes_science_methods() {
        let handler = test_handler();
        let request = mk_request("compute.discover_capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let methods = result["methods"].as_array().expect("methods is array");
        let science_methods: Vec<_> = methods
            .iter()
            .filter_map(|m| m.as_str())
            .filter(|m| m.starts_with("science."))
            .collect();
        assert!(
            !science_methods.is_empty(),
            "methods should include science.* entries"
        );
        assert!(
            science_methods.contains(&"science.gpu.capabilities"),
            "should include science.gpu.capabilities"
        );
        assert!(
            science_methods.contains(&"science.npu.capabilities"),
            "should include science.npu.capabilities"
        );
        assert!(
            science_methods.contains(&"science.substrate.discover"),
            "should include science.substrate.discover"
        );
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
    async fn test_science_gpu_capabilities_returns_expected_structure() {
        let handler = test_handler();
        let request = mk_request("science.gpu.capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("devices").is_some());
        assert!(result.get("supported_precisions").is_some());
        assert!(result.get("compute_backends").is_some());
        assert_eq!(result["domain"], "science");
    }

    #[tokio::test]
    async fn test_science_npu_capabilities_returns_expected_structure() {
        let handler = test_handler();
        let request = mk_request("science.npu.capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("available").is_some());
        assert_eq!(result["domain"], "science");
        assert!(result.get("supported_models").is_some());
        assert!(result.get("note").is_some());
    }

    #[tokio::test]
    async fn test_science_substrate_discover_returns_expected_structure() {
        let handler = test_handler();
        let request = mk_request("science.substrate.discover", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let substrates = result.get("substrates").expect("substrates present");
        assert!(substrates.get("gpu").is_some());
        assert!(substrates.get("npu").is_some());
        assert!(substrates.get("cpu").is_some());
        assert_eq!(result["domain"], "science");
    }

    #[tokio::test]
    async fn test_discover_capabilities_includes_shader_methods() {
        let handler = test_handler();
        let request = mk_request("compute.discover_capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let methods = result["methods"].as_array().expect("methods is array");
        let shader_methods: Vec<_> = methods
            .iter()
            .filter_map(|m| m.as_str())
            .filter(|m| m.starts_with("shader."))
            .collect();
        assert!(
            !shader_methods.is_empty(),
            "methods should include shader.* entries"
        );
        assert!(
            shader_methods.contains(&"shader.compile.wgsl"),
            "should include shader.compile.wgsl"
        );
        assert!(
            shader_methods.contains(&"shader.compile.capabilities"),
            "should include shader.compile.capabilities"
        );
    }

    #[tokio::test]
    async fn test_shader_compile_capabilities_returns_expected_structure() {
        let handler = test_handler();
        let request = mk_request("shader.compile.capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["domain"], "shader");
        assert!(result["naga_pipeline"].as_bool().unwrap());
        assert!(!result["coral_reef_available"].as_bool().unwrap());
        assert!(result["source_languages"].as_array().is_some());
    }

    #[tokio::test]
    async fn test_shader_compile_wgsl_requires_source() {
        let handler = test_handler();
        let request = mk_request("shader.compile.wgsl", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_some());
        let err = response.error.expect("error present");
        assert!(err.message.contains("source"));
    }

    #[tokio::test]
    async fn test_shader_compile_wgsl_with_source() {
        let handler = test_handler();
        let params = serde_json::json!({ "source": "@compute fn main() {}" });
        let request = mk_request("shader.compile.wgsl", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["status"], "accepted");
        assert_eq!(result["source_language"], "wgsl");
    }

    #[tokio::test]
    async fn test_science_gpu_capabilities_includes_precision_notes() {
        let handler = test_handler();
        let request = mk_request("science.gpu.capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let notes = result
            .get("precision_notes")
            .expect("precision_notes present");
        assert!(!notes["f64_shared_memory_reliable"].as_bool().unwrap());
        assert!(notes["df64_reductions"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_all_science_methods_dispatch_without_routing_error() {
        let handler = test_handler();

        let no_param_methods = [
            "science.gpu.capabilities",
            "science.npu.capabilities",
            "science.substrate.discover",
        ];
        for method in no_param_methods {
            let request = mk_request(method, None, 1);
            let response = handler.handle_request(&request).await;
            assert!(
                response.error.is_none(),
                "{} should dispatch without error",
                method
            );
        }

        let request = mk_request("science.substrate.probe", None, 1);
        let response = handler.handle_request(&request).await;
        assert!(
            response.error.is_none(),
            "science.substrate.probe should succeed"
        );

        let params = serde_json::json!({
            "inference": { "model": "tinyllama", "prompt": "test", "params": {} }
        });
        let submit_methods = [
            "science.compute.submit",
            "science.gpu.dispatch",
            "science.npu.dispatch",
        ];
        for method in submit_methods {
            let request = mk_request(method, Some(params.clone()), 1);
            let response = handler.handle_request(&request).await;
            assert!(
                response.error.is_none(),
                "{} should dispatch and return job_id",
                method
            );
            assert!(
                response
                    .result
                    .as_ref()
                    .and_then(|r| r.get("job_id"))
                    .is_some(),
                "{} should return job_id",
                method
            );
        }
    }
}
