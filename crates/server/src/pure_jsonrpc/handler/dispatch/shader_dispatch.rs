// SPDX-License-Identifier: AGPL-3.0-only
//! `shader.dispatch` — accepts a compiled shader binary (from coralReef's
//! `shader.compile.wgsl` response or raw bytes) and dispatches it to the
//! target GPU via VFIO or DRM, returning readback results.
//!
//! This closes the ludoSpring V35 / coralReef Iter 70 E2E gap:
//! `coralReef (compile) → toadStool (dispatch) → consumer (validate)`

use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::DispatchHandler;
use super::routing::{detect_dispatch_mode, resolve_dispatch_bdf};
use super::types::{DispatchJob, DispatchStatus};
use crate::pure_jsonrpc::types::JsonRpcError;
use std::sync::atomic::Ordering;

impl DispatchHandler {
    /// Handle `shader.dispatch` — the dispatch half of the sovereign shader pipeline.
    ///
    /// Accepts compiled GPU binary in two forms:
    /// - **Direct**: `{ "binary": "<base64 string>" }` or `{ "binary": [u8, ...] }`
    /// - **Pipeline chaining**: `{ "compile_result": { "binary": [...], "arch": "sm89" } }`
    ///
    /// Auto-detects binary encoding: base64 string or JSON array of u8 numbers.
    pub async fn shader_dispatch(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params(
                "Expected { binary | compile_result, workgroup_size?, buffers?, \
                 bdf?, dispatch_mode?, readback?, timeout_ms? }",
            )
        })?;

        let (binary_bytes, source_arch) = extract_binary(p)?;

        if binary_bytes.is_empty() {
            return Err(JsonRpcError::invalid_params(
                "Shader binary must not be empty",
            ));
        }

        let bdf = resolve_dispatch_bdf(p)?;
        let dispatch_mode = detect_dispatch_mode(p, &bdf);

        let thermal = super::super::hw_learn::check_thermal_for_bdf_pub(&bdf);
        if let Some(ref status) = thermal
            && !status.compute_safe()
        {
            return Err(JsonRpcError::internal_error(format!(
                "GPU {bdf} thermal status {status:?} — refusing shader dispatch"
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
        let readback = p.get("readback").and_then(|v| v.as_bool()).unwrap_or(true);

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
                    "coralReef not available — sovereign shader dispatch requires coralReef".into(),
                );
            }
            return Ok(serde_json::json!({
                "domain": "shader.dispatch",
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
            let mut dispatch_params = serde_json::json!({
                "binary": binary_bytes,
                "bdf": bdf,
                "workgroup_size": workgroup_size,
                "buffers": buffer_descs,
                "timeout_ms": timeout_ms,
                "dispatch_mode": dispatch_mode,
                "readback": readback,
            });

            if let Some(arch) = &source_arch {
                dispatch_params["arch"] = serde_json::Value::String(arch.clone());
            }

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
                            "domain": "shader.dispatch",
                            "job_id": job_id,
                            "status": "completed",
                            "bdf": bdf,
                            "dispatch_mode": dispatch_mode,
                            "binary_size": binary_bytes.len(),
                            "arch": source_arch,
                            "thermal_checked": thermal.is_some(),
                            "workgroup_size": workgroup_size,
                            "readback": readback,
                            "result": result,
                        }));
                    }
                    Err(e) => {
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Failed(e.to_string());
                        }
                        return Ok(serde_json::json!({
                            "domain": "shader.dispatch",
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
            "domain": "shader.dispatch",
            "job_id": job_id,
            "status": "submitted",
            "bdf": bdf,
            "dispatch_mode": dispatch_mode,
            "binary_size": binary_bytes.len(),
            "arch": source_arch,
            "thermal_checked": thermal.is_some(),
            "workgroup_size": workgroup_size,
            "readback": readback,
        }))
    }
}

/// Extract binary bytes and optional architecture from the request.
///
/// Supports three shapes:
/// 1. `{ "compile_result": { "binary": [...], "arch": "sm89" } }` (pipeline chaining)
/// 2. `{ "binary": "<base64 string>" }` (compact transport)
/// 3. `{ "binary": [u8, u8, ...] }` (backward-compatible array)
fn extract_binary(p: &serde_json::Value) -> Result<(Vec<u8>, Option<String>), JsonRpcError> {
    if let Some(cr) = p.get("compile_result") {
        let arch = cr
            .get("arch")
            .and_then(serde_json::Value::as_str)
            .map(String::from);
        let binary_val = cr.get("binary").ok_or_else(|| {
            JsonRpcError::invalid_params("compile_result is missing 'binary' field")
        })?;
        let bytes = decode_binary_value(binary_val)?;
        return Ok((bytes, arch));
    }

    let binary_val = p.get("binary").ok_or_else(|| {
        JsonRpcError::invalid_params(
            "Missing 'binary' (base64 string or u8 array) or 'compile_result' object",
        )
    })?;
    let arch = p
        .get("arch")
        .and_then(serde_json::Value::as_str)
        .map(String::from);
    let bytes = decode_binary_value(binary_val)?;
    Ok((bytes, arch))
}

/// Decode a binary value from either a base64 string or a JSON array of u8.
fn decode_binary_value(val: &serde_json::Value) -> Result<Vec<u8>, JsonRpcError> {
    if let Some(s) = val.as_str() {
        return STANDARD
            .decode(s)
            .map_err(|e| JsonRpcError::invalid_params(format!("Invalid base64 binary: {e}")));
    }

    if let Some(arr) = val.as_array() {
        return Ok(arr.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect());
    }

    Err(JsonRpcError::invalid_params(
        "'binary' must be a base64 string or a JSON array of u8 values",
    ))
}
