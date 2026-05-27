// SPDX-License-Identifier: AGPL-3.0-or-later
//! Devinit and catalyst handoff handlers.

use crate::pure_jsonrpc::types::JsonRpcError;
use serde_json::Value;
use tracing::info;

pub(super) const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;

/// `sovereign.devinit` — Run PMU FALCON devinit (VBIOS-based memory training).
///
/// Reads VBIOS from PROM, parses BIT table, uploads PMU firmware, executes
/// devinit. Falls back to host-side VBIOS interpreter if FALCON fails.
/// This is the K80/Kepler cold-boot path to bring up GDDR5 VRAM.
///
/// Params:
/// - `bdf` (required): PCI BDF address
/// - `bar0_source` (optional): `"sysfs"` (default) or `"vfio"`. Use `"vfio"` for
///   GPUs bound to vfio-pci where sysfs resource0 is not accessible.
/// - `vbios_path` (optional): Path to pre-dumped VBIOS ROM file
#[expect(
    clippy::collection_is_never_read,
    reason = "VfioDevice anchor must outlive MappedBar — keeps mmap valid"
)]
pub fn sovereign_devinit(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    use toadstool_cylinder::vfio::channel::devinit;

    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

    let bar0_source = params
        .and_then(|p| p.get("bar0_source"))
        .and_then(Value::as_str)
        .unwrap_or("sysfs");

    info!(bdf = %bdf, bar0_source, "sovereign.devinit: opening BAR0");

    let mut _vfio_anchor = None;

    let bar0 = if bar0_source == "vfio" {
        let dev = toadstool_cylinder::vfio::VfioDevice::open_no_busmaster(bdf)
            .map_err(|e| {
                JsonRpcError::internal_error(format!(
                    "VFIO device open failed for {bdf}: {e}"
                ))
            })?;
        let bar = dev.map_bar(0).map_err(|e| {
            JsonRpcError::internal_error(format!(
                "VFIO BAR0 map failed for {bdf}: {e}"
            ))
        })?;
        _vfio_anchor = Some(dev);
        bar
    } else {
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("BAR0 open failed for {bdf}: {e}"))
            })?
    };

    let diag = devinit::FalconDiagnostic::probe(&bar0, Some(bdf));

    let diag_json = serde_json::json!({
        "needs_post": diag.status.needs_post,
        "devinit_reg": format!("{:#010x}", diag.status.devinit_reg),
        "pmu_id": format!("{:#010x}", diag.status.pmu_id),
        "pmu_hwcfg": format!("{:#010x}", diag.status.pmu_hwcfg),
        "pmu_ctrl": format!("{:#010x}", diag.status.pmu_ctrl),
        "pmu_mbox0": format!("{:#010x}", diag.status.pmu_mbox0),
        "prom_accessible": diag.prom_accessible,
        "prom_signature": format!("{:#010x}", diag.prom_signature),
        "secure_boot": diag.secure_boot,
        "falcon_halted": diag.falcon_halted,
        "falcon_pc": format!("{:#010x}", diag.falcon_pc),
        "imem_kb": diag.imem_size_kb,
        "dmem_kb": diag.dmem_size_kb,
        "vbios_sources": diag.vbios_sources.iter().map(|(name, ok, detail)| {
            serde_json::json!({"name": name, "available": ok, "detail": detail})
        }).collect::<Vec<_>>(),
    });

    if !diag.status.needs_post {
        return Ok(serde_json::json!({
            "bdf": bdf,
            "action": "none",
            "reason": "devinit already complete",
            "diagnostic": diag_json,
        }));
    }

    info!(bdf = %bdf, prom = diag.prom_accessible, secure = diag.secure_boot,
          "sovereign.devinit: GPU needs POST, attempting devinit");

    match devinit::execute_devinit_with_diagnostics(&bar0, Some(bdf)) {
        Ok(true) => {
            info!(bdf = %bdf, "sovereign.devinit: VRAM alive after devinit");
            Ok(serde_json::json!({
                "bdf": bdf,
                "action": "devinit_executed",
                "vram_alive": true,
                "diagnostic": diag_json,
            }))
        }
        Ok(false) => {
            Ok(serde_json::json!({
                "bdf": bdf,
                "action": "devinit_not_needed",
                "vram_alive": false,
                "diagnostic": diag_json,
            }))
        }
        Err(e) => {
            info!(bdf = %bdf, error = %e, "sovereign.devinit: devinit failed");
            Ok(serde_json::json!({
                "bdf": bdf,
                "action": "devinit_failed",
                "error": e.to_string(),
                "diagnostic": diag_json,
            }))
        }
    }
}

