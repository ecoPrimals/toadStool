// SPDX-License-Identifier: AGPL-3.0-or-later

use super::DispatchHandler;
use super::routing::{detect_dispatch_mode, resolve_dispatch_bdf};
use super::types::{DispatchJob, DispatchStatus};
use crate::pure_jsonrpc::types::JsonRpcError;
use base64::Engine;
use std::sync::atomic::Ordering;

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

        let thermal = super::super::hw_learn::check_thermal_for_bdf_pub(&bdf);
        if let Some(ref status) = thermal
            && !status.compute_safe()
        {
            return Err(JsonRpcError::internal_error(format!(
                "GPU {bdf} thermal status {status:?} — refusing dispatch"
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

        let timeout_ms = p
            .get("timeout_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(
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

        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = DispatchStatus::Running;
            }
        }

        let needs_coral = matches!(dispatch_mode.as_str(), "vfio" | "drm");

        if needs_coral && !self.coral_client.is_available().await {
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

            let dispatch_params = serde_json::json!({
                "binary": dispatch_binary,
                "bdf": bdf,
                "workgroup_size": workgroup_size,
                "buffers": buffer_descs,
                "timeout_ms": timeout_ms,
                "dispatch_mode": dispatch_mode,
                "encrypted": encrypted,
            });

            let client = &self.coral_client;
            if let Some(inner) = client.client_ref().await {
                match inner
                    .call("compute.dispatch.execute", dispatch_params)
                    .await
                {
                    Ok(result) => {
                        let decrypted = self.decrypt_result(&result).await?;
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
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": dispatch_mode,
                                "binary_size": binary_bytes.len(),
                                "thermal_checked": thermal.is_some(),
                                "workgroup_size": workgroup_size,
                                "encrypted": encrypted,
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
                            "operation": "submit",
                            "job_id": job_id,
                            "status": "failed",
                            "output": null,
                            "error": e.to_string(),
                            "metadata": {
                                "bdf": bdf,
                                "dispatch_mode": dispatch_mode,
                                "binary_size": binary_bytes.len(),
                            },
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
            "output": null,
            "error": null,
            "metadata": {
                "bdf": bdf,
                "dispatch_mode": dispatch_mode,
                "binary_size": binary_bytes.len(),
                "thermal_checked": thermal.is_some(),
                "workgroup_size": workgroup_size,
            },
        }))
    }
}
