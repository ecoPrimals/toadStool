// SPDX-License-Identifier: AGPL-3.0-or-later
//! Target PBDMA discovery for direct GP_PUT writes on warm-caught Volta+ GPUs.

use crate::vfio::channel::registers::{pbdma, pfifo};
use crate::vfio::channel::VfioChannel;
use crate::vfio::device::MappedBar;

use super::DoorbellKind;

/// Discover the target PBDMA for direct GP_PUT writes.
///
/// On warm-caught GV100, the scheduler doesn't reliably propagate USERD GP_PUT
/// to the PBDMA; direct register writes ensure GPFIFO consumption.
pub(crate) fn find_target_pbdma(
    bar0: &MappedBar,
    channel: &VfioChannel,
    doorbell: DoorbellKind,
    log_context: &str,
) -> Option<usize> {
    if !matches!(doorbell, DoorbellKind::Usermode) {
        return None;
    }

    let pbdma_map = bar0.read_u32(pfifo::PBDMA_MAP).unwrap_or(0);
    let runlist_id = channel.runlist_id_hint();
    let mut found: Option<usize> = None;
    let mut seq = 0_usize;
    for pid in 0..32_usize {
        if pbdma_map & (1 << pid) == 0 {
            continue;
        }
        let rl = bar0
            .read_u32(pfifo::PBDMA_RUNL_MAP + seq * 4)
            .unwrap_or(0xFFFF);
        if rl == runlist_id {
            found = Some(pbdma::base(pid));
            tracing::info!(
                pbdma = pid,
                runlist = rl,
                "target PBDMA for direct GP_PUT{log_context}"
            );
            break;
        }
        seq += 1;
    }
    found
}
