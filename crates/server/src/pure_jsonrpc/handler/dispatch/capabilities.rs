// SPDX-License-Identifier: AGPL-3.0-only

use super::DispatchHandler;
use crate::pure_jsonrpc::types::JsonRpcError;
use std::sync::atomic::Ordering;

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
            "methods": [
                "compute.dispatch.submit",
                "compute.dispatch.status",
                "compute.dispatch.result",
                "compute.dispatch.forward",
                "compute.dispatch.capabilities",
                "shader.dispatch",
            ],
            "vfio_gpus": vfio_gpus,
            "drm_gpus": drm_gpus,
            "total_dispatch_count": self.dispatch_count.load(Ordering::Relaxed),
        }))
    }
}
