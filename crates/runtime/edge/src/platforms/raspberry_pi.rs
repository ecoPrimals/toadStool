//! # Raspberry Pi Platform Support
//!
//! Placeholder implementation for Raspberry Pi support in ToadStool Edge Runtime.
//!
//! This module provides stub discovery and factory functions that return
//! `PlatformNotAvailable` errors. Full implementation would include:
//! - SSH connections for remote Pi devices
//! - GPIO control via rppal (when `raspberry-pi` feature enabled)
//! - Linux-based execution on Pi OS
//! - Model detection (Pi 1–5, Zero, Pico, Compute Module)

use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

use super::{EdgeDeviceInfo, PiModel, PiOS};

/// Error returned when Raspberry Pi platform is not available.
fn platform_not_available() -> ToadStoolError {
    toadstool_common::error::SystemError::NotSupported {
        feature: "raspberry_pi".to_string(),
        reason: "Raspberry Pi platform not implemented. Enable 'raspberry-pi' feature for rppal GPIO support, or use LinuxEdge for generic Linux devices.".to_string(),
    }
    .into()
}

/// Discover Raspberry Pi devices on the local network or USB.
///
/// Returns `Err(PlatformNotAvailable)` since full discovery is not implemented.
pub fn discover_raspberry_pi_devices() -> ToadStoolResult<Vec<RaspberryPiDevice>> {
    Err(platform_not_available())
}

/// Create a Raspberry Pi device from connection info.
///
/// Returns `Err(PlatformNotAvailable)` since the platform stub does not support device creation.
pub fn create_raspberry_pi_device(
    _model: PiModel,
    _os: PiOS,
    _address: String,
    _port: Option<u16>,
) -> ToadStoolResult<RaspberryPiDevice> {
    Err(platform_not_available())
}

/// Placeholder Raspberry Pi device type.
///
/// Used when the platform module is loaded but no real device implementation exists.
/// All operations return `PlatformNotAvailable`.
#[derive(Debug)]
pub struct RaspberryPiDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
}

impl RaspberryPiDevice {
    /// Create a placeholder device (for type compatibility).
    /// Returns `Err` since the platform is not implemented.
    #[allow(dead_code)]
    pub fn new(_model: PiModel, _os: PiOS, _address: String) -> ToadStoolResult<Self> {
        Err(platform_not_available())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_raspberry_pi_returns_platform_not_available() {
        let result = discover_raspberry_pi_devices();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("raspberry_pi"));
    }

    #[test]
    fn test_create_raspberry_pi_device_returns_platform_not_available() {
        let result = create_raspberry_pi_device(
            PiModel::Pi4,
            PiOS::RaspberryPiOS,
            "192.168.1.100".to_string(),
            Some(22),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_raspberry_pi_device_new_returns_platform_not_available() {
        let result = RaspberryPiDevice::new(
            PiModel::Pi4,
            PiOS::RaspberryPiOS,
            "192.168.1.100".to_string(),
        );
        assert!(result.is_err());
    }
}
