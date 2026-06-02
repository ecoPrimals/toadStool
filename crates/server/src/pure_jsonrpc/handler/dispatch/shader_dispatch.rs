// SPDX-License-Identifier: AGPL-3.0-or-later
//! `shader.dispatch` — accepts a compiled shader binary (from the visualization/shader
//! service's `shader.compile.wgsl` response or raw bytes) and dispatches it to the
//! target GPU via VFIO or DRM, returning readback results.
//!
//! Pipeline: **shader compile (visualization service) → ToadStool dispatch → consumer validate**

use base64::{Engine as _, engine::general_purpose::STANDARD};

use super::DispatchHandler;
use super::routing::{detect_dispatch_mode, resolve_dispatch_bdf};
use super::submit::enforce_envelope;
use super::types::{DispatchJob, DispatchStatus};
use crate::pure_jsonrpc::handler::method_gate::CallerContext;
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
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production code uses shader_dispatch_with_context; tests use this convenience wrapper"
        )
    )]
    pub async fn shader_dispatch(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        self.shader_dispatch_with_context(params, &CallerContext::anonymous())
            .await
    }

    /// Context-aware shader dispatch for JH-2 envelope enforcement.
    pub async fn shader_dispatch_with_context(
        &self,
        params: Option<&serde_json::Value>,
        ctx: &CallerContext,
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

        let workgroup_size =
            p.get("workgroup_size")
                .and_then(|v| v.as_array())
                .map_or([256, 1, 1], |arr| {
                    let x = arr
                        .first()
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(256) as u32;
                    let y = arr.get(1).and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
                    let z = arr.get(2).and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
                    [x, y, z]
                });

        let buffer_descs = p.get("buffers").cloned().unwrap_or(serde_json::json!([]));
        let readback = p
            .get("readback")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let timeout_ms = p
            .get("timeout_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(
                toadstool_common::constants::timeouts::DISPATCH_DEFAULT_TIMEOUT.as_millis() as u64,
            );

        let workgroup_total =
            u64::from(workgroup_size[0]) * u64::from(workgroup_size[1]) * u64::from(workgroup_size[2]);
        enforce_envelope(ctx, binary_bytes.len(), workgroup_total, timeout_ms)?;

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

        let needs_shader_service = matches!(dispatch_mode.as_str(), "vfio" | "drm");

        // Phase D: try local dispatch via cylinder before coral_client IPC.
        if needs_shader_service {
            self.acquire_device_handle(&bdf).await;

            if let Some(local_result) = self
                .try_local_dispatch(
                    &bdf,
                    &binary_bytes,
                    workgroup_size,
                    None,
                    &buffer_descs,
                )
                .await
            {
                match local_result {
                    Ok(local_output) => {
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Completed;
                            job.result = Some(local_output.clone());
                        }
                        return Ok(serde_json::json!({
                            "domain": "compute.dispatch",
                            "operation": "shader",
                            "job_id": job_id,
                            "status": "completed",
                            "output": local_output,
                            "error": null,
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": "local_cylinder",
                                "binary_size": binary_bytes.len(),
                                "arch": source_arch,
                                "thermal_checked": thermal.is_some(),
                                "workgroup_size": workgroup_size,
                                "readback": readback,
                            },
                        }));
                    }
                    Err(e) => {
                        tracing::warn!(bdf, error = %e, "shader local dispatch failed — falling through to coral_client");
                    }
                }
            }
        }

        // Phase E: try wgpu dispatch for DRM-bound GPUs (Vulkan compute path).
        if needs_shader_service {
            let wgsl_source = p.get("wgsl_source").and_then(serde_json::Value::as_str);
            if let Some(wgpu_result) = super::wgpu_dispatch::try_wgpu_dispatch(
                &binary_bytes,
                wgsl_source,
                workgroup_size,
                &buffer_descs,
            ) {
                match wgpu_result {
                    Ok(wgpu_output) => {
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Completed;
                            job.result = Some(wgpu_output.clone());
                        }
                        return Ok(serde_json::json!({
                            "domain": "compute.dispatch",
                            "operation": "shader",
                            "job_id": job_id,
                            "status": "completed",
                            "output": wgpu_output,
                            "error": null,
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": "wgpu",
                                "binary_size": binary_bytes.len(),
                                "arch": source_arch,
                                "thermal_checked": thermal.is_some(),
                                "workgroup_size": workgroup_size,
                                "readback": readback,
                            },
                        }));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "wgpu dispatch failed — falling through to coral_client");
                    }
                }
            }
        }

        if needs_shader_service && !self.coral_client.is_available().await {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = DispatchStatus::Failed(
                    "visualization/shader service not available — vfio/drm dispatch requires it"
                        .into(),
                );
            }
            return Ok(serde_json::json!({
                "domain": "compute.dispatch",
                "operation": "shader",
                "job_id": job_id,
                "status": "failed",
                "output": null,
                "error": "visualization/shader service not available — sovereign GPU dispatch requires a running shader provider",
                "metadata": {
                    "bdf": bdf,
                    "dispatch_mode": dispatch_mode,
                    "binary_size": binary_bytes.len(),
                    "note": "Start a shader/visualization capability provider — ToadStool discovers providers at runtime via capability registry",
                },
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
                    .call("compute.dispatch.submit", dispatch_params)
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
                            "operation": "shader",
                            "job_id": job_id,
                            "status": "completed",
                            "output": result,
                            "error": null,
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": dispatch_mode,
                                "binary_size": binary_bytes.len(),
                                "arch": source_arch,
                                "thermal_checked": thermal.is_some(),
                                "workgroup_size": workgroup_size,
                                "readback": readback,
                            },
                        }));
                    }
                    Err(e) => {
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Failed(e.to_string());
                        }
                        return Ok(serde_json::json!({
                            "domain": "compute.dispatch",
                            "operation": "shader",
                            "job_id": job_id,
                            "status": "failed",
                            "output": null,
                            "error": e.to_string(),
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": dispatch_mode,
                                "binary_size": binary_bytes.len(),
                                "arch": source_arch,
                                "thermal_checked": thermal.is_some(),
                                "workgroup_size": workgroup_size,
                                "readback": readback,
                            },
                        }));
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "shader",
            "job_id": job_id,
            "status": "submitted",
            "output": null,
            "error": null,
            "metadata": {
                "bdf": bdf,
                "dispatch_mode": dispatch_mode,
                "binary_size": binary_bytes.len(),
                "arch": source_arch,
                "thermal_checked": thermal.is_some(),
                "workgroup_size": workgroup_size,
                "readback": readback,
            },
        }))
    }
}

