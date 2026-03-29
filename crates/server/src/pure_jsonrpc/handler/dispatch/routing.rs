// SPDX-License-Identifier: AGPL-3.0-only

use crate::pure_jsonrpc::types::JsonRpcError;

pub(super) fn resolve_dispatch_bdf(params: &serde_json::Value) -> Result<String, JsonRpcError> {
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

pub(super) fn detect_dispatch_mode(params: &serde_json::Value, bdf: &str) -> String {
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
