// SPDX-License-Identifier: AGPL-3.0-or-later
//! Per-stage implementations for [`crate::vfio::sovereign_init::sovereign_init`].

use std::time::{Duration, Instant};

use crate::error::SovereignStagesError;
use crate::vfio::channel::hbm2_training::{self, Hbm2Controller};
use crate::vfio::device::MappedBar;
use crate::vfio::sovereign_types::SovereignInitOptions;

pub(crate) const PMC_BOOT_0: usize = 0x0000_0000;
pub(crate) const PMC_ENABLE: usize = 0x0000_0200;
pub(crate) const PMC_INTR_EN_0: usize = 0x0000_0140;
pub(crate) const PTIMER_TIME_0: usize = 0x0000_9400;
pub(crate) const PTIMER_TIME_1: usize = 0x0000_9410;

pub(crate) const ISOLATE_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) fn bar0_probe(bar0: &MappedBar) -> Result<(u32, u32), SovereignStagesError> {
    let result = bar0.isolated_read_u32(PMC_BOOT_0 as u32, ISOLATE_TIMEOUT);
    let boot0 = match result {
        super::isolation::IsolationResult::Ok(v) => v,
        super::isolation::IsolationResult::Timeout => {
            return Err(SovereignStagesError::Bar0ProbeTimeout);
        }
        super::isolation::IsolationResult::ChildFailed { status } => {
            return Err(SovereignStagesError::Bar0ProbeChildFailed { status });
        }
        super::isolation::IsolationResult::ForkError(e) => {
            return Err(SovereignStagesError::Bar0ProbeFork(e));
        }
    };

    if boot0 == 0 || boot0 == 0xFFFF_FFFF {
        return Err(SovereignStagesError::Bar0ProbeNonResponsive { boot0 });
    }

    let chip_id = (boot0 >> 20) & 0x1FF;
    tracing::info!(
        boot0 = format!("0x{boot0:08x}"),
        chip_id = format!("0x{chip_id:03x}"),
        "BAR0 probe OK"
    );
    Ok((boot0, chip_id))
}

/// Staged PMC_ENABLE write using the generation's power safety profile.
///
/// For pre-firmware generations (Kepler, Maxwell), this writes only a
/// conservative mask to avoid bulk-ungating all engine clocks on a cold
/// GPU — the inrush current from 0xFFFF_FFFF on an aged K80 with
/// uninitialised GDDR5 is what caused the fire in Experiment 199.
///
/// Returns `(before, after, mask_used)` for logging.
pub(crate) fn pmc_enable(
    bar0: &MappedBar,
    power: &crate::nv::generation::PowerSafetyProfile,
) -> Result<PmcEnableResult, SovereignStagesError> {
    let before = bar0.read_u32(PMC_ENABLE).unwrap_or(0xDEAD_DEAD);
    tracing::debug!(pmc_before = format!("0x{before:08x}"), "PMC_ENABLE before");

    let mask = power.initial_pmc_mask;
    tracing::info!(
        mask = format!("0x{mask:08x}"),
        full_after_devinit = power.full_enable_after_devinit,
        rollback_on_failure = power.rollback_on_devinit_failure,
        "PMC_ENABLE staged write"
    );

    match bar0.isolated_write_u32(PMC_ENABLE as u32, mask, ISOLATE_TIMEOUT) {
        super::isolation::IsolationResult::Ok(()) => {}
        super::isolation::IsolationResult::Timeout => {
            return Err(SovereignStagesError::PmcEnableWriteTimeout);
        }
        other => {
            return Err(SovereignStagesError::PmcEnableIsolationFailure {
                message: format!("PMC_ENABLE write failed: {other:?}"),
            });
        }
    }
    std::thread::sleep(Duration::from_millis(50));

    let after = bar0.read_u32(PMC_ENABLE).unwrap_or(0xDEAD_DEAD);
    tracing::debug!(pmc_after = format!("0x{after:08x}"), "PMC_ENABLE after");

    if after == 0 || after == 0xDEAD_DEAD {
        return Err(SovereignStagesError::PmcEnableStuck { after });
    }

    match bar0.isolated_write_u32(PMC_INTR_EN_0 as u32, 0xFFFF_FFFF, ISOLATE_TIMEOUT) {
        super::isolation::IsolationResult::Ok(()) => {}
        other => {
            tracing::warn!("PMC_INTR_EN_0 write issue: {other:?}");
        }
    }

    Ok(PmcEnableResult { before, after, mask })
}

/// Result of a staged PMC_ENABLE write, kept for rollback.
#[derive(Debug, Clone)]
pub(crate) struct PmcEnableResult {
    pub before: u32,
    pub after: u32,
    pub mask: u32,
}

impl PmcEnableResult {
    pub fn detail(&self) -> String {
        format!(
            "before=0x{:08x} after=0x{:08x} mask=0x{:08x}",
            self.before, self.after, self.mask
        )
    }
}

/// Roll back PMC_ENABLE to its pre-pipeline value.
///
/// Called when devinit fails on a pre-firmware GPU to prevent the
/// partially-clocked state from persisting across power cycles.
pub(crate) fn pmc_enable_rollback(
    bar0: &MappedBar,
    restore_value: u32,
) -> Result<(), SovereignStagesError> {
    tracing::warn!(
        restore = format!("0x{restore_value:08x}"),
        "Rolling back PMC_ENABLE after devinit failure"
    );
    match bar0.isolated_write_u32(PMC_ENABLE as u32, restore_value, ISOLATE_TIMEOUT) {
        super::isolation::IsolationResult::Ok(()) => {
            std::thread::sleep(Duration::from_millis(20));
            let readback = bar0.read_u32(PMC_ENABLE).unwrap_or(0xDEAD_DEAD);
            tracing::info!(
                readback = format!("0x{readback:08x}"),
                "PMC_ENABLE rollback complete"
            );
            Ok(())
        }
        other => {
            tracing::error!("PMC_ENABLE rollback failed: {other:?}");
            Err(SovereignStagesError::PmcEnableIsolationFailure {
                message: format!("PMC_ENABLE rollback failed: {other:?}"),
            })
        }
    }
}

/// Post-devinit full enable for firmware-managed generations.
///
/// Only called after VBIOS devinit succeeds AND the profile allows it.
pub(crate) fn pmc_enable_full(bar0: &MappedBar) -> Result<String, SovereignStagesError> {
    let before = bar0.read_u32(PMC_ENABLE).unwrap_or(0xDEAD_DEAD);
    tracing::info!(
        pmc_before = format!("0x{before:08x}"),
        "PMC_ENABLE full ungating (post-devinit)"
    );

    match bar0.isolated_write_u32(PMC_ENABLE as u32, 0xFFFF_FFFF, ISOLATE_TIMEOUT) {
        super::isolation::IsolationResult::Ok(()) => {}
        super::isolation::IsolationResult::Timeout => {
            return Err(SovereignStagesError::PmcEnableWriteTimeout);
        }
        other => {
            return Err(SovereignStagesError::PmcEnableIsolationFailure {
                message: format!("PMC_ENABLE full write failed: {other:?}"),
            });
        }
    }
    std::thread::sleep(Duration::from_millis(50));

    let after = bar0.read_u32(PMC_ENABLE).unwrap_or(0xDEAD_DEAD);
    tracing::debug!(pmc_after = format!("0x{after:08x}"), "PMC_ENABLE full ungating done");

    Ok(format!("post_devinit_enable before=0x{before:08x} after=0x{after:08x}"))
}

/// PGRAPH engine reset via PMC_ENABLE bit 12 toggle.
///
/// After UEFI POST or driver handoff, PGRAPH's internal PRI fabric
/// (GPCs, FECS, GPCCS) can be in an inconsistent state — registers
/// read back PRI fault sentinels even though PMC reports the engine
/// as enabled. Toggling the GR bit resets PGRAPH's internal ring
/// stations and falcon state machines, matching nouveau's `mc_init`
/// sequence.
///
/// Must run *before* CG sweep and PRI recovery — those stages can't
/// clear faults inside a stale PGRAPH fabric.
pub(crate) fn pgraph_engine_reset(bar0: &MappedBar) -> Result<String, SovereignStagesError> {
    const GR_BIT: u32 = 1 << 12;
    const PGRAPH_STATUS: usize = 0x0040_0700;

    let pmc_before = bar0.read_u32(PMC_ENABLE).unwrap_or(0);
    let gr_was_enabled = pmc_before & GR_BIT != 0;

    if !gr_was_enabled {
        tracing::info!("PGRAPH not enabled in PMC — enabling without reset");
        let _ = bar0.write_u32(PMC_ENABLE, pmc_before | GR_BIT);
        std::thread::sleep(Duration::from_millis(10));
        let after = bar0.read_u32(PMC_ENABLE).unwrap_or(0);
        return Ok(format!(
            "pgraph_enable pmc=0x{pmc_before:08x}->0x{after:08x}"
        ));
    }

    // Toggle: clear GR bit, wait, re-set
    let _ = bar0.write_u32(PMC_ENABLE, pmc_before & !GR_BIT);
    std::thread::sleep(Duration::from_millis(10));

    let _ = bar0.write_u32(PMC_ENABLE, pmc_before | GR_BIT);
    std::thread::sleep(Duration::from_millis(20));

    // Poll PGRAPH_STATUS for up to 100ms — wait for PRI fault to clear
    let deadline = std::time::Instant::now() + Duration::from_millis(100);
    let mut status = bar0.read_u32(PGRAPH_STATUS).unwrap_or(0xDEAD_DEAD);
    while std::time::Instant::now() < deadline {
        if !crate::nv::pri::is_pri_fault(status) {
            break;
        }
        std::thread::sleep(Duration::from_micros(500));
        status = bar0.read_u32(PGRAPH_STATUS).unwrap_or(0xDEAD_DEAD);
    }

    let pmc_after = bar0.read_u32(PMC_ENABLE).unwrap_or(0);
    let fecs_cpuctl = bar0.read_u32(0x0040_9100).unwrap_or(0xDEAD_DEAD);
    let fecs_imem_sz = bar0.read_u32(0x0040_9140).unwrap_or(0);

    tracing::info!(
        pmc_before = format!("{pmc_before:#010x}"),
        pmc_after = format!("{pmc_after:#010x}"),
        pgraph_status = format!("{status:#010x}"),
        fecs_cpuctl = format!("{fecs_cpuctl:#010x}"),
        fecs_imem_kb = fecs_imem_sz,
        "PGRAPH engine reset complete"
    );

    Ok(format!(
        "pgraph_reset pmc=0x{pmc_before:08x}->0x{pmc_after:08x} status=0x{status:08x} fecs_imem={fecs_imem_sz}KB"
    ))
}

