// SPDX-License-Identifier: AGPL-3.0-or-later

use super::DispatchHandler;
use super::routing::{detect_dispatch_mode, resolve_dispatch_bdf};
use super::types::{DispatchJob, DispatchStatus};
use crate::pure_jsonrpc::handler::method_gate::CallerContext;
use crate::pure_jsonrpc::types::JsonRpcError;
use base64::Engine;
use std::sync::atomic::Ordering;

/// Enforce resource envelope limits on a dispatch request (JH-2).
///
/// When the caller has a token with a [`ResourceEnvelope`], this checks:
/// - `binary_size` against `mem_mb` (binary rounded up to MB ≤ envelope limit)
/// - `workgroup_total` against `cpu_cores` (total threads ≤ core limit × 1024)
/// - `timeout_ms` against `max_timeout_ms`
///
/// Returns `Ok(())` if no envelope is present or all checks pass.
pub(super) fn enforce_envelope(
    ctx: &CallerContext,
    binary_size: usize,
    workgroup_total: u64,
    timeout_ms: u64,
) -> Result<(), JsonRpcError> {
    let Some(ref env) = ctx.envelope else {
        return Ok(());
    };

    if let Some(mem_mb) = env.mem_mb {
        let binary_mb = (binary_size as u64).saturating_add(1024 * 1024 - 1) / (1024 * 1024);
        if binary_mb > mem_mb {
            return Err(resource_exhausted(format!(
                "Binary size ({binary_mb} MB) exceeds token envelope mem_mb ({mem_mb} MB)"
            )));
        }
    }

    if let Some(cpu_cores) = env.cpu_cores {
        let thread_cap = u64::from(cpu_cores) * 1024;
        if workgroup_total > thread_cap {
            return Err(resource_exhausted(format!(
                "Workgroup total ({workgroup_total} threads) exceeds token envelope \
                 cpu_cores ({cpu_cores}) \u{00d7} 1024 = {thread_cap} thread cap"
            )));
        }
    }

    if let Some(max_timeout) = env.max_timeout_ms
        && timeout_ms > max_timeout
    {
        return Err(resource_exhausted(format!(
            "Requested timeout ({timeout_ms} ms) exceeds token envelope \
             max_timeout_ms ({max_timeout} ms)"
        )));
    }

    Ok(())
}

/// Resolve the binary payload from either `binary_b64` (base64, preferred) or
/// `binary` (JSON u8 array, legacy).
pub(super) fn resolve_binary_param(p: &serde_json::Value) -> Result<Vec<u8>, JsonRpcError> {
    if let Some(b64) = p.get("binary_b64").and_then(|v| v.as_str()) {
        return base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| {
                JsonRpcError::invalid_params(format!("binary_b64 base64 decode error: {e}"))
            });
    }

    if let Some(arr) = p.get("binary").and_then(|v| v.as_array()) {
        return Ok(arr.iter().map(|v| v.as_u64().unwrap_or(0) as u8).collect());
    }

    Err(JsonRpcError::invalid_params(
        "Missing 'binary' (u8 array) or 'binary_b64' (base64 string)",
    ))
}

/// Resolve workgroup/dispatch dimensions from `dispatch_dims` (trio standard)
/// or `workgroup_size` (legacy), defaulting to [256, 1, 1].
pub(super) fn resolve_workgroup_size(p: &serde_json::Value) -> [u32; 3] {
    let dims = p
        .get("dispatch_dims")
        .or_else(|| p.get("workgroup_size"))
        .and_then(|v| v.as_array());

    dims.map_or([256, 1, 1], |arr| {
        let x = arr
            .first()
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(256) as u32;
        let y = arr.get(1).and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
        let z = arr.get(2).and_then(serde_json::Value::as_u64).unwrap_or(1) as u32;
        [x, y, z]
    })
}

/// Resolve buffer descriptors. Accepts trio-standard `buffers[]` with `data_b64`
/// fields — decodes them to `data` (u8 arrays) for downstream consumption.
pub(super) fn resolve_buffers(p: &serde_json::Value) -> serde_json::Value {
    let Some(buffers) = p.get("buffers").and_then(|v| v.as_array()) else {
        return serde_json::json!([]);
    };

    let resolved: Vec<serde_json::Value> = buffers
        .iter()
        .map(|buf| {
            if let Some(b64) = buf.get("data_b64").and_then(|v| v.as_str()) {
                let mut out = buf.clone();
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(b64) {
                    out["data"] = serde_json::json!(decoded);
                    if let Some(obj) = out.as_object_mut() {
                        obj.remove("data_b64");
                    }
                }
                out
            } else {
                buf.clone()
            }
        })
        .collect();

    serde_json::json!(resolved)
}

