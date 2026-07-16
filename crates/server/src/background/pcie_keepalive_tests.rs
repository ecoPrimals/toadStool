// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from pcie_keepalive.rs (S335).

use std::sync::atomic::Ordering;

use super::pcie_keepalive::*;
use toadstool_ember::plx_keepalive::is_pci_bdf;

#[test]
fn pci_base_subclass_bridge() {
    // PLX PEX 8747: class register 0x060400ca
    assert_eq!(pci_base_subclass(0x0604_00ca), PCI_CLASS_BRIDGE_PCI);
}

#[test]
fn pci_base_subclass_3d_controller() {
    // Tesla K80 GK210: class register 0x030200a1
    assert_eq!(pci_base_subclass(0x0302_00a1), PCI_CLASS_3D);
}

#[test]
fn pci_base_subclass_vga() {
    // RTX 5060: class register 0x030000a1
    assert_eq!(pci_base_subclass(0x0300_00a1), PCI_CLASS_VGA);
}

#[test]
fn pci_base_subclass_dead_device() {
    assert_eq!(pci_base_subclass(0xFFFF_FFFF), 0xFFFF);
}

#[test]
fn read_config_u16_nonexistent() {
    assert!(read_config_u16("9999:99:99.9", 0x00).is_none());
}

#[test]
fn read_config_u32_nonexistent() {
    assert!(read_config_u32("9999:99:99.9", 0x08).is_none());
}

#[test]
fn discover_plx_bridges_runs_without_panic() {
    let bridges = discover_plx_bridges();
    for bdf in &bridges {
        assert!(is_pci_bdf(bdf), "invalid BDF in PLX bridges: {bdf}");
    }
}

#[test]
fn discover_gpu_bridges_runs_without_panic() {
    let bridges = discover_gpu_bridges();
    for bdf in &bridges {
        assert!(is_pci_bdf(bdf), "invalid BDF in GPU bridges: {bdf}");
    }
}

#[test]
fn discover_ancestry_runs_without_panic() {
    let bridges = discover_plx_bridges_via_gpu_ancestry();
    for bdf in &bridges {
        assert!(is_pci_bdf(bdf), "invalid BDF from ancestry walk: {bdf}");
    }
}

#[test]
fn activity_tracker_integration() {
    let tracker = activity_tracker();
    // Initially no activity
    assert!(tracker.ms_since_last() > 1_000_000 || tracker.ms_since_last() == u64::MAX);

    tracker.record();
    assert!(tracker.ms_since_last() < 1000);
}

#[test]
fn swap_guard_refcount() {
    assert_eq!(SWAP_GUARD_COUNT.load(Ordering::Relaxed), 0);
    let guard = SwapGuard::enter();
    assert_eq!(SWAP_GUARD_COUNT.load(Ordering::Relaxed), 1);
    assert_eq!(current_interval(), BURST_INTERVAL);
    drop(guard);
    assert_eq!(SWAP_GUARD_COUNT.load(Ordering::Relaxed), 0);
    assert_eq!(current_interval(), KEEPALIVE_INTERVAL);
}

#[test]
fn current_interval_normal() {
    assert_eq!(current_interval(), KEEPALIVE_INTERVAL);
}
