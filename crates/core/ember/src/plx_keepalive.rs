// SPDX-License-Identifier: AGPL-3.0-or-later

//! PCIe bridge keepalive — prevents D3cold on PCIe switch fabrics.
//!
//! PCIe switches (PLX PEX 8747, AMD Matisse, Broadcom PEX, etc.) enter
//! D3cold when the kernel's runtime power management detects no PCIe
//! traffic for a sustained period. Once D3cold hits, the entire
//! downstream fabric goes dark — config space reads return `0xFFFFFFFF`
//! and the device is unrecoverable without a physical power cycle.
//!
//! [`PcieBridgeKeepalive`] (aliased as [`PlxKeepalive`] for backward
//! compatibility) prevents this by performing a lightweight config
//! space read on a timer. A single 4-byte PCI config read every few
//! seconds is enough to keep any bridge powered. The read targets the
//! PCI Vendor/Device ID register (offset 0x00) which is always safe.
//!
//! # Usage
//!
//! ```rust,no_run
//! use toadstool_ember::plx_keepalive::PcieBridgeKeepalive;
//!
//! let keepalive = PcieBridgeKeepalive::new("0000:4b:00.0", std::time::Duration::from_secs(5));
//! let handle = keepalive.spawn();
//! // ... device is safe from D3cold ...
//! handle.stop(); // stop when done
//! ```
//!
//! # History
//!
//! Originally named `PlxKeepalive` after the PLX PEX 8747 on the Tesla
//! K80 (Experiment 193/195). Generalized to any PCIe bridge topology —
//! PLX discovery is retained as a priority hint, not the identity of the
//! subsystem.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::time::MissedTickBehavior;

use crate::observation::epoch_ms;

/// Tracks real PCIe activity so the keepalive can skip redundant
/// synthetic heartbeats when the device is actively being used.
///
/// Share an `ActivityTracker` between the keepalive and any code that
/// performs PCIe operations (VFIO opens, config reads, BAR accesses).
#[derive(Debug, Clone, Default)]
pub struct ActivityTracker(Arc<AtomicU64>);

impl ActivityTracker {
    /// Create a new tracker with no recorded activity.
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }

    /// Record that real PCIe traffic just occurred.
    pub fn record(&self) {
        self.0.store(epoch_ms(), Ordering::Release);
    }

    /// Milliseconds since the last recorded activity (`u64::MAX` if none).
    #[must_use]
    pub fn ms_since_last(&self) -> u64 {
        let last = self.0.load(Ordering::Acquire);
        if last == 0 {
            return u64::MAX;
        }
        epoch_ms().saturating_sub(last)
    }
}

/// Keepalive state for a PCIe-bridged device.
///
/// Performs periodic config space reads on the device and its bridge
/// ancestry to prevent the kernel from putting the PCIe fabric into D3cold.
/// Uses `tokio::time::interval` for drift-free timing with immediate first
/// tick, and skips synthetic heartbeats when real PCIe traffic was recent.
#[derive(Debug)]
pub struct PcieBridgeKeepalive {
    /// PCI BDF of the device behind the PCIe bridge.
    bdf: String,

    /// How often to perform the keepalive read.
    interval: Duration,

    /// All BDFs in the bridge hierarchy (device + ancestors).
    /// Populated by [`detect_bridge_chain`].
    bridge_chain: Vec<String>,

    /// Optional activity tracker for backpressure.
    activity: Option<ActivityTracker>,
}

/// Backward-compatible alias. Prefer [`PcieBridgeKeepalive`].
pub type PlxKeepalive = PcieBridgeKeepalive;

/// Handle to a running keepalive task.
#[derive(Debug, Clone)]
pub struct KeepaliveHandle {
    running: Arc<AtomicBool>,
    heartbeats: Arc<AtomicU64>,
    bdf: String,
}

impl KeepaliveHandle {
    /// Stop the keepalive task.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
        tracing::info!(bdf = %self.bdf, "PCIe bridge keepalive stopped");
    }

    /// Whether the keepalive is still running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Total number of heartbeats performed since start.
    #[must_use]
    pub fn heartbeat_count(&self) -> u64 {
        self.heartbeats.load(Ordering::Relaxed)
    }

    /// The device BDF this keepalive protects.
    #[must_use]
    pub fn bdf(&self) -> &str {
        &self.bdf
    }
}

