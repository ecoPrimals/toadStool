// SPDX-License-Identifier: AGPL-3.0-or-later

use super::DispatchHandler;
use super::types::DispatchStatus;
use crate::pure_jsonrpc::types::JsonRpcError;

impl DispatchHandler {
    pub async fn dispatch_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = params
            .and_then(|p| p.get("job_id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'job_id'"))?;

        let jobs = self.jobs.read().await;
        let job = jobs.get(job_id).ok_or_else(|| {
            JsonRpcError::internal_error(format!("Dispatch job {job_id} not found"))
        })?;

        let (status_str, error_str) = match &job.status {
            DispatchStatus::Failed(msg) => ("failed", Some(msg.clone())),
            other => (other.as_str(), None),
        };

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "status",
            "job_id": job_id,
            "status": status_str,
            "output": null,
            "error": error_str,
            "metadata": {
                "bdf": job.bdf,
                "binary_size": job.binary_size,
                "elapsed_ms": job.submitted_at.elapsed().as_millis() as u64,
            },
        }))
    }

    pub async fn dispatch_result(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let job_id = params
            .and_then(|p| p.get("job_id"))
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| JsonRpcError::invalid_params("Missing 'job_id'"))?;

        let jobs = self.jobs.read().await;
        let job = jobs.get(job_id).ok_or_else(|| {
            JsonRpcError::internal_error(format!("Dispatch job {job_id} not found"))
        })?;

        let (status_str, error_str) = match &job.status {
            DispatchStatus::Failed(msg) => ("failed", Some(msg.clone())),
            other => (other.as_str(), None),
        };

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "result",
            "job_id": job_id,
            "status": status_str,
            "output": job.result,
            "error": error_str,
            "metadata": {},
        }))
    }
}
