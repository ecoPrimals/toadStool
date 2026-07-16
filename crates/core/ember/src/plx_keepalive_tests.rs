// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from plx_keepalive.rs (S334).

use super::plx_keepalive::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Duration;

#[test]
fn detect_bridge_chain_includes_device() {
    let chain = detect_bridge_chain("9999:99:99.9");
    assert_eq!(chain, vec!["9999:99:99.9"]);
}

#[test]
fn config_read_nonexistent_returns_false() {
    assert!(!config_read_heartbeat("9999:99:99.9"));
}

#[test]
fn keepalive_new_nonexistent_device() {
    let ka = PcieBridgeKeepalive::new("9999:99:99.9", Duration::from_secs(5));
    assert_eq!(ka.bdf, "9999:99:99.9");
    assert_eq!(ka.bridge_chain.len(), 1);
    assert!(!ka.has_bridges());
    assert!(ka.activity.is_none());
}

#[test]
fn keepalive_with_secs() {
    let ka = PcieBridgeKeepalive::with_secs("9999:99:99.9", 3);
    assert_eq!(ka.interval, Duration::from_secs(3));
}

#[test]
fn keepalive_with_activity_tracker() {
    let tracker = ActivityTracker::new();
    let ka = PcieBridgeKeepalive::new("9999:99:99.9", Duration::from_secs(5))
        .with_activity_tracker(tracker);
    assert!(ka.activity.is_some());
}

#[test]
fn plx_alias_works() {
    let _ka: PlxKeepalive = PcieBridgeKeepalive::new("9999:99:99.9", Duration::from_secs(5));
}

#[test]
fn activity_tracker_initial_state() {
    let tracker = ActivityTracker::new();
    assert_eq!(tracker.ms_since_last(), u64::MAX);
}

#[test]
fn activity_tracker_record_and_check() {
    let tracker = ActivityTracker::new();
    tracker.record();
    assert!(tracker.ms_since_last() < 1000);
}

#[test]
fn activity_tracker_clone_shares_state() {
    let tracker = ActivityTracker::new();
    let clone = tracker.clone();
    tracker.record();
    assert!(clone.ms_since_last() < 1000);
}

#[test]
fn heartbeat_once_nonexistent() {
    let ka = PcieBridgeKeepalive::new("9999:99:99.9", Duration::from_secs(5));
    assert!(!ka.heartbeat_once());
}

#[test]
fn detect_pcie_bridges_nonexistent() {
    let bridges = detect_pcie_bridges("9999:99:99.9");
    assert!(bridges.is_empty());
}

#[test]
fn is_pci_bdf_valid() {
    assert!(is_pci_bdf("0000:49:00.0"));
    assert!(is_pci_bdf("0000:4a:08.0"));
    assert!(is_pci_bdf("0001:00:00.0"));
}

#[test]
fn is_pci_bdf_rejects_domain_root() {
    assert!(!is_pci_bdf("pci0000:40"));
    assert!(!is_pci_bdf("pci0000:00"));
}

#[test]
fn is_pci_bdf_rejects_garbage() {
    assert!(!is_pci_bdf(""));
    assert!(!is_pci_bdf("not-a-bdf"));
    assert!(!is_pci_bdf("pci0000"));
}

#[test]
fn detect_plx_bridge_nonexistent() {
    assert!(detect_plx_bridge("9999:99:99.9").is_none());
}

#[test]
fn keepalive_handle_initial_state() {
    let running = Arc::new(AtomicBool::new(true));
    let heartbeats = Arc::new(AtomicU64::new(0));
    let handle = KeepaliveHandle {
        running,
        heartbeats,
        bdf: "0000:4b:00.0".into(),
    };
    assert!(handle.is_running());
    assert_eq!(handle.heartbeat_count(), 0);
    assert_eq!(handle.bdf(), "0000:4b:00.0");
}

#[test]
fn keepalive_handle_stop() {
    let running = Arc::new(AtomicBool::new(true));
    let heartbeats = Arc::new(AtomicU64::new(42));
    let handle = KeepaliveHandle {
        running,
        heartbeats,
        bdf: "0000:4b:00.0".into(),
    };
    assert!(handle.is_running());
    handle.stop();
    assert!(!handle.is_running());
    assert_eq!(handle.heartbeat_count(), 42);
}
