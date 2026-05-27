// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tier classification, kernel health, and runtime services probe handlers.

use crate::pure_jsonrpc::types::JsonRpcError;
use serde_json::Value;
use tracing::{debug, info};

pub(super) const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;

/// `sovereign.classify_tier` — generation-aware sovereignty tier classification.
///
/// Uses `GenerationProfile` offsets instead of hardcoded Volta values.
/// Auto-detects SM version from BOOT0, falls back to SM 70 (Volta).
///
/// Params:
/// - `bdf` (required): PCI BDF address
/// - `sm_version` (optional): override SM version for profile lookup
pub fn sovereign_classify_tier(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf'"))?;

    let bar0 =
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
            .map_err(|e| JsonRpcError::internal_error(format!("BAR0 open failed for {bdf}: {e}")))?;

    let sm_override = params
        .and_then(|p| p.get("sm_version"))
        .and_then(Value::as_u64)
        .map(|v| v as u32);

    let sm = sm_override.unwrap_or_else(|| {
        let boot0 = bar0.read_u32(0x0000_0000).unwrap_or(0);
        let chip_id = (boot0 >> 20) & 0x1FF;
        let synthetic = chip_id << 20;
        toadstool_cylinder::nv::identity::boot0_to_sm(synthetic).unwrap_or(70)
    });

    let profile = toadstool_cylinder::nv::generation::profile_for_sm(sm);

    let evidence =
        toadstool_cylinder::vfio::sovereign_tiers::classify_tier_for_profile(&bar0, profile);

    debug!(bdf = %bdf, sm = sm, tier = ?evidence.tier, generation = profile.name,
           "sovereign.classify_tier");

    Ok(serde_json::json!({
        "bdf": bdf,
        "sm_version": sm,
        "generation": profile.name,
        "ce_class": format!("{:#06X}", profile.ce_class),
        "tier": evidence.tier,
        "tier_level": evidence.tier.level(),
        "tier_description": evidence.tier.description(),
        "evidence": {
            "pmc_enable": format!("{:#010x}", evidence.pmc_enable),
            "pmc_popcount": evidence.pmc_popcount,
            "pramin_accessible": evidence.pramin_accessible,
            "fecs_pc": evidence.fecs_pc.map(|v| format!("{:#010x}", v)),
            "gpc_enables": evidence.gpc_enables.map(|v| format!("{:#010x}", v)),
            "ce_status": evidence.ce_status.map(|v| format!("{:#010x}", v)),
            "gr_status": evidence.gr_status.map(|v| format!("{:#010x}", v)),
            "pbdma_intr": evidence.pbdma_intr.map(|v| format!("{:#010x}", v)),
            "ce_runlist": evidence.ce_runlist,
            "tpc_status": evidence.tpc_status.map(|v| format!("{:#010x}", v)),
            "tpc_alive": evidence.tpc_alive,
        },
        "profile_offsets": {
            "fecs_pc": format!("{:#010x}", profile.fecs_pc_offset),
            "gpc_broadcast": format!("{:#010x}", profile.gpc_broadcast_offset),
            "ce0_base": format!("{:#010x}", profile.ce0_base_offset),
            "pgraph_status": format!("{:#010x}", profile.pgraph_status_offset),
        }
    }))
}

/// `sovereign.kernel_health` — kernel build environment health check.
///
/// Runs a 3-layer detection (autoconf freshness, struct layout probe,
/// reference cross-check) and returns a full health report. Optionally
/// attempts repair if `repair` is set to `true`.
///
/// Params:
/// - `repair` (optional, bool): attempt to repair via .deb cache if unhealthy
/// - `repair_strategy` (optional, string): `"PackageRestore"` (default) or `"PackageReinstall"`
pub fn sovereign_kernel_health(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    use toadstool_cylinder::vfio::kernel_health;

    let report = kernel_health::full_kernel_health_check()
        .map_err(|e| JsonRpcError::internal_error(format!("kernel health check failed: {e}")))?;

    let should_repair = params
        .and_then(|p| p.get("repair"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let mut repair_result = None;

    if should_repair && !report.layout_matches {
        let strategy = params
            .and_then(|p| p.get("repair_strategy"))
            .and_then(Value::as_str)
            .unwrap_or("PackageRestore");

        let strat = match strategy {
            "PackageReinstall" => kernel_health::RepairStrategy::PackageReinstall,
            _ => kernel_health::RepairStrategy::PackageRestore,
        };

        match kernel_health::repair_autoconf(strat) {
            Ok(path) => {
                info!(path = %path.display(), "autoconf.h repaired");
                repair_result = Some(serde_json::json!({
                    "success": true,
                    "restored_path": path.display().to_string(),
                    "strategy": strategy,
                }));
            }
            Err(e) => {
                repair_result = Some(serde_json::json!({
                    "success": false,
                    "error": e.to_string(),
                    "strategy": strategy,
                }));
            }
        }
    }

    let report_json = serde_json::to_value(&report)
        .map_err(|e| JsonRpcError::internal_error(format!("serialization: {e}")))?;

    Ok(serde_json::json!({
        "report": report_json,
        "repair": repair_result,
    }))
}

/// `sovereign.runtime_services_probe` — Probe nvidia's live state for runtime services.
///
/// When nvidia stays bound as a runtime compute service, this RPC probes what
/// nvidia has established: driver binding, FECS context, TPC stations, channels.
///
/// Params:
/// - `bdf` (required): PCI BDF address (e.g. `"0000:41:00.0"`)
pub fn sovereign_runtime_services_probe(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    use toadstool_cylinder::vfio::sovereign_handoff;

    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

    info!(bdf = bdf, "sovereign.runtime_services_probe: probing nvidia state");

    let probe = sovereign_handoff::probe_runtime_services(bdf);

    Ok(serde_json::json!({
        "bdf": probe.bdf,
        "driver": probe.driver,
        "nvidia_loaded": probe.nvidia_loaded,
        "fecs_state": probe.fecs_state,
        "tpc_alive": probe.tpc_alive,
        "nvidia_channels": probe.nvidia_channels,
        "runtime_services_ready": probe.nvidia_loaded && probe.tpc_alive,
    }))
}
