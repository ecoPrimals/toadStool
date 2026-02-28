//! # Industrial Platform Support
//!
//! Placeholder implementation for industrial control systems in ToadStool Edge Runtime.
//!
//! This module provides stub discovery and factory functions that return
//! `PlatformNotAvailable` errors. Full implementation would include:
//! - Modbus, Profibus, Profinet, EtherCAT protocol support
//! - PLC, SCADA, HMI, DCS, RTU device integration
//! - Industrial safety and real-time constraints

use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

use super::{EdgeDeviceInfo, IndustrialProtocol, IndustrialSystemType};

/// Error returned when Industrial platform is not available.
fn platform_not_available() -> ToadStoolError {
    toadstool_common::error::SystemError::NotSupported {
        feature: "industrial".to_string(),
        reason: "Industrial platform (PLC, SCADA, Modbus, etc.) not implemented. Requires industrial protocol stack integration.".to_string(),
    }
    .into()
}

/// Discover industrial devices on the network (Modbus, Profinet, etc.).
///
/// Returns `Err(PlatformNotAvailable)` since full discovery is not implemented.
pub fn discover_industrial_devices() -> ToadStoolResult<Vec<IndustrialDevice>> {
    Err(platform_not_available())
}

/// Create an industrial device from connection info.
///
/// Returns `Err(PlatformNotAvailable)` since the platform stub does not support device creation.
pub fn create_industrial_device(
    _system_type: IndustrialSystemType,
    _protocol: IndustrialProtocol,
    _address: String,
    _port: Option<u16>,
) -> ToadStoolResult<IndustrialDevice> {
    Err(platform_not_available())
}

/// Placeholder industrial device type.
///
/// Used when the platform module is loaded but no real device implementation exists.
/// All operations return `PlatformNotAvailable`.
#[derive(Debug)]
pub struct IndustrialDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
}

impl IndustrialDevice {
    /// Create a placeholder device (for type compatibility).
    /// Returns `Err` since the platform is not implemented.
    #[allow(dead_code)]
    pub fn new(
        _system_type: IndustrialSystemType,
        _protocol: IndustrialProtocol,
        _address: String,
    ) -> ToadStoolResult<Self> {
        Err(platform_not_available())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_industrial_returns_platform_not_available() {
        let result = discover_industrial_devices();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("industrial"));
    }

    #[test]
    fn test_create_industrial_device_returns_platform_not_available() {
        let result = create_industrial_device(
            IndustrialSystemType::PLC,
            IndustrialProtocol::Modbus,
            "192.168.1.10".to_string(),
            Some(502),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_industrial_device_new_returns_platform_not_available() {
        let result = IndustrialDevice::new(
            IndustrialSystemType::PLC,
            IndustrialProtocol::Modbus,
            "192.168.1.10".to_string(),
        );
        assert!(result.is_err());
    }
}