/// `sovereign.catalyst_diff` — domain-scoped BAR0 twin-card differential for catalyst analysis.
///
/// Captures BAR0 snapshots (known Volta domains only) from two BDFs (cold
/// baseline vs catalyst-warmed), computes the diff, and produces a minimal
/// replay sequence containing only registers the catalyst changed.
///
/// Params:
/// - `bdf_cold` (required): PCI BDF of the cold/baseline GPU
/// - `bdf_warm` (required): PCI BDF of the catalyst-warmed GPU
/// - `persist_path` (optional): Directory to write diff + replay JSONs
pub fn sovereign_catalyst_diff(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    use toadstool_cylinder::nv::gr_init::{ChipFamily, InitSource};
    use toadstool_cylinder::nv::pri::VOLTA_BAR0_DOMAINS;
    use toadstool_cylinder::vfio::warm_capture::{Bar0Snapshot, Bar0Diff};

    let bdf_cold = params
        .and_then(|p| p.get("bdf_cold"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf_cold' string parameter"))?;

    let bdf_warm = params
        .and_then(|p| p.get("bdf_warm"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf_warm' string parameter"))?;

    let persist_path = params
        .and_then(|p| p.get("persist_path"))
        .and_then(Value::as_str);

    let domains = &VOLTA_BAR0_DOMAINS;
    info!(bdf_cold, bdf_warm, num_domains = domains.len(),
          "sovereign.catalyst_diff: capturing domain-scoped BAR0 snapshots");

    let bar0_cold =
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf_cold, DEFAULT_BAR0_SIZE)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("BAR0 open failed for cold {bdf_cold}: {e}"))
            })?;

    let bar0_warm =
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf_warm, DEFAULT_BAR0_SIZE)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("BAR0 open failed for warm {bdf_warm}: {e}"))
            })?;

    let snap_cold = Bar0Snapshot::capture_domains(&bar0_cold, bdf_cold, "cold-baseline", domains);
    let snap_warm = Bar0Snapshot::capture_domains(&bar0_warm, bdf_warm, "catalyst-warm", domains);

    let diff = Bar0Diff::from_snapshots(&snap_cold, &snap_warm);

    let replay = diff.to_replay_sequence(
        ChipFamily::Volta,
        InitSource::Catalyst {
            driver_version: "470.256.02".into(),
            bdf: bdf_warm.to_string(),
        },
        VOLTA_BAR0_DOMAINS,
    );

    info!(
        bdf_cold, bdf_warm,
        changed = diff.changed_count(),
        replay_writes = replay.len(),
        domains = replay.domains().len(),
        "sovereign.catalyst_diff: diff complete"
    );

    // Persist artifacts if requested
    let mut persisted = serde_json::json!({});
    if let Some(dir) = persist_path {
        if let Err(e) = std::fs::create_dir_all(dir) {
            info!(err = %e, dir, "catalyst_diff: could not create persist dir");
        } else {
            let diff_path = format!("{dir}/gv100_catalyst_delta.json");
            if let Ok(json) = diff.to_json() {
                let _ = std::fs::write(&diff_path, &json);
                persisted["delta_path"] = serde_json::json!(diff_path);
            }

            let replay_path = format!("{dir}/gv100_catalyst_replay.json");
            if let Ok(json) = replay.to_json() {
                let _ = std::fs::write(&replay_path, &json);
                persisted["replay_path"] = serde_json::json!(replay_path);
            }

            let cold_path = format!("{dir}/gv100_cold_bar0.json");
            if let Ok(json) = snap_cold.to_json() {
                let _ = std::fs::write(&cold_path, &json);
                persisted["cold_snapshot_path"] = serde_json::json!(cold_path);
            }

            let warm_path = format!("{dir}/gv100_catalyst_bar0.json");
            if let Ok(json) = snap_warm.to_json() {
                let _ = std::fs::write(&warm_path, &json);
                persisted["warm_snapshot_path"] = serde_json::json!(warm_path);
            }
        }
    }

    let domain_summary: Vec<_> = replay.domain_summary()
        .into_iter()
        .map(|(d, c)| serde_json::json!({"domain": d, "writes": c}))
        .collect();

    Ok(serde_json::json!({
        "bdf_cold": bdf_cold,
        "bdf_warm": bdf_warm,
        "cold_alive_count": snap_cold.alive_count(),
        "warm_alive_count": snap_warm.alive_count(),
        "diff": {
            "changed_count": diff.changed_count(),
            "unchanged_count": diff.unchanged_count,
            "total_compared": diff.total_compared,
        },
        "replay": {
            "writes": replay.len(),
            "domains": domain_summary,
            "description": replay.description,
        },
        "persisted": persisted,
    }))
}