/// Clock gating sweep for Volta+ GPUs.
///
/// Disables ELCG/BLCG/SLCG across all known domains so that PGRAPH,
/// FBPA, LTC, and PFB registers become accessible. Without this, cold
/// GPUs return `0xBADF1100` / `0xBADF3000` on reads to clock-gated domains,
/// blocking HBM2 training and falcon DMA boot.
///
/// This is the sovereign_init equivalent of glowplug's `run_step_clock_gating`.
pub(crate) fn cg_sweep(bar0: &MappedBar) -> CgSweepResult {
    use crate::nv::pri::is_pri_fault;
    use crate::vfio::channel::registers::cg;

    let mut changes = 0u32;
    let mut faulted = 0u32;
    let mut detail_lines: Vec<String> = Vec::new();

    // Phase 1: Sweep all known CG control registers
    for &(offset, name) in cg::CG_SWEEP_TARGETS {
        let old = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
        if is_pri_fault(old) {
            faulted += 1;
            tracing::debug!(
                name,
                offset = format!("{offset:#08x}"),
                val = format!("{old:#010x}"),
                "CG sweep: domain unreachable"
            );
        } else if old != cg::CG_DISABLE {
            let _ = bar0.write_u32(offset, cg::CG_DISABLE);
            let new = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
            if old != new {
                changes += 1;
                detail_lines.push(format!("{name}: {old:#010x}->{new:#010x}"));
            }
        }
    }

    // Phase 2: Per-FBPA clock gating disable
    for i in 0..cg::FBPA_COUNT {
        let reg = cg::FBPA0_BASE + i * cg::FBPA_STRIDE + cg::FBPA_CG_OFFSET;
        let old = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        let _ = bar0.write_u32(reg, cg::CG_DISABLE);
        if is_pri_fault(old) {
            faulted += 1;
        } else {
            let new = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
            if old != new {
                changes += 1;
                detail_lines.push(format!("FBPA{i}: {old:#010x}->{new:#010x}"));
            }
        }
    }

    // Phase 3: Per-LTC clock gating disable
    for i in 0..cg::LTC_COUNT {
        let reg = cg::LTC0_BASE + i * cg::LTC_STRIDE + cg::LTC_CG_OFFSET;
        let old = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
        let _ = bar0.write_u32(reg, cg::CG_DISABLE);
        if is_pri_fault(old) {
            faulted += 1;
        } else {
            let new = bar0.read_u32(reg).unwrap_or(0xDEAD_DEAD);
            if old != new {
                changes += 1;
                detail_lines.push(format!("LTC{i}: {old:#010x}->{new:#010x}"));
            }
        }
    }

    tracing::info!(
        changes,
        faulted,
        "CG sweep complete"
    );

    CgSweepResult {
        changes,
        faulted,
        detail: if detail_lines.is_empty() {
            format!("{changes} changed, {faulted} faulted")
        } else {
            format!(
                "{changes} changed, {faulted} faulted [{}]",
                detail_lines.join(", ")
            )
        },
    }
}

#[derive(Debug, Clone)]
pub(crate) struct CgSweepResult {
    pub changes: u32,
    pub faulted: u32,
    pub detail: String,
}

