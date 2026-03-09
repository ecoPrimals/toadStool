// SPDX-License-Identifier: AGPL-3.0-only
//! Workload executor for JSON-RPC handler.

use std::sync::Arc;

use tracing::info;

use crate::pure_jsonrpc::types::{JsonRpcError, JsonWorkloadSubmission};

/// Handles high-level workload execution (toadstool.* namespace).
pub(super) struct WorkloadHandler {
    pub(super) executor: Arc<dyn crate::tarpc_server::WorkloadExecutor + Send + Sync>,
}

impl WorkloadHandler {
    pub(super) fn new(
        executor: Arc<dyn crate::tarpc_server::WorkloadExecutor + Send + Sync>,
    ) -> Self {
        Self { executor }
    }

    pub(super) async fn submit_workload(
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

    pub(super) async fn cancel_workload(
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

    pub(super) async fn query_capabilities(&self) -> Result<serde_json::Value, JsonRpcError> {
        info!("Querying capabilities (self-knowledge)");

        let caps = self
            .executor
            .query_capabilities()
            .await
            .map_err(JsonRpcError::internal_error)?;

        serde_json::to_value(caps)
            .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))
    }
}
