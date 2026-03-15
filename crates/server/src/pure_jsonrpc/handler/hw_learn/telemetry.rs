// SPDX-License-Identifier: AGPL-3.0-only
//! GPU telemetry handler — report thermal and power data for detected GPUs.

use super::helpers::check_thermal_for_bdf;
use super::HwLearnHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

impl HwLearnHandler {
    /// `gpu.telemetry` — Report thermal and power data for all detected GPUs.
    ///
    /// Returns per-GPU temperature, power, safety status.
    ///
    /// # Errors
    ///
    /// This function does not return errors.
    #[expect(
        clippy::unused_async,
        reason = "async for JSON-RPC handler trait consistency"
    )]
    pub async fn gpu_telemetry(
        &self,
        _params: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, JsonRpcError> {
        let sysmon_gpus = toadstool_sysmon::discover_gpus();

        let mut gpu_entries = Vec::new();
        for gpu in &sysmon_gpus {
            let telemetry = gpu.telemetry();
            let safety =
                check_thermal_for_bdf(&gpu.pci_slot).unwrap_or(nvpmu::SafetyStatus::Unknown);

            gpu_entries.push(serde_json::json!({
                "card_index": gpu.card_index,
                "driver": gpu.driver,
                "pci_slot": gpu.pci_slot,
                "vendor": format!("{:?}", gpu.vendor),
                "device_id": format!("{:#06x}", gpu.device_id),
                "temperature_celsius": telemetry.temperature_celsius,
                "power_watts": telemetry.power_watts,
                "power_cap_watts": telemetry.power_cap_watts,
                "core_clock_mhz": telemetry.core_clock_mhz,
                "fan_rpm": telemetry.fan_rpm,
                "utilization_percent": telemetry.utilization_percent,
                "vram_total_bytes": telemetry.vram_total_bytes,
                "vram_used_bytes": telemetry.vram_used_bytes,
                "safety_status": format!("{:?}", safety),
                "compute_safe": safety.compute_safe(),
            }));
        }

        Ok(serde_json::json!({
            "domain": "gpu",
            "operation": "telemetry",
            "gpus": gpu_entries,
            "gpu_count": gpu_entries.len(),
        }))
    }
}
