// SPDX-License-Identifier: AGPL-3.0-only
//! Sovereign dispatch handler — accepts compiled GPU binaries from coralReef
//! and routes them to the target GPU via VFIO or DRM.
//!
//! This is the missing link in the sovereign compute pipeline:
//! barraCuda WGSL → coralReef compile → **toadStool dispatch** → GPU result

use crate::coral_reef_client::SharedCoralReefClient;
use crate::pure_jsonrpc::types::JsonRpcError;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

/// Tracks an in-flight dispatch job.
#[derive(Debug, Clone)]
struct DispatchJob {
    #[expect(
        dead_code,
        reason = "stored for logging/diagnostics in dispatch pipeline"
    )]
    id: String,
    bdf: String,
    status: DispatchStatus,
    submitted_at: std::time::Instant,
    binary_size: usize,
    result: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DispatchStatus {
    Submitted,
    #[expect(
        dead_code,
        reason = "used once VFIO dispatch pipeline tracks in-flight jobs"
    )]
    Running,
    Completed,
    Failed(String),
}

impl std::fmt::Display for DispatchStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Submitted => write!(f, "submitted"),
            Self::Running => write!(f, "running"),
            Self::Completed => write!(f, "completed"),
            Self::Failed(msg) => write!(f, "failed: {msg}"),
        }
    }
}

/// Handler for `compute.dispatch.*` JSON-RPC methods.
pub struct DispatchHandler {
    coral_client: SharedCoralReefClient,
    jobs: Arc<RwLock<HashMap<String, DispatchJob>>>,
    dispatch_count: AtomicU64,
}

impl DispatchHandler {
    pub fn new(coral_client: SharedCoralReefClient) -> Self {
        Self {
            coral_client,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            dispatch_count: AtomicU64::new(0),
        }
    }