/// PRI bus recovery — acknowledge pending PRIV_RING faults and re-probe.
///
/// After a CG sweep, some domains may have generated PRI faults during
/// the transition. This clears the fault state so subsequent register
/// reads don't hit stale backpressure.
///
/// Also clears PRI ringmaster-level errors (0x12200c) and re-enumerates
/// ring stations — without this, GPC/PGRAPH registers remain unreachable
/// after UEFI POST because the ringmaster retains stale fault state from
/// firmware handoff.
pub(crate) fn pri_bus_recover(bar0: &MappedBar) -> PriRecoveryResult {
    use crate::vfio::channel::pri_monitor::PriBusMonitor;
    use crate::vfio::channel::registers::pri;

    // Phase 0: Clear PRI ringmaster errors (0x122xxx layer).
    // The station-level ACK at 0x12004c doesn't touch these. Stale
    // ringmaster faults from UEFI/firmware handoff block all GPC and
    // PGRAPH register access.
    let rm_intr = bar0.read_u32(pri::PRI_RINGMASTER_INTR_STATUS).unwrap_or(0);
    if rm_intr != 0 {
        tracing::info!(
            rm_intr = format!("{rm_intr:#010x}"),
            "PRI ringmaster has pending errors — clearing and re-enumerating"
        );
        // Write-back to clear ringmaster interrupt bits
        let _ = bar0.write_u32(pri::PRI_RINGMASTER_INTR_STATUS, rm_intr);
        std::thread::sleep(Duration::from_millis(5));

        // Re-enumerate all ring stations so they re-register with the master
        let _ = bar0.write_u32(pri::PRI_RINGMASTER_COMMAND, pri::PRI_RINGMASTER_CMD_ENUMERATE);
        std::thread::sleep(Duration::from_millis(20));

        let rm_after = bar0.read_u32(pri::PRI_RINGMASTER_INTR_STATUS).unwrap_or(0);
        tracing::info!(
            rm_after = format!("{rm_after:#010x}"),
            "PRI ringmaster after enumerate"
        );
    }

    // Phase 1: Station-level fault recovery
    let mut monitor = PriBusMonitor::new(bar0);
    let health = monitor.probe_all_domains();
    let alive = health
        .iter()
        .filter(|(_, _, h)| {
            matches!(
                h,
                crate::vfio::channel::pri_monitor::DomainHealth::Alive
            )
        })
        .count();
    let faulted = health
        .iter()
        .filter(|(_, _, h)| {
            matches!(
                h,
                crate::vfio::channel::pri_monitor::DomainHealth::Faulted { .. }
            )
        })
        .count();

    let recovered = if faulted > 0 {
        monitor.attempt_recovery()
    } else {
        true
    };

    std::thread::sleep(Duration::from_millis(50));

    tracing::info!(
        alive,
        faulted,
        recovered,
        "PRI bus recovery after CG sweep"
    );

    PriRecoveryResult {
        alive,
        faulted,
        recovered,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PriRecoveryResult {
    pub alive: usize,
    pub faulted: usize,
    pub recovered: bool,
}

/// PGOB disable for Volta+ cold boot.
///
/// Ungates GPC compute domains via PMC clock gate + PGRAPH GPC broadcast.
/// Required before falcon DMA boot on cold GPUs where PGRAPH is power-gated.
/// Delegates to the bridge's implementation (NvGspBridge has the register
/// sequence, StubGspBridge no-ops).
pub(crate) fn pgob_ungating(
    bar0: &MappedBar,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
) -> Result<String, SovereignStagesError> {
    if !bridge.supports_pgob() {
        return Ok("pgob: skipped (no firmware provider)".into());
    }

    bridge.pgob_diagnostic(bar0, "sovereign::pre-PGOB");
    match bridge.pgob_disable(bar0) {
        Ok(out) => {
            bridge.pgob_diagnostic(bar0, "sovereign::post-PGOB");
            tracing::info!(gpc_alive = out.gpc_alive, "PGOB ungating succeeded");
            Ok(format!("pgob: {} GPCs alive", out.gpc_alive))
        }
        Err(e) => {
            tracing::warn!(%e, "PGOB ungating failed — GPCs may remain gated");
            Ok(format!("pgob: failed ({e})"))
        }
    }
}

pub(crate) fn run_hbm2_training(
    bar0: &MappedBar,
    bdf: &str,
    fbpa_count: usize,
    opts: &SovereignInitOptions,
) -> Result<crate::vfio::channel::hbm2_training::TrainingLog, SovereignStagesError> {
    let mut ctrl = Hbm2Controller::new(bar0, Some(bdf), fbpa_count);

    if let Some(golden) = &opts.golden_state {
        ctrl = ctrl.with_backend(hbm2_training::TrainingBackend::DifferentialReplay {
            golden_state: golden.clone(),
        });
    } else if let Some(rom) = &opts.vbios_rom {
        ctrl = ctrl
            .with_backend(hbm2_training::TrainingBackend::VbiosInterpreter { rom: rom.clone() });
    }

    let phy = ctrl.enable_phy()?;
    let linked = phy.train_links()?;
    let dram = linked.init_dram()?;

    match dram.verify_vram() {
        Ok(verified) => {
            let log = verified.training_log().clone();
            tracing::info!(
                writes = log.write_count(),
                "HBM2 training complete — VRAM verified"
            );
            Ok(log)
        }
        Err(e) => Err(e.into()),
    }
}

/// GDDR5 memory training for Kepler GPUs.
///
/// Cold K80s return `0xbad0fb0*` from PRAMIN because GDDR5 hasn't been
/// trained after PCI reset.  This function reads the VBIOS, runs the
/// DEVINIT script interpreter (via PMU falcon or host-side fallback),
/// and verifies PRAMIN returns valid data afterward.
pub(crate) fn gddr5_training(bar0: &MappedBar, bdf: &str) -> Result<String, SovereignStagesError> {
    use crate::vfio::channel::devinit;

    if pramin_sentinel_test(bar0) {
        return Ok("GDDR5 already trained (PRAMIN sentinel OK)".into());
    }

    tracing::info!("GDDR5 cold detected — running DEVINIT for memory training");

    match devinit::execute_devinit_with_diagnostics(bar0, Some(bdf)) {
        Ok(true) => {
            if pramin_sentinel_test(bar0) {
                Ok("GDDR5 trained via DEVINIT — PRAMIN verified".into())
            } else {
                Err(SovereignStagesError::Gddr5PraminDeadAfterDevinit)
            }
        }
        Ok(false) => {
            if pramin_sentinel_test(bar0) {
                Ok("DEVINIT reports already done — PRAMIN verified".into())
            } else {
                Err(SovereignStagesError::Gddr5PraminDeadDevinitSkipped)
            }
        }
        Err(e) => Err(e.into()),
    }
}

/// Strategy for memory training, keyed by [`MemoryType`].
///
/// Each GPU generation's `GenerationProfile::memory_type` maps to a
/// training strategy. The dispatch function runs the appropriate path
/// or returns a skip reason for types that don't need explicit training.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTrainingStrategy {
    /// GDDR5 (Kepler): DEVINIT script interpreter.
    Gddr5Devinit,
    /// HBM2/HBM2e (Volta, Ampere datacenter): typestate training controller.
    Hbm2Controller,
    /// Memory type exists but no sovereign training path implemented yet.
    Unsupported(&'static str),
}

impl MemoryTrainingStrategy {
    /// Determine the training strategy for a given memory type.
    #[must_use]
    pub fn for_memory_type(mem: crate::hardware::MemoryType) -> Self {
        use crate::hardware::MemoryType;
        match mem {
            MemoryType::Gddr5 => Self::Gddr5Devinit,
            MemoryType::Hbm2 => Self::Hbm2Controller,
            MemoryType::Hbm3 => Self::Unsupported("HBM3"),
            MemoryType::Gddr6 => Self::Unsupported("GDDR6"),
            MemoryType::Gddr6x => Self::Unsupported("GDDR6X"),
            MemoryType::Gddr7 => Self::Unsupported("GDDR7"),
        }
    }
}

/// Result of memory training dispatch.
pub enum MemoryTrainingResult {
    /// Training ran or was already warm — detail string.
    Ok(String),
    /// HBM2 training produced a full training log.
    OkWithLog(crate::vfio::channel::hbm2_training::TrainingLog),
    /// Skipped (already warm or unsupported type) — reason string.
    Skipped(String),
    /// Training failed.
    Failed(SovereignStagesError),
}

/// Dispatch memory training based on strategy and warm state.
///
/// Centralizes the branching logic that was previously inlined in
/// `sovereign_init`. Callers provide the strategy (from
/// `MemoryTrainingStrategy::for_memory_type`) and warm detection flag.
pub(crate) fn dispatch_memory_training(
    strategy: MemoryTrainingStrategy,
    bar0: &MappedBar,
    bdf: &str,
    warm_detected: bool,
    pmc_before: u32,
    opts: &SovereignInitOptions,
) -> MemoryTrainingResult {
    match strategy {
        MemoryTrainingStrategy::Gddr5Devinit => {
            if warm_detected {
                tracing::info!(
                    pmc_enable = format!("0x{pmc_before:08x}"),
                    "GDDR5 GPU warm — skipping memory training"
                );
                return MemoryTrainingResult::Skipped(format!(
                    "GDDR5 warm (pmc=0x{pmc_before:08x}, PRAMIN sentinel ok)"
                ));
            }
            match gddr5_training(bar0, bdf) {
                Ok(detail) => MemoryTrainingResult::Ok(detail),
                Err(e) => MemoryTrainingResult::Failed(e),
            }
        }
        MemoryTrainingStrategy::Hbm2Controller => {
            if warm_detected {
                tracing::info!(
                    pmc_enable = format!("0x{pmc_before:08x}"),
                    "warm GPU detected — skipping HBM2 training"
                );
                return MemoryTrainingResult::Skipped(format!(
                    "warm detected (pmc=0x{pmc_before:08x}, PRAMIN sentinel ok)"
                ));
            }
            // After early falcon boot, the PMU may have completed HBM2 training
            // autonomously.  Check PRAMIN before attempting the full controller path.
            if pramin_sentinel_test(bar0) {
                tracing::info!(
                    pmc_enable = format!("0x{pmc_before:08x}"),
                    "VRAM alive (PMU devinit completed) — skipping HBM2 training"
                );
                return MemoryTrainingResult::Skipped(format!(
                    "VRAM alive after falcon boot (pmc=0x{pmc_before:08x})"
                ));
            }

            // On cold secure-boot GPUs, try PMU FALCON devinit first.
            // The PMU firmware in the VBIOS ROM includes HBM2 training
            // sequences that the host-side interpreter cannot replicate.
            tracing::info!("HBM2 cold: trying PMU FALCON devinit before controller path");
            match crate::vfio::channel::devinit::execute_devinit_with_diagnostics(
                bar0,
                Some(bdf),
            ) {
                Ok(true) => {
                    tracing::info!("PMU FALCON devinit trained HBM2 — VRAM alive");
                    return MemoryTrainingResult::Ok(
                        "HBM2 trained via PMU FALCON devinit".into(),
                    );
                }
                Ok(false) => {
                    tracing::info!("PMU FALCON devinit: not needed or VRAM still dead");
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "PMU FALCON devinit failed — falling through to HBM2 controller"
                    );
                }
            }

            // Check PRAMIN again in case PMU devinit worked asynchronously
            if pramin_sentinel_test(bar0) {
                tracing::info!("VRAM alive after PMU devinit attempt");
                return MemoryTrainingResult::Ok(
                    "HBM2 trained via PMU FALCON devinit (delayed)".into(),
                );
            }

            let fbpa_count = opts.fbpa_count.unwrap_or(4);
            match run_hbm2_training(bar0, bdf, fbpa_count, opts) {
                Ok(log) => MemoryTrainingResult::OkWithLog(log),
                Err(e) => MemoryTrainingResult::Failed(e),
            }
        }
        MemoryTrainingStrategy::Unsupported(name) => {
            MemoryTrainingResult::Skipped(format!(
                "memory_type={name} (pmc=0x{pmc_before:08x})"
            ))
        }
    }
}

fn kepler_falcon_boot(
    bar0: &MappedBar,
    sm_version: u32,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
) -> Result<String, SovereignStagesError> {
    use crate::nv::falcon_pio::{falcon_upload_dmem, falcon_upload_imem};
    use crate::vfio::channel::registers::falcon;

    let profile = crate::nv::generation::profile_for_sm(sm_version);

    let _ = bar0.write_u32(0x260, 1);
    std::thread::sleep(Duration::from_millis(10));

    // PGOB disable: ungate GPC compute domains before any GR register access.
    // Without this, GPC reads return 0xBADF3000 and FECS boot fails.
    if crate::nv::generation::is_kepler(profile) {
        tracing::info!("Kepler falcon boot: running PGOB disable before GR init");
        bridge.pgob_diagnostic(bar0, "sovereign_stages::pre-PGOB");
        match bridge.pgob_disable(bar0) {
            Ok(out) => tracing::info!(gpc_alive = out.gpc_alive, "sovereign PGOB succeeded"),
            Err(e) => tracing::warn!(%e, "sovereign PGOB failed — GPCs may be gated"),
        }
        bridge.pgob_diagnostic(bar0, "sovereign_stages::post-PGOB");
    }

    let _ = bridge.apply_gr_bar0_init(bar0, sm_version);

    let fw_dir = format!("/lib/firmware/nvidia/{}", profile.firmware_chip);
    let load = |name: &str| -> Result<Vec<u8>, SovereignStagesError> {
        let path = format!("{fw_dir}/{name}");
        std::fs::read(&path).map_err(|e| SovereignStagesError::KeplerFirmwareRead {
            path: path.clone(),
            source: e,
        })
    };

    let gpccs_inst = load("gpccs_inst.bin")?;
    let gpccs_data = load("gpccs_data.bin")?;
    let fecs_inst = load("fecs_inst.bin")?;
    let fecs_data = load("fecs_data.bin")?;

    tracing::info!(
        fecs_inst = fecs_inst.len(),
        fecs_data = fecs_data.len(),
        gpccs_inst = gpccs_inst.len(),
        gpccs_data = gpccs_data.len(),
        "Kepler falcon boot: firmware loaded from {fw_dir}"
    );

    let boot_falcon = |name: &'static str,
                       base: usize,
                       inst: &[u8],
                       data: &[u8]|
     -> Result<(u32, u32), SovereignStagesError> {
        let cpuctl = bar0.read_u32(base + falcon::CPUCTL).unwrap_or(0xDEAD);
        tracing::info!(
            name,
            cpuctl = format!("{cpuctl:#010x}"),
            "Kepler {name}: starting PIO upload"
        );

        let _ = bar0.write_u32(base + falcon::CPUCTL, falcon::CPUCTL_HRESET);
        std::thread::sleep(Duration::from_millis(5));

        falcon_upload_dmem(bar0, base, 0, data);
        falcon_upload_imem(bar0, base, 0, inst, false);

        let _ = bar0.write_u32(base + falcon::BOOTVEC, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX0, 0);
        let _ = bar0.write_u32(base + falcon::MAILBOX1, 0);
        let _ = bar0.write_u32(base + falcon::CPUCTL, falcon::CPUCTL_IINVAL);
        std::thread::sleep(Duration::from_millis(1));
        let _ = bar0.write_u32(base + falcon::CPUCTL, falcon::CPUCTL_STARTCPU);

        let start = Instant::now();
        let timeout = Duration::from_secs(2);
        loop {
            std::thread::sleep(Duration::from_millis(5));
            let ctl = bar0.read_u32(base + falcon::CPUCTL).unwrap_or(0xDEAD);
            let mb0 = bar0.read_u32(base + falcon::MAILBOX0).unwrap_or(0);

            if mb0 != 0 {
                tracing::info!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    mb0 = format!("{mb0:#010x}"),
                    "mailbox response"
                );
                return Ok((ctl, mb0));
            }
            if ctl & falcon::CPUCTL_HALTED != 0 && ctl & falcon::CPUCTL_HRESET == 0 {
                tracing::warn!(
                    name,
                    cpuctl = format!("{ctl:#010x}"),
                    "halted without mailbox"
                );
                return Ok((ctl, 0));
            }
            if start.elapsed() > timeout {
                tracing::error!(name, cpuctl = format!("{ctl:#010x}"), "timeout");
                return Err(SovereignStagesError::KeplerFalconBootTimeout { name, cpuctl: ctl });
            }
        }
    };

    let (gpccs_ctl, gpccs_mb0) =
        boot_falcon("GPCCS", falcon::GPCCS_BASE, &gpccs_inst, &gpccs_data)?;
    let (fecs_ctl, fecs_mb0) = boot_falcon("FECS", falcon::FECS_BASE, &fecs_inst, &fecs_data)?;

    let fecs_running = fecs_ctl & falcon::CPUCTL_HALTED == 0 && fecs_mb0 != 0;

    let detail = format!(
        "Kepler PIO: FECS cpuctl={fecs_ctl:#010x} mb0={fecs_mb0:#010x} | \
         GPCCS cpuctl={gpccs_ctl:#010x} mb0={gpccs_mb0:#010x} | running={fecs_running}"
    );

    if fecs_running {
        Ok(detail)
    } else {
        Err(SovereignStagesError::KeplerFalconNotRunning { detail })
    }
}

