// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign GPU initialization JSON-RPC handler.
//!
//! Exposes `sovereign.init` — the staged diesel-engine pipeline that brings a
//! VFIO-bound GPU from cold/warm state to compute-ready.

use crate::pure_jsonrpc::types::JsonRpcError;
use serde_json::Value;
use tracing::info;

const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;

/// `sovereign.init` — Run the full sovereign init pipeline on a GPU.
///
/// Opens BAR0 via sysfs, runs `cylinder::sovereign_init` stages, returns
/// per-stage results. The GPU must be VFIO-bound (or unbound) with BAR0
/// accessible via `/sys/bus/pci/devices/{bdf}/resource0`.
///
/// Params:
/// - `bdf` (required): PCI BDF address (e.g. `"0000:4b:00.0"`)
/// - `halt_before` (optional): Stop before a stage (`"pmc_enable"`, `"hbm2_training"`,
///   `"kepler_pgraph_ungate"`, `"falcon_boot"`, `"gr_init"`, `"verify"`)
/// - `skip_gr_init` (optional, default false): Skip GR init stage
/// - `golden_state_path` (optional): Path to golden-state JSON for HBM2 replay
/// - `vbios_rom_path` (optional): Path to raw VBIOS ROM dump
/// - `sm_version` (optional): SM version override (auto-detected if omitted)
/// - `fbpa_count` (optional): FBPA partition count override (auto-detected)
pub fn sovereign_init(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

    info!(bdf = %bdf, "sovereign.init: opening BAR0");

    let bar0 = toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
        .map_err(|e| {
            JsonRpcError::internal_error(format!(
                "BAR0 open failed for {bdf}: {e}. Ensure vfio-pci is bound and resource0 is accessible."
            ))
        })?;

    let mut opts: toadstool_cylinder::vfio::sovereign_init::SovereignInitOptions =
        if let Some(p) = params {
            serde_json::from_value(p.clone()).unwrap_or_default()
        } else {
            Default::default()
        };

    if let Some(path) = opts.golden_state_path.as_ref() {
        match std::fs::read_to_string(path) {
            Ok(json_str) => {
                if let Ok(pairs) =
                    serde_json::from_str::<Vec<(usize, u32)>>(&json_str)
                {
                    opts.golden_state = Some(pairs);
                }
            }
            Err(e) => {
                info!(path = %path, error = %e, "golden_state_path read failed, continuing without");
            }
        }
    }

    if let Some(path) = opts.vbios_rom_path.as_ref() {
        match std::fs::read(path) {
            Ok(rom) => {
                opts.vbios_rom = Some(rom);
            }
            Err(e) => {
                info!(path = %path, error = %e, "vbios_rom_path read failed, continuing without");
            }
        }
    }

    let bridge = toadstool_cylinder::nv::gsp_bridge::StubGspBridge;

    info!(bdf = %bdf, halt_before = ?opts.halt_before, "sovereign.init: starting pipeline");

    let result = toadstool_cylinder::vfio::sovereign_init::sovereign_init(
        &bar0, bdf, &opts, &bridge,
    );

    info!(
        bdf = %bdf,
        all_ok = result.all_ok,
        compute_ready = result.compute_ready,
        total_ms = result.total_ms,
        stages = result.stages.len(),
        warm_detected = result.warm_detected,
        "sovereign.init: pipeline complete"
    );

    serde_json::to_value(&result)
        .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
}
