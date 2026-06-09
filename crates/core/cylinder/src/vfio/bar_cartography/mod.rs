// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(missing_docs, reason = "BAR cartography types; full docs planned")]
//! BAR0 register space cartography — systematic scanning and classification.
//!
//! Scans the entire BAR0 MMIO space in 4-byte strides, classifying each
//! register offset as readable, writable, dynamic, dead, or error-producing.
//! Groups contiguous regions with similar behavior into named domains.
//!
//! This is vendor-agnostic: it discovers what's there by probing, then
//! optionally labels regions using a vendor-specific domain map.

mod display;
mod helpers;
mod scan;
mod types;

pub use display::diff_bar_maps;
pub use scan::{diff_snapshots, scan_bar0, scan_ranges, snapshot_registers};
pub use types::{
    BarMap, BarMapDiff, DomainHint, RegisterAccess, RegisterPattern, RegisterProbe,
    RegisterRegion,
};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::BTreeMap;

    #[test]
    fn diff_snapshots_empty() {
        assert!(diff_snapshots(&[], &[]).is_empty());
    }

    #[test]
    fn diff_snapshots_no_changes() {
        let before = [(0x100usize, 1u32), (0x104, 2)];
        let after = [(0x100, 1), (0x104, 2)];
        assert!(diff_snapshots(&before, &after).is_empty());
    }

    #[test]
    fn diff_snapshots_some_changes() {
        let before = [(0x200, 10), (0x204, 20), (0x208, 30)];
        let after = [(0x200, 11), (0x204, 20), (0x208, 31)];
        let d = diff_snapshots(&before, &after);
        assert_eq!(d, vec![(0x200, 10, 11), (0x208, 30, 31)]);
    }

    #[test]
    fn diff_snapshots_all_changes() {
        let before = [(0x0, 0u32), (0x4, 1)];
        let after = [(0x0, 0xFFFF_FFFF), (0x4, 0xDEAD_BEEF)];
        let d = diff_snapshots(&before, &after);
        assert_eq!(d, vec![(0x0, 0, 0xFFFF_FFFF), (0x4, 1, 0xDEAD_BEEF)]);
    }

    #[test]
    fn diff_snapshots_mismatched_offsets_no_delta() {
        let before = [(0x0, 1u32)];
        let after = [(0x4, 2u32)];
        assert!(diff_snapshots(&before, &after).is_empty());
    }

    fn probe_dead(offset: usize, sig: u32) -> RegisterProbe {
        RegisterProbe {
            offset,
            read1: sig,
            read2: sig,
            writable: None,
            access: RegisterAccess::Dead,
            pattern: RegisterPattern::ErrorPattern(sig),
        }
    }

    fn probe_alive(offset: usize, v: u32) -> RegisterProbe {
        RegisterProbe {
            offset,
            read1: v,
            read2: v,
            writable: None,
            access: RegisterAccess::ReadOnly,
            pattern: RegisterPattern::Constant(v),
        }
    }

    fn synthetic_bar_map(register_map: BTreeMap<usize, RegisterProbe>) -> BarMap {
        BarMap {
            bar_index: 0,
            size: register_map
                .keys()
                .next_back()
                .map_or(0, |k| k + 4)
                .min(16 * 1024 * 1024),
            regions: Vec::new(),
            responsive_bytes: register_map
                .values()
                .filter(|p| p.access != RegisterAccess::Dead)
                .count()
                * 4,
            error_bytes: register_map
                .values()
                .filter(|p| p.access == RegisterAccess::Dead)
                .count()
                * 4,
            register_map,
        }
    }

    #[test]
    fn diff_bar_maps_woke_up_and_went_dead() {
        let mut before_map = BTreeMap::new();
        before_map.insert(0x100, probe_dead(0x100, 0xBADF_0000));
        before_map.insert(0x104, probe_alive(0x104, 0xA));

        let mut after_map = BTreeMap::new();
        after_map.insert(0x100, probe_alive(0x100, 0x1));
        after_map.insert(0x104, probe_dead(0x104, 0xFFFF_FFFF));

        let before = synthetic_bar_map(before_map);
        let after = synthetic_bar_map(after_map);
        let d = diff_bar_maps(&before, &after);
        assert_eq!(d.woke_up, vec![(0x100, 0x1)]);
        assert_eq!(d.went_dead, vec![(0x104, 0xA)]);
        assert!(d.value_changed.is_empty());
        assert_eq!(d.unchanged, 0);
    }

    #[test]
    fn diff_bar_maps_value_changed_and_unchanged() {
        let mut before_map = BTreeMap::new();
        before_map.insert(0x200, probe_alive(0x200, 1));
        before_map.insert(0x204, probe_alive(0x204, 2));

        let mut after_map = BTreeMap::new();
        after_map.insert(0x200, probe_alive(0x200, 9));
        after_map.insert(0x204, probe_alive(0x204, 2));

        let before = synthetic_bar_map(before_map);
        let after = synthetic_bar_map(after_map);
        let d = diff_bar_maps(&before, &after);
        assert_eq!(d.value_changed, vec![(0x200, 1, 9)]);
        assert_eq!(d.unchanged, 1);
        assert!(d.woke_up.is_empty());
        assert!(d.went_dead.is_empty());
    }

    #[test]
    fn bar_map_to_json_value_shape() {
        let mut m = BTreeMap::new();
        m.insert(0x0, probe_alive(0x0, 0));
        let map = synthetic_bar_map(m);
        let v = map.to_json_value();
        assert_eq!(v["bar_index"], json!(0));
        assert_eq!(v["responsive_bytes"], json!(4));
        assert_eq!(v["error_bytes"], json!(0));
        assert_eq!(v["region_count"], json!(0));
        assert!(v["regions"].is_array());
    }

    #[test]
    fn bar_map_diff_to_json_value_shape() {
        let d = BarMapDiff {
            woke_up: vec![(0x10, 0x1)],
            went_dead: vec![],
            value_changed: vec![(0x20, 0x2, 0x3)],
            unchanged: 4,
        };
        let v = d.to_json_value();
        assert_eq!(v["woke_up"], json!(1));
        assert_eq!(v["went_dead"], json!(0));
        assert_eq!(v["value_changed"], json!(1));
        assert_eq!(v["unchanged"], json!(4));
        let woke = v["woke_up_registers"].as_array().expect("array");
        assert_eq!(woke[0]["offset"], json!("0x10"));
        let ch = v["value_changed_registers"].as_array().expect("array");
        assert_eq!(ch[0]["before"], json!("0x00000002"));
        assert_eq!(ch[0]["after"], json!("0x00000003"));
    }

    #[test]
    fn is_dangerous_offset_for_test_matches_policy() {
        assert!(helpers::is_dangerous_offset_for_test(0x0000_0200));
        assert!(helpers::is_dangerous_offset_for_test(0x0000_2200));
        assert!(helpers::is_dangerous_offset_for_test(0x0000_9000));
        assert!(helpers::is_dangerous_offset_for_test(0x0010_A100));
        assert!(helpers::is_dangerous_offset_for_test(0x0061_0500));
        assert!(!helpers::is_dangerous_offset_for_test(0x0000_0204));
        assert!(!helpers::is_dangerous_offset_for_test(0x0000_0800));
    }

    #[test]
    fn error_signatures_classify_as_dead_like_scan_bar0() {
        let sigs = [0xBADF_1234, 0xBAD0_5678, 0xDEAD_DEAD, 0xFFFF_FFFF];
        for sig in sigs {
            let mut m = BTreeMap::new();
            m.insert(0, probe_dead(0, sig));
            let map = synthetic_bar_map(m);
            assert_eq!(map.register_map[&0].access, RegisterAccess::Dead);
            assert_eq!(map.error_bytes, 4);
        }
    }
}
