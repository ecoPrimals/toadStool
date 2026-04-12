// SPDX-License-Identifier: AGPL-3.0-or-later

use super::DispatchHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

impl DispatchHandler {
    pub async fn dispatch_forward(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params
            .ok_or_else(|| JsonRpcError::invalid_params("Expected { endpoint, binary, ... }"))?;

        let endpoint = p
            .get("endpoint")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'endpoint'"))?;

        let forward_params = p.get("params").cloned().unwrap_or_else(|| p.clone());

        match crate::cross_gate::RemoteDispatcher::forward(
            endpoint,
            "compute.dispatch.submit",
            forward_params,
        )
        .await
        {
            Ok(result) => Ok(serde_json::json!({
                "domain": "compute.dispatch",
                "operation": "forward",
                "job_id": null,
                "status": "completed",
                "output": result,
                "error": null,
                "metadata": {
                    "endpoint": endpoint,
                },
            })),
            Err(e) => Err(JsonRpcError::internal_error(format!(
                "Remote dispatch to {endpoint} failed: {e}"
            ))),
        }
    }
}
