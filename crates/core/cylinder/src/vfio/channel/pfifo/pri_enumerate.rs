// SPDX-License-Identifier: AGPL-3.0-or-later
//! PRI ring satellite enumeration after PFIFO PMC reset.
//!
//! After toggling PMC_ENABLE bit 8 (PFIFO reset), the per-runlist PRI domains
//! may be in a faulted state. Writing `PPRIV_RING_MASTER_COMMAND = 0x04`
//! (enumerate) forces the PRI ring master to re-register all satellite devices,
//! bringing per-runlist registers back online.
//!
//! Without this, `RUNLIST_BASE` writes silently fail (readback = 0) because the
//! target PRI domain is dead.
//!
//! Reference: Jun 1 2026 RCA — `HOTSPRING_TIER2_PBDMA_ROOT_CAUSE_JUN01_2026.md`

use crate::vfio::device::MappedBar;

const PPRIV_RING_MASTER_COMMAND: usize = 0x12_0004;
const PPRIV_RING_MASTER_STATUS: usize = 0x12_0058;
const PPRIV_RING_MASTER_ACK: usize = 0x12_004C;

const PPRIV_GPC_SATELLITE_INTR_RESET: usize = 0x12_2058;
const PPRIV_FBPA_SATELLITE_INTR_RESET: usize = 0x12_8058;

const ENUMERATE_CMD: u32 = 0x04;
const ACK_CMD: u32 = 0x02;
const PRI_ERROR_MASK: u32 = 0xBAD0_0000;

/// Enumerate PRI ring satellites to bring per-runlist PRI domains online.
///
/// Issues up to `max_rounds` enumerate commands to the PRI ring master,
/// ACK-ing any reported faults. Returns `true` if enumeration completed
/// cleanly (status == 0), `false` if faults persisted.
///
/// Safe to call multiple times; idempotent when PRI domains are healthy.
pub fn pri_ring_enumerate(bar0: &MappedBar, bdf: &str, max_rounds: u32) -> bool {
    let mut clean = false;

    for round in 0..max_rounds {
        let _ = bar0.write_u32(PPRIV_RING_MASTER_COMMAND, ENUMERATE_CMD);
        std::thread::sleep(std::time::Duration::from_millis(20));

        let status = bar0.read_u32(PPRIV_RING_MASTER_STATUS).unwrap_or(0);

        if status != 0 && status & PRI_ERROR_MASK != PRI_ERROR_MASK {
            let _ = bar0.write_u32(PPRIV_RING_MASTER_ACK, ACK_CMD);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let status_after = bar0.read_u32(PPRIV_RING_MASTER_STATUS).unwrap_or(0);

        tracing::info!(
            bdf = %bdf,
            round,
            status_before = format_args!("{status:#010x}"),
            status_after = format_args!("{status_after:#010x}"),
            "PRI ring enumerate + ACK"
        );

        if status_after == 0 {
            clean = true;
            break;
        }
        if status_after & PRI_ERROR_MASK == PRI_ERROR_MASK && round >= 2 {
            break;
        }
    }

    // Clear GPC and FBPA satellite interrupt state regardless of outcome.
    let _ = bar0.write_u32(PPRIV_GPC_SATELLITE_INTR_RESET, 0xFFFF_FFFF);
    let _ = bar0.write_u32(PPRIV_FBPA_SATELLITE_INTR_RESET, 0xFFFF_FFFF);
    std::thread::sleep(std::time::Duration::from_millis(10));

    if clean {
        tracing::info!(bdf = %bdf, "PRI ring enumerate complete — satellites online");
    } else {
        tracing::warn!(
            bdf = %bdf,
            "PRI ring enumerate finished with residual faults — \
             per-runlist registers may still be inaccessible"
        );
    }

    clean
}