/// Extract binary bytes and optional architecture from the request.
///
/// Supports three shapes:
/// 1. `{ "compile_result": { "binary": [...], "arch": "sm89" } }` (pipeline chaining)
/// 2. `{ "binary": "<base64 string>" }` (compact transport)
/// 3. `{ "binary": [u8, u8, ...] }` (backward-compatible array)
pub(crate) fn extract_binary(
    p: &serde_json::Value,
) -> Result<(Vec<u8>, Option<String>), JsonRpcError> {
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
pub(crate) fn decode_binary_value(val: &serde_json::Value) -> Result<Vec<u8>, JsonRpcError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pure_jsonrpc::types::JsonRpcError;
    use crate::visualization_client::create_visualization_client;

    #[test]
    fn decode_binary_value_rejects_invalid_base64() {
        let err = decode_binary_value(&serde_json::json!("not!!!valid!!!base64")).unwrap_err();
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(
            err.message.contains("Invalid base64"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn decode_binary_value_rejects_non_string_non_array() {
        let err = decode_binary_value(&serde_json::json!(42)).unwrap_err();
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(
            err.message.contains("base64 string") || err.message.contains("JSON array"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn decode_binary_value_accepts_empty_u8_array() {
        let bytes = decode_binary_value(&serde_json::json!([])).unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn decode_binary_value_maps_non_u8_json_numbers_to_zero_byte() {
        let bytes = decode_binary_value(&serde_json::json!([300, 1])).unwrap();
        assert_eq!(bytes, vec![44u8, 1]);
    }

    #[test]
    fn extract_binary_requires_binary_or_compile_result() {
        let err = extract_binary(&serde_json::json!({})).unwrap_err();
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("binary") || err.message.contains("compile_result"));
    }

    #[test]
    fn extract_binary_compile_result_requires_nested_binary() {
        let err = extract_binary(&serde_json::json!({ "compile_result": { "arch": "sm89" } }))
            .unwrap_err();
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("compile_result"));
    }

    #[test]
    fn extract_binary_from_compile_result_preserves_arch() {
        let (bytes, arch) = extract_binary(&serde_json::json!({
            "compile_result": { "binary": [1, 2, 3], "arch": "sm89" }
        }))
        .unwrap();
        assert_eq!(bytes, vec![1, 2, 3]);
        assert_eq!(arch.as_deref(), Some("sm89"));
    }

    #[test]
    fn extract_binary_from_top_level_base64() {
        let (bytes, arch) = extract_binary(&serde_json::json!({ "binary": "AQID" })).unwrap(); // [1,2,3]
        assert_eq!(bytes, vec![1, 2, 3]);
        assert!(arch.is_none());
    }

    #[test]
    fn extract_binary_top_level_arch_is_optional() {
        let (bytes, arch) = extract_binary(&serde_json::json!({
            "binary": [9],
            "arch": "sm90"
        }))
        .unwrap();
        assert_eq!(bytes, vec![9]);
        assert_eq!(arch.as_deref(), Some("sm90"));
    }

    #[tokio::test]
    async fn shader_dispatch_rejects_empty_binary_after_decode() {
        let handler = DispatchHandler::new(create_visualization_client(), None);
        let params = serde_json::json!({
            "binary": [],
            "bdf": "0000:00:00.0",
            "dispatch_mode": "drm",
        });
        let err = handler.shader_dispatch(Some(&params)).await.unwrap_err();
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("empty"));
    }

    #[tokio::test]
    async fn shader_dispatch_vfio_without_shader_service_returns_failed_capability_response() {
        let handler = DispatchHandler::new(create_visualization_client(), None);
        let params = serde_json::json!({
            "binary": [1u8],
            "bdf": "0000:00:00.0",
            "dispatch_mode": "vfio",
        });
        let result = handler
            .shader_dispatch(Some(&params))
            .await
            .expect("handler returns Ok JSON envelope on service miss");
        assert_eq!(result["status"], "failed");
        assert_eq!(result["domain"], "compute.dispatch");
        assert_eq!(result["operation"], "shader");
        assert!(
            result["error"]
                .as_str()
                .is_some_and(|s| s.contains("shader"))
        );
        let meta = result["metadata"].as_object().expect("metadata object");
        assert_eq!(meta["dispatch_mode"], "vfio");
        assert_eq!(meta["bdf"], "0000:00:00.0");
    }
}
