// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from warm_capture.rs (S334).

use super::warm_capture::*;
use crate::nv::gr_init::{ChipFamily, InitSource};

fn sample_domains() -> Vec<(&'static str, usize, usize)> {
    vec![
        ("PMC", 0x0000_0000, 0x0000_1000),
        ("PFIFO", 0x0000_2000, 0x0000_4000),
        ("PGRAPH", 0x0040_0000, 0x0042_0000),
    ]
}

#[test]
fn bar0_snapshot_alive_count() {
    let snap = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "test".into(),
        registers: vec![
            (0x000, 0x1234_5678), // alive
            (0x004, 0x0000_0000), // zero → not alive
            (0x008, 0xFFFF_FFFF), // error → not alive
            (0x00C, 0xBADF_5040), // error → not alive
            (0x010, 0x0000_0001), // alive
        ],
        timestamp_ms: 0,
    };
    assert_eq!(snap.alive_count(), 2);
    assert_eq!(snap.len(), 5);
}

#[test]
fn bar0_diff_from_snapshots() {
    let cold = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "cold".into(),
        registers: vec![(0x200, 0x0000_0000), (0x204, 0x1111_1111)],
        timestamp_ms: 0,
    };
    let warm = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "warm".into(),
        registers: vec![(0x200, 0x5fec_dff1), (0x204, 0x1111_1111)],
        timestamp_ms: 0,
    };
    let diff = Bar0Diff::from_snapshots(&cold, &warm);
    assert_eq!(diff.changed_count(), 1);
    assert_eq!(diff.unchanged_count, 1);
    assert_eq!(diff.total_compared, 2);
}

#[test]
fn bar0_diff_in_range() {
    let cold = Bar0Snapshot {
        bdf: "test".into(),
        label: "cold".into(),
        registers: vec![(0x200, 0), (0x2200, 0), (0x400700, 0)],
        timestamp_ms: 0,
    };
    let warm = Bar0Snapshot {
        bdf: "test".into(),
        label: "warm".into(),
        registers: vec![(0x200, 1), (0x2200, 2), (0x400700, 3)],
        timestamp_ms: 0,
    };
    let diff = Bar0Diff::from_snapshots(&cold, &warm);
    let pfifo = diff.in_range(0x2000, 0x4000);
    assert_eq!(pfifo.len(), 1);
    assert_eq!(pfifo[0].0, 0x2200);
}

#[test]
fn warm_state_capture_from_snapshots() {
    let cold = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "cold".into(),
        registers: vec![(0x200, 0), (0x204, 0), (0x2200, 0)],
        timestamp_ms: 0,
    };
    let warm = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "nouveau-warm".into(),
        registers: vec![(0x200, 0x5fec), (0x204, 0), (0x2200, 0x42)],
        timestamp_ms: 0,
    };
    let capture = WarmStateCapture::from_snapshots(
        cold,
        warm,
        ChipFamily::Volta,
        InitSource::NouveauDiff {
            version: "1.4.0".into(),
        },
        &sample_domains(),
    );
    assert_eq!(capture.diff.changed_count(), 2);
    assert_eq!(capture.gr_init.len(), 2);
    let summary = capture.summary();
    assert!(summary.contains("0000:41:00.0"));
}

#[test]
fn warm_state_capture_display() {
    let cold = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "cold".into(),
        registers: vec![(0x200, 0)],
        timestamp_ms: 0,
    };
    let warm = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "warm".into(),
        registers: vec![(0x200, 1)],
        timestamp_ms: 0,
    };
    let capture = WarmStateCapture::from_snapshots(
        cold,
        warm,
        ChipFamily::Kepler,
        InitSource::Manual { experiment: 1 },
        &sample_domains(),
    );
    let display = format!("{capture}");
    assert!(display.contains("WarmStateCapture"));
    assert!(display.contains("GR writes"));
}

#[test]
fn snapshot_serde_roundtrip() {
    let snap = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "test".into(),
        registers: vec![(0x200, 0x1234)],
        timestamp_ms: 12345,
    };
    let json = serde_json::to_string(&snap).unwrap();
    let back: Bar0Snapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(back.bdf, snap.bdf);
    assert_eq!(back.registers, snap.registers);
}

#[test]
fn catalyst_replay_filters_errors() {
    let snap = Bar0Snapshot {
        bdf: "0000:49:00.0".into(),
        label: "catalyst-warm".into(),
        registers: vec![
            (0x200, 0x5fec_dff1),    // alive — included
            (0x204, 0x0000_0000),    // zero — excluded
            (0x2200, 0xBADF_5040),   // PRI fault — excluded
            (0x400700, 0x0000_0042), // alive — included
        ],
        timestamp_ms: 0,
    };
    let replay = snap.to_catalyst_replay(ChipFamily::Volta, "470.256.02", &sample_domains());
    assert_eq!(replay.len(), 2);
    assert_eq!(replay.writes[0].offset, 0x200);
    assert_eq!(replay.writes[1].offset, 0x400700);
    assert!(replay.description.contains("catalyst replay"));
    assert!(replay.description.contains("470.256.02"));
}

#[test]
fn diff_to_replay_sequence() {
    let cold = Bar0Snapshot {
        bdf: "0000:02:00.0".into(),
        label: "cold".into(),
        registers: vec![
            (0x200, 0x0000_0000),
            (0x204, 0x1111_1111),
            (0x2200, 0x0000_0000),
        ],
        timestamp_ms: 0,
    };
    let warm = Bar0Snapshot {
        bdf: "0000:49:00.0".into(),
        label: "catalyst".into(),
        registers: vec![
            (0x200, 0x5fec_dff1),  // changed, alive
            (0x204, 0x1111_1111),  // unchanged
            (0x2200, 0xBADF_5040), // changed but PRI fault — excluded
        ],
        timestamp_ms: 0,
    };
    let diff = Bar0Diff::from_snapshots(&cold, &warm);
    let replay = diff.to_replay_sequence(
        ChipFamily::Volta,
        InitSource::Catalyst {
            driver_version: "470.256.02".into(),
            bdf: "0000:49:00.0".into(),
        },
        &sample_domains(),
    );
    assert_eq!(replay.len(), 1);
    assert_eq!(replay.writes[0].offset, 0x200);
    assert_eq!(replay.writes[0].value, 0x5fec_dff1);
    assert!(replay.description.contains("catalyst delta"));
}

#[test]
fn snapshot_to_json_roundtrip() {
    let snap = Bar0Snapshot {
        bdf: "0000:41:00.0".into(),
        label: "test-json".into(),
        registers: vec![(0x200, 0x1234), (0x204, 0x5678)],
        timestamp_ms: 99999,
    };
    let json = snap.to_json().unwrap();
    let back: Bar0Snapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(back.bdf, "0000:41:00.0");
    assert_eq!(back.registers.len(), 2);
}
