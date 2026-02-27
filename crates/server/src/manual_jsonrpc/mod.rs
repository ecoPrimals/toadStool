//! # Pure Manual JSON-RPC 2.0 Server over Unix Sockets
//!
//! **DEPRECATED** (since 2.2.0): Use `pure_jsonrpc::JsonRpcHandler` for new code.
//! This module is phased out in favor of `pure_jsonrpc`, which has SemanticMethodRegistry,
//! proper error types, and Cow<'static, str> for zero-copy version strings.
//!
//! See [MIGRATION.md](MIGRATION.md) for migration path. Do not delete — unibin still uses it.
//!
//! Educational implementation for other primals to learn from.
//! No library dependencies - just tokio, serde_json, and the JSON-RPC 2.0 spec.

mod connection;
mod handlers_cluster;
mod handlers_exec;
mod handlers_health;

#[cfg(test)]
#[allow(deprecated)]
mod tests;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::cross_gate::JobRouter;
use crate::gpu_job_queue::{GpuJobQueue, JobQueueConfig, JobQueueError};
use crate::ollama::{OllamaClient, OllamaConfig};
use crate::resource_estimator::ResourceEstimator;
use crate::resource_optimizer::ResourceOptimizer;
use crate::resource_validator::ResourceValidator;
use crate::tarpc_server::WorkloadExecutor;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Response (Success)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: Cow<'static, str>,
    pub result: Value,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: Cow<'static, str>,
    pub error: JsonRpcError,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: Cow<'static, str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Zero-copy JSON-RPC version for responses (always "2.0")
pub(crate) const JSONRPC_VERSION: Cow<'static, str> =
    Cow::Borrowed(toadstool_common::constants::jsonrpc::VERSION);

/// Fallback error message for serialization failures
pub(crate) const SERIALIZATION_FAILED: &str = "serialization failed";

// JSON-RPC 2.0 Error Codes
pub use toadstool_common::constants::jsonrpc::error_codes::{
    INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
};

/// Manual JSON-RPC 2.0 Server over Unix Sockets
pub struct ManualJsonRpcServer {
    pub(crate) executor: Arc<dyn WorkloadExecutor + Send + Sync>,
    pub(crate) version: String,
    pub(crate) estimator: ResourceEstimator,
    pub(crate) validator: ResourceValidator,
    pub(crate) optimizer: ResourceOptimizer,
    pub(crate) job_queue: GpuJobQueue,
    pub(crate) ollama: OllamaClient,
    pub(crate) router: Arc<tokio::sync::RwLock<JobRouter>>,
    pub(crate) error_count: Arc<AtomicU64>,
    pub(crate) start_time: std::time::Instant,
}

impl Clone for ManualJsonRpcServer {
    fn clone(&self) -> Self {
        Self {
            executor: Arc::clone(&self.executor),
            version: self.version.clone(),
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
            job_queue: self.job_queue.clone(),
            ollama: self.ollama.clone(),
            router: Arc::clone(&self.router),
            error_count: Arc::clone(&self.error_count),
            start_time: self.start_time,
        }
    }
}

