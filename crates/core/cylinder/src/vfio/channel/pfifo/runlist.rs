// SPDX-License-Identifier: AGPL-3.0-or-later
//! Runlist submission and scheduler cycle helpers for [`super::super::VfioChannel`].

use std::borrow::Cow;

use crate::error::{DriverError, DriverResult};
use crate::vfio::device::MappedBar;

use super::super::registers::{self, pccsr, pfifo, RUNLIST_IOVA};
use super::super::VfioChannel;

impl VfioChannel {
    /// Re-submit the runlist after modifying the instance block (e.g., adding
    /// a GR context pointer). Cycles the scheduler to force FECS to re-read
    /// the updated channel state.
    pub fn resubmit_runlist(&self, bar0: &MappedBar) -> DriverResult<()> {
        tracing::info!(
            channel_id = self.channel_id,
            runlist_id = self.runlist_id,
            "re-submitting runlist with scheduler cycle"
        );

        // 1. Disable scheduler
        let _ = bar0.write_u32(pfifo::SCHED_DISABLE, 0xFFFF_FFFF);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 2. Clear pending interrupts
        let _ = bar0.write_u32(pfifo::INTR, 0xFFFF_FFFF);

        // 3. Preempt old runlist state
        let _ = bar0.write_u32(pfifo::GV100_PREEMPT, 1u32 << self.runlist_id);
        std::thread::sleep(std::time::Duration::from_millis(20));
        let _ = bar0.write_u32(pfifo::INTR, pfifo::INTR_RL_COMPLETE);

        // 4. Re-enable channel
        let _ = bar0.write_u32(
            pccsr::channel(self.channel_id),
            pccsr::CHANNEL_ENABLE_SET,
        );

        // 5. Submit runlist
        self.submit_runlist(bar0)?;

        // 6. Re-enable scheduler
        let _ = bar0.write_u32(pfifo::SCHED_DISABLE, 0);
        let _ = bar0.write_u32(pfifo::SCHED_EN, 1);
        std::thread::sleep(std::time::Duration::from_millis(10));

        // 7. Wait for scheduler to process
        let mut loaded = false;
        for tick in 0..100 {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let pccsr_val = bar0.read_u32(pccsr::channel(self.channel_id)).unwrap_or(0);
            let status = pccsr::status(pccsr_val);
            if status >= 5 {
                tracing::info!(
                    tick,
                    status,
                    pccsr = format_args!("{pccsr_val:#010x}"),
                    "scheduler loaded channel after cycle (STATUS >= ON_PBDMA)"
                );
                loaded = true;
                break;
            }
            if tick % 20 == 19 {
                tracing::debug!(
                    tick,
                    status,
                    pccsr = format_args!("{pccsr_val:#010x}"),
                    "waiting for scheduler after cycle"
                );
            }
        }
        if !loaded {
            let pccsr_val = bar0.read_u32(pccsr::channel(self.channel_id)).unwrap_or(0);
            let status = pccsr::status(pccsr_val);
            tracing::info!(
                status,
                pccsr = format_args!("{pccsr_val:#010x}"),
                "scheduler still pending after cycle"
            );
        }
        Ok(())
    }

    /// Submit runlist to PFIFO using GV100 per-runlist registers.
    ///
    /// GV100 uses per-runlist registers at stride 0x10:
    ///   BASE(rl) = 0x2270 + rl*0x10   → lower_32(iova >> 12)
    ///   SUBMIT(rl) = 0x2274 + rl*0x10 → upper_32(iova >> 12) | (count << 16)
    /// Writing SUBMIT triggers the scheduler.
    /// Source: nouveau `gv100_runl_commit()`.
    pub(in crate::vfio::channel::pfifo) fn submit_runlist(&self, bar0: &MappedBar) -> DriverResult<()> {
        // GV100 runlist BASE register: plain (addr >> 12), NO target bits.
        // nouveau's gv100_runl_commit writes lower_32_bits(addr >> 12) directly.
        // Previously we OR'd in (TARGET_SYS_MEM_COHERENT << 28) which corrupted
        // the address, making the scheduler read runlist from 0x200000000000.
        let rl_base = registers::pfifo::gv100_runlist_base_value(RUNLIST_IOVA);
        let rl_submit = registers::pfifo::gv100_runlist_submit_value(RUNLIST_IOVA, 2);

        tracing::debug!(
            runlist_id = self.runlist_id,
            rl_base = format_args!("{rl_base:#010x}"),
            rl_submit = format_args!("{rl_submit:#010x}"),
            "submitting runlist (gv100 per-RL)"
        );

        bar0.write_u32(registers::pfifo::runlist_base(self.runlist_id), rl_base)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("runlist base: {e}"))))?;
        bar0.write_u32(registers::pfifo::runlist_submit(self.runlist_id), rl_submit)
            .map_err(|e| DriverError::SubmitFailed(Cow::Owned(format!("runlist submit: {e}"))))
    }
}
