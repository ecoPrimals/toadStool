//! JSON-RPC request handler and method router
//!
//! Routes JSON-RPC 2.0 requests to the appropriate executor or job queue.
//! Semantic method names are resolved through `SemanticMethodRegistry`
//! before dispatch, enabling both legacy `toadstool.*` names and the
//! standard `{domain}.{operation}` naming convention.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use toadstool::semantic_methods::SemanticMethodRegistry;
use tracing::{debug, error, info};

use crate::cross_gate::JobRouter;
use crate::gpu_job_queue::{GpuJobQueue, JobQueueConfig, JobQueueError};
use crate::ollama::{OllamaClient, OllamaConfig};
use crate::resource_estimator::ResourceEstimator;
use crate::resource_optimizer::ResourceOptimizer;
use crate::resource_validator::ResourceValidator;
use crate::rpc_types::HealthStatus;

use super::types::{
    JsonRpcError, JsonRpcRequest, JsonRpcResponse, JsonWorkloadSubmission, JSONRPC_VERSION,
};

/// Pure Rust JSON-RPC Handler
///
/// Routes requests to appropriate methods. Supports both legacy `toadstool.*`
/// names and semantic `{domain}.{operation}` names via the registry.
pub struct JsonRpcHandler {
    executor: Arc<dyn super::super::tarpc_server::WorkloadExecutor + Send + Sync>,
    version: String,
    start_time: std::time::Instant,
    job_queue: crate::gpu_job_queue::GpuJobQueue,
    error_count: Arc<AtomicU64>,
    /// Resolves semantic method names to implementation names for dispatch
    semantic_registry: SemanticMethodRegistry,
    estimator: ResourceEstimator,
    validator: ResourceValidator,
    optimizer: ResourceOptimizer,
    ollama: OllamaClient,
    router: Arc<tokio::sync::RwLock<JobRouter>>,
}

impl JsonRpcHandler {
    /// Create new handler with executor.
    ///
    /// Pass `error_count` to share the counter with other servers for unified monitoring.
    pub fn new(
        executor: Arc<dyn super::super::tarpc_server::WorkloadExecutor + Send + Sync>,
        version: String,
        error_count: Option<Arc<AtomicU64>>,
    ) -> Self {
        let local_gate_id = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("TOADSTOOL_GATE_ID"))
            .or_else(|_| std::fs::read_to_string("/etc/hostname").map(|h| h.trim().to_string()))
            .unwrap_or_else(|_| "local".to_string());
        Self {
            executor,
            version,
            start_time: std::time::Instant::now(),
            job_queue: GpuJobQueue::new(JobQueueConfig::default()),
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
            semantic_registry: SemanticMethodRegistry::new(),
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
            ollama: OllamaClient::new(OllamaConfig::default()),
            router: Arc::new(tokio::sync::RwLock::new(JobRouter::new(local_gate_id))),
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
        // Direct dispatch for all currently registered literal names
        match method {
            // High-level workload executor (`toadstool.*` namespace)
            "toadstool.submit_workload" => return self.submit_workload(params).await,
            "toadstool.query_status" => return self.query_status(params).await,
            "toadstool.cancel_workload" => return self.cancel_workload(params).await,
            "toadstool.list_workloads" => return self.list_workloads(params).await,
            "toadstool.query_capabilities" => return self.query_capabilities().await,
            "toadstool.health" => return self.health().await,
            "toadstool.version" => return self.version_info().await,

            // Resource handlers (estimate, validate, optimize)
            "toadstool.resources.estimate" | "resources.estimate" | "ai.local_inference" => {
                return self.resources_estimate(params).await
            }
            "toadstool.resources.validate_availability"
            | "resources.validate_availability"
            | "ai.local_execute" => return self.resources_validate_availability(params).await,
            "toadstool.resources.suggest_optimizations" | "resources.suggest_optimizations" => {
                return self.resources_suggest_optimizations(params).await
            }

            // compute.* health/version/capabilities
            "compute.health" => return self.health().await,
            "compute.version" => return self.version_info().await,
            "compute.capabilities" => return self.query_capabilities().await,
            "compute.discover_capabilities" => return self.discover_capabilities().await,

            // GPU job-queue (`compute.*` namespace — distinct from workload executor)
            "compute.submit" => return self.compute_submit(params).await,
            "compute.status" => return self.compute_status(params).await,
            "compute.result" => return self.compute_result(params).await,
            "compute.cancel" => return self.compute_cancel(params).await,
            "compute.list" => return self.compute_list(params).await,

            // GPU info handlers
            "gpu.info" => return self.gpu_info().await,
            "gpu.memory" => return self.gpu_memory().await,

            // Ollama handlers
            "ollama.list_models" => return self.ollama_list_models().await,
            "ollama.inference" => return self.ollama_inference(params).await,
            "ollama.load" => return self.ollama_load(params).await,
            "ollama.unload" => return self.ollama_unload(params).await,

            // Gate/cluster handlers
            "gate.update" => return self.gate_update(params).await,
            "gate.remove" => return self.gate_remove(params).await,
            "gate.list" => return self.gate_list().await,
            "gate.route" => return self.gate_route(params).await,

            _ => {}
        }

        // Semantic registry resolution: dispatch by implementation name
        if let Some(impl_name) = self.semantic_registry.resolve(method) {
            debug!("Semantic resolve: {} → {}", method, impl_name);
            return self.dispatch_by_impl_name(impl_name, params).await;
        }

        Err(JsonRpcError::method_not_found(method))
    }

