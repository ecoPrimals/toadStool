// SPDX-License-Identifier: AGPL-3.0-or-later

use super::DispatchHandler;
use crate::pure_jsonrpc::types::JsonRpcError;
use std::sync::atomic::Ordering;

/// Map (vendor, device_id) to a compute architecture string for the trio IPC
/// contract (Gate 2). Returns `None` for unrecognized devices.
fn gpu_architecture(vendor: toadstool_sysmon::GpuVendor, device_id: u32) -> Option<&'static str> {
    match vendor {
        toadstool_sysmon::GpuVendor::Nvidia => Some(match device_id {
            // Kepler: K80, K40, K20
            0x1023..=0x103F | 0x1180..=0x11FF => "sm35",
            // Maxwell: GTX 9xx, Titan X (Maxwell)
            0x13C0..=0x13FF | 0x1401..=0x147F => "sm50",
            // Pascal: GTX 10xx, Titan Xp, P100
            0x15F7..=0x15FF | 0x1B00..=0x1B8F | 0x1C00..=0x1C9F => "sm60",
            // Volta: Titan V, V100
            0x1D81 | 0x1DB1 | 0x1DB4..=0x1DBA => "sm70",
            // Turing: RTX 20xx
            0x1E02..=0x1EFF | 0x1F02..=0x1FBF | 0x2182..=0x21FF => "sm75",
            // Ampere: RTX 30xx, A100
            0x20B0..=0x20FF | 0x2204..=0x253F => "sm80",
            // Ada Lovelace: RTX 40xx
            0x2684..=0x283F => "sm89",
            _ => "sm_unknown",
        }),
        toadstool_sysmon::GpuVendor::Amd => Some(match device_id {
            // RDNA 2: Navi 21/22/23
            0x73BF | 0x73DF | 0x73FF | 0x7340..=0x73AF => "rdna2",
            // RDNA 3: Navi 31/32/33
            0x744C | 0x7448 | 0x7480..=0x749F | 0x7500..=0x751F => "rdna3",
            // GCN / older
            _ => "gcn",
        }),
        toadstool_sysmon::GpuVendor::Intel => Some("xe"),
        toadstool_sysmon::GpuVendor::Unknown => None,
    }
}

impl DispatchHandler {
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
                    "architecture": gpu_architecture(g.vendor, g.device_id),
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
                    "device_id": format!("{:#06x}", g.device_id),
                    "driver": g.driver,
                    "card_index": g.card_index,
                    "render_node": g.render_node().to_string_lossy(),
                    "architecture": gpu_architecture(g.vendor, g.device_id),
                })
            })
            .collect();

        let mut architectures: Vec<&str> = gpus
            .iter()
            .filter_map(|g| gpu_architecture(g.vendor, g.device_id))
            .collect();
        architectures.sort_unstable();
        architectures.dedup();

        let vfio_count = vfio_gpus.len();
        let gpu_count = gpus.len();

        Ok(serde_json::json!({
            "domain": "compute.dispatch",
            "operation": "capabilities",
            "job_id": null,
            "status": "completed",
            "output": {
                "sovereign_pipeline": true,
                "shader_compiler_available": coral_available,
                "dispatch_modes": ["vfio", "drm"],
                "methods": [
                    "compute.dispatch.submit",
                    "compute.dispatch.status",
                    "compute.dispatch.result",
                    "compute.dispatch.forward",
                    "compute.dispatch.capabilities",
                    "compute.dispatch.pipeline.submit",
                    "compute.dispatch.pipeline.status",
                    "shader.dispatch",
                ],
                "gpu_count": gpu_count,
                "architectures": architectures,
                "vfio_status": {
                    "available": vfio_count > 0,
                    "device_count": vfio_count,
                },
                "vfio_gpus": vfio_gpus,
                "drm_gpus": drm_gpus,
                "total_dispatch_count": self.dispatch_count.load(Ordering::Relaxed),
                "ember": {
                    "held_devices": self.held_device_count().await,
                    "phase": "D",
                    "local_dispatch": self.local_device_factory.is_some(),
                },
                "glowplug": {
                    "orchestrator": "SwapOrchestrator<SysfsSwapExecutor>",
                    "lifecycle_steps": 7,
                    "personalities": ["vfio", "nouveau", "nvidia", "nvidia-open", "nvidia-oracle", "amdgpu", "xe", "i915", "akida", "unbound"],
                },
            },
            "error": null,
            "metadata": {},
        }))
    }
}
