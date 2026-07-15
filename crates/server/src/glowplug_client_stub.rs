// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-Linux stub for [`super::glowplug_client`].
//!
//! GPU sysfs discovery and driver personality swaps are Linux-only. This module
//! preserves the public API so cross-compiled builds compile; methods return
//! empty or unavailable results at runtime.

use std::sync::Arc;

use toadstool_glowplug::boot::BootResult;

pub use crate::glowplug_types::{
    DeviceSwapResult, EmberDeviceInfo, EmberDeviceListEnriched, EmberReacquireResult, EmberStatus,
    ExperimentLifecycleResult,
};

/// Stub glowPlug client for non-Linux platforms.
#[derive(Debug, Default)]
pub struct GlowPlugClient;

impl GlowPlugClient {
    /// Creates a stub client.
    pub fn new() -> Self {
        Self
    }

    /// Always false on non-Linux — hardware subsystem unavailable.
    pub fn is_available(&self) -> bool {
        false
    }

    /// Returns an empty device list.
    pub fn list_devices(&self) -> EmberDeviceListEnriched {
        EmberDeviceListEnriched {
            devices: Vec::new(),
        }
    }

    /// No devices available on non-Linux.
    pub fn get_device(&self, _bdf: &str) -> Option<EmberDeviceInfo> {
        None
    }

    /// Returns unavailable status.
    pub fn status(&self) -> EmberStatus {
        EmberStatus {
            devices: Vec::new(),
            uptime_secs: 0,
        }
    }

    /// No-op orchestrated swap on non-Linux.
    pub async fn swap_device_orchestrated(&self, bdf: &str, _target: &str) -> BootResult {
        BootResult {
            device_id: bdf.to_string(),
            initial_personality: None,
            warm_cycle_performed: false,
            final_personality: None,
            init_result: None,
            steps: Vec::new(),
            success: false,
            summary: String::from("GPU device management is not available on this platform"),
        }
    }

    /// No-op swap on non-Linux.
    pub async fn swap(&self, bdf: &str, target: &str) -> DeviceSwapResult {
        DeviceSwapResult {
            bdf: bdf.to_string(),
            target: target.to_string(),
            success: false,
            steps: Vec::new(),
        }
    }

    /// Warm detection unavailable on non-Linux.
    pub fn warm_detect(&self, bdf: &str) -> serde_json::Value {
        serde_json::json!({
            "bdf": bdf,
            "warm_detected": false,
            "platform": "unsupported",
        })
    }

    /// Experiment lifecycle unavailable on non-Linux.
    pub fn experiment_lifecycle(&self, bdf: &str, action: &str) -> ExperimentLifecycleResult {
        ExperimentLifecycleResult {
            bdf: bdf.to_string(),
            action: action.to_string(),
            success: false,
            session: None,
        }
    }

    /// Reacquire unavailable on non-Linux.
    pub async fn reacquire(&self, bdf: &str) -> EmberReacquireResult {
        EmberReacquireResult {
            bdf: bdf.to_string(),
        }
    }
}

/// Shared glowPlug service wrapped in Arc for handler use.
pub type SharedGlowPlugClient = Arc<GlowPlugClient>;

/// Create a shared stub glowPlug service instance.
pub fn create_glowplug_client() -> SharedGlowPlugClient {
    Arc::new(GlowPlugClient::new())
}

/// No GPU BDFs on non-Linux stub.
#[must_use]
pub fn discover_gpu_bdfs() -> Vec<String> {
    Vec::new()
}
