//! Domain-specific JSON-RPC method handlers for ManualJsonRpcServer.
//!
//! Separated from the core server module for code size compliance (1000-line max).
//! These handlers implement toadstool.*, compute.*, gpu.*, ollama.*, gate.*, and resources.* domains.

use std::sync::atomic::Ordering;
#[allow(deprecated)]
use toadstool_common::interned_strings::primals;

use serde_json::Value;

use super::cross_gate::GateGpuInfo;
use super::gpu_job_queue::{query_gpu_devices, query_gpu_memory, JobType};
use super::graph_types::ExecutionGraph;
use super::manual_jsonrpc::{
    JsonRpcRequest, JsonRpcResponse, ManualJsonRpcServer, INTERNAL_ERROR, INVALID_PARAMS,
    JSONRPC_VERSION, SERIALIZATION_FAILED,
};

/// Core toadstool handlers (toadstool.*)
impl ManualJsonRpcServer {
    /// Handle health check
    #[allow(deprecated)]
    pub(crate) async fn handle_health(&self, request: JsonRpcRequest) -> Value {
        self.success_response(
            serde_json::json!({
                "healthy": true,
                "service": primals::TOADSTOOL,
                "version": self.version,
                "error_count": self.error_count.load(Ordering::Relaxed),
                "uptime_secs": self.start_time.elapsed().as_secs(),
            }),
            &request,
        )
    }

    /// Handle version query
    pub(crate) async fn handle_version(&self, request: JsonRpcRequest) -> Value {
        self.success_response(
            serde_json::json!({"version": self.version, "protocol": "json-rpc-2.0"}),
            &request,
        )
    }

    /// Handle discover_capabilities - returns all available methods
    #[allow(deprecated)]
    pub(crate) async fn handle_discover_capabilities(&self, request: JsonRpcRequest) -> Value {
        let capabilities = serde_json::json!({
            "capabilities": [
                "toadstool.health",
                "toadstool.version",
                "toadstool.query_capabilities",
                "toadstool.resources.estimate",
                "toadstool.resources.validate_availability",
                "toadstool.resources.suggest_optimizations",
                "compute.discover_capabilities",
                "compute.submit",
                "compute.status",
                "compute.result",
                "compute.cancel",
                "compute.list",
                "gpu.info",
                "gpu.memory",
                "ollama.list_models",
                "ollama.inference",
                "ollama.load",
                "ollama.unload",
                "gate.update",
                "gate.remove",
                "gate.list",
                "gate.route"
            ],
            "version": self.version,
            "primal": primals::TOADSTOOL
        });

        serde_json::to_value(JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            result: capabilities,
            id: request.id,
        })
        .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}))
    }
}

