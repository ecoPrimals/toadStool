// SPDX-License-Identifier: AGPL-3.0-or-later
//! Internal helpers for BAR cartography — region grouping and safety checks.

use std::collections::BTreeMap;

use super::types::{
    DomainHint, RegisterAccess, RegisterPattern, RegisterProbe, RegisterRegion,
};

pub(super) fn is_dangerous_offset(offset: usize) -> bool {
    matches!(
        offset,
        0x0000_9000..=0x0009_00FF   // PTIMER
        | 0x0010_0CBC              // MMU invalidation
        | 0x0010_0CB8              // MMU invalidation PDB
        | 0x0010_0CEC              // MMU invalidation PDB HI
        | 0x0010_0E24..=0x0010_0E54 // Fault buffer registers
        | 0x0010_A040..=0x0010_A048 // PMU mailboxes
        | 0x0010_A100              // PMU CPUCTL
        | 0x0061_0000..=0x0061_0FFF // PDISP
        | 0x0000_2200              // PFIFO_ENABLE
        | 0x0000_0200              // PMC_ENABLE
    )
}

/// Exposes [`is_dangerous_offset`] for unit tests (`tests` is a child module).
#[cfg(test)]
pub(super) fn is_dangerous_offset_for_test(offset: usize) -> bool {
    is_dangerous_offset(offset)
}

pub(super) fn group_into_regions(
    register_map: &BTreeMap<usize, RegisterProbe>,
    domain_hints: &[DomainHint],
) -> Vec<RegisterRegion> {
    if register_map.is_empty() {
        return Vec::new();
    }

    let mut regions = Vec::new();
    let mut current_start: Option<usize> = None;
    let mut current_access = RegisterAccess::Dead;
    let mut responsive = 0usize;
    let mut dead = 0usize;
    let mut prev_offset: Option<usize> = None;

    for (&offset, probe) in register_map {
        let is_contiguous = prev_offset.is_none_or(|p| offset == p + 4);
        let same_type = current_start.is_some() && probe.access == current_access && is_contiguous;

        if !same_type {
            if let Some(start) = current_start {
                let end = prev_offset.unwrap_or(start) + 4;
                let name = find_domain_name(start, domain_hints);
                regions.push(RegisterRegion {
                    start,
                    end,
                    name,
                    access: current_access,
                    pattern: RegisterPattern::Constant(0),
                    responsive_count: responsive,
                    dead_count: dead,
                });
            }
            current_start = Some(offset);
            current_access = probe.access;
            responsive = 0;
            dead = 0;
        }

        if probe.access == RegisterAccess::Dead {
            dead += 1;
        } else {
            responsive += 1;
        }
        prev_offset = Some(offset);
    }

    // Flush last region
    if let Some(start) = current_start {
        let end = prev_offset.unwrap_or(start) + 4;
        let name = find_domain_name(start, domain_hints);
        regions.push(RegisterRegion {
            start,
            end,
            name,
            access: current_access,
            pattern: RegisterPattern::Constant(0),
            responsive_count: responsive,
            dead_count: dead,
        });
    }

    regions
}

fn find_domain_name(offset: usize, hints: &[DomainHint]) -> Option<String> {
    hints
        .iter()
        .find(|h| offset >= h.start && offset < h.end)
        .map(|h| h.name.to_string())
}