pub(crate) fn falcon_boot(
    bar0: &MappedBar,
    sm_version: u32,
    dma: Option<&crate::vfio::device::DmaBackend>,
    warm_state: crate::vfio::sovereign_strategy::FalconWarmState,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
    boot_style: crate::vfio::sovereign_strategy::FalconBootStyle,
) -> Result<String, SovereignStagesError> {
    use crate::vfio::channel::registers::falcon;
    use crate::vfio::sovereign_strategy::{FalconBootStyle, FalconWarmState};

    match boot_style {
        FalconBootStyle::DirectPio => {
            tracing::info!(
                sm = sm_version,
                "DirectPio falcon boot — using PIO firmware upload (no ACR)"
            );
            return kepler_falcon_boot(bar0, sm_version, bridge);
        }
        FalconBootStyle::NoFalcons => {
            tracing::info!("No falcon engines on this hardware — skipping falcon boot");
            return Ok("no-falcons: hardware has no falcon microcontrollers".into());
        }
        FalconBootStyle::AcrDmaHs => {}
    }

    // ── FECS warm-preservation dispatch ──────────────────────────────
    //
    // The strategy has already classified the falcon thermal state via
    // `detect_falcon_warm_state()` — dispatch on the enum rather than
    // reading BAR0 registers inline.
    tracing::info!(warm_state = ?warm_state, "falcon warm-state detection result");

    match warm_state {
        FalconWarmState::WarmPreserved { cpuctl, mailbox0 } => {
            tracing::info!(
                "FECS warm-preserved (HALTED + mb0={mailbox0:#010x}) — skipping ACR/PIO"
            );
            return Ok(format!(
                "warm-preserved: FECS cpuctl={cpuctl:#010x} mb0={mailbox0:#010x}"
            ));
        }
        FalconWarmState::WarmRunning { cpuctl, pc, mailbox0 } => {
            tracing::info!(
                fecs_pc = format!("{pc:#010x}"),
                "FECS warm-running (active firmware, PC advancing) — skipping boot"
            );
            return Ok(format!(
                "warm-running: FECS cpuctl={cpuctl:#010x} pc={pc:#010x} mb0={mailbox0:#010x}"
            ));
        }
        FalconWarmState::Inconsistent { cpuctl } => {
            tracing::warn!(
                cpuctl = format!("{cpuctl:#010x}"),
                "FECS inconsistent teardown state — attempting PIO re-bootstrap"
            );
            let chip = crate::nv::identity::chip_name(sm_version);
            match bridge.boot_gr_falcons(bar0, chip) {
                Ok(result) if result.running => {
                    return Ok(format!(
                        "warm re-bootstrap OK: FECS cpuctl=0x{:08x} mb0=0x{:08x}",
                        result.cpuctl_after, result.mailbox0,
                    ));
                }
                Ok(result) => {
                    tracing::warn!(
                        cpuctl = format!("0x{:08x}", result.cpuctl_after),
                        "PIO re-bootstrap: FECS not running — falling through to cold path"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "PIO re-bootstrap failed — falling through to cold path"
                    );
                }
            }
        }
        FalconWarmState::Cold => {}
    }

    let chip = crate::nv::identity::chip_name(sm_version);

    let _ = bridge.apply_gr_bar0_init(bar0, sm_version);

    // Exp 173 proved nvidia-535 closed does NOT configure WPR on GV100 (pre-GSP).
    // WPR is a Turing+/Ampere+ concept for GSP-RM protection. On Volta, the RM
    // runs on the CPU and doesn't use WPR hardware boundaries. The ACR chain's
    // requirement for WPR cannot be satisfied on GV100 through vendor drivers.
    // The SEC2→HS→FECS bootstrap path requires a different approach for Volta.

    let wpr1_beg = bar0.read_u32(0x100CE4).unwrap_or(0xDEAD);
    let wpr1_end = bar0.read_u32(0x100CE8).unwrap_or(0xDEAD);
    let wpr2_beg = bar0.read_u32(0x100CEC).unwrap_or(0xDEAD);
    let wpr2_end = bar0.read_u32(0x100CF0).unwrap_or(0xDEAD);
    let wpr_configured = wpr2_beg != 0 && wpr2_end != 0 && wpr2_end > wpr2_beg;
    tracing::info!(
        wpr1_beg = format!("{wpr1_beg:#x}"),
        wpr1_end = format!("{wpr1_end:#x}"),
        wpr2_beg = format!("{wpr2_beg:#x}"),
        wpr2_end = format!("{wpr2_end:#x}"),
        wpr_configured,
        "pre-ACR WPR state"
    );

    tracing::info!(
        chip,
        dma_available = dma.is_some(),
        "falcon boot: trying ACR boot solver..."
    );

    let acr_detail = match bridge.acr_boot(bar0, sm_version, chip, dma.cloned()) {
        Ok(results) => {
            if results.iter().any(|r| r.success) {
                let cpuctl = bar0
                    .read_u32(falcon::FECS_BASE + falcon::CPUCTL)
                    .unwrap_or(0xDEAD_DEAD);
                let mb0 = bar0
                    .read_u32(falcon::FECS_BASE + falcon::MAILBOX0)
                    .unwrap_or(0);
                return Ok(format!(
                    "ACR boot OK: FECS cpuctl=0x{cpuctl:08x} mb0=0x{mb0:08x} ({} strategies)",
                    results.len()
                ));
            }
            let summary: Vec<String> = results
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let tail: Vec<&str> =
                        r.notes.iter().rev().take(40).map(|s| s.as_str()).collect();
                    format!("S{i}:{} [{}]", r.strategy, tail.join("; "))
                })
                .collect();
            summary.join(" | ")
        }
        Err(e) => format!("solver_err:{e}"),
    };

    tracing::info!(chip, "ACR failed, trying direct PIO upload...");
    match bridge.boot_gr_falcons(bar0, chip) {
        Ok(result) => {
            let detail = format!(
                "direct boot: FECS cpuctl=0x{:08x} mb0=0x{:08x} running={} | acr:[{}]",
                result.cpuctl_after, result.mailbox0, result.running, acr_detail,
            );
            if result.running {
                Ok(detail)
            } else {
                Err(SovereignStagesError::FalconBootNotRunning { detail })
            }
        }
        Err(e) => Err(SovereignStagesError::FalconBootPathsExhausted {
            detail: format!("{e} | acr:[{acr_detail}]"),
        }),
    }
}

