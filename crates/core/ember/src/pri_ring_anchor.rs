// SPDX-License-Identifier: AGPL-3.0-or-later

//! PRI Ring Anchor — holds evidence of firmware-initialized PRI ring state.
//!
//! Like [`VfioAnchor`](crate::VfioAnchor) holds VFIO file descriptors to
//! prevent bus resets, `PriRingAnchor` holds evidence of the PRI ring
//! topology that firmware boot services created. This is the GPU analog of
//! UEFI's memory map — the artifact that proves what the firmware initialized.
//!
//! The anchor does not hold hardware resources directly (PRI ring stations
//! are internal GPU state, not file descriptors). Instead, it holds the
//! [`BootServiceEvidence`] captured during `exit_boot_services()`, plus
//! ongoing health status from periodic PRI ring probes.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Evidence captured when firmware exits boot services mode.
///
/// Analogous to the UEFI memory map returned by `ExitBootServices()` — this
/// is the receipt proving what the firmware initialized before handing off
/// to the sovereign runtime.
///
/// This is the ember-level copy — glowplug re-exports this via
/// `firmware::BootServiceEvidence` for the trait interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootServiceEvidence {
    /// Human-readable description of what was preserved.
    pub description: String,
    /// Engine that provided boot services.
    pub engine: String,
    /// Key-value pairs of preserved hardware state evidence.
    pub preserved_state: HashMap<String, String>,
    /// Timestamp when boot services were exited.
    pub timestamp_epoch_secs: u64,
}

impl BootServiceEvidence {
    /// Create a new evidence record with the current timestamp.
    pub fn new(engine: impl Into<String>, description: impl Into<String>) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            description: description.into(),
            engine: engine.into(),
            preserved_state: HashMap::new(),
            timestamp_epoch_secs: ts,
        }
    }

    /// Record a piece of preserved state as evidence.
    pub fn record(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.preserved_state.insert(key.into(), value.into());
    }
}

/// Health status of the PRI ring after boot services have exited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriRingHealth {
    /// PRI ring stations are intact — firmware-initialized routing is live.
    Healthy,
    /// PRI ring has faults — some stations are unreachable.
    Degraded {
        /// Number of faulted PRI domains.
        faulted_domains: u32,
    },
    /// PRI ring is completely destroyed — all stations return PRI faults.
    Destroyed,
    /// Health is unknown (hasn't been probed since boot services exited).
    Unknown,
}

/// Holds evidence of PRI ring state across driver swaps.
///
/// Created when `exit_boot_services()` captures the firmware's work.
/// Updated by periodic health probes after the swap to vfio-pci.
/// If the PRI ring degrades, the orchestrator can trigger a firmware
/// re-boot (warm cycle) to re-establish the PRI topology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriRingAnchor {
    /// PCI BDF of the device this anchor belongs to.
    pub bdf: String,
    /// Evidence captured when boot services exited.
    pub evidence: BootServiceEvidence,
    /// Current health of the PRI ring (updated by health probes).
    pub health: PriRingHealth,
    /// Number of successful health probes since the anchor was created.
    pub probe_count: u64,
    /// Number of faults observed across all probes.
    pub total_faults: u64,
}

impl PriRingAnchor {
    /// Create a new anchor from boot service evidence.
    pub fn from_evidence(bdf: impl Into<String>, evidence: BootServiceEvidence) -> Self {
        Self {
            bdf: bdf.into(),
            evidence,
            health: PriRingHealth::Unknown,
            probe_count: 0,
            total_faults: 0,
        }
    }

    /// Update the anchor's health status from a PRI ring probe.
    pub fn update_health(&mut self, health: PriRingHealth) {
        self.probe_count += 1;
        if let PriRingHealth::Degraded { faulted_domains } = health {
            self.total_faults += u64::from(faulted_domains);
        } else if health == PriRingHealth::Destroyed {
            self.total_faults += 1;
        }
        self.health = health;
    }

    /// Whether the PRI ring is healthy enough for compute dispatch.
    pub fn is_compute_ready(&self) -> bool {
        self.health == PriRingHealth::Healthy
    }

    /// Whether a firmware re-boot is recommended (PRI ring degraded or destroyed).
    pub fn needs_reboot(&self) -> bool {
        matches!(
            self.health,
            PriRingHealth::Degraded { .. } | PriRingHealth::Destroyed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_evidence() -> BootServiceEvidence {
        let mut ev = BootServiceEvidence::new("gpu-falcon", "test evidence");
        ev.record("fecs_pc", "0x00000e24");
        ev.record("gpccs_cpuctl", "0x00000010");
        ev
    }

    #[test]
    fn from_evidence_defaults_to_unknown() {
        let anchor = PriRingAnchor::from_evidence("0000:02:00.0", sample_evidence());
        assert_eq!(anchor.health, PriRingHealth::Unknown);
        assert_eq!(anchor.probe_count, 0);
        assert!(!anchor.is_compute_ready());
    }

    #[test]
    fn healthy_is_compute_ready() {
        let mut anchor = PriRingAnchor::from_evidence("0000:02:00.0", sample_evidence());
        anchor.update_health(PriRingHealth::Healthy);
        assert!(anchor.is_compute_ready());
        assert!(!anchor.needs_reboot());
        assert_eq!(anchor.probe_count, 1);
    }

    #[test]
    fn degraded_needs_reboot() {
        let mut anchor = PriRingAnchor::from_evidence("0000:02:00.0", sample_evidence());
        anchor.update_health(PriRingHealth::Degraded { faulted_domains: 3 });
        assert!(!anchor.is_compute_ready());
        assert!(anchor.needs_reboot());
        assert_eq!(anchor.total_faults, 3);
    }

    #[test]
    fn destroyed_needs_reboot() {
        let mut anchor = PriRingAnchor::from_evidence("0000:02:00.0", sample_evidence());
        anchor.update_health(PriRingHealth::Destroyed);
        assert!(anchor.needs_reboot());
    }

    #[test]
    fn serde_roundtrip() {
        let mut anchor = PriRingAnchor::from_evidence("0000:49:00.0", sample_evidence());
        anchor.update_health(PriRingHealth::Healthy);
        let json = serde_json::to_string(&anchor).unwrap();
        let back: PriRingAnchor = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bdf, "0000:49:00.0");
        assert_eq!(back.health, PriRingHealth::Healthy);
        assert_eq!(back.probe_count, 1);
    }
}
