// SPDX-License-Identifier: AGPL-3.0-or-later
//! Sovereign GPU initialization JSON-RPC handler.
//!
//! Exposes `sovereign.init` — the staged diesel-engine pipeline that brings a
//! VFIO-bound GPU from cold/warm state to compute-ready.

use crate::pure_jsonrpc::types::JsonRpcError;
use serde_json::Value;
use toadstool_cylinder::vfio::sovereign_init::SovereignInitOptions;
use tracing::{info, debug};

const DEFAULT_BAR0_SIZE: usize = 16 * 1024 * 1024;

/// `sovereign.init` — Run the full sovereign init pipeline on a GPU.
///
/// Opens BAR0 via sysfs or VFIO device, runs `cylinder::sovereign_init`
/// stages, returns per-stage results.
///
/// Params:
/// - `bdf` (required): PCI BDF address (e.g. `"0000:4b:00.0"`)
/// - `bar0_source` (optional): `"sysfs"` (default) or `"vfio"`. Use `"vfio"` for
///   GPUs bound to vfio-pci where sysfs resource0 is not accessible (e.g. K80).
///   VFIO path uses `open_no_busmaster` to avoid PFIFO DMA faults on Kepler.
/// - `halt_before` (optional): Stop before a stage (`"pmc_enable"`, `"cg_sweep"`,
///   `"pgob_ungate"`, `"memory_training"`, `"engine_ungate"`, `"falcon_boot"`,
///   `"gr_init"`, `"verify"`)
/// - `skip_gr_init` (optional, default false): Skip GR init stage
/// - `golden_state_path` (optional): Path to golden-state JSON for HBM2 replay
/// - `engine_init_path` (optional): Path to `GrInitSequence` JSON for silicon-deistic
///   engine replay (captured via `WarmStateCapture`, replayed without vendor driver)
/// - `vbios_rom_path` (optional): Path to raw VBIOS ROM dump
/// - `sm_version` (optional): SM version override (auto-detected if omitted)
/// - `fbpa_count` (optional): FBPA partition count override (auto-detected)
#[expect(
    clippy::collection_is_never_read,
    reason = "VfioDevice anchor must outlive MappedBar — keeps mmap valid"
)]
pub fn sovereign_init(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

    let bar0_source = params
        .and_then(|p| p.get("bar0_source"))
        .and_then(Value::as_str)
        .unwrap_or("auto");

    info!(bdf = %bdf, bar0_source, "sovereign.init: opening BAR0");

    // VfioDevice must outlive MappedBar (mmap borrows device fd)
    let mut _vfio_anchor = None;
    let mut dma_backend_for_opts = None;

    let _gate_bypass = toadstool_cylinder::vfio::ember_gate::EmberGateBypass::enter();

    #[expect(clippy::single_match_else, reason = "bar0_source may grow more variants")]
    let bar0 = match bar0_source {
        "vfio" => {
            let dev = toadstool_cylinder::vfio::VfioDevice::open_no_busmaster(bdf)
                .map_err(|e| {
                    JsonRpcError::internal_error(format!(
                        "VFIO device open failed for {bdf}: {e}. Ensure vfio-pci is bound and IOMMU is configured."
                    ))
                })?;
            let bar = dev.map_bar(0).map_err(|e| {
                JsonRpcError::internal_error(format!(
                    "VFIO BAR0 map failed for {bdf}: {e}"
                ))
            })?;
            dma_backend_for_opts = Some(dev.dma_backend());
            info!(bdf = %bdf, "sovereign.init: VFIO opened (DMA backend available)");
            _vfio_anchor = Some(dev);
            bar
        }
        _ => {
            let bar = toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
                .map_err(|e| {
                    JsonRpcError::internal_error(format!(
                        "BAR0 open failed for {bdf}: {e}. Ensure vfio-pci is bound and resource0 is accessible."
                    ))
                })?;
            // Try to acquire a DMA backend via iommufd cdev even when using
            // sysfs BAR0 — this enables ACR falcon boot on warm GPUs where
            // the VFIO group may be busy but the cdev is accessible.
            match toadstool_cylinder::vfio::VfioDevice::open_no_busmaster(bdf) {
                Ok(dev) => {
                    dma_backend_for_opts = Some(dev.dma_backend());
                    info!(bdf = %bdf, "sovereign.init: DMA backend acquired via VFIO cdev (sysfs BAR0)");
                    _vfio_anchor = Some(dev);
                }
                Err(e) => {
                    info!(bdf = %bdf, err = %e, "sovereign.init: no DMA backend available (sysfs-only)");
                }
            }
            bar
        }
    };

    let mut opts: SovereignInitOptions = if let Some(p) = params {
        serde_json::from_value(p.clone()).unwrap_or_default()
    } else {
        SovereignInitOptions::default()
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

    if let Some(path) = opts.engine_init_path.as_ref() {
        match std::fs::read_to_string(path) {
            Ok(json_str) => {
                match toadstool_cylinder::nv::gr_init::GrInitSequence::from_json(&json_str) {
                    Ok(seq) => {
                        let engine = seq.chip.engine_label();
                        info!(path = %path, engine = %engine, writes = seq.len(),
                              "loaded golden-state GrInitSequence for engine replay");
                        opts.engine_init_sequences.push((
                            engine,
                            seq,
                            None,
                        ));
                    }
                    Err(e) => {
                        info!(path = %path, error = %e, "engine_init_path JSON parse failed, continuing without");
                    }
                }
            }
            Err(e) => {
                info!(path = %path, error = %e, "engine_init_path read failed, continuing without");
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

    opts.dma_backend = dma_backend_for_opts;

    let sm = opts.sm_version.unwrap_or_else(|| {
        let boot0 = bar0.read_u32(0).unwrap_or(0);
        let chip_id = (boot0 >> 20) & 0x1FF;
        let synthetic = chip_id << 20;
        toadstool_cylinder::nv::identity::boot0_to_sm(synthetic).unwrap_or(70)
    });
    let chip = toadstool_cylinder::nv::identity::chip_name(sm);

    let bridge: std::sync::Arc<dyn toadstool_cylinder::nv::gsp_bridge::GspBridge> = {
        let nv = toadstool_cylinder::nv::nv_gsp_bridge::NvGspBridge::new(chip);
        if nv.has_gr_firmware() {
            info!(chip, "sovereign.init: using NvGspBridge (firmware found)");
            std::sync::Arc::new(nv)
        } else {
            info!(chip, "sovereign.init: using StubGspBridge (no firmware)");
            std::sync::Arc::new(toadstool_cylinder::nv::gsp_bridge::StubGspBridge::default())
        }
    };

    let profile = toadstool_cylinder::nv::generation::profile_for_sm(sm);
    let strategy = toadstool_cylinder::vfio::sovereign_strategy::strategy_for_profile(
        profile, bridge, sm,
    );

    info!(bdf = %bdf, halt_before = ?opts.halt_before, "sovereign.init: starting pipeline");

    let result = toadstool_cylinder::vfio::sovereign_init::sovereign_init(
        &bar0, bdf, &opts, &*strategy,
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

/// `sovereign.experiment` — execute a staged warm compute experiment.
///
/// Runs a single experiment stage (1-6) on a VFIO-bound GPU, capturing
/// before/after register snapshots and the writes performed. Designed
/// for interactive exploration of Tier 1 → Tier 2 transitions.
///
/// Stages 4 and 6 now use `NvGspBridge` with real GV100 firmware
/// (`sw_nonctx.bin`) instead of `StubGspBridge`. Stage 6 is the full
/// 5-phase ungating sequence including PGRAPH reset (Exp 217).
///
/// Params:
/// - `bdf` (required): PCI BDF address (e.g. `"0000:02:00.0"`)
/// - `stage` (required): Stage number 1-6
///
/// Returns: `ExperimentResult` with before/after snapshots, diff, writes, and notes.
pub fn sovereign_experiment(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf'"))?;

    let stage = params
        .and_then(|p| p.get("stage"))
        .and_then(Value::as_u64)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'stage' (1-6)"))?
        as u32;

    let chip = params
        .and_then(|p| p.get("chip"))
        .and_then(Value::as_str);

    info!(bdf, stage, chip = ?chip, "sovereign.experiment: starting stage");

    let bar0 =
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("BAR0 open failed for {bdf}: {e}"))
            })?;

    let result = toadstool_cylinder::vfio::sovereign_stages::run_experiment_stage(&bar0, stage, chip)
        .map_err(JsonRpcError::invalid_params)?;

    info!(
        bdf,
        stage,
        diff_count = result.diff.len(),
        writes = result.writes.len(),
        "sovereign.experiment: stage complete"
    );

    // Also run tier classification after the experiment
    let tier = toadstool_cylinder::vfio::sovereign_tiers::classify_tier(&bar0);

    serde_json::to_value(serde_json::json!({
        "bdf": bdf,
        "stage": result.stage,
        "stage_name": result.stage_name,
        "diff": result.diff,
        "writes": result.writes,
        "notes": result.notes,
        "before": result.before,
        "after": result.after,
        "tier_after": {
            "tier": tier.tier,
            "tier_level": tier.tier.level(),
            "description": tier.tier.description(),
            "gpc_enables": tier.gpc_enables.map(|v| format!("{v:#010x}")),
            "ce_status": tier.ce_status.map(|v| format!("{v:#010x}")),
            "tpc_status": tier.tpc_status.map(|v| format!("{v:#010x}")),
            "tpc_alive": tier.tpc_alive,
        },
    }))
    .map_err(|e| JsonRpcError::internal_error(format!("serialization failed: {e}")))
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

/// `sovereign.snapshot` — read-only register snapshot + tier classification.
///
/// Captures a [`SovereignSnapshot`] and [`TierEvidence`] without performing
/// any mutating BAR0 writes. Suitable for baseline captures before experiments
/// and cross-GPU comparison.
///
/// Params:
/// - `bdf` (required): PCI BDF address (e.g. `"0000:02:00.0"`)
pub fn sovereign_snapshot(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

    info!(bdf, "sovereign.snapshot: capturing read-only snapshot");

    let bar0 =
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("BAR0 open failed for {bdf}: {e}"))
            })?;

    let (snapshot, tier) =
        toadstool_cylinder::vfio::sovereign_stages::sovereign_snapshot_only(&bar0);

    info!(
        bdf,
        tier = ?tier.tier,
        pmc_popcount = tier.pmc_popcount,
        "sovereign.snapshot: captured"
    );

    Ok(serde_json::json!({
        "bdf": bdf,
        "snapshot": snapshot,
        "tier": {
            "tier": tier.tier,
            "tier_level": tier.tier.level(),
            "tier_description": tier.tier.description(),
            "evidence": {
                "pmc_enable": format!("{:#010x}", tier.pmc_enable),
                "pmc_popcount": tier.pmc_popcount,
                "pramin_accessible": tier.pramin_accessible,
                "fecs_pc": tier.fecs_pc.map(|v| format!("{:#010x}", v)),
                "gpc_enables": tier.gpc_enables.map(|v| format!("{:#010x}", v)),
                "ce_status": tier.ce_status.map(|v| format!("{:#010x}", v)),
                "gr_status": tier.gr_status.map(|v| format!("{:#010x}", v)),
                "tpc_status": tier.tpc_status.map(|v| format!("{:#010x}", v)),
                "tpc_alive": tier.tpc_alive,
            }
        }
    }))
}

/// `sovereign.compare` — twin-card structured diff.
///
/// Captures [`SovereignSnapshot`] from two BDFs and returns both snapshots
/// plus a structured list of register deltas. This is the twin-study primitive
/// for cross-GPU comparison.
///
/// Params:
/// - `bdf_a` (required): First PCI BDF address
/// - `bdf_b` (required): Second PCI BDF address
pub fn sovereign_compare(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    use toadstool_cylinder::vfio::sovereign_stages::{SovereignSnapshot, sovereign_snapshot_only};

    let bdf_a = params
        .and_then(|p| p.get("bdf_a"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf_a' string parameter"))?;

    let bdf_b = params
        .and_then(|p| p.get("bdf_b"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf_b' string parameter"))?;

    info!(bdf_a, bdf_b, "sovereign.compare: capturing twin snapshots");

    let bar0_a =
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf_a, DEFAULT_BAR0_SIZE)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("BAR0 open failed for {bdf_a}: {e}"))
            })?;

    let bar0_b =
        toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf_b, DEFAULT_BAR0_SIZE)
            .map_err(|e| {
                JsonRpcError::internal_error(format!("BAR0 open failed for {bdf_b}: {e}"))
            })?;

    let (snap_a, tier_a) = sovereign_snapshot_only(&bar0_a);
    let (snap_b, tier_b) = sovereign_snapshot_only(&bar0_b);

    let deltas = SovereignSnapshot::diff_structured(&snap_a, &snap_b);

    info!(
        bdf_a,
        bdf_b,
        delta_count = deltas.len(),
        tier_a = ?tier_a.tier,
        tier_b = ?tier_b.tier,
        "sovereign.compare: diff complete"
    );

    Ok(serde_json::json!({
        "bdf_a": bdf_a,
        "bdf_b": bdf_b,
        "snapshot_a": snap_a,
        "snapshot_b": snap_b,
        "tier_a": {
            "tier": tier_a.tier,
            "tier_level": tier_a.tier.level(),
        },
        "tier_b": {
            "tier": tier_b.tier,
            "tier_level": tier_b.tier.level(),
        },
        "deltas": deltas,
        "delta_count": deltas.len(),
    }))
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

/// `sovereign.reagent_capture` — Capture firmware reagents while nvidia is loaded.
///
/// Orchestrates the full reagent capture pipeline:
/// 1. Probes nvidia state (driver bound, FECS/TPC liveness)
/// 2. Catalogs linux-firmware blobs for the chip
/// 3. Optionally distills mmiotrace to ACR recipe
/// 4. Copies existing catalyst artifacts
/// 5. Persists ReagentManifest to storage
///
/// Params:
/// - `bdf` (required): PCI BDF address (e.g. `"0000:41:00.0"`)
/// - `chip` (optional, default `"gv100"`): GPU chip identifier
/// - `driver_version` (optional, default `"470.256.02"`): nvidia driver version
/// - `mmiotrace_path` (optional): Path to mmiotrace log for ACR recipe distillation
pub fn sovereign_reagent_capture(params: Option<&Value>) -> Result<Value, JsonRpcError> {
    use toadstool_cylinder::vfio::reagent;

    let bdf = params
        .and_then(|p| p.get("bdf"))
        .and_then(Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'bdf' string parameter"))?;

    let chip = params
        .and_then(|p| p.get("chip"))
        .and_then(Value::as_str)
        .unwrap_or("gv100");

    let driver_version = params
        .and_then(|p| p.get("driver_version"))
        .and_then(Value::as_str)
        .unwrap_or("470.256.02");

    let mmiotrace_path = params
        .and_then(|p| p.get("mmiotrace_path"))
        .and_then(Value::as_str);

    info!(
        bdf = bdf,
        chip = chip,
        driver_version = driver_version,
        mmiotrace_path = ?mmiotrace_path,
        "sovereign.reagent_capture: starting capture pipeline"
    );

    let mut result = reagent::execute_reagent_capture(bdf, chip, driver_version);

    // Optionally distill mmiotrace
    if let Some(trace_path) = mmiotrace_path {
        let trace = std::path::Path::new(trace_path);
        if trace.exists() {
            let output = result.manifest.store_path().join("mmiotrace").join(
                format!("{}_recipe.json", trace.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "trace".to_owned()))
            );
            match reagent::distill_mmiotrace_to_reagent(trace, &output) {
                Ok(summary) => {
                    result.manifest.mmiotrace_recipe = Some(output);
                    result.manifest.completeness.mmiotrace_recipe = true;
                    result.strategy_results.insert(
                        "mmiotrace_distill".to_owned(),
                        reagent::StrategyResult {
                            success: true,
                            detail: format!(
                                "{} writes → {} recipe steps ({} ACR-relevant)",
                                summary.total_writes, summary.recipe_steps, summary.acr_steps
                            ),
                            artifacts: std::collections::HashMap::from([
                                ("recipe".to_owned(), summary.recipe_steps as u64),
                                ("acr_subset".to_owned(), summary.acr_steps as u64),
                            ]),
                        },
                    );
                }
                Err(e) => {
                    result.strategy_results.insert(
                        "mmiotrace_distill".to_owned(),
                        reagent::StrategyResult {
                            success: false,
                            detail: format!("Distillation failed: {e}"),
                            artifacts: std::collections::HashMap::new(),
                        },
                    );
                }
            }
        }
    }

    // Re-persist with updated mmiotrace data
    if let Ok(path) = result.manifest.persist() {
        result.manifest_path = Some(path);
    }

    let strategy_summary: Vec<_> = result
        .strategy_results
        .iter()
        .map(|(name, r)| {
            serde_json::json!({
                "strategy": name,
                "success": r.success,
                "detail": r.detail,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "chip": result.manifest.chip,
        "bdf": bdf,
        "driver_version": driver_version,
        "captured_at": result.manifest.captured_at,
        "manifest_path": result.manifest_path,
        "completeness": {
            "fraction": result.manifest.completeness.fraction(),
            "bar0_snapshot": result.manifest.completeness.bar0_snapshot,
            "linux_firmware": result.manifest.completeness.linux_firmware,
            "mmiotrace_recipe": result.manifest.completeness.mmiotrace_recipe,
            "falcon_firmware": result.manifest.completeness.falcon_firmware,
            "patch_set": result.manifest.completeness.patch_set,
            "vram_firmware": result.manifest.completeness.vram_firmware,
        },
        "linux_firmware_blobs": result.manifest.firmware.linux_firmware_blobs.len(),
        "strategies": strategy_summary,
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
        match sovereign_snapshot(Some(&serde_json::json!({"bdf": bdf}))) {
            Ok(snap) => {
                resp.as_object_mut().unwrap().insert("post_snapshot".to_owned(), snap);
            }
            Err(e) => {
                resp.as_object_mut()
                    .unwrap()
                    .insert("snapshot_error".to_owned(), Value::String(e.message.to_string()));
            }
        }
    }

    Ok(resp)
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