pub(crate) fn gr_init(
    bar0: &MappedBar,
    sm_version: u32,
    bridge: &dyn crate::nv::gsp_bridge::GspBridge,
) -> Result<String, SovereignStagesError> {
    let chip = crate::nv::identity::chip_name(sm_version);

    match bridge.boot_fecs(bar0, chip) {
        Ok(result) if result.running => Ok(format!(
            "GR ready: FECS mb0=0x{:08x} mb1=0x{:08x}",
            result.mailbox0, result.mailbox1,
        )),
        Ok(result) => Err(SovereignStagesError::GrFecsNotRunning {
            cpuctl: result.cpuctl_after,
        }),
        Err(e) => Err(SovereignStagesError::VfioCompute(Box::new(e))),
    }
}

pub(crate) fn verify(bar0: &MappedBar) -> Result<String, SovereignStagesError> {
    // PTIMER liveness: both low and high timer registers should be non-zero
    // on a running GPU.
    let ops = vec![
        (PTIMER_TIME_0 as u32, None),
        (PTIMER_TIME_1 as u32, None),
        (PMC_ENABLE as u32, None),
    ];

    let result = bar0.isolated_batch(&ops, ISOLATE_TIMEOUT);
    match result {
        super::isolation::IsolationResult::Ok(vals) => {
            let timer_lo = vals.first().copied().unwrap_or(0);
            let timer_hi = vals.get(1).copied().unwrap_or(0);
            let pmc = vals.get(2).copied().unwrap_or(0);

            if timer_lo == 0 && timer_hi == 0 {
                return Err(SovereignStagesError::VerifyPtimerDead { pmc });
            }

            // VRAM sentinel via PRAMIN
            let vram_ok = pramin_sentinel_test(bar0);

            let detail = format!(
                "ptimer=0x{timer_hi:08x}_{timer_lo:08x} pmc=0x{pmc:08x} vram={}",
                if vram_ok { "ok" } else { "FAILED" },
            );

            if vram_ok {
                tracing::info!("sovereign verify: {detail}");
                Ok(detail)
            } else {
                tracing::warn!("sovereign verify: VRAM sentinel failed but PTIMER alive");
                Err(SovereignStagesError::VerifyVramSentinelFailed { detail })
            }
        }
        super::isolation::IsolationResult::Timeout => Err(SovereignStagesError::VerifyTimeout),
        super::isolation::IsolationResult::ChildFailed { status } => {
            Err(SovereignStagesError::VerifyChildFailed { status })
        }
        super::isolation::IsolationResult::ForkError(e) => Err(SovereignStagesError::VerifyFork(e)),
    }
}

pub(crate) fn pramin_sentinel_test(bar0: &MappedBar) -> bool {
    use crate::vfio::memory::{MemoryRegion, PraminRegion};

    match PraminRegion::new(bar0, 0x0002_6000, 8) {
        Ok(mut region) => region.probe_sentinel(0, 0xCAFE_BEEF).is_working(),
        Err(_) => false,
    }
}

/// Map chip_id → SM version.
///
/// Delegates to the authoritative [`boot0_to_sm`](crate::nv::identity::boot0_to_sm)
/// by reconstructing a synthetic BOOT0 from the chip_id. Falls back to Volta
/// (SM 70) for unrecognized chipsets with a warning.
pub(crate) fn chip_id_to_sm(chip_id: u32) -> u32 {
    let synthetic_boot0 = chip_id << 20;
    match crate::nv::identity::boot0_to_sm(synthetic_boot0) {
        Some(sm) => sm,
        None => {
            tracing::warn!(
                chip_id = format!("0x{chip_id:03x}"),
                "unknown chip — defaulting to SM 70 (Volta)"
            );
            70
        }
    }
}

/// Heuristic to detect if the GPU has already been trained.
///
/// A "warm" GPU has most engines enabled (high popcount in PMC_ENABLE)
/// and accessible VRAM (PRAMIN sentinel test passes). A cold GPU after
/// PCI reset typically has PMC_ENABLE = 0x0 or very few bits set.
pub(crate) fn is_warm_gpu(pmc_enable: u32, bar0: &MappedBar) -> bool {
    let popcount = pmc_enable.count_ones();
    if popcount < 8 {
        return false;
    }
    pramin_sentinel_test(bar0)
}

// ── Sovereign Experiment Infrastructure ──────────────────────────────

/// A complete register snapshot of all tier-relevant GPU domains.
///
/// Captured in a single pass via BAR0 MMIO reads. Supports diff display
/// to visualize the effect of experiment stages on the GPU state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SovereignSnapshot {
    pub pmc_enable: u32,
    pub pmc_intr_en: u32,
    pub pfifo_enable: u32,
    pub pgraph_status: u32,
    pub gpc_bcast: u32,
    pub fecs_cpuctl: u32,
    pub fecs_cpuctl_alias: u32,
    pub fecs_pc: u32,
    pub fecs_mailbox0: u32,
    pub gpccs_cpuctl: u32,
    pub gpccs_pc: u32,
    pub pmu_cpuctl: u32,
    pub pmu_pc: u32,
    /// Per-GPC unit registers at 0x500000 + gpc*0x8000 (up to 6 GPCs).
    pub gpc_per_unit: Vec<u32>,
    /// Per-GPC TPC0 registers at 0x504000 + gpc*0x8000 (up to 6 GPCs).
    pub gpc_tpc0: Vec<u32>,
    /// CE instance registers: CE0..CE5 at 0x104000 + i*0x1000.
    pub ce: Vec<u32>,
    pub pbdma0_intr: u32,
    pub therm_gate: u32,
    pub pri_ringmaster_intr: u32,
}

impl SovereignSnapshot {
    /// Capture a full register snapshot from BAR0 in one pass.
    pub fn capture(bar0: &MappedBar) -> Self {
        use crate::vfio::channel::registers::falcon;

        let r = |off: usize| bar0.read_u32(off).unwrap_or(0xDEAD_DEAD);

        let mut gpc_per_unit = Vec::with_capacity(6);
        let mut gpc_tpc0 = Vec::with_capacity(6);
        for gpc in 0..6u32 {
            gpc_per_unit.push(r(0x500000 + gpc as usize * 0x8000));
            gpc_tpc0.push(r(0x504000 + gpc as usize * 0x8000));
        }

        let mut ce = Vec::with_capacity(6);
        for i in 0..6u32 {
            ce.push(r(0x104000 + i as usize * 0x1000));
        }

        SovereignSnapshot {
            pmc_enable: r(PMC_ENABLE),
            pmc_intr_en: r(PMC_INTR_EN_0),
            pfifo_enable: r(0x2200),
            pgraph_status: r(0x400700),
            gpc_bcast: r(0x41A004),
            fecs_cpuctl: r(falcon::FECS_BASE + falcon::CPUCTL),
            fecs_cpuctl_alias: r(falcon::FECS_BASE + falcon::CPUCTL_ALIAS),
            fecs_pc: r(falcon::FECS_BASE + falcon::PC),
            fecs_mailbox0: r(falcon::FECS_BASE + falcon::MAILBOX0),
            gpccs_cpuctl: r(falcon::GPCCS_BASE + falcon::CPUCTL),
            gpccs_pc: r(falcon::GPCCS_BASE + falcon::PC),
            pmu_cpuctl: r(falcon::PMU_BASE + falcon::CPUCTL),
            pmu_pc: r(falcon::PMU_BASE + falcon::PC),
            gpc_per_unit,
            gpc_tpc0,
            ce,
            pbdma0_intr: r(0x040100),
            therm_gate: r(0x020200),
            pri_ringmaster_intr: r(0x12200C),
        }
    }

    /// Produce a human-readable diff between `before` and `after` snapshots.
    pub fn diff(before: &Self, after: &Self) -> Vec<String> {
        let mut lines = Vec::new();
        macro_rules! cmp {
            ($field:ident, $name:expr) => {
                if before.$field != after.$field {
                    lines.push(format!(
                        "{}: {:#010x} -> {:#010x}",
                        $name, before.$field, after.$field
                    ));
                }
            };
        }
        cmp!(pmc_enable, "PMC_ENABLE");
        cmp!(pmc_intr_en, "PMC_INTR_EN");
        cmp!(pfifo_enable, "PFIFO_ENABLE");
        cmp!(pgraph_status, "PGRAPH_STATUS");
        cmp!(gpc_bcast, "GPC_BCAST");
        cmp!(fecs_cpuctl, "FECS_CPUCTL");
        cmp!(fecs_cpuctl_alias, "FECS_CPUCTL_ALIAS");
        cmp!(fecs_pc, "FECS_PC");
        cmp!(fecs_mailbox0, "FECS_MAILBOX0");
        cmp!(gpccs_cpuctl, "GPCCS_CPUCTL");
        cmp!(gpccs_pc, "GPCCS_PC");
        cmp!(pmu_cpuctl, "PMU_CPUCTL");
        cmp!(pmu_pc, "PMU_PC");
        cmp!(pbdma0_intr, "PBDMA0_INTR");
        cmp!(therm_gate, "THERM_GATE");
        cmp!(pri_ringmaster_intr, "PRI_RM_INTR");

        for (i, (b, a)) in before.gpc_per_unit.iter().zip(&after.gpc_per_unit).enumerate() {
            if b != a {
                lines.push(format!("GPC{i}_UNIT: {b:#010x} -> {a:#010x}"));
            }
        }
        for (i, (b, a)) in before.gpc_tpc0.iter().zip(&after.gpc_tpc0).enumerate() {
            if b != a {
                lines.push(format!("GPC{i}_TPC0: {b:#010x} -> {a:#010x}"));
            }
        }
        for (i, (b, a)) in before.ce.iter().zip(&after.ce).enumerate() {
            if b != a {
                lines.push(format!("CE{i}: {b:#010x} -> {a:#010x}"));
            }
        }

        if lines.is_empty() {
            lines.push("(no changes)".into());
        }
        lines
    }

