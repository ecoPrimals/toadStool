// SPDX-License-Identifier: AGPL-3.0-only
//! Ollama client integration for JSON-RPC handler.

use crate::ollama::{OllamaClient, OllamaConfig};

use crate::pure_jsonrpc::types::JsonRpcError;

/// Handles Ollama model listing and inference.
pub(super) struct OllamaHandler {
    pub(super) ollama: OllamaClient,
}

impl OllamaHandler {
    pub(super) fn new() -> Self {
        Self {
            ollama: OllamaClient::new(OllamaConfig::default()),
        }
    }

    pub(super) async fn ollama_list_models(&self) -> Result<serde_json::Value, JsonRpcError> {
        self.ollama
            .list_models()
            .await
            .map_err(|e| JsonRpcError::internal_error(e.to_string()))
            .map(|models| serde_json::json!({"models": models}))
    }

    pub(super) async fn ollama_inference(
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

    pub(super) async fn ollama_load(
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

    pub(super) async fn ollama_unload(
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
}
