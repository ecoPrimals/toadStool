// SPDX-License-Identifier: AGPL-3.0-or-later
//! PMC probe and enable stages for sovereign init.

use std::time::Duration;

use crate::error::SovereignStagesError;
use crate::vfio::device::MappedBar;

pub(crate) const PMC_BOOT_0: usize = 0x0000_0000;
pub(crate) const PMC_ENABLE: usize = 0x0000_0200;
pub(crate) const PMC_INTR_EN_0: usize = 0x0000_0140;
pub(crate) const PTIMER_TIME_0: usize = 0x0000_9400;
pub(crate) const PTIMER_TIME_1: usize = 0x0000_9410;

pub(crate) const ISOLATE_TIMEOUT: Duration = Duration::from_secs(3);

pub(crate) fn bar0_probe(bar0: &MappedBar) -> Result<(u32, u32), SovereignStagesError> {
    let result = bar0.isolated_read_u32(PMC_BOOT_0 as u32, ISOLATE_TIMEOUT);
    let boot0 = match result {
        crate::vfio::isolation::IsolationResult::Ok(v) => v,
        crate::vfio::isolation::IsolationResult::Timeout => {
            return Err(SovereignStagesError::Bar0ProbeTimeout);
        }
        crate::vfio::isolation::IsolationResult::ChildFailed { status } => {
            return Err(SovereignStagesError::Bar0ProbeChildFailed { status });
        }
        crate::vfio::isolation::IsolationResult::ForkError(e) => {
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
        crate::vfio::isolation::IsolationResult::Ok(()) => {}
        crate::vfio::isolation::IsolationResult::Timeout => {
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
        crate::vfio::isolation::IsolationResult::Ok(()) => {}
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
        crate::vfio::isolation::IsolationResult::Ok(()) => {
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
        crate::vfio::isolation::IsolationResult::Ok(()) => {}
        crate::vfio::isolation::IsolationResult::Timeout => {
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