    /// Produce a structured diff between two snapshots.
    ///
    /// Returns one [`SnapshotDelta`] per field that differs, suitable for
    /// JSON serialization and programmatic comparison.
    pub fn diff_structured(a: &Self, b: &Self) -> Vec<SnapshotDelta> {
        let mut deltas = Vec::new();
        macro_rules! cmp_field {
            ($field:ident, $name:expr) => {
                if a.$field != b.$field {
                    deltas.push(SnapshotDelta {
                        field: $name.into(),
                        before: a.$field,
                        after: b.$field,
                    });
                }
            };
        }
        cmp_field!(pmc_enable, "PMC_ENABLE");
        cmp_field!(pmc_intr_en, "PMC_INTR_EN");
        cmp_field!(pfifo_enable, "PFIFO_ENABLE");
        cmp_field!(pgraph_status, "PGRAPH_STATUS");
        cmp_field!(gpc_bcast, "GPC_BCAST");
        cmp_field!(fecs_cpuctl, "FECS_CPUCTL");
        cmp_field!(fecs_cpuctl_alias, "FECS_CPUCTL_ALIAS");
        cmp_field!(fecs_pc, "FECS_PC");
        cmp_field!(fecs_mailbox0, "FECS_MAILBOX0");
        cmp_field!(gpccs_cpuctl, "GPCCS_CPUCTL");
        cmp_field!(gpccs_pc, "GPCCS_PC");
        cmp_field!(pmu_cpuctl, "PMU_CPUCTL");
        cmp_field!(pmu_pc, "PMU_PC");
        cmp_field!(pbdma0_intr, "PBDMA0_INTR");
        cmp_field!(therm_gate, "THERM_GATE");
        cmp_field!(pri_ringmaster_intr, "PRI_RM_INTR");

        for (i, (va, vb)) in a.gpc_per_unit.iter().zip(&b.gpc_per_unit).enumerate() {
            if va != vb {
                deltas.push(SnapshotDelta {
                    field: format!("GPC{i}_UNIT"),
                    before: *va,
                    after: *vb,
                });
            }
        }
        for (i, (va, vb)) in a.gpc_tpc0.iter().zip(&b.gpc_tpc0).enumerate() {
            if va != vb {
                deltas.push(SnapshotDelta {
                    field: format!("GPC{i}_TPC0"),
                    before: *va,
                    after: *vb,
                });
            }
        }
        for (i, (va, vb)) in a.ce.iter().zip(&b.ce).enumerate() {
            if va != vb {
                deltas.push(SnapshotDelta {
                    field: format!("CE{i}"),
                    before: *va,
                    after: *vb,
                });
            }
        }

        deltas
    }
}

/// A single field difference between two [`SovereignSnapshot`]s.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotDelta {
    /// Register/field name (e.g. "PMC_ENABLE", "GPC0_TPC0").
    pub field: String,
    /// Value in the first snapshot.
    pub before: u32,
    /// Value in the second snapshot.
    pub after: u32,
}

/// Result of a single experiment stage execution.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperimentResult {
    pub stage: u32,
    pub stage_name: String,
    pub before: SovereignSnapshot,
    pub after: SovereignSnapshot,
    pub diff: Vec<String>,
    pub writes: Vec<ExperimentWrite>,
    pub notes: Vec<String>,
}

/// A single BAR0 write performed during an experiment stage.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExperimentWrite {
    pub offset: String,
    pub value: String,
    pub readback: String,
}

impl ExperimentWrite {
    fn new(bar0: &MappedBar, offset: usize, value: u32) -> Self {
        let _ = bar0.write_u32(offset, value);
        let readback = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
        ExperimentWrite {
            offset: format!("{offset:#08x}"),
            value: format!("{value:#010x}"),
            readback: format!("{readback:#010x}"),
        }
    }
}

/// Read-only snapshot capture — no mutating writes.
///
/// Returns the current [`SovereignSnapshot`] and [`TierEvidence`] for the
/// given BAR0 mapping. Used by the `sovereign.snapshot` RPC.
pub fn sovereign_snapshot_only(
    bar0: &MappedBar,
) -> (SovereignSnapshot, crate::vfio::sovereign_tiers::TierEvidence) {
    let snapshot = SovereignSnapshot::capture(bar0);
    let tier = crate::vfio::sovereign_tiers::classify_tier(bar0);
    (snapshot, tier)
}

/// Stage 1: PFIFO enable + CG sweep.
///
/// Enables the PFIFO engine and disables clock gating across all accessible
/// domains. This is the safest stage — standard init operations, fully reversible.
pub fn experiment_stage_1(bar0: &MappedBar) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    // Write PFIFO_ENABLE = 1
    writes.push(ExperimentWrite::new(bar0, 0x2200, 0x1));
    notes.push(format!("PFIFO_ENABLE: was {:#010x}", before.pfifo_enable));

    // CG sweep to disable clock gating
    let cg = cg_sweep(bar0);
    notes.push(format!("CG sweep: {}", cg.detail));

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 1,
        stage_name: "PFIFO enable + CG sweep".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 2: PGOB ungate — GPC power domain ungating.
///
/// Disables PMC clock gating, ensures PGRAPH is enabled in PMC, writes GPC
/// broadcast PGOB control registers, and polls PGRAPH_STATUS.
/// Medium risk — writes to power gating control.
pub fn experiment_stage_2(bar0: &MappedBar) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    // Step 1: PMC clock gate disable
    writes.push(ExperimentWrite::new(bar0, 0x260, 0x1));
    notes.push("PMC_CLKGATE_DISABLE = 1".into());

    // Step 2: Ensure GR engine enabled in PMC_ENABLE (bit 12)
    let pmc = bar0.read_u32(PMC_ENABLE).unwrap_or(0);
    if pmc & (1 << 12) == 0 {
        writes.push(ExperimentWrite::new(bar0, PMC_ENABLE, pmc | (1 << 12)));
        notes.push(format!("PMC_ENABLE: set GR bit (was {pmc:#010x})"));
    } else {
        notes.push(format!("PMC_ENABLE: GR bit already set ({pmc:#010x})"));
    }

    // Step 3: GPC broadcast PGOB control = 0x110
    writes.push(ExperimentWrite::new(bar0, 0x419000, 0x0000_0110));
    notes.push("GPC_BCAST_PGOB_CONTROL = 0x110".into());

    // Step 4: Per-GPC PGOB disable (broadcast offset + 0x1028)
    writes.push(ExperimentWrite::new(bar0, 0x41A028, 0x0));
    notes.push("GPC_PGOB_PER_GPC = 0x0 (disable power gating)".into());

    // Step 5: Poll PGRAPH_STATUS for up to 100ms
    let deadline = Instant::now() + Duration::from_millis(100);
    let mut last_status = 0xDEAD_DEAD_u32;
    while Instant::now() < deadline {
        last_status = bar0.read_u32(0x400700).unwrap_or(0xDEAD_DEAD);
        if last_status >> 16 != 0xBADF {
            break;
        }
        std::thread::sleep(Duration::from_micros(100));
    }
    notes.push(format!("PGRAPH_STATUS poll result: {last_status:#010x}"));

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 2,
        stage_name: "PGOB ungate".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 3: PRI ring recovery + enumerate.
///
/// Clears PRI faults, re-enumerates ring stations, and probes GPC per-unit
/// and TPC registers. Low risk — standard PRI recovery.
pub fn experiment_stage_3(bar0: &MappedBar) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    // Clear PRI ringmaster interrupt status
    let rm_intr = bar0.read_u32(0x12200C).unwrap_or(0);
    if rm_intr != 0 {
        writes.push(ExperimentWrite::new(bar0, 0x12200C, rm_intr));
        notes.push(format!("PRI_RM_INTR: cleared {rm_intr:#010x}"));
    } else {
        notes.push("PRI_RM_INTR: already clear".into());
    }

    // Re-enumerate ring stations
    writes.push(ExperimentWrite::new(bar0, 0x122000, 0x4));
    notes.push("PRI_RINGMASTER_CMD: ENUMERATE".into());
    std::thread::sleep(Duration::from_millis(20));

    // Run full PRI bus recovery
    let pri = pri_bus_recover(bar0);
    notes.push(format!(
        "PRI recovery: alive={}, faulted={}, recovered={}",
        pri.alive, pri.faulted, pri.recovered
    ));

    // Wait for ring to settle
    std::thread::sleep(Duration::from_millis(50));

    // Probe individual GPC registers for liveness
    for gpc in 0..6u32 {
        let unit = bar0.read_u32(0x500000 + gpc as usize * 0x8000).unwrap_or(0xDEAD_DEAD);
        let tpc0 = bar0.read_u32(0x504000 + gpc as usize * 0x8000).unwrap_or(0xDEAD_DEAD);
        let is_fault = crate::nv::pri::is_pri_fault(unit);
        if !is_fault {
            notes.push(format!(
                "GPC{gpc}: unit={unit:#010x} tpc0={tpc0:#010x} (alive)"
            ));
        }
    }

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 3,
        stage_name: "PRI ring recovery + enumerate".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 4: GPC MMU init + sw_nonctx.bin replay.
