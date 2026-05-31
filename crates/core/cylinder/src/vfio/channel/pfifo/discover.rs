// SPDX-License-Identifier: AGPL-3.0-or-later
//! CE runlist and PBDMA discovery from engine topology tables.

use crate::nv::generation::GenerationProfile;
use crate::vfio::device::MappedBar;

/// Discover the CE (Copy Engine) runlist ID from the engine topology table.
///
/// Returns `Some(runlist_id)` if a CE engine is found, `None` otherwise.
/// This is independent of the GR runlist used for compute dispatch.
///
/// Uses the PTOP DEVICE_INFO format from [`GenerationProfile`]:
/// - kind=1 (DATA): engine type at bits [7:2]
/// - kind=2 (ENUM): runlist at bits [17:14] (GV100+ V2 layout)
/// - bit 31: CHAIN (end of this engine's record)
pub fn discover_ce_runlist(bar0: &MappedBar, profile: &GenerationProfile) -> Option<u32> {
    let ptop_base = profile.ptop_device_info_base as usize;
    let mut cur_type: u32 = 0xFFFF;
    let mut cur_runlist: u32 = 0xFFFF;
    for i in 0..64_u32 {
        let data = bar0.read_u32(ptop_base + (i as usize) * 4).unwrap_or(0);
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
/// `RUNLIST_PBDMA_MAP(i)` at `profile.runlist_pbdma_map_base + i*4` (indexed
/// by runlist ID) contains a bitmask of PBDMAs that can service that runlist.
/// Returns the lowest-numbered PBDMA from the mask.
pub fn find_pbdma_for_runlist(
    bar0: &MappedBar,
    target_runlist: u32,
    profile: &GenerationProfile,
) -> Option<usize> {
    if target_runlist > 31 {
        return None;
    }
    let map_base = profile.runlist_pbdma_map_base as usize;
    let pbdma_mask = bar0
        .read_u32(map_base + (target_runlist as usize) * 4)
        .unwrap_or(0);
    if pbdma_mask == 0 || pbdma_mask > 0x00FF_FFFF {
        return None;
    }
    Some(pbdma_mask.trailing_zeros() as usize)
}
