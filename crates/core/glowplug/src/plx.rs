// SPDX-License-Identifier: AGPL-3.0-or-later

//! PCIe bridge health management for glowplug devices.
//!
//! Provides auto-detection of devices behind PCIe switches (PLX, AMD,
//! Broadcom, etc.) and manages keepalive tasks to prevent D3cold. This
//! module bridges ember's [`PcieBridgeKeepalive`] into glowplug's device
//! lifecycle.
//!
//! # Background
//!
//! PCIe switches enter D3cold when the kernel's runtime PM detects no
//! traffic for a sustained period. Once D3cold occurs, the entire
//! downstream fabric goes dark (config reads return `0xFFFFFFFF`), and
//! only a physical power cycle can recover the device. Originally
//! discovered on the Tesla K80's PLX PEX 8747, this applies to any
//! GPU behind a PCIe switch.

use std::collections::HashMap;
use std::time::Duration;

use toadstool_ember::plx_keepalive::{
    KeepaliveHandle, PcieBridgeKeepalive, detect_pcie_bridges, detect_plx_bridge,
};

use crate::device_id::DeviceId;

/// Default keepalive interval: 5 seconds.
///
/// A single PCI config space read is ~4 bytes / 200ns on the bus.
/// 5s is conservative enough to avoid measurable overhead while being
/// far shorter than the kernel's runtime PM autosuspend timeout (usually
/// 10+ seconds for PCI bridges).
const DEFAULT_KEEPALIVE_INTERVAL_SECS: u64 = 5;

/// Manages PCIe bridge keepalive tasks for a set of PCI devices.
///
/// Call [`BridgeGuardian::scan_and_protect`] after device discovery to
/// automatically detect bridge-attached devices and start keepalive tasks.
/// PLX bridges are detected with priority, but any PCIe bridge in the
/// device ancestry is protected.
#[derive(Debug)]
pub struct BridgeGuardian {
    handles: HashMap<String, KeepaliveHandle>,
    interval: Duration,
}

/// Backward-compatible alias. Prefer [`BridgeGuardian`].
pub type PlxGuardian = BridgeGuardian;

impl BridgeGuardian {
    /// Create a new guardian with the default keepalive interval.
    #[must_use]
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
            interval: Duration::from_secs(DEFAULT_KEEPALIVE_INTERVAL_SECS),
        }
    }

    /// Create a new guardian with a custom keepalive interval.
    #[must_use]
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            handles: HashMap::new(),
            interval,
        }
    }

    /// Scan a list of discovered devices and start keepalive tasks for
    /// any that sit behind PCIe bridges.
    ///
    /// Returns the number of new keepalive tasks started.
    pub fn scan_and_protect(&mut self, devices: &[DeviceId]) -> usize {
        let mut started = 0;

        for device in devices {
            let DeviceId::PciBdf(bdf) = device else {
                continue;
            };

            if self.handles.contains_key(bdf.as_str()) {
                continue;
            }

            let bridges = detect_pcie_bridges(bdf);
            if !bridges.is_empty() {
                let plx_hint = detect_plx_bridge(bdf);
                tracing::info!(
                    device = bdf.as_str(),
                    bridge_count = bridges.len(),
                    plx_bridge = ?plx_hint,
                    "PCIe bridge(s) detected — starting keepalive",
                );

                let keepalive = PcieBridgeKeepalive::new(bdf, self.interval);
                let handle = keepalive.spawn();
                self.handles.insert(bdf.clone(), handle);
                started += 1;
            }
        }

        started
    }

    /// Protect a single PCI device if it's behind a PCIe bridge.
    ///
    /// Returns `true` if a keepalive was started.
    pub fn protect(&mut self, bdf: &str) -> bool {
        if self.handles.contains_key(bdf) {
            return false;
        }

        if detect_pcie_bridges(bdf).is_empty() {
            false
        } else {
            let keepalive = PcieBridgeKeepalive::new(bdf, self.interval);
            let handle = keepalive.spawn();
            self.handles.insert(bdf.to_string(), handle);
            true
        }
    }

    /// Stop keepalive for a specific device.
    pub fn release(&mut self, bdf: &str) {
        if let Some(handle) = self.handles.remove(bdf) {
            handle.stop();
        }
    }

    /// Stop all keepalive tasks.
    pub fn release_all(&mut self) {
        for (_, handle) in self.handles.drain() {
            handle.stop();
        }
    }

    /// Whether a specific device is currently protected.
    #[must_use]
    pub fn is_protected(&self, bdf: &str) -> bool {
        self.handles
            .get(bdf)
            .is_some_and(KeepaliveHandle::is_running)
    }

    /// Number of active keepalive tasks.
    #[must_use]
    pub fn active_count(&self) -> usize {
        self.handles
            .values()
            .filter(|h| h.is_running())
            .count()
    }

    /// Get the keepalive handle for a specific device.
    #[must_use]
    pub fn handle(&self, bdf: &str) -> Option<&KeepaliveHandle> {
        self.handles.get(bdf)
    }

    /// Summary of all protected devices and their heartbeat counts.
    #[must_use]
    pub fn status_summary(&self) -> Vec<BridgeDeviceStatus> {
        self.handles
            .iter()
            .map(|(bdf, handle)| BridgeDeviceStatus {
                bdf: bdf.clone(),
                running: handle.is_running(),
                heartbeats: handle.heartbeat_count(),
            })
            .collect()
    }
}