///
/// Only proceeds if GPCs showed life in stage 2-3. Writes GPC MMU init
/// registers and replays sw_nonctx.bin firmware blob. Higher risk — large
/// write sequence.
pub fn experiment_stage_4(bar0: &MappedBar) -> ExperimentResult {
    experiment_stage_4_with_chip(bar0, "gv100", 70)
}

/// Stage 4 with explicit chip/SM version parameters.
pub fn experiment_stage_4_with_chip(bar0: &MappedBar, chip: &str, sm: u32) -> ExperimentResult {
    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    notes.push(format!("chip={chip}, sm={sm}"));

    // Pre-check: is GPC domain alive?
    let gpc0 = bar0.read_u32(0x500000).unwrap_or(0xDEAD_DEAD);
    let gpc_bcast = bar0.read_u32(0x41A004).unwrap_or(0);
    let gpc_alive = !crate::nv::pri::is_pri_fault(gpc0)
        || (!crate::nv::pri::is_pri_fault(gpc_bcast) && gpc_bcast != 0);

    if !gpc_alive {
        notes.push(format!(
            "ABORT: GPC domain not alive (gpc0={gpc0:#010x}, bcast={gpc_bcast:#010x}). \
             Run stages 2-3 first."
        ));
        let after = SovereignSnapshot::capture(bar0);
        let diff = SovereignSnapshot::diff(&before, &after);
        return ExperimentResult {
            stage: 4,
            stage_name: "GPC MMU init (aborted — GPCs not alive)".into(),
            before,
            after,
            diff,
            writes,
            notes,
        };
    }

    // GPC MMU init sequence (from nouveau gf100_grctx)
    let mmu_writes: &[(usize, u32)] = &[
        (0x418880, 0x0000_0001), // GPC_BCAST MMU_CTRL
        (0x418890, 0x0000_0000), // GPC_BCAST MMU_PM_UNIT_MASK
        (0x418894, 0x0000_0000), // GPC_BCAST MMU_PM_REQ_MASK
        (0x4188B0, 0x0000_0000), // GPC_BCAST MMU_DEBUG_CTRL
        (0x4188B4, 0xFFFF_FFFF), // GPC_BCAST MMU_DEBUG_WR
        (0x4188B8, 0x0000_0007), // GPC_BCAST MMU_DEBUG_RD
    ];

    for &(offset, value) in mmu_writes {
        writes.push(ExperimentWrite::new(bar0, offset, value));
    }
    notes.push(format!("GPC MMU init: {} writes applied", mmu_writes.len()));

    let bridge = crate::nv::nv_gsp_bridge::NvGspBridge::new(chip);
    use crate::nv::gsp_bridge::GspBridge;
    let has_fw = bridge.has_gr_firmware();
    notes.push(format!("NvGspBridge({chip}): firmware present = {has_fw}"));
    match bridge.apply_gr_bar0_init(bar0, sm) {
        Ok(()) => notes.push("sw_nonctx replay: completed with REAL firmware data".into()),
        Err(e) => notes.push(format!("sw_nonctx replay: {e}")),
    }

    // Probe TPC registers after sw_nonctx broadcast writes
    let tpc0_post = bar0.read_u32(0x504000).unwrap_or(0xDEAD_DEAD);
    let tpc0_sm_post = bar0.read_u32(0x504200).unwrap_or(0xDEAD_DEAD);
    let bcast_tpc_post = bar0.read_u32(0x419C04).unwrap_or(0xDEAD_DEAD);
    notes.push(format!(
        "Post-sw_nonctx TPC probe: tpc0_ctrl={tpc0_post:#010x}, \
         tpc0_sm={tpc0_sm_post:#010x}, bcast_tpc={bcast_tpc_post:#010x}"
    ));
    let tpc_alive = !crate::nv::pri::is_pri_fault(tpc0_post);
    notes.push(format!("TPC PRI station alive = {tpc_alive}"));

    // Post-init PRI recovery
    let pri = pri_bus_recover(bar0);
    notes.push(format!(
        "Post-init PRI: alive={}, faulted={}, recovered={}",
        pri.alive, pri.faulted, pri.recovered
    ));

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 4,
        stage_name: "GPC MMU init + sw_nonctx replay".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 5: FECS resume via CPUCTL_ALIAS.
///
/// Attempts to resume the halted FECS falcon by writing STARTCPU to
/// CPUCTL_ALIAS (0x409130). Only proceeds if GPCs are alive. Polls PC
/// for advancement, then tries INIT_CTXSW. Medium risk — FECS may
/// trigger falcon exception if TPCs are still gated.
pub fn experiment_stage_5(bar0: &MappedBar) -> ExperimentResult {
    use crate::vfio::channel::registers::falcon;

    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    // Pre-check: GPCs alive?
    let gpc0 = bar0.read_u32(0x500000).unwrap_or(0xDEAD_DEAD);
    let gpc_bcast = bar0.read_u32(0x41A004).unwrap_or(0);
    let gpc_alive = !crate::nv::pri::is_pri_fault(gpc0)
        || (!crate::nv::pri::is_pri_fault(gpc_bcast) && gpc_bcast != 0);

    if !gpc_alive {
        notes.push(format!(
            "ABORT: GPCs not alive (gpc0={gpc0:#010x}, bcast={gpc_bcast:#010x}). \
             Run stages 2-4 first."
        ));
        let after = SovereignSnapshot::capture(bar0);
        let diff = SovereignSnapshot::diff(&before, &after);
        return ExperimentResult {
            stage: 5,
            stage_name: "FECS resume (aborted — GPCs not alive)".into(),
            before,
            after,
            diff,
            writes,
            notes,
        };
    }

    // Capture pre-resume FECS state
    let pc_before = bar0
        .read_u32(falcon::FECS_BASE + falcon::PC)
        .unwrap_or(0xDEAD);
    let cpuctl_before = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    notes.push(format!(
        "FECS pre-resume: cpuctl_alias={cpuctl_before:#010x}, pc={pc_before:#010x}"
    ));

    // Write STARTCPU to CPUCTL_ALIAS to resume the halted falcon
    writes.push(ExperimentWrite::new(
        bar0,
        falcon::FECS_BASE + falcon::CPUCTL_ALIAS,
        falcon::CPUCTL_STARTCPU,
    ));
    notes.push("FECS CPUCTL_ALIAS <- STARTCPU (0x02)".into());

    // Poll FECS PC for advancement (up to 50ms)
    std::thread::sleep(Duration::from_millis(5));
    let mut pc_advanced = false;
    let deadline = Instant::now() + Duration::from_millis(50);
    let mut last_pc = pc_before;
    while Instant::now() < deadline {
        last_pc = bar0
            .read_u32(falcon::FECS_BASE + falcon::PC)
            .unwrap_or(0xDEAD);
        if last_pc != pc_before && !crate::nv::pri::is_pri_fault(last_pc) {
            pc_advanced = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }

    let cpuctl_after = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    let halted = cpuctl_after & falcon::CPUCTL_HALTED != 0;
    let in_hreset = cpuctl_after & falcon::CPUCTL_HRESET != 0;
    notes.push(format!(
        "FECS post-resume: cpuctl_alias={cpuctl_after:#010x} (halted={halted}, hreset={in_hreset}), \
         pc={last_pc:#010x} (advanced={pc_advanced})"
    ));

    // If PC advanced, try INIT_CTXSW via FECS method mailbox
    if pc_advanced && !halted {
        notes.push("FECS running — attempting INIT_CTXSW".into());
        let mb0_before = bar0
            .read_u32(falcon::FECS_BASE + falcon::MAILBOX0)
            .unwrap_or(0);

        // Write method: INIT_CTXSW = 0x10
        let _ = bar0.write_u32(falcon::FECS_BASE + falcon::MTHD_DATA, 0);
        writes.push(ExperimentWrite::new(
            bar0,
            falcon::FECS_BASE + falcon::MTHD_CMD,
            0x8000_0010, // INIT_CTXSW with trigger bit
        ));

        // Poll for completion (bit 0 of MTHD_CMD clears)
        let mthd_deadline = Instant::now() + Duration::from_millis(100);
        let mut mthd_done = false;
        while Instant::now() < mthd_deadline {
            let cmd = bar0
                .read_u32(falcon::FECS_BASE + falcon::MTHD_CMD)
                .unwrap_or(0xDEAD);
            if cmd & 0x8000_0000 == 0 {
                mthd_done = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }

        let mb0_after = bar0
            .read_u32(falcon::FECS_BASE + falcon::MAILBOX0)
            .unwrap_or(0);
        notes.push(format!(
            "INIT_CTXSW: done={mthd_done}, mb0: {mb0_before:#010x} -> {mb0_after:#010x}"
        ));
    } else if !pc_advanced {
        notes.push("FECS PC did not advance — falcon may need full re-bootstrap".into());
    }

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 5,
        stage_name: "FECS resume via CPUCTL_ALIAS".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Stage 6: Full 5-phase ungating sequence (Exp 217).
///
/// Combines CG sweep + PRI recovery + PGOB + real `sw_nonctx.bin` +
/// destructive PGRAPH reset + PRI re-enumerate + second `sw_nonctx.bin`
/// replay. This is the "throw everything at it" sequence from
/// `compute_device.rs`, extracted here for controlled experiment use.
///
/// Higher risk than stages 1-5 — includes PGRAPH engine reset which
/// may change FECS state. Use after stages 1-3 confirm GPC fabric is
/// alive but TPCs remain PRI-faulted.
pub fn experiment_stage_6(bar0: &MappedBar) -> ExperimentResult {
    experiment_stage_6_with_chip(bar0, "gv100", 70)
}

/// Stage 6 with explicit chip/SM version parameters.
pub fn experiment_stage_6_with_chip(bar0: &MappedBar, chip: &str, sm: u32) -> ExperimentResult {
    use std::time::Duration;

    let before = SovereignSnapshot::capture(bar0);
    let mut writes = Vec::new();
    let mut notes = Vec::new();

    notes.push(format!("chip={chip}, sm={sm}"));

    let bridge = crate::nv::nv_gsp_bridge::NvGspBridge::new(chip);
    use crate::nv::gsp_bridge::GspBridge;

    // Phase 1: CG sweep + PRI recovery + PGOB
    let cg = cg_sweep(bar0);
    notes.push(format!(
        "Phase 1a: CG sweep — {} changes, {} faulted",
        cg.changes, cg.faulted
    ));

    let pri1 = pri_bus_recover(bar0);
    notes.push(format!(
        "Phase 1b: PRI recovery — alive={}, faulted={}",
        pri1.alive, pri1.faulted
    ));

    // PGOB ungate via GPC broadcast
    let pgob_regs: &[(usize, u32)] = &[
        (0x000260, 0x0000_0000), // PMC_CLKGATE_DISABLE
        (0x41A028, 0x0000_0000), // GPC_BCAST_PGOB
        (0x419000, 0x0000_0110), // GPC_BCAST_ENGCTL
    ];
    for &(off, val) in pgob_regs {
        writes.push(ExperimentWrite::new(bar0, off, val));
    }
    notes.push("Phase 1c: PGOB ungate broadcast writes applied".into());

    // Phase 2: Force PRI enumerate
    writes.push(ExperimentWrite::new(bar0, 0x12004c, 2));
    std::thread::sleep(Duration::from_millis(10));
    notes.push("Phase 2: Forced PRI ringmaster enumerate".into());

    // Phase 3: GPC MMU init
    let mmu_writes: &[(usize, u32)] = &[
        (0x418880, 0x0000_0001),
        (0x418890, 0x0000_0000),
        (0x418894, 0x0000_0000),
        (0x4188B0, 0x0000_0000),
        (0x4188B4, 0xFFFF_FFFF),
        (0x4188B8, 0x0000_0007),
    ];
    for &(off, val) in mmu_writes {
        writes.push(ExperimentWrite::new(bar0, off, val));
    }

    // Extra GPC MMU writes from nouveau gm200_gr_init_gpc_mmu
    let a4 = bar0.read_u32(0x4188a4).unwrap_or(0);
    writes.push(ExperimentWrite::new(bar0, 0x4188a4, a4 | 0x0300_0000));
    notes.push("Phase 3: GPC MMU init + extended MMU writes".into());

    // Phase 4: sw_nonctx.bin replay
    match bridge.apply_gr_bar0_init(bar0, sm) {
        Ok(()) => notes.push("Phase 4: sw_nonctx.bin replay completed".into()),
        Err(e) => notes.push(format!("Phase 4: sw_nonctx.bin replay failed: {e}")),
    }

    // Phase 5: Second PRI recovery after sw_nonctx writes
    let pri2 = pri_bus_recover(bar0);
    notes.push(format!(
        "Phase 5: Post-init PRI recovery — alive={}, faulted={}",
        pri2.alive, pri2.faulted
    ));

    // Probe TPC + CE state post-ungating
    let tpc0 = bar0.read_u32(0x504000).unwrap_or(0xDEAD_DEAD);
    let tpc0_sm = bar0.read_u32(0x504200).unwrap_or(0xDEAD_DEAD);
    let ce0 = bar0.read_u32(0x104000).unwrap_or(0xDEAD_DEAD);
    let ce4 = bar0.read_u32(0x108000).unwrap_or(0xDEAD_DEAD);
    let fecs_pc = bar0.read_u32(0x409400).unwrap_or(0);
    let gpc0 = bar0.read_u32(0x500000).unwrap_or(0xDEAD_DEAD);

    notes.push(format!("Final TPC probe: tpc0_ctrl={tpc0:#010x}, tpc0_sm={tpc0_sm:#010x}"));
    notes.push(format!("Final CE probe: ce0={ce0:#010x}, ce4={ce4:#010x}"));
    notes.push(format!("Final state: gpc0={gpc0:#010x}, fecs_pc={fecs_pc:#010x}"));

    let tpc_alive = !crate::nv::pri::is_pri_fault(tpc0);
    notes.push(format!("TPC PRI station alive = {tpc_alive}"));

    // If TPC still faulted, try destructive PGRAPH reset as last resort
    if !tpc_alive {
        notes.push("TPC still faulted — attempting destructive PGRAPH reset".into());

        // PMC GR engine reset: clear bit 12, wait, set bit 12
        let pmc = bar0.read_u32(0x200).unwrap_or(0);
        let _ = bar0.write_u32(0x200_usize, pmc & !(1 << 12));
        std::thread::sleep(Duration::from_millis(10));
        let _ = bar0.write_u32(0x200_usize, pmc | (1 << 12));
        std::thread::sleep(Duration::from_millis(50));
        notes.push("PGRAPH reset: PMC bit 12 toggled".into());

        // PRI re-enumerate after reset
        let _ = bar0.write_u32(0x12004c_usize, 2);
        std::thread::sleep(Duration::from_millis(10));

        // Re-apply sw_nonctx.bin after reset
        match bridge.apply_gr_bar0_init(bar0, 70) {
            Ok(()) => notes.push("Post-reset sw_nonctx.bin replay completed".into()),
            Err(e) => notes.push(format!("Post-reset sw_nonctx.bin replay failed: {e}")),
        }

        // Final PRI recovery
        let pri3 = pri_bus_recover(bar0);
        notes.push(format!(
            "Post-reset PRI recovery — alive={}, faulted={}",
            pri3.alive, pri3.faulted
        ));

        let tpc0_final = bar0.read_u32(0x504000).unwrap_or(0xDEAD_DEAD);
        let tpc_alive_final = !crate::nv::pri::is_pri_fault(tpc0_final);
        notes.push(format!(
            "Post-reset TPC: tpc0={tpc0_final:#010x}, alive={tpc_alive_final}"
        ));
    }

    let after = SovereignSnapshot::capture(bar0);
    let diff = SovereignSnapshot::diff(&before, &after);

    ExperimentResult {
        stage: 6,
        stage_name: "Full 5-phase ungating + PGRAPH reset (Exp 217)".into(),
        before,
        after,
        diff,
        writes,
        notes,
    }
}

/// Auto-detect chip name and SM version from BOOT0 register.
///
/// Returns `(chip_name, sm_version)`. Falls back to `("gv100", 70)` if
/// BOOT0 is unreadable or unrecognized.
pub fn detect_chip(bar0: &MappedBar) -> (&'static str, u32) {
    let boot0 = bar0.read_u32(0x0000_0000).unwrap_or(0);
    let chip_id = (boot0 >> 20) & 0x1FF;
    let sm = chip_id_to_sm(chip_id);
    let chip = crate::nv::identity::chip_name(sm);
    (chip, sm)
}

/// Execute an experiment stage by number (1-6).
///
/// Accepts an optional `chip` override (e.g. `"gv100"`, `"gk210"`).
/// When `None`, auto-detects from BOOT0.
pub fn run_experiment_stage(
    bar0: &MappedBar,
    stage: u32,
    chip_override: Option<&str>,
) -> Result<ExperimentResult, String> {
    let (auto_chip, auto_sm) = detect_chip(bar0);
    let chip = chip_override.unwrap_or(auto_chip);
    let sm = auto_sm;

    match stage {
        1 => Ok(experiment_stage_1(bar0)),
        2 => Ok(experiment_stage_2(bar0)),
        3 => Ok(experiment_stage_3(bar0)),
        4 => Ok(experiment_stage_4_with_chip(bar0, chip, sm)),
        5 => Ok(experiment_stage_5(bar0)),
        6 => Ok(experiment_stage_6_with_chip(bar0, chip, sm)),
        _ => Err(format!("invalid stage {stage}: must be 1-6")),
    }
}
