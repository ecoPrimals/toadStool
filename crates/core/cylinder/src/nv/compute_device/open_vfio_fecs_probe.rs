// SPDX-License-Identifier: AGPL-3.0-or-later
//! FECS state probe and deferred-boot preparation before channel creation.

use crate::nv::registers::{gpc, pmc};
use crate::vfio::device::MappedBar;

use super::super::generation::GenerationProfile;
use super::super::nv_gsp_bridge::NvGspBridge;

/// Probe FECS state and prepare for deferred boot (after channel creation).
pub(super) fn probe_fecs_for_deferred_boot(
    bar0: &MappedBar,
    bdf: &str,
    profile: &GenerationProfile,
    pmc_was_cold: &mut bool,
    fecs_bridge: &mut Option<NvGspBridge>,
) {
    use crate::vfio::channel::registers::falcon;

    let pmc_before_read = crate::nv::register_read::RegisterRead::from_result(bar0.read_u32(pmc::ENABLE as usize));
    let pmc_before = pmc_before_read.raw().unwrap_or(0);
    // An unreadable device counts as zero engines, not 32.
    if pmc_before_read.count_ones().unwrap_or(0) < 8 {
        *pmc_was_cold = true;
        tracing::info!(
            bdf = %bdf,
            pmc_before = format_args!("{pmc_before:#010x}"),
            "PMC cold after VFIO FLR — enabling all engines"
        );
        let _ = bar0.write_u32(pmc::ENABLE as usize, 0xFFFF_FFFF);
        std::thread::sleep(std::time::Duration::from_millis(50));
        let pmc_after = bar0.read_u32(pmc::ENABLE as usize).unwrap_or(0);
        tracing::info!(
            bdf = %bdf,
            pmc_after = format_args!("{pmc_after:#010x}"),
            popcount = pmc_after.count_ones(),
            "PMC engines enabled"
        );

        // PRI ring re-initialization after cold boot.
        // VFIO FLR leaves the PRI ring bus in a faulted state — GPCs,
        // PGRAPH, and scheduler registers return 0xbadfXXXX. Toggle the
        // PRIV_RING engine (PMC bit 3) to bring the ring hardware back,
        // then enumerate all satellites so GPC PRI domains are reachable.
        tracing::info!(bdf = %bdf, "PRI ring cold init: resetting PRIV_RING (PMC bit 3)");
        let priv_ring_bit = 1u32 << 3;
        let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_after & !priv_ring_bit);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let _ = bar0.write_u32(pmc::ENABLE as usize, pmc_after | priv_ring_bit);
        std::thread::sleep(std::time::Duration::from_millis(20));

        // NV_PPRIV_SYS_MASTER_COMMAND = 0x12_0004 (0x4 = enumerate)
        for round in 0..5u32 {
            let _ = bar0.write_u32(0x12_0004, 0x04);
            std::thread::sleep(std::time::Duration::from_millis(20));

            // NV_PPRIV_SYS_MASTER_INTR_STATUS = 0x12_0058
            let rm_stat = bar0.read_u32(0x12_0058).unwrap_or(0xDEAD);
            // NV_PPRIV_SYS_MASTER_INTR_ACK = 0x12_004C
            if rm_stat != 0 && rm_stat & 0xBAD0_0000 != 0xBAD0_0000 {
                let _ = bar0.write_u32(0x12_004C, 0x02);
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let rm_after = bar0.read_u32(0x12_0058).unwrap_or(0xDEAD);
            let is_clear = rm_after == 0;
            let is_pri_fault = rm_after & 0xBAD0_0000 == 0xBAD0_0000;
            tracing::info!(
                bdf = %bdf,
                round,
                rm_stat = format_args!("{rm_stat:#010x}"),
                rm_after = format_args!("{rm_after:#010x}"),
                is_clear,
                "PRI ring cold init: enumerate + ACK"
            );
            if is_clear {
                break;
            }
            if is_pri_fault && round >= 2 {
                break;
            }
        }

        // Clear GPC station interrupts to fully settle the ring.
        let _ = bar0.write_u32(0x12_2058, 0xFFFF_FFFF); // GPC master interrupt clear
        let _ = bar0.write_u32(0x12_8058, 0xFFFF_FFFF); // FBP station interrupt clear
        std::thread::sleep(std::time::Duration::from_millis(10));

        let gpc_enables = bar0.read_u32(gpc::BCAST_ENABLES as usize).unwrap_or(0xDEAD);
        let is_gpc_fault = gpc_enables & 0xBAD0_0000 == 0xBAD0_0000;
        tracing::info!(
            bdf = %bdf,
            gpc_enables = format_args!("{gpc_enables:#010x}"),
            gpc_alive = !is_gpc_fault,
            "PRI ring cold init: GPC reachability after PRIV_RING reset"
        );
    }

    let fecs_alias = bar0
        .read_u32(falcon::FECS_BASE + falcon::CPUCTL_ALIAS)
        .unwrap_or(0xDEAD);
    let fecs_pc = bar0
        .read_u32(falcon::FECS_BASE + falcon::PC)
        .unwrap_or(0xDEAD);
    let is_bad_read = fecs_alias & 0xBADF_0000 == 0xBADF_0000;
    let fecs_in_hreset = !is_bad_read && (fecs_alias & falcon::CPUCTL_HRESET != 0);
    let fecs_running = !is_bad_read && !fecs_in_hreset && (fecs_alias & falcon::CPUCTL_HALTED == 0);
    let fecs_needs_boot = is_bad_read || fecs_in_hreset;

    let fecs_fw_wiped = *pmc_was_cold && fecs_pc < 0x100;

    if fecs_fw_wiped {
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
            fecs_pc = format_args!("{fecs_pc:#010x}"),
            pmc_was_cold = *pmc_was_cold,
            "FECS firmware wiped by VFIO FLR — need PIO reload"
        );
        let bridge = NvGspBridge::new(profile.firmware_chip);
        if bridge.has_gr_firmware() {
            tracing::info!(
                bdf = %bdf,
                chip = profile.firmware_chip,
                "FECS firmware available — deferring PIO boot to after channel creation"
            );
            *fecs_bridge = Some(bridge);
        } else {
            tracing::warn!(
                bdf = %bdf,
                chip = profile.firmware_chip,
                "No FECS firmware found on disk — FECS methods will fail"
            );
        }
    } else if fecs_running {
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
            fecs_pc = format_args!("{fecs_pc:#010x}"),
            "FECS already running (warm handoff preserved) — skipping boot"
        );
    } else if fecs_needs_boot {
        tracing::info!(
            bdf = %bdf,
            fecs_cpuctl_alias = format_args!("{fecs_alias:#010x}"),
            fecs_pc = format_args!("{fecs_pc:#010x}"),
            bad_read = is_bad_read,
            "FECS not alive — preparing deferred HS boot"
        );
        let bridge = NvGspBridge::new(profile.firmware_chip);
        if bridge.has_gr_firmware() {
            tracing::info!(
                bdf = %bdf,
                "FECS firmware available — deferring boot to after channel creation"
            );
            *fecs_bridge = Some(bridge);
        }
    }
}
