// SPDX-License-Identifier: AGPL-3.0-or-later
//! CE runlist and PBDMA discovery from engine topology tables.

use crate::vfio::device::MappedBar;

/// Discover the CE (Copy Engine) runlist ID from the engine topology table.
///
/// Returns `Some(runlist_id)` if a CE engine is found, `None` otherwise.
/// This is independent of the GR runlist used for compute dispatch.
///
/// Uses the GV100 PTOP_DEVICE_INFO_V2 format:
/// - kind=1 (DATA): engine type at bits [7:2]
/// - kind=2 (ENUM): runlist at bits [17:14]
/// - bit 31: CHAIN (end of this engine's record)
pub fn discover_ce_runlist(bar0: &MappedBar) -> Option<u32> {
    let mut cur_type: u32 = 0xFFFF;
    let mut cur_runlist: u32 = 0xFFFF;
    for i in 0..64_u32 {
        let data = bar0.read_u32(0x0002_2700 + (i as usize) * 4).unwrap_or(0);
        if data == 0 {
            break;
        }
        let kind = data & 3;
        match kind {
            1 => cur_type = (data >> 2) & 0x3F,
            2 => cur_runlist = (data >> 14) & 0xF,
            _ => {}
        }
        if data & (1 << 31) != 0 {
            if cur_type == 1 && cur_runlist != 0xFFFF {
                return Some(cur_runlist);
            }
            cur_type = 0xFFFF;
            cur_runlist = 0xFFFF;
        }
    }
    None
}

/// Find the PBDMA that serves a given runlist ID.
///
/// On GV100, `RUNLIST_PBDMA_MAP(i)` at `0x2390 + i*4` (indexed by runlist ID)
/// contains a bitmask of PBDMAs that can service that runlist. Returns the
/// lowest-numbered PBDMA from the mask.
pub fn find_pbdma_for_runlist(bar0: &MappedBar, target_runlist: u32) -> Option<usize> {
    if target_runlist > 31 {
        return None;
    }
    let pbdma_mask = bar0.read_u32(0x0000_2390 + (target_runlist as usize) * 4).unwrap_or(0);
    if pbdma_mask == 0 || pbdma_mask > 0x00FF_FFFF {
        return None;
    }
    Some(pbdma_mask.trailing_zeros() as usize)
}