/// Compute domain handlers (compute.*)
impl ManualJsonRpcServer {
    /// Handle compute.submit - Submit a compute job with cross-gate routing
    ///
    /// Params: `{ "type": "inference"|"transform"|"custom", ...type-specific fields, "priority": 0 }`
    pub(crate) async fn handle_compute_submit(&self, mut request: JsonRpcRequest) -> Value {
        let params = match request.params.take() {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let priority = params.get("priority").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let vram_hint = params
            .get("vram_required_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096);

        let job_type: JobType = match serde_json::from_value(params) {
            Ok(jt) => jt,
            Err(e) => {
                return self.error_response(
                    INVALID_PARAMS,
                    format!("Invalid job type: {e}"),
                    &request,
                )
            }
        };

        // Cross-gate routing: determine optimal execution target
        let routing = {
            let router = self.router.read().await;
            let model = match &job_type {
                JobType::Inference { model, .. } => model.as_str(),
                _ => "",
            };
            router.route(model, vram_hint)
        };

        match self.job_queue.submit(job_type, priority).await {
            Ok(job_id) => self.success_response(
                serde_json::json!({
                    "job_id": job_id,
                    "routing": {
                        "gate_id": routing.gate_id,
                        "reason": routing.reason,
                        "estimated_wait_ms": routing.estimated_wait_ms,
                    }
                }),
                &request,
            ),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    /// Handle compute.status - Check job status
    ///
    /// Params: `{ "job_id": "uuid" }`
    pub(crate) async fn handle_compute_status(&self, request: JsonRpcRequest) -> Value {
        let job_id = match self.extract_job_id(&request) {
            Ok(id) => id,
            Err(resp) => return resp,
        };

        match self.job_queue.status(job_id).await {
            Ok(job) => {
                let result = serde_json::to_value(job)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => self.job_queue_error_response(e, &request),
        }
    }

    /// Handle compute.result - Get completed job result
    ///
    /// Params: `{ "job_id": "uuid" }`
    pub(crate) async fn handle_compute_result(&self, request: JsonRpcRequest) -> Value {
        let job_id = match self.extract_job_id(&request) {
            Ok(id) => id,
            Err(resp) => return resp,
        };

        match self.job_queue.result(job_id).await {
            Ok(result) => self.success_response(result, &request),
            Err(e) => self.job_queue_error_response(e, &request),
        }
    }

    /// Handle compute.cancel - Cancel a pending/running job
    ///
    /// Params: `{ "job_id": "uuid" }`
    pub(crate) async fn handle_compute_cancel(&self, request: JsonRpcRequest) -> Value {
        let job_id = match self.extract_job_id(&request) {
            Ok(id) => id,
            Err(resp) => return resp,
        };

        match self.job_queue.cancel(job_id).await {
            Ok(()) => self.success_response(serde_json::json!({"cancelled": true}), &request),
            Err(e) => self.job_queue_error_response(e, &request),
        }
    }

    /// Handle compute.list - List all jobs
    ///
    /// Params: optional `{ "state": "pending"|"running"|"completed"|"failed"|"cancelled" }`
    pub(crate) async fn handle_compute_list(&self, request: JsonRpcRequest) -> Value {
        let state_filter = request
            .params
            .as_ref()
            .and_then(|p| p.get("state"))
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let jobs = self.job_queue.list(state_filter).await;
        let counts = self.job_queue.counts().await;

        self.success_response(
            serde_json::json!({ "jobs": jobs, "counts": counts }),
            &request,
        )
    }
}

/// GPU info handlers (gpu.*)
impl ManualJsonRpcServer {
    /// Handle gpu.info - Return GPU device information
    pub(crate) async fn handle_gpu_info(&self, request: JsonRpcRequest) -> Value {
        let result = serde_json::json!({
            "devices": query_gpu_devices(),
            "driver": "wgpu",
            "compute_backends": ["vulkan", "metal", "dx12"],
        });
        self.success_response(result, &request)
    }

    /// Handle gpu.memory - Return GPU memory usage
    pub(crate) async fn handle_gpu_memory(&self, request: JsonRpcRequest) -> Value {
        self.success_response(
            serde_json::json!({ "devices": query_gpu_memory() }),
            &request,
        )
    }
}

/// Ollama integration handlers (ollama.*)
impl ManualJsonRpcServer {
    /// Handle ollama.list_models - List available models
    pub(crate) async fn handle_ollama_list_models(&self, request: JsonRpcRequest) -> Value {
        match self.ollama.list_models().await {
            Ok(models) => self.success_response(serde_json::json!({"models": models}), &request),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    /// Handle ollama.inference - Run model inference
    ///
    /// Params: `{ "model": "name", "prompt": "text", "params": {...} }`
    pub(crate) async fn handle_ollama_inference(&self, request: JsonRpcRequest) -> Value {
        let params = match &request.params {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let model = match params.get("model").and_then(|v| v.as_str()) {
            Some(m) => m,
            None => return self.error_response(INVALID_PARAMS, "Missing 'model' param", &request),
        };

        let prompt = match params.get("prompt").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing 'prompt' param", &request),
        };

        let extra_params = params.get("params").cloned().unwrap_or(Value::Null);

        match self.ollama.inference(model, prompt, &extra_params).await {
            Ok(response) => self.success_response(response, &request),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    /// Handle ollama.load - Preload model into VRAM
    ///
    /// Params: `{ "model": "name" }`
    pub(crate) async fn handle_ollama_load(&self, request: JsonRpcRequest) -> Value {
        let model = match request
            .params
            .as_ref()
            .and_then(|p| p.get("model"))
            .and_then(|v| v.as_str())
        {
            Some(m) => m,
            None => return self.error_response(INVALID_PARAMS, "Missing 'model' param", &request),
        };

        match self.ollama.load(model).await {
            Ok(()) => self.success_response(
                serde_json::json!({"loaded": true, "model": model}),
                &request,
            ),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }

    /// Handle ollama.unload - Free VRAM by unloading model
    ///
    /// Params: `{ "model": "name" }`
    pub(crate) async fn handle_ollama_unload(&self, request: JsonRpcRequest) -> Value {
        let model = match request
            .params
            .as_ref()
            .and_then(|p| p.get("model"))
            .and_then(|v| v.as_str())
        {
            Some(m) => m,
            None => return self.error_response(INVALID_PARAMS, "Missing 'model' param", &request),
        };

        match self.ollama.unload(model).await {
            Ok(()) => self.success_response(
                serde_json::json!({"unloaded": true, "model": model}),
                &request,
            ),
            Err(e) => self.error_response(INTERNAL_ERROR, e.to_string(), &request),
        }
    }
}

/// Cross-gate routing handlers (gate.*)
impl ManualJsonRpcServer {
    /// Handle gate.update - Register or update a remote gate's GPU capabilities
    ///
    /// Params: `GateGpuInfo` fields (gate_id, gpu_model, vram_total_mb, etc.)
    pub(crate) async fn handle_gate_update(&self, mut request: JsonRpcRequest) -> Value {
        let params = match request.params.take() {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let gate_info: GateGpuInfo = match serde_json::from_value(params) {
            Ok(info) => info,
            Err(e) => {
                return self.error_response(
                    INVALID_PARAMS,
                    format!("Invalid gate info: {e}"),
                    &request,
                )
            }
        };

        let gate_id = gate_info.gate_id.clone();
        self.router.write().await.update_gate(gate_info);
        self.success_response(
            serde_json::json!({"updated": true, "gate_id": gate_id}),
            &request,
        )
    }

    /// Handle gate.remove - Remove a gate from the routing table
    ///
    /// Params: `{ "gate_id": "string" }`
    pub(crate) async fn handle_gate_remove(&self, request: JsonRpcRequest) -> Value {
        let gate_id = match request
            .params
            .as_ref()
            .and_then(|p| p.get("gate_id"))
            .and_then(|v| v.as_str())
        {
            Some(id) => id,
            None => {
                return self.error_response(INVALID_PARAMS, "Missing 'gate_id' param", &request)
            }
        };

        self.router.write().await.remove_gate(gate_id);
        self.success_response(
            serde_json::json!({"removed": true, "gate_id": gate_id}),
            &request,
        )
    }

    /// Handle gate.list - List all known gates and their capabilities
    pub(crate) async fn handle_gate_list(&self, request: JsonRpcRequest) -> Value {
        let router = self.router.read().await;
        let gates: Vec<&GateGpuInfo> = router.gates().values().collect();
        self.success_response(serde_json::json!({"gates": gates}), &request)
    }

    /// Handle gate.route - Preview routing decision for a model without submitting
    ///
    /// Params: `{ "model": "string", "vram_required_mb": 4096 }`
    pub(crate) async fn handle_gate_route(&self, request: JsonRpcRequest) -> Value {
        let params = match &request.params {
            Some(p) => p,
            None => return self.error_response(INVALID_PARAMS, "Missing params", &request),
        };

        let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("");
        let vram = params
            .get("vram_required_mb")
            .and_then(|v| v.as_u64())
            .unwrap_or(4096);

        let router = self.router.read().await;
        let decision = router.route(model, vram);
        self.success_response(
            serde_json::json!({
                "gate_id": decision.gate_id,
                "reason": decision.reason,
                "estimated_wait_ms": decision.estimated_wait_ms,
            }),
            &request,
        )
    }
}

/// Resource management handlers (resources.*)
impl ManualJsonRpcServer {
    /// Extract an `ExecutionGraph` from request params
    fn extract_graph(&self, request: &JsonRpcRequest) -> Result<ExecutionGraph, Value> {
        let params = request.params.as_ref().ok_or_else(|| {
            self.error_response(INVALID_PARAMS, "Missing 'graph' parameter", request)
        })?;

        let graph_value = params.get("graph").cloned().unwrap_or(Value::Null);
        serde_json::from_value(graph_value).map_err(|e| {
            self.error_response(
                INVALID_PARAMS,
                format!("Invalid graph parameter: {e}"),
                request,
            )
        })
    }

    /// Handle capabilities query
    pub(crate) async fn handle_query_capabilities(&self, request: JsonRpcRequest) -> Value {
        match self.executor.query_capabilities().await {
            Ok(caps) => {
                let result = serde_json::to_value(caps)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => self.error_response(
                INTERNAL_ERROR,
                format!("Failed to query capabilities: {e}"),
                &request,
            ),
        }
    }

    /// Handle resources.estimate - Estimate resource requirements for a graph
    pub(crate) async fn handle_resources_estimate(&self, request: JsonRpcRequest) -> Value {
        let graph = match self.extract_graph(&request) {
            Ok(g) => g,
            Err(resp) => return resp,
        };

        match self.estimator.estimate(&graph) {
            Ok(estimate) => {
                let result = serde_json::to_value(estimate)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => {
                self.error_response(INTERNAL_ERROR, format!("Estimation failed: {e}"), &request)
            }
        }
    }

    /// Handle resources.validate_availability - Check if system can execute graph
    pub(crate) async fn handle_resources_validate_availability(
        &self,
        request: JsonRpcRequest,
    ) -> Value {
        let graph = match self.extract_graph(&request) {
            Ok(g) => g,
            Err(resp) => return resp,
        };

        match self.validator.validate_availability(&graph).await {
            Ok(result) => {
                let result_value = serde_json::to_value(result)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result_value, &request)
            }
            Err(e) => {
                self.error_response(INTERNAL_ERROR, format!("Validation failed: {e}"), &request)
            }
        }
    }

    /// Handle resources.suggest_optimizations - Suggest optimizations for graph
    pub(crate) async fn handle_resources_suggest_optimizations(
        &self,
        request: JsonRpcRequest,
    ) -> Value {
        let graph = match self.extract_graph(&request) {
            Ok(g) => g,
            Err(resp) => return resp,
        };

        match self.optimizer.suggest_optimizations(&graph).await {
            Ok(suggestions) => {
                let result = serde_json::to_value(suggestions)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => self.error_response(
                INTERNAL_ERROR,
                format!("Optimization failed: {e}"),
                &request,
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(deprecated)] // Tests use deprecated primal constants for legacy interop testing
    use toadstool_common::interned_strings::primals;

    use super::*;
    use std::sync::Arc;

    use crate::manual_jsonrpc::JsonRpcRequest;
    use crate::tarpc_server::StandaloneExecutor;

    fn test_server() -> ManualJsonRpcServer {
        let executor = Arc::new(StandaloneExecutor::new());
        ManualJsonRpcServer::new(executor, "test-1.0.0".to_string(), None)
    }

    fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(serde_json::json!(id)),
        }
    }

    #[tokio::test]
    async fn test_handle_compute_submit_inference() {
        let server = test_server();
        let params = serde_json::json!({
            "inference": {
                "model": "tinyllama",
                "prompt": "Hello",
                "params": {}
            }
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        let result = &obj["result"];
        assert!(result["job_id"].as_str().is_some());
        assert!(result["routing"]["gate_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_handle_compute_submit_transform() {
        let server = test_server();
        let params = serde_json::json!({
            "transform": {
                "operation": "embed",
                "input": {"text": "test"}
            }
        });
        let request = mk_request("compute.submit", Some(params), 2);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
        assert!(obj["result"]["job_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_handle_compute_submit_missing_params() {
        let server = test_server();
        let request = mk_request("compute.submit", None, 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_compute_status_nonexistent_job() {
        let server = test_server();
        let job_id = uuid::Uuid::new_v4();
        let params = serde_json::json!({"job_id": job_id.to_string()});
        let request = mk_request("compute.status", Some(params), 1);
        let response = server.handle_compute_status(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], -32601);
    }

    #[tokio::test]
    async fn test_handle_compute_status_missing_job_id() {
        let server = test_server();
        let request = mk_request("compute.status", Some(serde_json::json!({})), 1);
        let response = server.handle_compute_status(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_compute_list_empty() {
        let server = test_server();
        let request = mk_request("compute.list", None, 1);
        let response = server.handle_compute_list(request).await;
        let obj = response.as_object().expect("object");
        let result = &obj["result"];
        assert!(result["jobs"].is_array());
        assert!(result["counts"].is_object());
    }

    #[tokio::test]
    async fn test_handle_compute_list_with_state_filter() {
        let server = test_server();
        let params = serde_json::json!({"state": "pending"});
        let request = mk_request("compute.list", Some(params), 1);
        let response = server.handle_compute_list(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj["result"]["jobs"].is_array());
    }

    #[tokio::test]
    async fn test_handle_gpu_info() {
        let server = test_server();
        let request = mk_request("gpu.info", None, 1);
        let response = server.handle_gpu_info(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        let result = &obj["result"];
        assert!(result["devices"].is_array());
        assert_eq!(result["driver"], "wgpu");
    }

    #[tokio::test]
    async fn test_handle_gpu_memory() {
        let server = test_server();
        let request = mk_request("gpu.memory", None, 1);
        let response = server.handle_gpu_memory(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj["result"]["devices"].is_array());
    }

    #[tokio::test]
    async fn test_handle_gate_update() {
        let server = test_server();
        let params = serde_json::json!({
            "gate_id": "test-gate",
            "gpu_model": "RTX 4070",
            "vram_total_mb": 12288,
            "vram_available_mb": 8000,
            "loaded_models": [],
            "queue_depth": 0,
            "reachable": true
        });
        let request = mk_request("gate.update", Some(params), 1);
        let response = server.handle_gate_update(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["result"]["updated"], true);
        assert_eq!(obj["result"]["gate_id"], "test-gate");
    }

    #[tokio::test]
    async fn test_handle_gate_update_missing_params() {
        let server = test_server();
        let request = mk_request("gate.update", None, 1);
        let response = server.handle_gate_update(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_gate_update_invalid_gate_info() {
        let server = test_server();
        let params = serde_json::json!({"gate_id": 123});
        let request = mk_request("gate.update", Some(params), 1);
        let response = server.handle_gate_update(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
        assert!(obj["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Invalid gate info"));
    }

    #[tokio::test]
    async fn test_handle_gate_remove_success() {
        let server = test_server();
        let update_params = serde_json::json!({
            "gate_id": "remove-me",
            "gpu_model": "RTX 4070",
            "vram_total_mb": 12288,
            "vram_available_mb": 8000,
            "loaded_models": [],
            "queue_depth": 0,
            "reachable": true
        });
        server
            .handle_gate_update(mk_request("gate.update", Some(update_params), 0))
            .await;

        let remove_params = serde_json::json!({"gate_id": "remove-me"});
        let request = mk_request("gate.remove", Some(remove_params), 1);
        let response = server.handle_gate_remove(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["result"]["removed"], true);
        assert_eq!(obj["result"]["gate_id"], "remove-me");
    }

    #[tokio::test]
    async fn test_handle_gate_remove_missing_gate_id() {
        let server = test_server();
        let request = mk_request("gate.remove", Some(serde_json::json!({})), 1);
        let response = server.handle_gate_remove(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_gate_route_empty_params_defaults() {
        let server = test_server();
        let params = serde_json::json!({});
        let request = mk_request("gate.route", Some(params), 1);
        let response = server.handle_gate_route(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
        assert!(obj["result"]["gate_id"].as_str().is_some());
    }

    // ---- ollama handlers: param validation and error paths ----
    #[tokio::test]
    async fn test_handle_ollama_list_models() {
        let server = test_server();
        let request = mk_request("ollama.list_models", None, 1);
        let response = server.handle_ollama_list_models(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        if obj.get("result").is_some() {
            assert!(obj["result"]["models"].is_array());
        } else {
            assert_eq!(obj["error"]["code"], INTERNAL_ERROR);
        }
    }

    #[tokio::test]
    async fn test_handle_ollama_inference_missing_params() {
        let server = test_server();
        let request = mk_request("ollama.inference", None, 1);
        let response = server.handle_ollama_inference(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_ollama_inference_missing_model() {
        let server = test_server();
        let params = serde_json::json!({"prompt": "Hello"});
        let request = mk_request("ollama.inference", Some(params), 1);
        let response = server.handle_ollama_inference(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
        assert!(obj["error"]["message"].as_str().unwrap().contains("model"));
    }

    #[tokio::test]
    async fn test_handle_ollama_inference_missing_prompt() {
        let server = test_server();
        let params = serde_json::json!({"model": "llama"});
        let request = mk_request("ollama.inference", Some(params), 1);
        let response = server.handle_ollama_inference(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
        assert!(obj["error"]["message"].as_str().unwrap().contains("prompt"));
    }

    #[tokio::test]
    async fn test_handle_ollama_load_missing_model() {
        let server = test_server();
        let request = mk_request("ollama.load", Some(serde_json::json!({})), 1);
        let response = server.handle_ollama_load(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_ollama_unload_missing_model() {
        let server = test_server();
        let request = mk_request("ollama.unload", Some(serde_json::json!({})), 1);
        let response = server.handle_ollama_unload(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    // ---- resources handlers and toadstool.query_capabilities ----
    #[tokio::test]
    async fn test_handle_query_capabilities() {
        let server = test_server();
        let request = mk_request("toadstool.query_capabilities", None, 1);
        let response = server.handle_query_capabilities(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["jsonrpc"], "2.0");
        assert!(obj.get("result").is_some());
    }

    fn minimal_valid_graph_json() -> serde_json::Value {
        serde_json::json!({
            "graph": {
                "id": "test-graph",
                "nodes": [
                    {
                        "id": "node-1",
                        "primal": primals::TOADSTOOL,
                        "operation": "cpu_compute",
                        "requirements": {},
                        "metadata": {}
                    }
                ],
                "edges": [],
                "metadata": {}
            }
        })
    }

    #[tokio::test]
    async fn test_handle_resources_estimate_success() {
        let server = test_server();
        let params = minimal_valid_graph_json();
        let request = mk_request("toadstool.resources.estimate", Some(params), 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
        assert!(obj["result"]["graph_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_handle_resources_estimate_missing_params() {
        let server = test_server();
        let request = mk_request("toadstool.resources.estimate", None, 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_resources_estimate_invalid_graph() {
        let server = test_server();
        let params = serde_json::json!({"graph": {"id": "x", "nodes": [], "edges": []}});
        let request = mk_request("toadstool.resources.estimate", Some(params), 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("error").is_some());
    }

    #[tokio::test]
    async fn test_handle_resources_validate_availability_success() {
        let server = test_server();
        let params = minimal_valid_graph_json();
        let request = mk_request("toadstool.resources.validate_availability", Some(params), 1);
        let response = server.handle_resources_validate_availability(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
    }

    #[tokio::test]
    async fn test_handle_resources_validate_availability_missing_graph() {
        let server = test_server();
        let request = mk_request("toadstool.resources.validate_availability", None, 1);
        let response = server.handle_resources_validate_availability(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_resources_suggest_optimizations_success() {
        let server = test_server();
        let params = minimal_valid_graph_json();
        let request = mk_request("toadstool.resources.suggest_optimizations", Some(params), 1);
        let response = server.handle_resources_suggest_optimizations(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
    }

    #[tokio::test]
    async fn test_handle_resources_suggest_optimizations_missing_graph() {
        let server = test_server();
        let request = mk_request("toadstool.resources.suggest_optimizations", None, 1);
        let response = server.handle_resources_suggest_optimizations(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_resources_extract_graph_invalid_json() {
        let server = test_server();
        let params = serde_json::json!({"graph": "not a graph object"});
        let request = mk_request("toadstool.resources.estimate", Some(params), 1);
        let response = server.handle_resources_estimate(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_gate_list() {
        let server = test_server();
        let request = mk_request("gate.list", None, 1);
        let response = server.handle_gate_list(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj["result"]["gates"].is_array());
    }

    #[tokio::test]
    async fn test_handle_gate_route() {
        let server = test_server();
        let params = serde_json::json!({
            "model": "llama3:8b",
            "vram_required_mb": 4096
        });
        let request = mk_request("gate.route", Some(params), 1);
        let response = server.handle_gate_route(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj["result"]["gate_id"].as_str().is_some());
        assert!(obj["result"]["reason"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_handle_gate_route_no_params() {
        let server = test_server();
        let request = mk_request("gate.route", None, 1);
        let response = server.handle_gate_route(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    // ---- compute.submit: invalid job type, custom job type, params.take() ----
    #[tokio::test]
    async fn test_handle_compute_submit_invalid_job_type() {
        let server = test_server();
        let params = serde_json::json!({"unknown_variant": {}});
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
        assert!(obj["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Invalid job type"));
    }

    #[tokio::test]
    async fn test_handle_compute_submit_custom_job_type() {
        let server = test_server();
        let params = serde_json::json!({
            "custom": {
                "plugin": "my_plugin",
                "payload": {"key": "value"}
            }
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
        assert!(obj["result"]["job_id"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_handle_compute_submit_routing_response() {
        let server = test_server();
        let params = serde_json::json!({
            "inference": {
                "model": "test",
                "prompt": "Hi",
                "params": {}
            }
        });
        let request = mk_request("compute.submit", Some(params), 1);
        let response = server.handle_compute_submit(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
        assert!(obj["result"]["routing"]["estimated_wait_ms"]
            .as_u64()
            .is_some());
    }

    // ---- compute.status: success path ----
    #[tokio::test]
    async fn test_handle_compute_status_success() {
        let server = test_server();
        let params = serde_json::json!({
            "inference": {"model": "x", "prompt": "y", "params": {}}
        });
        let submit_req = mk_request("compute.submit", Some(params), 0);
        let submit_resp = server.handle_compute_submit(submit_req).await;
        let job_id = submit_resp["result"]["job_id"]
            .as_str()
            .unwrap()
            .to_string();

        let status_params = serde_json::json!({"job_id": job_id});
        let status_req = mk_request("compute.status", Some(status_params), 1);
        let response = server.handle_compute_status(status_req).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
        assert_eq!(obj["result"]["state"], "pending");
    }

    // ---- compute.result: error paths ----
    #[tokio::test]
    async fn test_handle_compute_result_missing_job_id() {
        let server = test_server();
        let request = mk_request("compute.result", Some(serde_json::json!({})), 1);
        let response = server.handle_compute_result(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_compute_result_nonexistent_job() {
        let server = test_server();
        let job_id = uuid::Uuid::new_v4();
        let params = serde_json::json!({"job_id": job_id.to_string()});
        let request = mk_request("compute.result", Some(params), 1);
        let response = server.handle_compute_result(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("error").is_some());
    }

    #[tokio::test]
    async fn test_handle_compute_result_job_not_complete() {
        let server = test_server();
        let params = serde_json::json!({
            "inference": {"model": "x", "prompt": "y", "params": {}}
        });
        let submit_req = mk_request("compute.submit", Some(params), 0);
        let submit_resp = server.handle_compute_submit(submit_req).await;
        let job_id = submit_resp["result"]["job_id"]
            .as_str()
            .unwrap()
            .to_string();

        let result_params = serde_json::json!({"job_id": job_id});
        let result_req = mk_request("compute.result", Some(result_params), 1);
        let response = server.handle_compute_result(result_req).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("error").is_some());
    }

    // ---- compute.cancel: success and error paths ----
    #[tokio::test]
    async fn test_handle_compute_cancel_success() {
        let server = test_server();
        let params = serde_json::json!({
            "inference": {"model": "x", "prompt": "y", "params": {}}
        });
        let submit_req = mk_request("compute.submit", Some(params), 0);
        let submit_resp = server.handle_compute_submit(submit_req).await;
        let job_id = submit_resp["result"]["job_id"]
            .as_str()
            .unwrap()
            .to_string();

        let cancel_params = serde_json::json!({"job_id": job_id});
        let cancel_req = mk_request("compute.cancel", Some(cancel_params), 1);
        let response = server.handle_compute_cancel(cancel_req).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("result").is_some());
        assert_eq!(obj["result"]["cancelled"], true);
    }

    #[tokio::test]
    async fn test_handle_compute_cancel_missing_job_id() {
        let server = test_server();
        let request = mk_request("compute.cancel", Some(serde_json::json!({})), 1);
        let response = server.handle_compute_cancel(request).await;
        let obj = response.as_object().expect("object");
        assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    }

    #[tokio::test]
    async fn test_handle_compute_cancel_nonexistent_job() {
        let server = test_server();
        let job_id = uuid::Uuid::new_v4();
        let params = serde_json::json!({"job_id": job_id.to_string()});
        let request = mk_request("compute.cancel", Some(params), 1);
        let response = server.handle_compute_cancel(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj.get("error").is_some());
    }

    // ---- compute.list: invalid state filter (becomes None, lists all) ----
    #[tokio::test]
    async fn test_handle_compute_list_invalid_state_filter() {
        let server = test_server();
        let params = serde_json::json!({"state": "invalid_state"});
        let request = mk_request("compute.list", Some(params), 1);
        let response = server.handle_compute_list(request).await;
        let obj = response.as_object().expect("object");
        assert!(obj["result"]["jobs"].is_array());
    }
}