/// `sovereign.recipe_replay` — Replay a captured RecipeStep JSON on a VFIO GPU.
///
/// Loads a recipe (distilled mmiotrace or BAR0 diff) and writes every step
/// to BAR0 in order via VFIO.  Post-replay validates PTIMER and PMC_BOOT_0,
/// then takes a sovereign snapshot to show tier advancement.
///
/// Params:
/// - `bdf`         (required): PCI BDF address (e.g. `"0000:02:00.0"`)
/// - `recipe_path` (required): Absolute path to a `RecipeStep[]` JSON file
/// - `snapshot`    (optional, default true): Take a sovereign snapshot after replay
pub fn sovereign_recipe_replay(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    use toadstool_cylinder::vfio::channel::diagnostic::replay;

    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

    let recipe_path = params
        .and_then(|p| p.get("recipe_path"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'recipe_path' string parameter"))?;

    let take_snapshot = params
        .and_then(|p| p.get("snapshot"))
        .and_then(Value::as_bool)
        .unwrap_or(true);

    info!(bdf, recipe_path, "sovereign.recipe_replay: loading recipe");

    let recipe = replay::load_recipe(std::path::Path::new(recipe_path)).map_err(|e| {
        JsonRpcError::internal_error(format!("Failed to load recipe: {e}"))
    })?;

    info!(bdf, steps = recipe.len(), "sovereign.recipe_replay: opening sysfs BAR0");

    let bar0 = toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
        .map_err(|e| {
            JsonRpcError::internal_error(format!("BAR0 open failed for {bdf}: {e}"))
        })?;

    info!(bdf, steps = recipe.len(), "sovereign.recipe_replay: applying recipe");

    let result = replay::apply_recipe_to_bar0(&bar0, &recipe).map_err(|e| {
        JsonRpcError::internal_error(format!("Replay failed: {e}"))
    })?;

    let domain_summary: serde_json::Map<String, Value> = result
        .domain_counts
        .iter()
        .map(|(k, (ok, fail))| {
            (k.clone(), serde_json::json!({"applied": ok, "failed": fail}))
        })
        .collect();

    let mut resp = serde_json::json!({
        "bdf": bdf,
        "recipe_path": recipe_path,
        "total_steps": recipe.len(),
        "applied": result.applied,
        "failed": result.failed,
        "pmc_boot_0": format!("{:#010x}", result.pmc_boot_0),
        "ptimer_ticking": result.ptimer_ticking,
        "is_alive": result.is_alive(),
        "domains": domain_summary,
    });

    if take_snapshot {
        match super::capture::sovereign_snapshot(Some(&serde_json::json!({"bdf": bdf}))) {
            Ok(snap) => {
                if let Some(obj) = resp.as_object_mut() {
                    obj.insert("post_snapshot".to_owned(), snap);
                }
            }
            Err(e) => {
                if let Some(obj) = resp.as_object_mut() {
                    obj.insert(
                        "snapshot_error".to_owned(),
                        Value::String(e.message.to_string()),
                    );
                }
            }
        }
    }

    Ok(resp)
}