    /// `compute.dispatch.submit` — Accept a compiled binary and dispatch to GPU.
    pub async fn dispatch_submit(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params(
                "Expected { binary, bdf?, workgroup_size?, buffers?, dispatch_mode?, timeout_ms? }",
            )
        })?;

        let binary = p.get("binary").and_then(|v| v.as_array()).ok_or_else(|| {
            JsonRpcError::invalid_params("Missing 'binary' array (compiled GPU binary bytes)")
        })?;

        let binary_bytes: Vec<u8> = binary
            .iter()
            .map(|v| v.as_u64().unwrap_or(0) as u8)
            .collect();

        if binary_bytes.is_empty() {
            return Err(JsonRpcError::invalid_params("binary must not be empty"));
        }

        let bdf = resolve_dispatch_bdf(p)?;
        let dispatch_mode = detect_dispatch_mode(p, &bdf);

        let thermal = super::hw_learn::check_thermal_for_bdf_pub(&bdf);
        if let Some(ref status) = thermal
            && !status.compute_safe()
        {
            return Err(JsonRpcError::internal_error(format!(
                "GPU {bdf} thermal status {status:?} — refusing dispatch"
            )));
        }

        let workgroup_size = p
            .get("workgroup_size")
            .and_then(|v| v.as_array())
            .map(|arr| {
                let x = arr.first().and_then(|v| v.as_u64()).unwrap_or(256) as u32;
                let y = arr.get(1).and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                let z = arr.get(2).and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                [x, y, z]
            })
            .unwrap_or([256, 1, 1]);

        let buffer_descs = p.get("buffers").cloned().unwrap_or(serde_json::json!([]));

        let timeout_ms = p.get("timeout_ms").and_then(|v| v.as_u64()).unwrap_or(
            toadstool_common::constants::timeouts::DISPATCH_DEFAULT_TIMEOUT.as_millis() as u64,
        );

        let job_id = uuid::Uuid::new_v4().to_string();
        let job = DispatchJob {
            id: job_id.clone(),
            bdf: bdf.clone(),
            status: DispatchStatus::Submitted,
            submitted_at: std::time::Instant::now(),
            binary_size: binary_bytes.len(),
            result: None,
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job);
        }

        self.dispatch_count.fetch_add(1, Ordering::Relaxed);

        let needs_coral = matches!(dispatch_mode.as_str(), "vfio" | "drm");

        if needs_coral && !self.coral_client.is_available().await {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = DispatchStatus::Failed(
                    "coralReef not available — sovereign dispatch requires coralReef".into(),
                );
            }
            return Ok(serde_json::json!({
                "domain": "compute.dispatch",
                "operation": "submit",
                "job_id": job_id,
                "status": "failed",
                "bdf": bdf,
                "dispatch_mode": dispatch_mode,
                "binary_size": binary_bytes.len(),
                "error": "coralReef not available — sovereign dispatch requires coralReef driver",
                "note": "Start coralReef or set CORALREEF_URL to enable sovereign GPU dispatch",
            }));
        }

        if self.coral_client.is_available().await {
            let dispatch_params = serde_json::json!({
                "binary": binary_bytes,
                "bdf": bdf,
                "workgroup_size": workgroup_size,
                "buffers": buffer_descs,
                "timeout_ms": timeout_ms,
                "dispatch_mode": dispatch_mode,
            });

            let client = &self.coral_client;
            if let Some(inner) = client.client_ref().await {
                match inner
                    .call("compute.dispatch.execute", dispatch_params)
                    .await
                {
                    Ok(result) => {
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Completed;
                            job.result = Some(result.clone());
                        }
                        return Ok(serde_json::json!({
                            "domain": "compute.dispatch",
                            "operation": "submit",
                            "job_id": job_id,
                            "status": "completed",
                            "bdf": bdf,
                            "dispatch_mode": dispatch_mode,
                            "binary_size": binary_bytes.len(),
                            "thermal_checked": thermal.is_some(),
                            "workgroup_size": workgroup_size,
                            "result": result,
                        }));
                    }
                    Err(e) => {
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Failed(e.to_string());
                        }
                        return Ok(serde_json::json!({
                            "domain": "compute.dispatch",
                            "operation": "submit",
                            "job_id": job_id,
                            "status": "failed",
                            "bdf": bdf,
                            "dispatch_mode": dispatch_mode,
                            "binary_size": binary_bytes.len(),
                            "error": e.to_string(),
                        }));
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "submit",
            "job_id": job_id,
            "status": "submitted",
            "bdf": bdf,
            "dispatch_mode": dispatch_mode,
            "binary_size": binary_bytes.len(),
            "thermal_checked": thermal.is_some(),
            "workgroup_size": workgroup_size,
        }))
    }

    /// `compute.dispatch.status` — Query dispatch job status.
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

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "status",
            "job_id": job_id,
            "status": job.status.to_string(),
            "bdf": job.bdf,
            "binary_size": job.binary_size,
            "elapsed_ms": job.submitted_at.elapsed().as_millis() as u64,
        }))
    }

    /// `compute.dispatch.result` — Retrieve dispatch result data.
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

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "result",
            "job_id": job_id,
            "status": job.status.to_string(),
            "result": job.result,
        }))
    }

    /// `compute.dispatch.forward` — Forward a sovereign dispatch to a remote gate.
    ///
    /// Params: { "endpoint": "...", "binary": [...], "bdf": "...", ... }
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
                "endpoint": endpoint,
                "status": "completed",
                "result": result,
            })),
            Err(e) => Err(JsonRpcError::internal_error(format!(
                "Remote dispatch to {endpoint} failed: {e}"
            ))),
        }
    }

    /// `compute.dispatch.capabilities` — Report dispatch capabilities.
    pub async fn dispatch_capabilities(
        &self,
        _params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let coral_available = self.coral_client.is_available().await;
        let gpus = toadstool_sysmon::discover_gpus();

        let vfio_gpus: Vec<_> = gpus
            .iter()
            .filter(|g| g.driver == "vfio-pci")
            .map(|g| {
                serde_json::json!({
                    "pci_slot": g.pci_slot,
                    "vendor": format!("{:?}", g.vendor),
                    "device_id": format!("{:#06x}", g.device_id),
                })
            })
            .collect();

        let drm_gpus: Vec<_> = gpus
            .iter()
            .filter(|g| g.driver != "vfio-pci")
            .map(|g| {
                serde_json::json!({
                    "pci_slot": g.pci_slot,
                    "vendor": format!("{:?}", g.vendor),
                    "driver": g.driver,
                    "card_index": g.card_index,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "capabilities",
            "sovereign_pipeline": true,
            "coral_reef_available": coral_available,
            "dispatch_modes": ["vfio", "drm"],
            "vfio_gpus": vfio_gpus,
            "drm_gpus": drm_gpus,
            "total_dispatch_count": self.dispatch_count.load(Ordering::Relaxed),
        }))
    }
}

