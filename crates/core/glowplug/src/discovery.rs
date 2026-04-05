// SPDX-License-Identifier: AGPL-3.0-or-later

//! Device discovery — enumerate hardware by class.
//!
//! Each hardware class implements [`DeviceDiscovery`] to scan sysfs,
//! procfs, devfs, or other platform-specific sources for devices of
//! that class. Discovery is the first step before glowPlug can manage
//! a device.

use crate::device_id::DeviceId;

/// Discovers devices of a specific hardware class.
///
/// Implementations scan the system for available hardware and return
/// a list of device identities that glowPlug can then manage.
#[async_trait::async_trait]
pub trait DeviceDiscovery: Send + Sync {
    /// Error type for discovery failures.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Human-readable name for this hardware class (e.g. `"gpu"`, `"npu"`, `"usb"`).
    fn hardware_class(&self) -> &str;

    /// Scan for available devices of this class.
    ///
    /// # Errors
    ///
    /// Returns an error if the scan itself fails (e.g. sysfs unreadable).
    /// An empty list is not an error — it means no devices of this class exist.
    async fn discover(&self) -> Result<Vec<DeviceId>, Self::Error>;

    /// Whether a specific device is still present on the system.
    ///
    /// # Errors
    ///
    /// Returns an error if the check itself fails.
    async fn is_present(&self, id: &DeviceId) -> Result<bool, Self::Error>;
}
