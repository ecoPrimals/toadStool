// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serde types for the glowPlug / ember device management service.
//!
//! Separated from `glowplug_client.rs` to keep per-file complexity under 750 lines.
//! These are wire types used by `ember.*` and `device.*` JSON-RPC methods.

use serde::{Deserialize, Serialize};

/// Device entry returned by `ember.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberDeviceList {
    /// PCI BDF addresses of held devices.
    pub devices: Vec<String>,
}

/// Enriched device info for `ember.list` / `device.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberDeviceInfo {
    /// PCI BDF address.
    pub bdf: String,
    /// Human-readable device name, if known.
    pub name: Option<String>,
    /// PCI vendor ID.
    pub vendor_id: u16,
    /// Active personality (e.g. compute, display).
    pub personality: String,
    /// Whether the device is protected from experiment takeover.
    #[serde(default)]
    pub protected: bool,
    /// Whether VRAM is responding to probes.
    pub vram_alive: bool,
    /// Number of faulted engine domains.
    pub domains_faulted: usize,
}

/// Enriched list response from `ember.list` / `device.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberDeviceListEnriched {
    /// Held devices with enriched metadata.
    pub devices: Vec<EmberDeviceInfo>,
}

/// Status response from `ember.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberStatus {
    /// Held device BDFs.
    pub devices: Vec<String>,
    /// Daemon uptime in seconds.
    pub uptime_secs: u64,
}

/// Reacquire result from `ember.reacquire`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmberReacquireResult {
    /// BDF of the reacquired device.
    pub bdf: String,
}

/// Swap result returned by `device.swap`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSwapResult {
    /// BDF of the swapped device.
    pub bdf: String,
    /// Target personality the device was swapped to.
    pub target: String,
    /// Whether the swap succeeded.
    pub success: bool,
    /// Per-step timing from the orchestrator.
    pub steps: Vec<DeviceSwapStep>,
}

/// Active experiment session on a held device.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentSession {
    /// PCI BDF address under experiment.
    pub bdf: String,
    /// Session start time (seconds since daemon start).
    pub started_at: u64,
    /// Whether the session is still active.
    pub active: bool,
}

/// Result of `experiment.start` / `experiment.stop` lifecycle calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentLifecycleResult {
    /// PCI BDF address.
    pub bdf: String,
    /// Lifecycle action performed (`start` or `stop`).
    pub action: String,
    /// Whether the action succeeded.
    pub success: bool,
    /// Current session state, if applicable.
    pub session: Option<ExperimentSession>,
}

/// Single step in a device swap lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSwapStep {
    /// Step identifier (e.g. "detect_driver", "swap_to_vfio").
    pub name: String,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: u64,
    /// Whether this step succeeded.
    pub success: bool,
}