fn resolve_dispatch_bdf(params: &serde_json::Value) -> Result<String, JsonRpcError> {
    if let Some(bdf) = params.get("bdf").and_then(serde_json::Value::as_str) {
        return Ok(bdf.to_string());
    }

    let gpus = toadstool_sysmon::discover_gpus();
    if let Some(vfio_gpu) = gpus.iter().find(|g| g.driver == "vfio-pci") {
        return Ok(vfio_gpu.pci_slot.clone());
    }
    gpus.first()
        .map(|g| g.pci_slot.clone())
        .ok_or_else(|| JsonRpcError::internal_error("No GPUs found for dispatch"))
}

fn detect_dispatch_mode(params: &serde_json::Value, bdf: &str) -> String {
    if let Some(mode) = params
        .get("dispatch_mode")
        .and_then(serde_json::Value::as_str)
    {
        return mode.to_string();
    }

    let gpus = toadstool_sysmon::discover_gpus();
    if gpus
        .iter()
        .any(|g| g.pci_slot == bdf && g.driver == "vfio-pci")
    {
        "vfio".to_string()
    } else {
        "drm".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_handler() -> DispatchHandler {
        DispatchHandler::new(crate::coral_reef_client::create_coral_reef_client())
    }

    fn submit_params(bdf: &str, dispatch_mode: &str) -> serde_json::Value {
        serde_json::json!({
            "binary": [1u8, 2, 3],
            "bdf": bdf,
            "dispatch_mode": dispatch_mode,
        })
    }

    #[tokio::test]
    async fn dispatch_capabilities_returns_expected_structure() {
        let handler = test_handler();
        let result = handler
            .dispatch_capabilities(None)
            .await
            .expect("capabilities");
        assert_eq!(result["domain"], "compute.dispatch");
        assert_eq!(result["operation"], "capabilities");
        assert!(result["sovereign_pipeline"].as_bool().unwrap());
        assert!(result["dispatch_modes"].as_array().is_some());
        assert!(result["vfio_gpus"].as_array().is_some());
        assert!(result["drm_gpus"].as_array().is_some());
        assert!(result["total_dispatch_count"].as_u64().is_some());
        assert!(result["coral_reef_available"].is_boolean());
    }

    #[tokio::test]
    async fn dispatch_capabilities_total_dispatch_count_increments_after_submit() {
        let handler = test_handler();
        let before = handler
            .dispatch_capabilities(None)
            .await
            .expect("capabilities")["total_dispatch_count"]
            .as_u64()
            .expect("total_dispatch_count");

        let params = submit_params("0000:03:00.0", "passthrough");
        handler
            .dispatch_submit(Some(&params))
            .await
            .expect("submit");

        let after = handler
            .dispatch_capabilities(None)
            .await
            .expect("capabilities")["total_dispatch_count"]
            .as_u64()
            .expect("total_dispatch_count");
        assert_eq!(after, before + 1);
    }

    #[tokio::test]
    async fn dispatch_submit_missing_params_returns_invalid_params() {
        let handler = test_handler();
        let err = handler
            .dispatch_submit(None)
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("binary"));
    }

    #[tokio::test]
    async fn dispatch_submit_empty_binary_returns_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "binary": [] });
        let err = handler
            .dispatch_submit(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("empty"));
    }

    #[tokio::test]
    async fn dispatch_submit_missing_binary_field_returns_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "bdf": "0000:03:00.0" });
        let err = handler
            .dispatch_submit(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn dispatch_submit_binary_not_array_returns_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({
            "binary": "not-an-array",
            "bdf": "0000:03:00.0",
        });
        let err = handler
            .dispatch_submit(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn dispatch_submit_vfio_mode_without_coral_returns_failed_payload() {
        let coral = crate::coral_reef_client::create_coral_reef_client();
        if coral.is_available().await {
            return;
        }
        let handler = DispatchHandler::new(coral);
        let params = submit_params("0000:03:00.0", "vfio");
        let result = handler
            .dispatch_submit(Some(&params))
            .await
            .expect("submit");
        assert_eq!(result["domain"], "compute.dispatch");
        assert_eq!(result["status"], "failed");
        assert!(
            result["error"]
                .as_str()
                .is_some_and(|s| s.contains("coralReef"))
        );
    }

    #[tokio::test]
    async fn dispatch_submit_drm_mode_without_coral_returns_failed_payload() {
        let coral = crate::coral_reef_client::create_coral_reef_client();
        if coral.is_available().await {
            return;
        }
        let handler = DispatchHandler::new(coral);
        let params = submit_params("0000:03:00.0", "drm");
        let result = handler
            .dispatch_submit(Some(&params))
            .await
            .expect("submit");
        assert_eq!(result["status"], "failed");
        assert!(
            result["error"]
                .as_str()
                .is_some_and(|s| s.contains("coralReef"))
        );
    }

    #[tokio::test]
    async fn dispatch_submit_custom_dispatch_mode_registers_job_for_status_and_result() {
        let handler = test_handler();
        let params = serde_json::json!({
            "binary": [1u8, 2, 3],
            "bdf": "0000:03:00.0",
            "dispatch_mode": "passthrough",
            "workgroup_size": [128, 2, 4],
            "buffers": [{ "name": "a", "size": 16 }],
            "timeout_ms": 9999u64,
        });
        let result = handler
            .dispatch_submit(Some(&params))
            .await
            .expect("submit");
        assert_eq!(result["domain"], "compute.dispatch");
        let job_id = result["job_id"].as_str().expect("job_id");
        assert_eq!(result["workgroup_size"], serde_json::json!([128, 2, 4]));

        let status = handler
            .dispatch_status(Some(&serde_json::json!({ "job_id": job_id })))
            .await
            .expect("status");
        assert_eq!(status["job_id"], job_id);
        assert!(status["status"].as_str().is_some());
        assert_eq!(status["bdf"], "0000:03:00.0");

        let got = handler
            .dispatch_result(Some(&serde_json::json!({ "job_id": job_id })))
            .await
            .expect("result");
        assert_eq!(got["job_id"], job_id);
    }

    #[tokio::test]
    async fn dispatch_status_unknown_job_returns_error() {
        let handler = test_handler();
        let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
        let err = handler
            .dispatch_status(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
        assert!(err.message.contains("not found"));
    }

    #[tokio::test]
    async fn dispatch_status_missing_job_id_returns_invalid_params() {
        let handler = test_handler();
        let err = handler
            .dispatch_status(None)
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);

        let err = handler
            .dispatch_status(Some(&serde_json::json!({})))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn dispatch_status_job_id_not_string_returns_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "job_id": 12345 });
        let err = handler
            .dispatch_status(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn dispatch_result_unknown_job_returns_error() {
        let handler = test_handler();
        let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
        let err = handler
            .dispatch_result(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
        assert!(err.message.contains("not found"));
    }

    #[tokio::test]
    async fn dispatch_result_missing_job_id_returns_invalid_params() {
        let handler = test_handler();
        let err = handler
            .dispatch_result(None)
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn dispatch_result_job_id_not_string_returns_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "job_id": true });
        let err = handler
            .dispatch_result(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn dispatch_forward_missing_params_returns_invalid_params() {
        let handler = test_handler();
        let err = handler
            .dispatch_forward(None)
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn dispatch_forward_missing_endpoint_returns_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "binary": [1] });
        let err = handler
            .dispatch_forward(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("endpoint"));
    }

    #[tokio::test]
    async fn dispatch_forward_unreachable_returns_internal_error() {
        let handler = test_handler();
        let params = serde_json::json!({
            "endpoint": "127.0.0.1:1",
            "binary": [1, 2],
            "bdf": "0000:03:00.0",
        });
        let err = handler
            .dispatch_forward(Some(&params))
            .await
            .expect_err("expected error");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
        assert!(err.message.contains("127.0.0.1:1") || err.message.contains("failed"));
    }

    #[tokio::test]
    async fn dispatch_forward_uses_nested_params_when_present() {
        let handler = test_handler();
        let params = serde_json::json!({
            "endpoint": "127.0.0.1:1",
            "params": {
                "binary": [9],
                "bdf": "0000:03:00.0",
            },
        });
        let err = handler
            .dispatch_forward(Some(&params))
            .await
            .expect_err("expected transport error");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    }
}
