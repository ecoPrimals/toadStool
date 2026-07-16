// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from pmu_init.rs (S334).

use super::pmu_init::*;
use crate::nv::gr_init::ChipFamily;

#[test]
fn pmu_bootstrap_default() {
    let pmu = PmuBootstrap::default();
    assert_eq!(pmu.chip, ChipFamily::Kepler);
    assert_eq!(pmu.boot_vector, 0);
    assert_eq!(pmu.mailbox_init, 0x0000_5000);
    assert_eq!(pmu.mailbox_done_mask, 0x2000);
    assert_eq!(pmu.timeout_ms, 2000);
}

#[test]
fn pmu_bootstrap_kepler() {
    let pmu = PmuBootstrap::kepler();
    assert_eq!(pmu.chip, ChipFamily::Kepler);
}

#[test]
fn pmu_bootstrap_from_warm_snapshot() {
    let snapshot = PmuSnapshot {
        cpuctl: 0x10,
        bootvec: 0x1234,
        hwcfg: 0x0002_0100,
        os: 0,
        mailbox0: 0x7000,
        mailbox1: 0,
        pc: 0x5678,
        sctl: 0,
        pfifo_enable: 1,
        is_running: false,
        pfifo_enabled: true,
    };
    let pmu = PmuBootstrap::from_warm_snapshot(ChipFamily::Kepler, &snapshot);
    assert_eq!(pmu.boot_vector, 0x1234);
    assert_eq!(pmu.chip, ChipFamily::Kepler);
}

#[test]
fn pmu_snapshot_sizes() {
    let snapshot = PmuSnapshot {
        cpuctl: 0,
        bootvec: 0,
        hwcfg: 0x0040_0100,
        os: 0,
        mailbox0: 0,
        mailbox1: 0,
        pc: 0,
        sctl: 0,
        pfifo_enable: 0,
        is_running: false,
        pfifo_enabled: false,
    };
    assert_eq!(snapshot.imem_size_kb(), 16);
    assert_eq!(snapshot.dmem_size_kb(), 64);
}

#[test]
fn pmu_snapshot_signed_check() {
    let unsigned = PmuSnapshot {
        cpuctl: 0,
        bootvec: 0,
        hwcfg: 0x0000_0000,
        os: 0,
        mailbox0: 0,
        mailbox1: 0,
        pc: 0,
        sctl: 0,
        pfifo_enable: 0,
        is_running: false,
        pfifo_enabled: false,
    };
    assert!(!unsigned.requires_signed());

    let signed = PmuSnapshot {
        hwcfg: 0x0000_0100,
        ..unsigned.clone()
    };
    assert!(signed.requires_signed());
}

#[test]
fn pmu_snapshot_summary() {
    let snapshot = PmuSnapshot {
        cpuctl: 0x10,
        bootvec: 0x100,
        hwcfg: 0x0020_0080,
        os: 0,
        mailbox0: 0x7000,
        mailbox1: 0,
        pc: 0x42,
        sctl: 0,
        pfifo_enable: 1,
        is_running: false,
        pfifo_enabled: true,
    };
    let s = snapshot.summary();
    assert!(s.contains("PMU"));
    assert!(s.contains("pfifo=true"));
}

#[test]
fn pmu_bootstrap_display() {
    let pmu = PmuBootstrap::kepler();
    let s = format!("{pmu}");
    assert!(s.contains("Kepler"));
    assert!(s.contains("bootvec=0x0"));
}

#[test]
fn pmu_boot_result_display() {
    let result = PmuBootResult {
        success: true,
        post_state: PmuSnapshot {
            cpuctl: 0,
            bootvec: 0,
            hwcfg: 0,
            os: 0,
            mailbox0: 0x7000,
            mailbox1: 0,
            pc: 0x42,
            sctl: 0,
            pfifo_enable: 1,
            is_running: true,
            pfifo_enabled: true,
        },
        pfifo_unlocked: true,
        duration_ms: 150,
        detail: "test".into(),
    };
    let s = format!("{result}");
    assert!(s.contains("success=true"));
    assert!(s.contains("unlocked"));
    assert!(s.contains("150ms"));
}

#[test]
fn pmu_snapshot_serde_roundtrip() {
    let snapshot = PmuSnapshot {
        cpuctl: 0x10,
        bootvec: 0x100,
        hwcfg: 0x0020_0080,
        os: 0,
        mailbox0: 0x7000,
        mailbox1: 0,
        pc: 0x42,
        sctl: 0,
        pfifo_enable: 1,
        is_running: false,
        pfifo_enabled: true,
    };
    let json = serde_json::to_string(&snapshot).unwrap();
    let back: PmuSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(back.cpuctl, snapshot.cpuctl);
    assert_eq!(back.bootvec, snapshot.bootvec);
    assert_eq!(back.pfifo_enabled, snapshot.pfifo_enabled);
}

#[test]
fn pmu_bootstrap_serde_roundtrip() {
    let pmu = PmuBootstrap {
        chip: ChipFamily::Kepler,
        boot_vector: 0x1234,
        mailbox_init: 0x5000,
        mailbox_done_mask: 0x2000,
        timeout_ms: 3000,
    };
    let json = serde_json::to_string(&pmu).unwrap();
    let back: PmuBootstrap = serde_json::from_str(&json).unwrap();
    assert_eq!(back.boot_vector, 0x1234);
    assert_eq!(back.timeout_ms, 3000);
}
