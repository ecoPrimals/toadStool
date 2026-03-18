// SPDX-License-Identifier: AGPL-3.0-or-later
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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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

    #[tokio::test]
    async fn dispatch_capabilities_returns_expected_structure() {
        let handler = test_handler();
        let result = handler.dispatch_capabilities(None).await.unwrap();
        assert_eq!(result["domain"], "compute.dispatch");
        assert_eq!(result["operation"], "capabilities");
        assert!(result["sovereign_pipeline"].as_bool().unwrap());
        assert!(result["dispatch_modes"].as_array().is_some());
        assert!(result["vfio_gpus"].as_array().is_some());
        assert!(result["drm_gpus"].as_array().is_some());
        assert!(result["total_dispatch_count"].as_u64().is_some());
    }

    #[tokio::test]
    async fn dispatch_submit_missing_params_returns_invalid_params() {
        let handler = test_handler();
        let err = handler.dispatch_submit(None).await.unwrap_err();
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("binary"));
    }

    #[tokio::test]
    async fn dispatch_submit_empty_binary_returns_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "binary": [] });
        let err = handler.dispatch_submit(Some(&params)).await.unwrap_err();
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("empty"));
    }

    #[tokio::test]
    async fn dispatch_status_unknown_job_returns_error() {
        let handler = test_handler();
        let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
        let err = handler.dispatch_status(Some(&params)).await.unwrap_err();
        assert!(err.message.contains("not found"));
    }

    #[tokio::test]
    async fn dispatch_result_unknown_job_returns_error() {
        let handler = test_handler();
        let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
        let err = handler.dispatch_result(Some(&params)).await.unwrap_err();
        assert!(err.message.contains("not found"));
    }
}
