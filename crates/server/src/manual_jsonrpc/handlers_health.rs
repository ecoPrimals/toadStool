//! Health, version, capabilities, and GPU info handlers

use std::sync::atomic::Ordering;

#[allow(deprecated)]
use toadstool_common::interned_strings::primals;

use crate::gpu_job_queue::{query_gpu_devices, query_gpu_memory};

use super::{
    JsonRpcRequest, JsonRpcResponse, ManualJsonRpcServer, JSONRPC_VERSION, SERIALIZATION_FAILED,
};

impl ManualJsonRpcServer {
    #[allow(deprecated)]
    pub(crate) async fn handle_health(&self, request: JsonRpcRequest) -> serde_json::Value {
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

    pub(crate) async fn handle_version(&self, request: JsonRpcRequest) -> serde_json::Value {
        self.success_response(
            serde_json::json!({"version": self.version, "protocol": "json-rpc-2.0"}),
            &request,
        )
    }

    #[allow(deprecated)]
    pub(crate) async fn handle_discover_capabilities(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
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

    pub(crate) async fn handle_gpu_info(&self, request: JsonRpcRequest) -> serde_json::Value {
        let result = serde_json::json!({
            "devices": query_gpu_devices(),
            "driver": "wgpu",
            "compute_backends": ["vulkan", "metal", "dx12"],
        });
        self.success_response(result, &request)
    }

    pub(crate) async fn handle_gpu_memory(&self, request: JsonRpcRequest) -> serde_json::Value {
        self.success_response(
            serde_json::json!({ "devices": query_gpu_memory() }),
            &request,
        )
    }

    pub(crate) async fn handle_query_capabilities(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
        match self.executor.query_capabilities().await {
            Ok(caps) => {
                let result = serde_json::to_value(caps)
                    .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}));
                self.success_response(result, &request)
            }
            Err(e) => self.error_response(
                super::INTERNAL_ERROR,
                format!("Failed to query capabilities: {e}"),
                &request,
            ),
        }
    }
}
