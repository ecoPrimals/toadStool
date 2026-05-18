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
/// - `vbios_rom_path` (optional): Path to raw VBIOS ROM dump
/// - `sm_version` (optional): SM version override (auto-detected if omitted)
/// - `fbpa_count` (optional): FBPA partition count override (auto-detected)
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

    // Keep the VfioDevice alive for the duration of the pipeline — the MappedBar
    // borrows the device fd's mmap, so the device must outlive it.
    let mut _vfio_device;

    let mut dma_backend_for_opts = None;

    let _gate_bypass = toadstool_cylinder::vfio::ember_gate::EmberGateBypass::enter();

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
            _vfio_device = Some(dev);
            bar
        }
        _ => {
            _vfio_device = None;
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
                    _vfio_device = Some(dev);
                }
                Err(e) => {
                    info!(bdf = %bdf, err = %e, "sovereign.init: no DMA backend available (sysfs-only)");
                }
            }
            bar
        }
    };

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
        &profile, bridge, sm,
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

    let _vfio_device;

    let bar0 = match bar0_source {
        "vfio" => {
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
            _vfio_device = Some(dev);
            bar
        }
        _ => {
            _vfio_device = None;
            toadstool_cylinder::vfio::device::MappedBar::from_sysfs_rw(bdf, DEFAULT_BAR0_SIZE)
                .map_err(|e| {
                    JsonRpcError::internal_error(format!("BAR0 open failed for {bdf}: {e}"))
                })?
        }
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
