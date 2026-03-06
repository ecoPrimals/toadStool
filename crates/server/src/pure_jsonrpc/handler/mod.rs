// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC request handler and method router
//!
//! Routes JSON-RPC 2.0 requests to the appropriate executor or job queue.
//! Semantic method names are resolved through `SemanticMethodRegistry`
//! before dispatch, enabling both legacy `toadstool.*` names and the
//! standard `{domain}.{operation}` naming convention.

mod job;
mod ollama;
mod resources;
mod transport;
mod workload;

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use toadstool::semantic_methods::SemanticMethodRegistry;
use tracing::{debug, error, info};

use crate::rpc_types::HealthStatus;

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
            "toadstool.health" => return self.health().await,
            "toadstool.version" => return self.version_info().await,

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

            "compute.health" => return self.health().await,
            "compute.version" => return self.version_info().await,
            "compute.capabilities" => return self.workload.query_capabilities().await,
            "compute.discover_capabilities" => return self.discover_capabilities().await,

            "compute.submit" => return self.job.compute_submit(params).await,
            "compute.status" => return self.job.compute_status(params).await,
            "compute.result" => return self.job.compute_result(params).await,
            "compute.cancel" => return self.job.compute_cancel(params).await,
            "compute.list" => return self.job.compute_list(params).await,

            "gpu.info" => return self.gpu_info().await,
            "gpu.memory" => return self.gpu_memory().await,

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
            _ => Err(JsonRpcError::method_not_found(impl_name)),
        }
    }

    #[allow(clippy::unused_async)]
    async fn health(&self) -> Result<serde_json::Value, JsonRpcError> {
        let uptime = self.start_time.elapsed();
        #[allow(clippy::cast_possible_truncation)]
        let error_count = self.error_count.load(Ordering::Relaxed) as usize;
        let status = HealthStatus {
            healthy: true,
            version: self.version.to_string(),
            uptime_secs: uptime.as_secs(),
            active_workloads: 0,
            queued_workloads: 0,
            error_count,
            resource_utilization: 0.0,
        };
        serde_json::to_value(status)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))
    }

    #[allow(clippy::unused_async)]
    async fn version_info(&self) -> Result<serde_json::Value, JsonRpcError> {
        let mut info = HashMap::new();
        info.insert("version".to_string(), self.version.to_string());
        info.insert("protocol".to_string(), "JSON-RPC 2.0".to_string());
        info.insert("service".to_string(), "ToadStool Compute".to_string());
        info.insert(
            "implementation".to_string(),
            "Pure Rust (ecoPrimals sovereign pattern)".to_string(),
        );
        Ok(serde_json::json!(info))
    }

    #[allow(clippy::unused_async)]
    async fn discover_capabilities(&self) -> Result<serde_json::Value, JsonRpcError> {
        let capabilities = serde_json::json!({
            "node_capabilities": [
                "compute", "workload", "orchestration", "ai_local",
                "gpu", "wasm", "container", "hardware_transport"
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
                "gate.update", "gate.remove", "gate.list", "gate.route",
                "transport.discover", "transport.list", "transport.route"
            ],
            "version": self.version,
            "primal": toadstool_common::constants::PRIMAL_NAME
        });
        Ok(capabilities)
    }

    #[allow(clippy::unused_async)]
    async fn gpu_info(&self) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "devices": crate::gpu_system::query_gpu_devices(),
            "driver": "wgpu",
            "compute_backends": ["vulkan", "metal", "dx12"],
        }))
    }

    #[allow(clippy::unused_async)]
    async fn gpu_memory(&self) -> Result<serde_json::Value, JsonRpcError> {
        Ok(serde_json::json!({
            "devices": crate::gpu_system::query_gpu_memory(),
        }))
    }
}