impl PcieBridgeKeepalive {
    /// Create a new keepalive for a device behind a PCIe bridge.
    ///
    /// Automatically detects the full bridge hierarchy by walking sysfs
    /// ancestry. The keepalive reads config space on the device AND every
    /// upstream bridge to prevent any level of the fabric from sleeping.
    /// PLX bridges are discovered with priority, but any PCI-to-PCI bridge
    /// in the ancestry chain is protected.
    #[must_use]
    pub fn new(bdf: &str, interval: Duration) -> Self {
        let bridge_chain = detect_bridge_chain(bdf);

        if bridge_chain.len() > 1 {
            tracing::info!(
                bdf,
                bridges = bridge_chain.len() - 1,
                interval_ms = interval.as_millis() as u64,
                "PCIe bridge keepalive created with {} bridge(s) in chain",
                bridge_chain.len() - 1,
            );
        }

        Self {
            bdf: bdf.to_string(),
            interval,
            bridge_chain,
            activity: None,
        }
    }

    /// Create with a specific interval in seconds.
    #[must_use]
    pub fn with_secs(bdf: &str, secs: u64) -> Self {
        Self::new(bdf, Duration::from_secs(secs))
    }

    /// Attach an [`ActivityTracker`] for backpressure. When real PCIe
    /// traffic is recorded via the tracker, the keepalive skips its
    /// synthetic heartbeat for that cycle.
    #[must_use]
    pub fn with_activity_tracker(mut self, tracker: ActivityTracker) -> Self {
        self.activity = Some(tracker);
        self
    }

    /// The BDFs that will be read on each heartbeat.
    #[must_use]
    pub fn bridge_chain(&self) -> &[String] {
        &self.bridge_chain
    }

    /// Whether this device has upstream bridges (i.e., actually needs keepalive).
    #[must_use]
    pub fn has_bridges(&self) -> bool {
        self.bridge_chain.len() > 1
    }

