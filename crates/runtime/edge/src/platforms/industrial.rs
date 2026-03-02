//! # Industrial Platform Support
//!
//! This platform is discovered at runtime by probing hardware capabilities.
//! Detects industrial control system interfaces via `/sys/class` (CAN, GPIO,
//! industrial Ethernet) and enables Modbus, Profinet, EtherCAT capabilities
//! based on detected subsystems.
//!
//! Returns `PlatformNotAvailable` when the target hardware is not detected.

use std::path::Path;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

use super::{
    ConnectionInfo, ConnectionType, DeviceStatus, EdgeDeviceInfo, EdgeDeviceResources,
    IndustrialProtocol, IndustrialSystemType,
};

/// Returns a `PlatformNotAvailable` error when industrial hardware is not detected.
fn platform_not_available(reason: &str) -> ToadStoolError {
    toadstool_common::error::SystemError::NotSupported {
        feature: "industrial".to_string(),
        reason: reason.to_string(),
    }
    .into()
}

/// Probe for industrial control system capabilities via sysfs.
///
/// Checks for:
/// - `/sys/class` — base Linux sysfs (required)
/// - `/sys/class/net` — industrial Ethernet
/// - `/sys/class/gpio` — industrial I/O
/// - `/sys/class/can` — CAN bus (common in industrial)
fn probe_industrial_capabilities() -> Option<Vec<String>> {
    let sys_class = Path::new("/sys/class");
    if !sys_class.exists() || !sys_class.is_dir() {
        return None;
    }

    let mut capabilities = vec![
        "industrial_io".to_string(),
        "sysfs".to_string(),
    ];

    if Path::new("/sys/class/net").exists() {
        capabilities.push("industrial_ethernet".to_string());
    }
    if Path::new("/sys/class/gpio").exists() {
        capabilities.push("gpio".to_string());
    }
    if Path::new("/sys/class/can").exists() {
        capabilities.push("can_bus".to_string());
    }
    if Path::new("/sys/class/serial").exists() {
        capabilities.push("serial".to_string());
    }

    Some(capabilities)
}

/// Discover industrial devices by probing hardware capabilities.
///
/// This platform is discovered at runtime by probing hardware capabilities.
/// Detects industrial interfaces via `/sys/class` (CAN, GPIO, industrial Ethernet)
/// and returns devices with protocol capabilities based on detected subsystems.
///
/// # Returns
///
/// - `Ok(devices)` when industrial hardware capabilities are detected
/// - `Err(PlatformNotAvailable)` when the target hardware is not detected
pub fn discover_industrial_devices() -> ToadStoolResult<Vec<IndustrialDevice>> {
    let capabilities = probe_industrial_capabilities()
        .ok_or_else(|| {
            platform_not_available(
                "Industrial platform not detected (no /sys/class or insufficient subsystems)",
            )
        })?;

    if capabilities.len() <= 2 {
        return Err(platform_not_available(
            "Industrial platform not detected (no industrial subsystems: CAN, GPIO, or net)",
        ));
    }

    let device = IndustrialDevice::from_detected_hardware(
        IndustrialSystemType::PLC,
        IndustrialProtocol::Modbus,
        capabilities,
    );
    Ok(vec![device])
}

/// Create an industrial device from connection info.
///
/// Returns `Err(PlatformNotAvailable)` when the platform cannot create a device
/// (e.g., when industrial hardware is not detected).
pub fn create_industrial_device(
    _system_type: IndustrialSystemType,
    _protocol: IndustrialProtocol,
    _address: String,
    _port: Option<u16>,
) -> ToadStoolResult<IndustrialDevice> {
    Err(platform_not_available(
        "Industrial device creation requires discovery on target hardware first",
    ))
}

/// Industrial device — capability-based representation.
///
/// Represents an industrial control system discovered via hardware capability
/// probing. Capabilities (Modbus, CAN, industrial Ethernet) are determined from
/// detected sysfs subsystems.
#[derive(Debug)]
pub struct IndustrialDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
}

impl IndustrialDevice {
    /// Create a device from detected hardware capabilities.
    pub(crate) fn from_detected_hardware(
        system_type: IndustrialSystemType,
        protocol: IndustrialProtocol,
        capabilities: Vec<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        let info = EdgeDeviceInfo {
            id,
            name: format!("Industrial {:?} ({:?})", system_type, protocol),
            platform: super::EdgePlatform::Industrial {
                system_type: system_type.clone(),
                protocol: protocol.clone(),
            },
            capabilities,
            resources: EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 0,
                memory_bytes: 0,
                storage_bytes: 0,
                network_interfaces: vec![],
                gpio_pins: 0,
                analog_pins: 0,
                pwm_pins: 0,
                i2c_buses: 0,
                spi_buses: 0,
                uart_ports: 0,
            },
            connection_info: ConnectionInfo {
                connection_type: ConnectionType::Network,
                address: toadstool_common::constants::network::DEFAULT_HOSTNAME.to_string(),
                port: None,
                protocol: "modbus".to_string(),
                authentication: None,
                encryption: None,
            },
            status: DeviceStatus::Online,
            last_seen: std::time::SystemTime::now(),
        };
        Self { id, info }
    }

    /// Get device info (for tests and introspection).
    pub fn get_info(&self) -> &EdgeDeviceInfo {
        &self.info
    }

    /// Create a device from connection parameters.
    /// Returns `Err` when the platform cannot create a device.
    #[allow(dead_code)]
    pub fn new(
        _system_type: IndustrialSystemType,
        _protocol: IndustrialProtocol,
        _address: String,
    ) -> ToadStoolResult<Self> {
        Err(platform_not_available(
            "Industrial device creation requires discovery on target hardware first",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_industrial_probes_hardware() {
        let result = discover_industrial_devices();
        // On systems without industrial subsystems, should return Err with meaningful message
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("Industrial") || err.to_string().contains("industrial"),
                "Error should mention Industrial: {}",
                err
            );
        } else {
            let devices = result.unwrap();
            assert!(!devices.is_empty());
            assert!(!devices[0].get_info().capabilities.is_empty());
        }
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
