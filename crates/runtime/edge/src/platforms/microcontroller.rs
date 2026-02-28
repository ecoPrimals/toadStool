//! # Microcontroller Platform Support
//!
//! Placeholder implementation for generic microcontroller support in ToadStool Edge Runtime.
//!
//! This module provides stub discovery and factory functions that return
//! `PlatformNotAvailable` errors. Full implementation would include:
//! - ARM, AVR, RISC-V, PIC, MSP430, and other architectures
//! - Vendor-specific toolchains (ARM GCC, AVR-GCC, etc.)
//! - Serial/JTAG/SWD debugging and flashing
//! - no_std and embedded-hal integration

use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

use super::{EdgeDeviceInfo, MicrocontrollerArch};

/// Error returned when Microcontroller platform is not available.
fn platform_not_available() -> ToadStoolError {
    toadstool_common::error::SystemError::NotSupported {
        feature: "microcontroller".to_string(),
        reason: "Generic microcontroller platform not implemented. Use Arduino or ESP32 modules for specific board support.".to_string(),
    }
    .into()
}

/// Discover microcontroller devices (serial, JTAG, etc.).
///
/// Returns `Err(PlatformNotAvailable)` since full discovery is not implemented.
pub fn discover_microcontroller_devices() -> ToadStoolResult<Vec<MicrocontrollerDevice>> {
    Err(platform_not_available())
}

/// Create a microcontroller device from connection info.
///
/// Returns `Err(PlatformNotAvailable)` since the platform stub does not support device creation.
pub fn create_microcontroller_device(
    _arch: MicrocontrollerArch,
    _vendor: String,
    _model: String,
    _address: String,
) -> ToadStoolResult<MicrocontrollerDevice> {
    Err(platform_not_available())
}

/// Placeholder microcontroller device type.
///
/// Used when the platform module is loaded but no real device implementation exists.
/// All operations return `PlatformNotAvailable`.
#[derive(Debug)]
pub struct MicrocontrollerDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
}

impl MicrocontrollerDevice {
    /// Create a placeholder device (for type compatibility).
    /// Returns `Err` since the platform is not implemented.
    #[allow(dead_code)]
    pub fn new(
        _arch: MicrocontrollerArch,
        _vendor: String,
        _model: String,
        _address: String,
    ) -> ToadStoolResult<Self> {
        Err(platform_not_available())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_microcontroller_returns_platform_not_available() {
        let result = discover_microcontroller_devices();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("microcontroller"));
    }

    #[test]
    fn test_create_microcontroller_device_returns_platform_not_available() {
        let result = create_microcontroller_device(
            MicrocontrollerArch::ARM,
            "STMicroelectronics".to_string(),
            "STM32F4".to_string(),
            "/dev/ttyUSB0".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_microcontroller_device_new_returns_platform_not_available() {
        let result = MicrocontrollerDevice::new(
            MicrocontrollerArch::RISCV,
            "SiFive".to_string(),
            "FE310".to_string(),
            "/dev/ttyUSB0".to_string(),
        );
        assert!(result.is_err());
    }
}
