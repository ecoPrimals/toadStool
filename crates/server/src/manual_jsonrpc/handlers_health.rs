// SPDX-License-Identifier: AGPL-3.0-or-later
//! Health, version, capabilities, and GPU info handlers

use std::sync::atomic::Ordering;

use toadstool_common::constants::PRIMAL_NAME;

use crate::gpu_system::{query_gpu_devices, query_gpu_memory};

use super::{
    JsonRpcRequest, JsonRpcResponse, ManualJsonRpcServer, JSONRPC_VERSION, SERIALIZATION_FAILED,
};

impl ManualJsonRpcServer {
    #[allow(clippy::unused_async)] // JSON-RPC handler; async for API consistency
    pub(crate) async fn handle_health(&self, request: JsonRpcRequest) -> serde_json::Value {
        self.success_response(
            serde_json::json!({
                "healthy": true,
                "service": PRIMAL_NAME,
                "version": self.version,
                "error_count": self.error_count.load(Ordering::Relaxed),
                "uptime_secs": self.start_time.elapsed().as_secs(),
            }),
            &request,
        )
    }

    #[allow(clippy::unused_async)] // JSON-RPC handler; async for API consistency
    pub(crate) async fn handle_version(&self, request: JsonRpcRequest) -> serde_json::Value {
        self.success_response(
            serde_json::json!({"version": self.version, "protocol": "json-rpc-2.0"}),
            &request,
        )
    }

    #[allow(clippy::unused_async)] // JSON-RPC handler; async for API consistency
    pub(crate) async fn handle_discover_capabilities(
        &self,
        request: JsonRpcRequest,
    ) -> serde_json::Value {
        let capabilities = serde_json::json!({
            // Semantic capabilities (biomeOS node_atomic_compute.toml)
            "node_capabilities": [
                "compute", "workload", "orchestration", "ai_local",
                "gpu", "wasm", "container"
            ],
            // Concrete JSON-RPC methods on this socket
            "methods": [
                // Canonical toadstool.* namespace
                "toadstool.health",
                "toadstool.version",
                "toadstool.query_capabilities",
                "toadstool.resources.estimate",
                "toadstool.resources.validate_availability",
                "toadstool.resources.suggest_optimizations",
                // biomeOS node_atomic_compute.toml aliases (no namespace prefix)
                "resources.estimate",
                "resources.validate_availability",
                "resources.suggest_optimizations",
                // compute.* (biomeOS canonical + GPU job queue)
                "compute.health",
                "compute.version",
                "compute.capabilities",
                "compute.discover_capabilities",
                "compute.submit",
                "compute.status",
                "compute.result",
                "compute.cancel",
                "compute.list",
                // ai.* (biomeOS ai_local capability)
                "ai.local_inference",
                "ai.local_execute",
                // gpu.* (hardware info)
                "gpu.info",
                "gpu.memory",
                // ollama.* (local LLM)
                "ollama.list_models",
                "ollama.inference",
                "ollama.load",
                "ollama.unload",
                // gate.* (distributed routing)
                "gate.update",
                "gate.remove",
                "gate.list",
                "gate.route"
            ],
            "version": self.version,
            "primal": PRIMAL_NAME
        });

        serde_json::to_value(JsonRpcResponse {
            jsonrpc: JSONRPC_VERSION.clone(),
            result: capabilities,
            id: request.id,
        })
        .unwrap_or_else(|_| serde_json::json!({"error": SERIALIZATION_FAILED}))
    }

    #[allow(clippy::unused_async)] // JSON-RPC handler; sync query_gpu_devices()
    pub(crate) async fn handle_gpu_info(&self, request: JsonRpcRequest) -> serde_json::Value {
        let result = serde_json::json!({
            "devices": query_gpu_devices(),
            "driver": "wgpu",
            "compute_backends": ["vulkan", "metal", "dx12"],
        });
        self.success_response(result, &request)
    }

    #[allow(clippy::unused_async)] // JSON-RPC handler; sync query_gpu_memory()
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
