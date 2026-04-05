// SPDX-License-Identifier: AGPL-3.0-or-later
//! Device listing and lookup.

use crate::types::{DeviceId, UniversalComputeDevice};

use super::UniversalGpuEngine;

impl UniversalGpuEngine {
    /// Get list of available devices
    pub async fn get_available_devices(&self) -> Vec<UniversalComputeDevice> {
        self.devices.read().await.values().cloned().collect()
    }

    /// Get specific device by ID
    pub async fn get_device(&self, device_id: &DeviceId) -> Option<UniversalComputeDevice> {
        self.devices.read().await.get(device_id).cloned()
    }
}
