// SPDX-License-Identifier: AGPL-3.0-or-later
//! BAR0 scanning — systematic probing and register classification.

use std::collections::BTreeMap;

use crate::vfio::device::MappedBar;

use super::helpers::{group_into_regions, is_dangerous_offset};
use super::types::{BarMap, DomainHint, RegisterAccess, RegisterPattern, RegisterProbe};

/// Scan BAR0 register space and classify every 4-byte offset.
///
/// `scan_size` limits how much of BAR0 to probe (the full 16MB BAR0 on
/// NVIDIA GPUs would take ~4M reads, so smaller scans are useful for
/// quick probing).
///
/// `safe_write_test` enables write-readback testing on registers that don't
/// look like they'll trigger side effects. Defaults to false for safety.
pub fn scan_bar0(
    bar0: &MappedBar,
    scan_size: usize,
    safe_write_test: bool,
    domain_hints: &[DomainHint],
) -> BarMap {
    let mut register_map = BTreeMap::new();
    let mut responsive_bytes = 0usize;
    let mut error_bytes = 0usize;

    let end = scan_size.min(16 * 1024 * 1024);

    for offset in (0..end).step_by(4) {
        let r1 = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);

        let is_error = r1 == 0xFFFF_FFFF
            || (r1 & 0xFFFF_0000) == 0xBADF_0000
            || (r1 & 0xFFFF_0000) == 0xBAD0_0000
            || r1 == 0xDEAD_DEAD;

        if is_error {
            error_bytes += 4;
            register_map.insert(
                offset,
                RegisterProbe {
                    offset,
                    read1: r1,
                    read2: r1,
                    writable: None,
                    access: RegisterAccess::Dead,
                    pattern: RegisterPattern::ErrorPattern(r1),
                },
            );
            continue;
        }

        responsive_bytes += 4;
        let r2 = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);

        let pattern = if r1 == 0 && r2 == 0 {
            RegisterPattern::Zeros
        } else if r1 != r2 {
            RegisterPattern::Dynamic
        } else {
            RegisterPattern::Constant(r1)
        };

        let writable = if safe_write_test && !is_dangerous_offset(offset) {
            let original = r1;
            let test_val = !original;
            let _ = bar0.write_u32(offset, test_val);
            let readback = bar0.read_u32(offset).unwrap_or(original);
            let _ = bar0.write_u32(offset, original);
            Some(readback == test_val)
        } else {
            None
        };

        let access = match (pattern, writable) {
            (RegisterPattern::Dynamic, _) => RegisterAccess::ReadOnly,
            (_, Some(true)) => RegisterAccess::ReadWrite,
            (RegisterPattern::Zeros, _) => RegisterAccess::WriteOnly,
            _ => RegisterAccess::ReadOnly,
        };

        register_map.insert(
            offset,
            RegisterProbe {
                offset,
                read1: r1,
                read2: r2,
                writable,
                access,
                pattern,
            },
        );
    }

    let regions = group_into_regions(&register_map, domain_hints);

    BarMap {
        bar_index: 0,
        size: end,
        regions,
        responsive_bytes,
        error_bytes,
        register_map,
    }
}

/// Quick scan of specific register ranges rather than the full BAR.
///
/// Much faster — only probes the ranges provided, which is useful for
/// targeted domain analysis.
pub fn scan_ranges(bar0: &MappedBar, ranges: &[(&str, usize, usize)]) -> BarMap {
    let mut register_map = BTreeMap::new();
    let mut responsive_bytes = 0usize;
    let mut error_bytes = 0usize;
    let mut regions = Vec::new();

    for &(name, start, end) in ranges {
        let mut region_responsive = 0;
        let mut region_dead = 0;

        for offset in (start..end).step_by(4) {
            let r1 = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
            let is_error = r1 == 0xFFFF_FFFF
                || (r1 & 0xFFFF_0000) == 0xBADF_0000
                || (r1 & 0xFFFF_0000) == 0xBAD0_0000
                || r1 == 0xDEAD_DEAD;

            if is_error {
                error_bytes += 4;
                region_dead += 1;
                register_map.insert(
                    offset,
                    RegisterProbe {
                        offset,
                        read1: r1,
                        read2: r1,
                        writable: None,
                        access: RegisterAccess::Dead,
                        pattern: RegisterPattern::ErrorPattern(r1),
                    },
                );
            } else {
                responsive_bytes += 4;
                region_responsive += 1;
                let r2 = bar0.read_u32(offset).unwrap_or(0xDEAD_DEAD);
                let pattern = if r1 == 0 && r2 == 0 {
                    RegisterPattern::Zeros
                } else if r1 != r2 {
                    RegisterPattern::Dynamic
                } else {
                    RegisterPattern::Constant(r1)
                };
                let access = match pattern {
                    RegisterPattern::Dynamic => RegisterAccess::ReadOnly,
                    RegisterPattern::Zeros => RegisterAccess::WriteOnly,
                    _ => RegisterAccess::ReadOnly,
                };
                register_map.insert(
                    offset,
                    RegisterProbe {
                        offset,
                        read1: r1,
                        read2: r2,
                        writable: None,
                        access,
                        pattern,
                    },
                );
            }
        }

        let predominant_access = if region_responsive > region_dead {
            RegisterAccess::ReadOnly
        } else {
            RegisterAccess::Dead
        };
        let predominant_pattern = if region_dead > region_responsive {
            RegisterPattern::ErrorPattern(0xFFFF_FFFF)
        } else {
            RegisterPattern::Constant(0)
        };

        regions.push(super::types::RegisterRegion {
            start,
            end,
            name: Some(name.to_string()),
            access: predominant_access,
            pattern: predominant_pattern,
            responsive_count: region_responsive,
            dead_count: region_dead,
        });
    }

    BarMap {
        bar_index: 0,
        size: regions.iter().map(|r| r.end).max().unwrap_or(0),
        regions,
        responsive_bytes,
        error_bytes,
        register_map,
    }
}

/// Snapshot specific registers and return (offset, value) pairs.
///
/// Useful for before/after comparison across power state transitions.
pub fn snapshot_registers(bar0: &MappedBar, offsets: &[usize]) -> Vec<(usize, u32)> {
    offsets
        .iter()
        .map(|&off| (off, bar0.read_u32(off).unwrap_or(0xDEAD_DEAD)))
        .collect()
}

/// Compare two register snapshots and return deltas.
pub fn diff_snapshots(before: &[(usize, u32)], after: &[(usize, u32)]) -> Vec<(usize, u32, u32)> {
    before
        .iter()
        .zip(after.iter())
        .filter(|((o1, v1), (o2, v2))| o1 == o2 && v1 != v2)
        .map(|((off, v_before), (_, v_after))| (*off, *v_before, *v_after))
        .collect()
}
