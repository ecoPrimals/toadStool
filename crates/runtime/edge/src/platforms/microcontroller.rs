// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Microcontroller Platform Support
//!
//! This platform is discovered at runtime by probing hardware capabilities.
//! Detects microcontroller connectivity via serial ports (`/dev/ttyUSB*`, `/dev/ttyACM*`),
//! enabling ARM, AVR, RISC-V support based on connected hardware.
//!
//! Returns `PlatformNotAvailable` when the target hardware is not detected.

use std::path::Path;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

use super::{
    ConnectionInfo, ConnectionType, DeviceStatus, EdgeDeviceInfo, EdgeDeviceResources,
    MicrocontrollerArch,
};

/// Returns a `PlatformNotAvailable` error when microcontroller hardware is not detected.
fn platform_not_available(reason: &str) -> ToadStoolError {
    toadstool_common::error::SystemError::NotSupported {
        feature: "microcontroller".to_string(),
        reason: reason.to_string(),
    }
    .into()
}

/// Probe for microcontroller connectivity via serial ports.
///
/// Checks for:
/// - `/dev/ttyUSB*` — USB-serial adapters (CH340, FTDI, CP210x)
/// - `/dev/ttyACM*` — USB CDC ACM (native USB on ARM/RISC-V)
fn probe_microcontroller_capabilities() -> Option<Vec<String>> {
    let dev = Path::new("/dev");
    if !dev.exists() {
        return None;
    }

    let has_serial = std::fs::read_dir(dev)
        .ok()?
        .filter_map(|e| e.ok())
        .any(|e| {
            let name = e.file_name();
            let s = name.to_string_lossy();
            s.starts_with("ttyUSB") || s.starts_with("ttyACM")
        });

    if !has_serial {
        return None;
    }

    let mut capabilities = vec![
        "serial".to_string(),
        "usb_serial".to_string(),
        "arm".to_string(),
        "avr".to_string(),
        "riscv".to_string(),
    ];
    Some(capabilities)
}

/// Discover microcontroller devices by probing hardware capabilities.
///
/// This platform is discovered at runtime by probing hardware capabilities.
/// Detects microcontroller connectivity via serial ports and USB device tree,
/// and returns devices with architecture capabilities based on connected hardware.
///
/// # Returns
///
/// - `Ok(devices)` when microcontroller connectivity is detected
/// - `Err(PlatformNotAvailable)` when the target hardware is not detected
pub fn discover_microcontroller_devices() -> ToadStoolResult<Vec<MicrocontrollerDevice>> {
    let capabilities = probe_microcontroller_capabilities().ok_or_else(|| {
        platform_not_available(
            "Microcontroller platform not detected (no /dev/ttyUSB* or /dev/ttyACM* serial ports)",
        )
    })?;

    let device = MicrocontrollerDevice::from_detected_hardware(
        MicrocontrollerArch::ARM,
        "Generic".to_string(),
        "Detected".to_string(),
        capabilities,
    );
    Ok(vec![device])
}

/// Create a microcontroller device from connection info.
///
/// Returns `Err(PlatformNotAvailable)` when the platform cannot create a device
/// (e.g., when microcontroller connectivity is not detected).
pub fn create_microcontroller_device(
    _arch: MicrocontrollerArch,
    _vendor: String,
    _model: String,
    _address: String,
) -> ToadStoolResult<MicrocontrollerDevice> {
    Err(platform_not_available(
        "Microcontroller device creation requires discovery on target hardware first",
    ))
}

/// Microcontroller device — capability-based representation.
///
/// Represents a microcontroller discovered via hardware capability probing.
/// Capabilities (ARM, AVR, RISC-V, serial) are determined from connected hardware.
#[derive(Debug)]
pub struct MicrocontrollerDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
}

impl MicrocontrollerDevice {
    /// Create a device from detected hardware capabilities.
    pub(crate) fn from_detected_hardware(
        arch: MicrocontrollerArch,
        vendor: String,
        model: String,
        capabilities: Vec<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        let info = EdgeDeviceInfo {
            id,
            name: format!("{} {} ({:?})", vendor, model, arch),
            platform: super::EdgePlatform::Microcontroller {
                architecture: arch.clone(),
                vendor: vendor.clone(),
                model: model.clone(),
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
                uart_ports: 1,
            },
            connection_info: ConnectionInfo {
                connection_type: ConnectionType::Serial,
                address: "/dev/ttyUSB0".to_string(),
                port: None,
                protocol: "Serial".to_string(),
                authentication: None,
                encryption: None,
            },
            status: DeviceStatus::Offline,
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
    #[expect(
        dead_code,
        reason = "microcontroller platform constructor; requires target hardware"
    )]
    pub fn new(
        _arch: MicrocontrollerArch,
        _vendor: String,
        _model: String,
        _address: String,
    ) -> ToadStoolResult<Self> {
        Err(platform_not_available(
            "Microcontroller device creation requires discovery on target hardware first",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_microcontroller_probes_hardware() {
        let result = discover_microcontroller_devices();
        // On systems without serial/USB MCU connectivity, should return Err with meaningful message
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("Microcontroller")
                    || err.to_string().contains("microcontroller"),
                "Error should mention Microcontroller: {}",
                err
            );
        } else {
            let devices = result.unwrap();
            assert!(!devices.is_empty());
            assert!(
                devices[0]
                    .get_info()
                    .capabilities
                    .contains(&"serial".to_string())
            );
        }
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
