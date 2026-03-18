// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shader compilation pipeline IPC for coralReef.
//!
//! Proxies WGSL/SPIR-V compilation requests to coralReef when available.
//! Falls back to naga-only pipeline metadata when coralReef is not
//! discovered at runtime.

use toadstool_common::interned_strings::capabilities;

use crate::coral_reef_client::SharedCoralReefClient;
use crate::pure_jsonrpc::types::JsonRpcError;

/// Handles shader compilation requests via coralReef.
pub(super) struct ShaderHandler {
    coral_reef: SharedCoralReefClient,
}

impl ShaderHandler {
    pub(super) fn new(coral_reef: SharedCoralReefClient) -> Self {
        Self { coral_reef }
    }

    pub(super) async fn compile_wgsl(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let shader_source = params
            .and_then(|p| p.get("source"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        if shader_source.is_empty() {
            return Err(JsonRpcError::invalid_params(
                "Missing required 'source' parameter with WGSL shader source",
            ));
        }

        let arch = params.and_then(|p| p.get("arch")).and_then(|v| v.as_str());
        let opt_level = params
            .and_then(|p| p.get("opt_level"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        let target_device = params
            .and_then(|p| p.get("target_device"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        if let Some(result) = self
            .coral_reef
            .compile_wgsl(shader_source, arch, opt_level, target_device)
            .await
        {
            return Ok(serde_json::json!({
                "status": "compiled",
                "pipeline": capabilities::SHADER_COMPILE_NATIVE,
                "source_language": "wgsl",
                "native_compiler_available": true,
                "result": result
            }));
        }

        Ok(serde_json::json!({
            "status": "accepted",
            "pipeline": "naga_wgsl_to_spirv",
            "source_language": "wgsl",
            "target": "spirv",
            "native_compiler_available": false,
            "note": "Compilation routed through naga. Native compiler not available for binary output."
        }))
    }

    pub(super) async fn compile_wgsl_multi(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        use crate::coral_reef_client::MultiDeviceCompileRequest;

        let request: MultiDeviceCompileRequest = params
            .and_then(|p| serde_json::from_value(p.clone()).ok())
            .ok_or_else(|| {
                JsonRpcError::invalid_params(
                    "Expected MultiDeviceCompileRequest with 'wgsl_source' and 'target_devices'",
                )
            })?;

        if request.target_devices.is_empty() {
            return Err(JsonRpcError::invalid_params(
                "target_devices must not be empty",
            ));
        }

        let precision_advice = build_precision_advice(&request.target_devices);

        if let Some(resp) = self.coral_reef.compile_wgsl_multi(&request).await {
            let mut val = serde_json::to_value(resp).unwrap_or_default();
            if let Some(obj) = val.as_object_mut() {
                obj.insert("precision_advice".to_string(), precision_advice);
            }
            return Ok(val);
        }

        Ok(serde_json::json!({
            "status": "accepted",
            "pipeline": "naga_wgsl_to_spirv",
            "native_compiler_available": false,
            "note": "Multi-device compilation routed through naga fallback. coralReef not available.",
            "target_count": request.target_devices.len(),
            "precision_advice": precision_advice,
        }))
    }

    pub(super) async fn compile_spirv(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let spirv_binary = params.and_then(|p| p.get("spirv_binary"));

        if spirv_binary.is_none() {
            return Err(JsonRpcError::invalid_params(
                "Missing required 'spirv_binary' parameter with base64-encoded SPIR-V",
            ));
        }

        let arch = params.and_then(|p| p.get("arch")).and_then(|v| v.as_str());

        if let Some(words) = spirv_binary.and_then(|b| b.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_u64().map(|n| n as u32))
                .collect::<Vec<u32>>()
        }) && let Some(result) = self.coral_reef.compile_spirv(&words, arch).await
        {
            return Ok(serde_json::json!({
                "status": "compiled",
                "pipeline": capabilities::SHADER_COMPILE_NATIVE,
                "source_language": "spirv",
                "native_compiler_available": true,
                "result": result
            }));
        }

        Ok(serde_json::json!({
            "status": "accepted",
            "pipeline": "spirv_passthrough",
            "source_language": "spirv",
            "native_compiler_available": false,
            "note": "SPIR-V accepted. Native compiler not available for binary compilation."
        }))
    }

    pub(super) async fn compile_status(
        &self,
        params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let compile_id = params
            .and_then(|p| p.get("compile_id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");

        let coral_available = self.coral_reef.is_available().await;

        Ok(serde_json::json!({
            "compile_id": compile_id,
            "status": if coral_available { "tracking_available" } else { "not_found" },
            "native_compiler_available": coral_available,
            "note": if coral_available {
                "Native shader pipeline active. Compilation results available synchronously."
            } else {
                "Native compiler not discovered. Compilation tracking requires active compiler."
            }
        }))
    }

    pub(super) async fn compile_capabilities(&self) -> Result<serde_json::Value, JsonRpcError> {
        let health = self.coral_reef.health().await;
        let coral_available = health.is_some();

        let mut target_formats = vec!["spirv"];
        let mut supported_archs = Vec::<String>::new();
        if let Some(h) = &health {
            target_formats.push("native");
            supported_archs.clone_from(&h.supported_archs);
        }

        Ok(serde_json::json!({
            "source_languages": ["wgsl"],
            "target_formats": target_formats,
            "native_binary_compilation": coral_available,
            "native_compiler_available": coral_available,
            "compiler_version": health.as_ref().map(|h| h.version.as_str()),
            "supported_archs": supported_archs,
            "coral_driver_available": false,
            "naga_pipeline": true,
            "domain": "shader"
        }))
    }
}

/// Build per-device precision tier advice for multi-GPU compilation.
///
/// Uses the same NVVM safety classification as `gpu.info` to advise
/// which precision tiers are safe to compile for each target device.
fn build_precision_advice(targets: &[crate::coral_reef_client::DeviceTarget]) -> serde_json::Value {
    let sysmon_gpus = toadstool_sysmon::discover_gpus();

    let advice: Vec<serde_json::Value> = targets
        .iter()
        .map(|target| {
            let gpu = sysmon_gpus
                .iter()
                .find(|g| g.card_index == target.card_index);

            let Some(gpu) = gpu else {
                return serde_json::json!({
                    "card_index": target.card_index,
                    "safe_tiers": ["F32"],
                    "avoid_transcendentals": true,
                    "note": "GPU not found in sysfs — defaulting to conservative F32-only",
                });
            };

            let driver = gpu.driver.as_str();
            let is_nvk = driver.contains("nvk") || driver.contains("nouveau");
            let is_radv = driver.contains("radv");
            let is_nvidia_prop = driver.contains("nvidia") && !is_nvk;

            let (safe_tiers, avoid_transcendentals) = if is_nvk || is_radv {
                (
                    serde_json::json!(["F32", "F64", "F64Precise", "DF64"]),
                    false,
                )
            } else if is_nvidia_prop {
                (serde_json::json!(["F32", "F64"]), true)
            } else {
                (serde_json::json!(["F32"]), true)
            };

            serde_json::json!({
                "card_index": target.card_index,
                "driver": driver,
                "safe_tiers": safe_tiers,
                "avoid_transcendentals": avoid_transcendentals,
            })
        })
        .collect();

    serde_json::json!(advice)
}