    /// Spawn the keepalive as a tokio task.
    ///
    /// Returns a [`KeepaliveHandle`] that can be used to stop the task
    /// and query heartbeat count. The task runs until stopped or the
    /// tokio runtime shuts down.
    ///
    /// Uses `tokio::time::interval` with `MissedTickBehavior::Skip` for
    /// drift-free scheduling. The first tick fires immediately — no
    /// initial delay before the first heartbeat.
    #[must_use]
    pub fn spawn(self) -> KeepaliveHandle {
        let running = Arc::new(AtomicBool::new(true));
        let heartbeats = Arc::new(AtomicU64::new(0));

        let handle = KeepaliveHandle {
            running: Arc::clone(&running),
            heartbeats: Arc::clone(&heartbeats),
            bdf: self.bdf.clone(),
        };

        let interval = self.interval;
        let chain = self.bridge_chain;
        let bdf = self.bdf;
        let activity = self.activity;

        tokio::spawn(async move {
            tracing::info!(
                bdf = %bdf,
                chain_len = chain.len(),
                interval_ms = interval.as_millis() as u64,
                activity_aware = activity.is_some(),
                "PCIe bridge keepalive started",
            );

            // Pin power on all bridges at startup
            for bridge_bdf in &chain {
                crate::sysfs::pin_power(bridge_bdf);
            }

            let mut ticker = tokio::time::interval(interval);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            while running.load(Ordering::Acquire) {
                ticker.tick().await;

                // Skip synthetic heartbeat if real traffic was recent
                if let Some(ref tracker) = activity {
                    if tracker.ms_since_last() < interval.as_millis() as u64 {
                        heartbeats.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                }

                let mut all_alive = true;

                for target_bdf in &chain {
                    let alive = config_read_heartbeat(target_bdf);
                    if !alive {
                        all_alive = false;
                        tracing::warn!(
                            bdf = %target_bdf,
                            "keepalive: config space read returned 0xFFFFFFFF (D3cold?)",
                        );
                    }
                }

                heartbeats.fetch_add(1, Ordering::Relaxed);

                if !all_alive {
                    for bridge_bdf in &chain {
                        crate::sysfs::pin_power(bridge_bdf);
                    }
                }
            }

            tracing::info!(
                bdf = %bdf,
                total_heartbeats = heartbeats.load(Ordering::Relaxed),
                "PCIe bridge keepalive task exited",
            );
        });

        handle
    }

    /// Perform a single heartbeat (non-async, for testing).
    #[must_use]
    pub fn heartbeat_once(&self) -> bool {
        let mut all_alive = true;
        for target_bdf in &self.bridge_chain {
            if !config_read_heartbeat(target_bdf) {
                all_alive = false;
            }
        }
        all_alive
    }
}

/// Read PCI config space register 0x00 (Vendor/Device ID) via sysfs.
///
/// Returns `true` if the read succeeded and didn't return `0xFFFFFFFF`
/// (which indicates the device is in D3cold or the link is down).
fn config_read_heartbeat(bdf: &str) -> bool {
    let config_path = format!("/sys/bus/pci/devices/{bdf}/config");
    let path = Path::new(&config_path);

    if !path.exists() {
        return false;
    }

    // Read 4 bytes at offset 0 (Vendor ID + Device ID)
    match std::fs::read(&config_path) {
        Ok(data) if data.len() >= 4 => {
            let val = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            val != 0xFFFF_FFFF
        }
        _ => false,
    }
}

/// Check whether a sysfs directory name looks like a PCI BDF (`DDDD:BB:DD.F`).
///
/// Rejects PCI domain roots like `pci0000:40` which contain a colon but
/// no dot, and would otherwise pollute bridge chain walks.
#[must_use]
pub fn is_pci_bdf(name: &str) -> bool {
    name.contains(':') && name.contains('.')
}

/// Walk sysfs ancestry to find all PCI bridges between a device and the root.
///
/// Returns a vec starting with the device BDF, followed by each upstream
/// bridge BDF in order (nearest first). Stops at the first non-PCI parent.
fn detect_bridge_chain(bdf: &str) -> Vec<String> {
    let mut chain = vec![bdf.to_string()];

    let device_link = Path::new("/sys/bus/pci/devices").join(bdf);
    let Ok(canonical) = std::fs::canonicalize(&device_link) else {
        return chain;
    };

    let mut current = canonical.as_path().parent();

    while let Some(parent) = current {
        let Some(name) = parent.file_name().and_then(|n| n.to_str()) else {
            break;
        };

        if !is_pci_bdf(name) {
            break;
        }

        chain.push(name.to_string());
        current = parent.parent();
    }

    chain
}

/// PLX/Broadcom vendor ID — used as a priority discovery hint.
pub const PLX_VENDOR_ID: u16 = 0x10b5;

/// Detect PCIe bridges in a device's ancestry.
///
/// Walks the sysfs hierarchy and returns all upstream bridge BDFs.
/// If a PLX/Broadcom bridge (`0x10b5`) is present, it is returned first
/// (priority hint for keepalive targeting).
#[must_use]
pub fn detect_pcie_bridges(bdf: &str) -> Vec<String> {
    let chain = detect_bridge_chain(bdf);
    let mut plx = Vec::new();
    let mut others = Vec::new();

    for bridge_bdf in chain.iter().skip(1) {
        let vendor = crate::sysfs::read_pci_id(bridge_bdf, "vendor");
        if vendor == PLX_VENDOR_ID {
            plx.push(bridge_bdf.clone());
        } else {
            others.push(bridge_bdf.clone());
        }
    }

    plx.extend(others);
    plx
}

/// Detect whether a device sits behind a PLX/Broadcom PCIe switch.
///
/// Backward-compatible convenience — returns the first PLX bridge BDF if found.
#[must_use]
pub fn detect_plx_bridge(bdf: &str) -> Option<String> {
    detect_pcie_bridges(bdf)
        .into_iter()
        .next()
        .filter(|b| {
            crate::sysfs::read_pci_id(b, "vendor") == PLX_VENDOR_ID
        })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