impl ManualJsonRpcServer {
    /// Create new manual JSON-RPC server
    pub fn new(
        executor: Arc<dyn WorkloadExecutor + Send + Sync>,
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
            estimator: ResourceEstimator::new(),
            validator: ResourceValidator::new(),
            optimizer: ResourceOptimizer::new(),
            job_queue: GpuJobQueue::new(JobQueueConfig::default()),
            ollama: OllamaClient::new(OllamaConfig::default()),
            router: Arc::new(tokio::sync::RwLock::new(JobRouter::new(local_gate_id))),
            error_count: error_count.unwrap_or_else(|| Arc::new(AtomicU64::new(0))),
            start_time: std::time::Instant::now(),
        }
    }

    /// Handle JSON-RPC request
    pub(crate) async fn handle_jsonrpc_request(&self, request: JsonRpcRequest) -> Value {
        if request.jsonrpc != toadstool_common::constants::jsonrpc::VERSION {
            return self.error_response(INVALID_REQUEST, "Invalid jsonrpc version", &request);
        }

        match request.method.as_str() {
            // ── Canonical toadstool.* methods ────────────────────────────────
            "toadstool.health" => self.handle_health(request).await,
            "toadstool.version" => self.handle_version(request).await,
            "toadstool.query_capabilities" => self.handle_query_capabilities(request).await,
            "toadstool.resources.estimate" => self.handle_resources_estimate(request).await,
            "toadstool.resources.validate_availability" => {
                self.handle_resources_validate_availability(request).await
            }
            "toadstool.resources.suggest_optimizations" => {
                self.handle_resources_suggest_optimizations(request).await
            }
            // ── biomeOS Node Atomic aliases (node_atomic_compute.toml) ───────
            // The biomeOS neural API translates capability calls before routing:
            //   compute.estimate    → resources.estimate
            //   compute.validate    → resources.validate_availability
            //   compute.optimize    → resources.suggest_optimizations
            //   ai.local_inference  → resources.estimate
            //   ai.local_execute    → resources.validate_availability
            "resources.estimate" | "ai.local_inference" => {
                self.handle_resources_estimate(request).await
            }
            "resources.validate_availability" | "ai.local_execute" => {
                self.handle_resources_validate_availability(request).await
            }
            "resources.suggest_optimizations" => {
                self.handle_resources_suggest_optimizations(request).await
            }
            // ── compute.* methods ────────────────────────────────────────────
            "compute.health" => self.handle_health(request).await,
            "compute.version" => self.handle_version(request).await,
            "compute.capabilities" => self.handle_query_capabilities(request).await,
            "compute.discover_capabilities" => self.handle_discover_capabilities(request).await,
            "compute.submit" => self.handle_compute_submit(request).await,
            "compute.status" => self.handle_compute_status(request).await,
            "compute.result" => self.handle_compute_result(request).await,
            "compute.cancel" => self.handle_compute_cancel(request).await,
            "compute.list" => self.handle_compute_list(request).await,
            "gpu.info" => self.handle_gpu_info(request).await,
            "gpu.memory" => self.handle_gpu_memory(request).await,
            "ollama.list_models" => self.handle_ollama_list_models(request).await,
            "ollama.inference" => self.handle_ollama_inference(request).await,
            "ollama.load" => self.handle_ollama_load(request).await,
            "ollama.unload" => self.handle_ollama_unload(request).await,
            "gate.update" => self.handle_gate_update(request).await,
            "gate.remove" => self.handle_gate_remove(request).await,
            "gate.list" => self.handle_gate_list(request).await,
            "gate.route" => self.handle_gate_route(request).await,
            _ => self.error_response(
                METHOD_NOT_FOUND,
                format!("Method not found: {}", request.method),
                &request,
            ),
        }
    }

    /// Extract job_id from request params
    pub(crate) fn extract_job_id(&self, request: &JsonRpcRequest) -> Result<uuid::Uuid, Value> {
        let job_id_str = request
            .params
            .as_ref()
            .and_then(|p| p.get("job_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                self.error_response(INVALID_PARAMS, "Missing 'job_id' param", request)
            })?;

        uuid::Uuid::parse_str(job_id_str)
            .map_err(|_| self.error_response(INVALID_PARAMS, "Invalid job_id UUID", request))
    }

    /// Build a success JSON-RPC response
    pub(crate) fn success_response(&self, result: Value, request: &JsonRpcRequest) -> Value {
        serde_json::to_value(JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            result,
            id: request.id.clone(),
        })
        .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}))
    }

    /// Build an error JSON-RPC response
    pub(crate) fn error_response(
        &self,
        code: i32,
        message: impl Into<Cow<'static, str>>,
        request: &JsonRpcRequest,
    ) -> Value {
        self.error_count.fetch_add(1, Ordering::Relaxed);
        serde_json::to_value(JsonRpcErrorResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            error: JsonRpcError {
                code,
                message: message.into(),
                data: None,
            },
            id: request.id.clone(),
        })
        .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}))
    }

    /// Map job queue errors to appropriate JSON-RPC error codes
    pub(crate) fn job_queue_error_response(
        &self,
        err: JobQueueError,
        request: &JsonRpcRequest,
    ) -> Value {
        let code = match &err {
            JobQueueError::JobNotFound { .. } => METHOD_NOT_FOUND,
            JobQueueError::QueueFull { .. } => INTERNAL_ERROR,
            _ => INTERNAL_ERROR,
        };
        self.error_response(code, err.to_string(), request)
    }
}