/// Resolve shader metadata from JSON, accepting both toadStool-native and
/// coralReef `CompilationInfoResponse` field names.
///
/// toadStool names: `gpr_count`, `shared_mem_bytes`, `barrier_count`, `wave_size`, `local_mem_bytes`
/// coralReef names: `gprs`, `shared_memory`, `barriers`, `wave_size`, `local_memory`
pub(super) fn resolve_shader_info(
    si: &serde_json::Value,
    workgroup: [u32; 3],
) -> toadstool_cylinder::ShaderInfo {
    let u32_field = |primary: &str, alias: &str| -> u32 {
        si.get(primary)
            .or_else(|| si.get(alias))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32
    };

    toadstool_cylinder::ShaderInfo {
        gpr_count: u32_field("gpr_count", "gprs"),
        shared_mem_bytes: u32_field("shared_mem_bytes", "shared_memory"),
        barrier_count: u32_field("barrier_count", "barriers"),
        workgroup,
        wave_size: si
            .get("wave_size")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(32) as u32,
        local_mem_bytes: si
            .get("local_mem_bytes")
            .or_else(|| si.get("local_memory"))
            .and_then(serde_json::Value::as_u64)
            .map(|v| v as u32),
    }
}

fn resource_exhausted(msg: String) -> JsonRpcError {
    JsonRpcError {
        code: toadstool_common::constants::jsonrpc::error_codes::RESOURCE_EXHAUSTED,
        message: std::borrow::Cow::Owned(msg),
        data: None,
    }
}

#[expect(
    deprecated,
    reason = "SecurityClient delegates to crypto.encrypt/decrypt; crypto_integration migration tracked"
)]
impl DispatchHandler {
    /// Lazily fetch and cache the `compute` purpose key from BearDog secrets.
    async fn get_purpose_key(&self) -> Result<toadstool::encryption::EncryptionKey, JsonRpcError> {
        {
            let guard = self.cached_purpose_key.read().await;
            if let Some(ref key) = *guard {
                return Ok(key.clone());
            }
        }

        let client = self.security_client.as_ref().ok_or_else(|| {
            JsonRpcError::internal_error("security client unavailable for purpose key retrieval")
        })?;

        let key = client
            .retrieve_purpose_key("compute", None)
            .await
            .map_err(|e| {
                JsonRpcError::internal_error(format!("purpose key retrieval failed: {e}"))
            })?;

        let mut guard = self.cached_purpose_key.write().await;
        *guard = Some(key.clone());
        Ok(key)
    }

