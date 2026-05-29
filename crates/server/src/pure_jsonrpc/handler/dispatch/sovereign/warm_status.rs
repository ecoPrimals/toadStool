// SPDX-License-Identifier: AGPL-3.0-or-later

use super::{classify_tier_sysfs, probe_boot_state_sysfs, DispatchHandler};

/// Lightweight warm keepalive status for all known GPUs.
///
/// Reports anchor state, boot state probe (via sysfs BAR0), and fd store
/// capability without running any pipeline. Used to verify fd persistence
/// across daemon restarts.
pub(crate) async fn sovereign_warm_status(
    handler: &DispatchHandler,
) -> Result<serde_json::Value, crate::pure_jsonrpc::types::JsonRpcError> {
    let anchors = handler.anchor_store.lock().await;
    let fd_store_capable = std::env::var(toadstool_common::interned_strings::socket_env::NOTIFY_SOCKET).is_ok();

    let mut devices = serde_json::Map::new();

    // Report on anchored devices
    for (bdf, _anchor) in anchors.iter() {
        let boot_probe = probe_boot_state_sysfs(bdf);
        let tier = classify_tier_sysfs(bdf);
        devices.insert(bdf.clone(), serde_json::json!({
            "anchor_held": true,
            "boot_state": boot_probe.as_ref().map_or("unknown", |s| s.0.as_str()),
            "pmc_enable": boot_probe.as_ref().map_or("n/a", |s| s.1.as_str()),
            "pramin_ok": boot_probe.as_ref().is_some_and(|s| s.2),
            "fd_store_capable": fd_store_capable,
            "sovereign_tier": tier.as_ref().map(|t| t.tier.level()),
            "sovereign_tier_name": tier.as_ref().map(|t| t.tier.description()),
        }));
    }

    // Also report cached devices not yet anchored
    let cache = handler.cached_devices.lock().await;
    for bdf in cache.keys() {
        if !devices.contains_key(bdf) {
            let boot_probe = probe_boot_state_sysfs(bdf);
            let tier = classify_tier_sysfs(bdf);
            devices.insert(bdf.clone(), serde_json::json!({
                "anchor_held": false,
                "boot_state": boot_probe.as_ref().map_or("unknown", |s| s.0.as_str()),
                "pmc_enable": boot_probe.as_ref().map_or("n/a", |s| s.1.as_str()),
                "pramin_ok": boot_probe.as_ref().is_some_and(|s| s.2),
                "fd_store_capable": fd_store_capable,
                "sovereign_tier": tier.as_ref().map(|t| t.tier.level()),
                "sovereign_tier_name": tier.as_ref().map(|t| t.tier.description()),
            }));
        }
    }

    Ok(serde_json::json!({
        "anchor_count": anchors.len(),
        "fd_store_capable": fd_store_capable,
        "devices": devices,
    }))
}