    /// Dispatch using the implementation name returned by the semantic registry.
    ///
    /// Implementation names correspond 1-to-1 with handler methods, allowing
    /// semantic aliases like `runtime.workload.submit` to reach the same handler
    /// as the legacy `toadstool.submit_workload`.
    async fn dispatch_by_impl_name(
        &self,
        impl_name: &str,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        match impl_name {
            "execute_workload" | "submit_workload" => self.submit_workload(params).await,
            "get_workload_status" | "query_status" => self.query_status(params).await,
            "cancel_workload" => self.cancel_workload(params).await,
            "list_workloads" => self.list_workloads(params).await,
            "query_capabilities" => self.query_capabilities().await,
            _ => Err(JsonRpcError::method_not_found(impl_name)),
        }
    }

    // ── High-level workload executor methods ─────────────────────────────────

    async fn submit_workload(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let submission: JsonWorkloadSubmission = serde::Deserialize::deserialize(params)
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid params: {e}")))?;

        info!("Submitting workload: {}", submission.workload_id);

        let tarpc_submission = submission
            .into_tarpc()
            .map_err(JsonRpcError::invalid_params)?;

        let result = self
            .executor
            .execute(tarpc_submission)
            .await
            .map_err(JsonRpcError::internal_error)?;

        serde_json::to_value(result)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))
    }

    async fn query_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let workload_id = params
            .as_str()
            .ok_or_else(|| JsonRpcError::invalid_params("workload_id must be a string"))?;

        info!("Querying status: {}", workload_id);

        let job_id = uuid::Uuid::parse_str(workload_id)
            .map_err(|_| JsonRpcError::invalid_params("Invalid job ID format"))?;

        match self.job_queue.status(job_id).await {
            Ok(job) => serde_json::to_value(job)
                .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}"))),
            Err(e) => Err(JsonRpcError::internal_error(e.to_string())),
        }
    }

    async fn cancel_workload(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let workload_id = params
            .as_str()
            .ok_or_else(|| JsonRpcError::invalid_params("workload_id must be a string"))?;

        info!("Canceling workload: {}", workload_id);

        self.executor
            .cancel(workload_id)
            .await
            .map_err(JsonRpcError::internal_error)?;

        Ok(serde_json::json!({"success": true}))
    }

    async fn list_workloads(
        &self,
        _params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        info!("Listing workloads");

        let jobs = self.job_queue.list(None).await;
        let counts = self.job_queue.counts().await;

        Ok(serde_json::json!({
            "jobs": jobs,
            "counts": counts,
        }))
    }

    async fn query_capabilities(&self) -> Result<serde_json::Value, JsonRpcError> {
        info!("Querying capabilities (self-knowledge)");

        let caps = self
            .executor
            .query_capabilities()
            .await
            .map_err(JsonRpcError::internal_error)?;

        serde_json::to_value(caps)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))
    }

    async fn health(&self) -> Result<serde_json::Value, JsonRpcError> {
        let uptime = self.start_time.elapsed();

        let status = HealthStatus {
            healthy: true,
            version: self.version.clone(),
            uptime_secs: uptime.as_secs(),
            active_workloads: 0,
            queued_workloads: 0,
            error_count: self.error_count.load(Ordering::Relaxed) as usize,
            resource_utilization: 0.0,
        };

        serde_json::to_value(status)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))
    }

    async fn version_info(&self) -> Result<serde_json::Value, JsonRpcError> {
        let mut info = HashMap::new();
        info.insert("version".to_string(), self.version.clone());
        info.insert("protocol".to_string(), "JSON-RPC 2.0".to_string());
        info.insert("service".to_string(), "ToadStool Compute".to_string());
        info.insert(
            "implementation".to_string(),
            "Pure Rust (BearDog pattern)".to_string(),
        );

        Ok(serde_json::json!(info))
    }

    // ── GPU compute job-queue methods (`compute.*`) ───────────────────────────

    async fn compute_submit(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;

        let priority = params
            .get("priority")
            .and_then(serde_json::Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0);
        let vram_hint = params
            .get("vram_required_mb")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(4096);

        let job_type: crate::gpu_job_queue::JobType = serde::Deserialize::deserialize(params)
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid job type: {e}")))?;

        let routing = {
            let router = self.router.read().await;
            let model = match &job_type {
                crate::gpu_job_queue::JobType::Inference { model, .. } => model.as_str(),
                _ => "",
            };
            router.route(model, vram_hint)
        };

        match self.job_queue.submit(job_type, priority).await {
            Ok(job_id) => Ok(serde_json::json!({
                "job_id": job_id,
                "routing": {
                    "gate_id": routing.gate_id,
                    "reason": routing.reason,
                    "estimated_wait_ms": routing.estimated_wait_ms,
                }
            })),
            Err(e) => Err(self.job_queue_error(e)),
        }
    }

    async fn compute_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = self.extract_job_id(params)?;
        match self.job_queue.status(job_id).await {
            Ok(job) => serde_json::to_value(job)
                .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}"))),
            Err(e) => Err(self.job_queue_error(e)),
        }
    }

    async fn compute_result(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = self.extract_job_id(params)?;
        self.job_queue
            .result(job_id)
            .await
            .map_err(|e| self.job_queue_error(e))
    }

    async fn compute_cancel(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = self.extract_job_id(params)?;
        self.job_queue
            .cancel(job_id)
            .await
            .map(|()| serde_json::json!({"cancelled": true}))
            .map_err(|e| self.job_queue_error(e))
    }

    fn job_queue_error(&self, err: JobQueueError) -> JsonRpcError {
        let code = match &err {
            JobQueueError::JobNotFound { .. } => JsonRpcError::METHOD_NOT_FOUND,
            _ => JsonRpcError::INTERNAL_ERROR,
        };
        JsonRpcError {
            code,
            message: Cow::Owned(err.to_string()),
            data: None,
        }
    }

    fn extract_graph(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<crate::graph_types::ExecutionGraph, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let graph_value = params
            .get("graph")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        serde_json::from_value(graph_value)
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid graph parameter: {e}")))
    }

    async fn resources_estimate(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let graph = self.extract_graph(params)?;
        self.estimator
            .estimate(&graph)
            .map_err(|e| JsonRpcError::internal_error(format!("Estimation failed: {e}")))
            .and_then(|estimate| {
                serde_json::to_value(estimate)
                    .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}")))
            })
    }

    async fn resources_validate_availability(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let graph = self.extract_graph(params)?;
        self.validator
            .validate_availability(&graph)
            .await
            .map_err(|e| JsonRpcError::internal_error(format!("Validation failed: {e}")))
            .and_then(|result| {
                serde_json::to_value(result)
                    .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}")))
            })
    }

    async fn resources_suggest_optimizations(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let graph = self.extract_graph(params)?;
        self.optimizer
            .suggest_optimizations(&graph)
            .await
            .map_err(|e| JsonRpcError::internal_error(format!("Optimization failed: {e}")))
            .and_then(|suggestions| {
                serde_json::to_value(suggestions)
                    .map_err(|e| JsonRpcError::internal_error(format!("Serialization: {e}")))
            })
    }

    async fn discover_capabilities(&self) -> Result<serde_json::Value, JsonRpcError> {
        let capabilities = serde_json::json!({
            "node_capabilities": [
                "compute", "workload", "orchestration", "ai_local",
                "gpu", "wasm", "container"
            ],
            "methods": [
                "toadstool.health", "toadstool.version", "toadstool.query_capabilities",
                "toadstool.resources.estimate", "toadstool.resources.validate_availability",
                "toadstool.resources.suggest_optimizations",
                "resources.estimate", "resources.validate_availability", "resources.suggest_optimizations",
                "compute.health", "compute.version", "compute.capabilities",
                "compute.discover_capabilities", "compute.submit", "compute.status",
                "compute.result", "compute.cancel", "compute.list",
                "ai.local_inference", "ai.local_execute",
                "gpu.info", "gpu.memory",
                "ollama.list_models", "ollama.inference", "ollama.load", "ollama.unload",
                "gate.update", "gate.remove", "gate.list", "gate.route"
            ],
            "version": self.version,
            "primal": toadstool_common::constants::PRIMAL_NAME
        });
        Ok(capabilities)
    }

    async fn gpu_info(&self) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "devices": crate::gpu_system::query_gpu_devices(),
            "driver": "wgpu",
            "compute_backends": ["vulkan", "metal", "dx12"],
        }))
    }

    async fn gpu_memory(&self) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "devices": crate::gpu_system::query_gpu_memory(),
        }))
    }

    async fn ollama_list_models(&self) -> Result<serde_json::Value, JsonRpcError> {
        self.ollama
            .list_models()
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))
            .map(|models| serde_json::json!({"models": models}))
    }

    async fn ollama_inference(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let model = params
            .get("model")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'model' param"))?;
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'prompt' param"))?;
        let extra_params = params
            .get("params")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        self.ollama
            .inference(model, prompt, &extra_params)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))
    }

    async fn ollama_load(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let model = params
            .and_then(|p| p.get("model"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'model' param"))?;
        self.ollama
            .load(model)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))
            .map(|()| serde_json::json!({"loaded": true, "model": model}))
    }

    async fn ollama_unload(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let model = params
            .and_then(|p| p.get("model"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'model' param"))?;
        self.ollama
            .unload(model)
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))
            .map(|()| serde_json::json!({"unloaded": true, "model": model}))
    }

    async fn gate_update(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let gate_info: crate::cross_gate::GateGpuInfo = serde_json::from_value(params.clone())
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid gate info: {e}")))?;
        let gate_id = gate_info.gate_id.clone();
        self.router.write().await.update_gate(gate_info);
        Ok(serde_json::json!({"updated": true, "gate_id": gate_id}))
    }

    async fn gate_remove(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let gate_id = params
            .and_then(|p| p.get("gate_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'gate_id' param"))?;
        self.router.write().await.remove_gate(gate_id);
        Ok(serde_json::json!({"removed": true, "gate_id": gate_id}))
    }

    async fn gate_list(&self) -> Result<serde_json::Value, JsonRpcError> {
        let router = self.router.read().await;
        let gates: Vec<&crate::cross_gate::GateGpuInfo> = router.gates().values().collect();
        Ok(serde_json::json!({"gates": gates}))
    }

    async fn gate_route(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("");
        let vram = params
            .get("vram_required_mb")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(4096);
        let router = self.router.read().await;
        let decision = router.route(model, vram);
        Ok(serde_json::json!({
            "gate_id": decision.gate_id,
            "reason": decision.reason,
            "estimated_wait_ms": decision.estimated_wait_ms,
        }))
    }

    async fn compute_list(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let state_filter = params
            .and_then(|p| p.get("state"))
            .and_then(|v| serde::Deserialize::deserialize(v).ok());
        let jobs = self.job_queue.list(state_filter).await;
        let counts = self.job_queue.counts().await;
        Ok(serde_json::json!({"jobs": jobs, "counts": counts}))
    }

    fn extract_job_id(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<uuid::Uuid, JsonRpcError> {
        let params = params.ok_or_else(|| JsonRpcError::invalid_params("Missing params"))?;
        let id_str = params
            .get("job_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'job_id'"))?;
        uuid::Uuid::parse_str(id_str)
            .map_err(|_| JsonRpcError::invalid_params("Invalid job_id UUID"))
    }
}
