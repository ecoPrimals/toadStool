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
