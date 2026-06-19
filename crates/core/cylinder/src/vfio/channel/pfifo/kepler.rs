// SPDX-License-Identifier: AGPL-3.0-or-later
//! Kepler (GK104/GK110) PFIFO engine initialization.

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::nv::hardware_guard::GuardedBar;

use super::super::registers::{pbdma, pfifo, pmc, pri};

/// Kepler (GK104/GK110) PFIFO engine initialization.
///
/// GK104+ PFIFO init following nouveau's `gk104_fifo_init()`.
///
/// On GK104+, PBDMA count comes from `PMC_SUBDEV_ENABLE` (0x204), not
/// the `PFIFO_PBDMA_MAP` register (which is unreliable on warm handoff).
/// Uses GK104 global runlist base/submit. Returns `(runq, runlist_id)`.
pub fn init_pfifo_engine_kepler(guard: &GuardedBar<'_>) -> DriverResult<(u32, u32)> {
    let gw = |reg: u32, val: u32| {
        guard.write_u32(reg, val).map_err(|refusal| {
            DriverError::SubmitFailed(Cow::Owned(format!("PFIFO init {reg:#x}: {refusal}")))
        })
    };

    let boot0 = guard.read_u32(0).unwrap_or(0);
    if boot0 == 0xFFFF_FFFF {
        return Err(DriverError::SubmitFailed(Cow::Borrowed(
            "BAR0 returns 0xFFFFFFFF — GPU in D3hot",
        )));
    }
    tracing::info!(
        boot0 = format_args!("{boot0:#010x}"),
        "Kepler PFIFO init start"
    );

    // Clear PRIV_RING faults
    let priv_intr = guard
        .read_u32(pri::PRIV_RING_INTR_STATUS as u32)
        .unwrap_or(0);
    if priv_intr != 0 {
        for _ in 0..5 {
            gw(pri::PRIV_RING_COMMAND as u32, pri::PRIV_RING_CMD_ACK)?;
            std::thread::sleep(std::time::Duration::from_millis(20));
            if guard
                .read_u32(pri::PRIV_RING_INTR_STATUS as u32)
                .unwrap_or(0)
                == 0
            {
                break;
            }
        }
    }

    // Check PFIFO domain using INTR register (0x2100) — a register that
    // exists on all Kepler+ GPUs. Do NOT use PBDMA_MAP (0x2004) which is
    // GV100+ only and always PRI-faults on Kepler.
    {
        let pfifo_intr_pre = guard.read_u32(pfifo::INTR as u32).unwrap_or(0xDEAD);
        let pbdma0_intr = guard.read_u32(pbdma::intr(0) as u32).unwrap_or(0xDEAD);
        let pfifo_faulted = pri::is_pri_error(pfifo_intr_pre) || pfifo_intr_pre == 0xDEAD_DEAD;
        tracing::info!(
            pfifo_intr = format_args!("{pfifo_intr_pre:#010x}"),
            pbdma0_intr = format_args!("{pbdma0_intr:#010x}"),
            faulted = pfifo_faulted,
            "Kepler PFIFO domain check (via 0x2100, NOT 0x2004)"
        );
    }

    // On GK210B, the PFIFO scheduler sub-block registers (0x2004, 0x2204-0x2253,
    // 0x22C0, 0x2300, 0x2504, 0x2600) are permanently PRI-faulted after VFIO
    // legacy bind. No combination of PMC resets, PRI ring re-init, or PBUS
    // resets brings them online.
    //
    // However, key registers ARE accessible:
    //   - 0x2270/0x2274: GK104 runlist base/submit (WORKS!)
    //   - 0x2390+seq*4:  PBDMA→runlist assignment table (read-only, WORKS!)
    //   - 0x252C/0x254C: BIND_ERROR/SCHED_ERROR (WORKS!)
    //   - 0x040000+:     PBDMA registers (WORKS!)
    //
    // Strategy: read the hardware's existing PBDMA→runlist assignment from
    // 0x2390 (left by Nouveau) and use that runlist ID for our submission.
    // Skip writing to the PRI-faulted registers entirely.

    // Discover PBDMA count from PMC subdevice enable (0x204).
    let pbdma_en = guard.read_u32(pmc::PBDMA_ENABLE as u32).unwrap_or(0);
    let pbdma_nr = pbdma_en.count_ones();
    if pbdma_nr == 0 {
        return Err(DriverError::SubmitFailed(Cow::Borrowed(
            "no PBDMAs enabled in PMC_PBDMA_ENABLE (0x204)",
        )));
    }

    // Re-enable PBDMAs (idempotent if already set by nouveau).
    gw(pmc::PBDMA_ENABLE as u32, (1u32 << pbdma_nr) - 1)?;

    // Read the hardware's PBDMA→runlist assignment table at 0x2390+seq*4.
    // This table IS accessible on GK210B (unlike 0x2600 which PRI-faults).
    // Nouveau populated it during init; we reuse whatever mapping exists.
    let mut gr_runlist_id: Option<u32> = None;
    for seq in 0..pbdma_nr {
        let rl = guard.read_u32(0x2390 + seq * 4).unwrap_or(0xFFFF);
        tracing::info!(seq, runlist = rl, "PBDMA→runlist assignment (0x2390)");
        // Runlist IDs > 31 are garbage from stale/uninitialized state.
        // Take the first valid runlist ID as our GR runlist.
        if gr_runlist_id.is_none() && rl < 32 {
            gr_runlist_id = Some(rl);
        }
    }

    let target_runlist = gr_runlist_id.unwrap_or(0);
    tracing::info!(
        pbdma_en = format_args!("{pbdma_en:#010x}"),
        pbdma_nr,
        target_runlist,
        "Kepler PBDMA discovery: using runlist from hw assignment table"
    );

    // Configure PBDMAs (nouveau gk104_fifo_init pattern).
    // Clear stale channel context from Nouveau — scheduler fails (code 32)
    // if PBDMAs still have an old channel loaded.
    for id in 0..pbdma_nr as usize {
        let b = (0x040000 + id * 0x2000) as u32;
        let pbdma_ctrl = b + 0x13C;
        let ctrl_val = guard.read_u32(pbdma_ctrl).unwrap_or(0);
        gw(pbdma_ctrl, ctrl_val & !0x1000_0100)?;

        // Clear stale GP_BASE/PUT/GET, USERD, STATE, SIGNATURE
        gw(b + 0x040, 0)?; // GP_BASE_LO
        gw(b + 0x044, 0)?; // GP_BASE_HI
        gw(b + 0x054, 0)?; // GP_PUT
        gw(b + 0x058, 0)?; // GP_GET
        gw(b + 0x0B0, 0)?; // STATE
        gw(b + 0x0D0, 0)?; // USERD_LO
        gw(b + 0x0D4, 0)?; // USERD_HI
        gw(b + 0x0C0, 0)?; // SIGNATURE

        gw(pbdma::intr(id) as u32, 0xFFFF_FFFF)?;
        gw(pbdma::intr_en(id) as u32, 0xFFFF_FEFF)?;
    }

    // Skip 0x2600 writes — PRI-faulted on GK210B. The hardware's existing
    // PBDMA→runlist assignment (from 0x2390) is used instead.

    // Clear PFIFO interrupts. Skip 0x2200 write (doesn't stick on GK210B).
    gw(pfifo::INTR as u32, 0xFFFF_FFFF)?;

    // Try to enable PFIFO caches (Nouveau: nvkm_mask 0x2200, 1, 1).
    // On GK210B this may not stick, but write it anyway for correctness.
    let _ = gw(pfifo::ENABLE as u32, 1);

    let pfifo_en = guard.read_u32(pfifo::ENABLE as u32).unwrap_or(0xDEAD);
    let rl_base = guard.read_u32(0x2270).unwrap_or(0xDEAD);
    tracing::info!(
        pfifo_en = format_args!("{pfifo_en:#010x}"),
        rl_base = format_args!("{rl_base:#010x}"),
        pbdma_nr,
        target_runlist,
        "Kepler PFIFO engine initialized (using hw runlist assignment)"
    );
    Ok((0, target_runlist))
}