    /// Encrypt binary payload via Tower `crypto.encrypt`.
    /// Returns the original bytes unchanged if no security client is present.
    #[expect(
        deprecated,
        reason = "SecurityClient types are deprecated in favor of crypto_integration; wire protocol is the same"
    )]
    async fn encrypt_payload(&self, data: &[u8]) -> Result<Vec<u8>, JsonRpcError> {
        let Some(ref client) = self.security_client else {
            return Ok(data.to_vec());
        };

        let key = self.get_purpose_key().await?;

        let request = toadstool_distributed::security::types::EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: toadstool_distributed::security::types::EncryptionOperation::Encrypt,
            data: data.to_vec(),
            key_id: Some(key.id.clone()),
            algorithm: Some(key.algorithm.clone()),
            security_level: toadstool_distributed::security::types::SecurityLevel::Enhanced,
        };

        let response = client
            .encrypt(request)
            .await
            .map_err(|e| JsonRpcError::internal_error(format!("crypto.encrypt failed: {e}")))?;

        let nonce_b64 = response
            .metadata
            .get("nonce")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let envelope = serde_json::json!({
            "v": 1,
            "ct": base64::engine::general_purpose::STANDARD.encode(&response.data),
            "n": nonce_b64,
            "alg": response.algorithm,
        });

        serde_json::to_vec(&envelope).map_err(|e| {
            JsonRpcError::internal_error(format!("envelope serialization failed: {e}"))
        })
    }

    /// Decrypt result payload from Tower `crypto.decrypt`.
    /// Returns the value unchanged if no security client is present.
    #[expect(
        deprecated,
        reason = "SecurityClient types are deprecated in favor of crypto_integration; wire protocol is the same"
    )]
    async fn decrypt_result(
        &self,
        result: &serde_json::Value,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let Some(ref client) = self.security_client else {
            return Ok(result.clone());
        };

        let Some(ct_b64) = result.get("ct").and_then(|v| v.as_str()) else {
            return Ok(result.clone());
        };

        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(ct_b64)
            .map_err(|e| JsonRpcError::internal_error(format!("ciphertext base64 decode: {e}")))?;

        let key = self.get_purpose_key().await?;

        let request = toadstool_distributed::security::types::EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: toadstool_distributed::security::types::EncryptionOperation::Decrypt,
            data: ciphertext,
            key_id: Some(key.id.clone()),
            algorithm: Some(key.algorithm.clone()),
            security_level: toadstool_distributed::security::types::SecurityLevel::Enhanced,
        };

        let response = client
            .decrypt(request)
            .await
            .map_err(|e| JsonRpcError::internal_error(format!("crypto.decrypt failed: {e}")))?;

        Ok(serde_json::from_slice(&response.data).unwrap_or_else(|_| {
            serde_json::Value::String(
                base64::engine::general_purpose::STANDARD.encode(&response.data),
            )
        }))
    }

    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production code uses dispatch_submit_with_context; tests use this convenience wrapper"
        )
    )]
    pub async fn dispatch_submit(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        self.dispatch_submit_with_context(params, &CallerContext::anonymous())
            .await
    }

    /// Submit with full caller context for JH-2 envelope enforcement.
    ///
    /// **IPC contract**: All param values (BDF addresses, buffer references) must
    /// be pre-resolved by the caller. No `${VAR}` expansion is performed —
    /// env expansion is a CLI-only convenience for local workload files.
    pub async fn dispatch_submit_with_context(
        &self,
        params: Option<&serde_json::Value>,
        ctx: &CallerContext,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let p = params.ok_or_else(|| {
            JsonRpcError::invalid_params(
                "Expected { binary|binary_b64, bdf?, workgroup_size|dispatch_dims?, \
                 buffers?, shader_info?, dispatch_mode?, timeout_ms? }",
            )
        })?;

        let binary_bytes = resolve_binary_param(p)?;

        if binary_bytes.is_empty() {
            return Err(JsonRpcError::invalid_params("binary must not be empty"));
        }

        let shader_info = p.get("shader_info").cloned();

        let bdf = resolve_dispatch_bdf(p)?;
        let dispatch_mode = detect_dispatch_mode(p, &bdf);

        let thermal = super::super::hw_learn::check_thermal_for_bdf_pub(&bdf);
        if let Some(ref status) = thermal
            && !status.compute_safe()
        {
            return Err(JsonRpcError::internal_error(format!(
                "GPU {bdf} thermal status {status:?} — refusing dispatch"
            )));
        }

        let workgroup_size = resolve_workgroup_size(p);

        let buffer_descs = resolve_buffers(p);

        let timeout_ms = p
            .get("timeout_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(
                toadstool_common::constants::timeouts::DISPATCH_DEFAULT_TIMEOUT.as_millis() as u64,
            );

        let workgroup_total =
            u64::from(workgroup_size[0]) * u64::from(workgroup_size[1]) * u64::from(workgroup_size[2]);
        enforce_envelope(ctx, binary_bytes.len(), workgroup_total, timeout_ms)?;

        self.acquire_device_handle(&bdf).await;

        let job_id = uuid::Uuid::new_v4().to_string();
        let submit_instant = std::time::Instant::now();
        let job = DispatchJob {
            id: job_id.clone(),
            bdf: bdf.clone(),
            status: DispatchStatus::Submitted,
            submitted_at: submit_instant,
            binary_size: binary_bytes.len(),
            result: None,
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id.clone(), job);
        }

        self.dispatch_count.fetch_add(1, Ordering::Relaxed);

        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = DispatchStatus::Running;
            }
        }

        let needs_coral = matches!(dispatch_mode.as_str(), "vfio" | "drm");

        // Phase D: try local dispatch via cylinder before coral_client IPC.
        if needs_coral
            && let Some(local_result) = self
                .try_local_dispatch(&bdf, &binary_bytes, workgroup_size, shader_info.as_ref(), &buffer_descs)
                .await
        {
            let dispatch_ms = submit_instant.elapsed().as_millis() as u64;
            match local_result {
                Ok(local_output) => {
                    let mut jobs = self.jobs.write().await;
                    if let Some(job) = jobs.get_mut(&job_id) {
                        job.status = DispatchStatus::Completed;
                        job.result = Some(local_output.clone());
                    }
                    return Ok(serde_json::json!({
                        "domain": "compute.dispatch",
                        "operation": "submit",
                        "job_id": job_id,
                        "status": "completed",
                        "output": local_output,
                        "error": null,
                        "timing": {
                            "dispatch_ms": dispatch_ms,
                            "readback_ms": local_output.get("readback_ms").and_then(serde_json::Value::as_u64).unwrap_or(0),
                        },
                        "metadata": {
                            "bdf": bdf,
                            "dispatch_mode": "local_cylinder",
                            "binary_size": binary_bytes.len(),
                            "thermal_checked": thermal.is_some(),
                            "workgroup_size": workgroup_size,
                        },
                    }));
                }
                Err(e) => {
                    tracing::warn!(bdf, error = %e, "local dispatch failed — falling through to coral_client");
                }
            }
        }

        if needs_coral && !self.coral_client.is_available().await {
            let dispatch_ms = submit_instant.elapsed().as_millis() as u64;
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = DispatchStatus::Failed(
                    "visualization service not available — sovereign dispatch requires shader compiler".into(),
                );
            }
            return Ok(serde_json::json!({
                "domain": "compute.dispatch",
                "operation": "submit",
                "job_id": job_id,
                "status": "failed",
                "output": null,
                "error": "visualization service not available — sovereign dispatch requires shader compiler driver",
                "timing": { "dispatch_ms": dispatch_ms, "readback_ms": 0 },
                "metadata": {
                    "bdf": bdf,
                    "dispatch_mode": dispatch_mode,
                    "binary_size": binary_bytes.len(),
                    "note": "Start a visualization capability provider — ToadStool discovers providers at runtime via capability registry",
                },
            }));
        }

        if self.coral_client.is_available().await {
            let encrypted = self.security_client.is_some();
            let dispatch_binary = if encrypted {
                self.encrypt_payload(&binary_bytes).await?
            } else {
                binary_bytes.clone()
            };

            let mut dispatch_params = serde_json::json!({
                "binary": dispatch_binary,
                "bdf": bdf,
                "workgroup_size": workgroup_size,
                "buffers": buffer_descs,
                "timeout_ms": timeout_ms,
                "dispatch_mode": dispatch_mode,
                "encrypted": encrypted,
            });
            if let Some(ref si) = shader_info {
                dispatch_params["shader_info"] = si.clone();
            }

            let client = &self.coral_client;
            if let Some(inner) = client.client_ref().await {
                let pre_dispatch = std::time::Instant::now();
                match inner
                    .call("compute.dispatch.execute", dispatch_params)
                    .await
                {
                    Ok(result) => {
                        let dispatch_ms = pre_dispatch.elapsed().as_millis() as u64;
                        let readback_start = std::time::Instant::now();
                        let decrypted = self.decrypt_result(&result).await?;
                        let readback_ms = readback_start.elapsed().as_millis() as u64;
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Completed;
                            job.result = Some(decrypted.clone());
                        }
                        return Ok(serde_json::json!({
                            "domain": "compute.dispatch",
                            "operation": "submit",
                            "job_id": job_id,
                            "status": "completed",
                            "output": decrypted,
                            "error": null,
                            "timing": { "dispatch_ms": dispatch_ms, "readback_ms": readback_ms },
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": dispatch_mode,
                                "binary_size": binary_bytes.len(),
                                "thermal_checked": thermal.is_some(),
                                "workgroup_size": workgroup_size,
                                "encrypted": encrypted,
                                "shader_info": shader_info,
                            },
                        }));
                    }
                    Err(e) => {
                        let dispatch_ms = pre_dispatch.elapsed().as_millis() as u64;
                        let mut jobs = self.jobs.write().await;
                        if let Some(job) = jobs.get_mut(&job_id) {
                            job.status = DispatchStatus::Failed(e.to_string());
                        }
                        return Ok(serde_json::json!({
                            "domain": "compute.dispatch",
                            "operation": "submit",
                            "job_id": job_id,
                            "status": "failed",
                            "output": null,
                            "error": e.to_string(),
                            "timing": { "dispatch_ms": dispatch_ms, "readback_ms": 0 },
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": dispatch_mode,
                                "binary_size": binary_bytes.len(),
                                "thermal_checked": thermal.is_some(),
                                "workgroup_size": workgroup_size,
                            },
                        }));
                    }
                }
            }
        }

        let dispatch_ms = submit_instant.elapsed().as_millis() as u64;
        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "submit",
            "job_id": job_id,
            "status": "submitted",
            "output": null,
            "error": null,
            "timing": { "dispatch_ms": dispatch_ms, "readback_ms": 0 },
            "metadata": {
                "bdf": bdf,
                "dispatch_mode": dispatch_mode,
                "binary_size": binary_bytes.len(),
                "thermal_checked": thermal.is_some(),
                "workgroup_size": workgroup_size,
                "shader_info": shader_info,
            },
        }))
    }
}