impl Default for BridgeGuardian {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of a single bridge-protected device.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BridgeDeviceStatus {
    /// PCI BDF of the protected device.
    pub bdf: String,
    /// Whether the keepalive task is still running.
    pub running: bool,
    /// Total heartbeats performed.
    pub heartbeats: u64,
}

/// Backward-compatible alias. Prefer [`BridgeDeviceStatus`].
pub type PlxDeviceStatus = BridgeDeviceStatus;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guardian_new_is_empty() {
        let g = BridgeGuardian::new();
        assert_eq!(g.active_count(), 0);
        assert!(g.status_summary().is_empty());
    }

    #[test]
    fn guardian_default_same_as_new() {
        let g = BridgeGuardian::default();
        assert_eq!(g.active_count(), 0);
    }

    #[test]
    fn guardian_with_custom_interval() {
        let g = BridgeGuardian::with_interval(Duration::from_secs(10));
        assert_eq!(g.interval, Duration::from_secs(10));
    }

    #[test]
    fn scan_non_pci_devices_ignored() {
        let mut g = BridgeGuardian::new();
        let devices = vec![
            DeviceId::UsbPath("1-2".into()),
            DeviceId::Serial("SN123".into()),
            DeviceId::Platform("test".into()),
        ];
        let started = g.scan_and_protect(&devices);
        assert_eq!(started, 0);
    }

    #[test]
    fn protect_nonexistent_device_returns_false() {
        let mut g = BridgeGuardian::new();
        assert!(!g.protect("9999:99:99.9"));
    }

    #[test]
    fn is_protected_nonexistent() {
        let g = BridgeGuardian::new();
        assert!(!g.is_protected("9999:99:99.9"));
    }

    #[test]
    fn release_nonexistent_is_noop() {
        let mut g = BridgeGuardian::new();
        g.release("9999:99:99.9");
    }

    #[test]
    fn release_all_on_empty_is_noop() {
        let mut g = BridgeGuardian::new();
        g.release_all();
    }

    #[test]
    fn bridge_device_status_serde_roundtrip() {
        let status = BridgeDeviceStatus {
            bdf: "0000:4b:00.0".into(),
            running: true,
            heartbeats: 42,
        };
        let json = serde_json::to_string(&status).unwrap();
        let back: BridgeDeviceStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bdf, "0000:4b:00.0");
        assert!(back.running);
        assert_eq!(back.heartbeats, 42);
    }

    #[test]
    fn plx_alias_works() {
        let _g: PlxGuardian = BridgeGuardian::new();
    }
}
