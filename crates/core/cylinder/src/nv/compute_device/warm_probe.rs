// SPDX-License-Identifier: AGPL-3.0-or-later
//! BAR0 warm FECS state probing after nouveau → vfio-pci handoff.

use super::NvVfioComputeDevice;

impl NvVfioComputeDevice {
    /// Probe BAR0 for warm FECS state.
    ///
    /// After a nouveau → vfio-pci warm handoff, FECS is in one of two
    /// valid warm states depending on the teardown strategy:
    ///
    /// - **Live warm**: FECS still running (not halted), PMC engines enabled.
    ///   Occurs with NOP'd-teardown patched nouveau — FECS was never stopped.
    /// - **Preserved warm**: FECS halted + MAILBOX0 ≠ 0, firmware resident in
    ///   IMEM/DMEM. Occurs with standard teardown interception (kprobe/livepatch).
    ///
    /// Both states indicate the GPU is compute-ready. Cold state (PMC popcount < 8)
    /// means no prior driver session initialized the GPU.
    ///
    /// Also probes BOOT0 for chip identification if capabilities are unknown.
    /// Returns `true` if warm FECS was detected and the device is compute-ready.
    pub fn probe_warm_fecs(&mut self) -> bool {
        use crate::nv::registers::{falcon as nv_falcon, pmc};
        use crate::vfio::channel::registers::falcon;

        const BAR0_MIN_SIZE: usize = 0x41_A000;

        enum Bar0Source {
            Sysfs(crate::vfio::sysfs_bar0::SysfsBar0),
            #[expect(
                dead_code,
                reason = "VfioDevice must outlive MappedBar to keep the fd alive"
            )]
            Vfio(crate::vfio::device::MappedBar, crate::vfio::VfioDevice),
        }

        impl Bar0Source {
            fn read(&self, offset: usize) -> u32 {
                match self {
                    Self::Sysfs(b) => b.read_u32(offset),
                    Self::Vfio(b, _) => b.read_u32(offset).unwrap_or(0xDEAD_DEAD),
                }
            }
        }

        let bar0 = match crate::vfio::sysfs_bar0::SysfsBar0::open(&self.bdf, BAR0_MIN_SIZE) {
            Ok(b) => Bar0Source::Sysfs(b),
            Err(e) => {
                tracing::debug!(bdf = %self.bdf, error = %e, "sysfs BAR0 failed — trying VFIO API");
                match crate::vfio::VfioDevice::open(&self.bdf)
                    .and_then(|dev| dev.map_bar(0).map(|bar| (bar, dev)))
                {
                    Ok((bar, dev)) => {
                        tracing::info!(bdf = %self.bdf, "warm probe via VFIO BAR0 mmap");
                        Bar0Source::Vfio(bar, dev)
                    }
                    Err(e2) => {
                        tracing::debug!(bdf = %self.bdf, error = %e2, "VFIO BAR0 also failed");
                        return false;
                    }
                }
            }
        };

        if self.caps.vendor == crate::hardware::Vendor::Unknown {
            let boot0 = bar0.read(pmc::BOOT0 as usize);
            if let Some(sm) = super::super::identity::boot0_to_sm(boot0) {
                let profile = super::super::generation::profile_for_sm(sm);
                self.caps = profile.to_capabilities();
                self.sm = sm;
                tracing::info!(
                    bdf = %self.bdf, sm,
                    chip = super::super::identity::chip_name(sm),
                    "warm probe: identified NVIDIA GPU from BOOT0"
                );
            }
        }

        let pmc_enable = bar0.read(pmc::ENABLE as usize);
        if pmc_enable.count_ones() < 8 {
            tracing::debug!(
                bdf = %self.bdf,
                pmc_enable = format!("{pmc_enable:#010x}"),
                popcount = pmc_enable.count_ones(),
                "cold GPU: PMC_ENABLE popcount < 8"
            );
            return false;
        }

        let fecs_cpuctl_alias =
            bar0.read((nv_falcon::FECS_BASE + nv_falcon::CPUCTL_ALIAS) as usize);
        let fecs_cpuctl_raw = bar0.read((nv_falcon::FECS_BASE + nv_falcon::CPUCTL) as usize);
        let fecs_mb0 = bar0.read((nv_falcon::FECS_BASE + nv_falcon::MAILBOX0) as usize);
        let fecs_pc = bar0.read((nv_falcon::FECS_BASE + nv_falcon::PC) as usize);

        let halted = fecs_cpuctl_alias & falcon::CPUCTL_HALTED != 0;
        let in_hreset = fecs_cpuctl_alias & falcon::CPUCTL_HRESET != 0;
        let running = !halted && !in_hreset;

        tracing::info!(
            bdf = %self.bdf,
            fecs_cpuctl_alias = format!("{fecs_cpuctl_alias:#010x}"),
            fecs_cpuctl_raw = format!("{fecs_cpuctl_raw:#010x}"),
            fecs_pc = format!("{fecs_pc:#010x}"),
            fecs_mb0 = format!("{fecs_mb0:#010x}"),
            halted,
            in_hreset,
            running,
            pmc_popcount = pmc_enable.count_ones(),
            "FECS warm-state probe (CPUCTL_ALIAS)"
        );

        let preserved_warm = halted && fecs_mb0 != 0;
        let live_warm = running && pmc_enable.count_ones() >= 16;

        // Detect post-catalyst state by FECS PC range. RM firmware PCs live in
        // the 0x18b3xxxx range; nouveau firmware idles at ~0x6000. When FECS PC
        // is in the RM range, the catalyst pipeline warmed this GPU and we must
        // skip destructive PRI operations in open_vfio() to preserve TPC state.
        //
        // On Volta HS, CPUCTL_ALIAS may read 0x00000000 (HS security gate zeros
        // the register), so we cannot rely on the halted flag for detection.
        let is_catalyst_pc = fecs_pc >= 0x1000_0000 && pmc_enable.count_ones() >= 16;

        if preserved_warm {
            tracing::info!(
                bdf = %self.bdf,
                "FECS warm-preserved (halted + firmware resident) — compute-ready"
            );
            self.fecs_ready = true;
            if is_catalyst_pc {
                self.catalyst_warm = true;
            }
            return true;
        }

        if live_warm {
            tracing::info!(
                bdf = %self.bdf,
                pmc_popcount = pmc_enable.count_ones(),
                fecs_pc = format!("{fecs_pc:#010x}"),
                catalyst = is_catalyst_pc,
                "FECS live-warm (still running, NOP'd teardown) — compute-ready"
            );
            self.fecs_ready = true;
            if is_catalyst_pc {
                self.catalyst_warm = true;
                tracing::info!(
                    bdf = %self.bdf,
                    "catalyst_warm set: FECS PC in RM firmware range, \
                     open_vfio will skip destructive ungating"
                );
            }
            return true;
        }

        // Fallback: halted + RM firmware PC (CPUCTL_ALIAS reported halted).
        if is_catalyst_pc {
            tracing::info!(
                bdf = %self.bdf,
                fecs_pc = format!("{fecs_pc:#010x}"),
                pmc_popcount = pmc_enable.count_ones(),
                "FECS catalyst-warm (RM firmware, TPC state preserved) — compute-ready"
            );
            self.fecs_ready = true;
            self.catalyst_warm = true;
            return true;
        }

        tracing::debug!(
            bdf = %self.bdf,
            "FECS not warm (halted={halted}, mb0={fecs_mb0:#x}, pmc_pop={})",
            pmc_enable.count_ones(),
        );
        false
    }
}
