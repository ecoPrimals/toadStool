// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Raspberry Pi Platform Support
//!
//! This platform is discovered at runtime by probing hardware capabilities.
//! Detects Raspberry Pi hardware via `/proc/device-tree/model` and enables
//! GPIO, camera, and compute capabilities based on the detected model.
//!
//! Returns `PlatformNotAvailable` when the target hardware is not detected.

use std::path::Path;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use uuid::Uuid;

use super::{
    ConnectionInfo, ConnectionType, DeviceStatus, EdgeDeviceInfo, EdgeDeviceResources,
    NetworkInterface, NetworkInterfaceType, PiModel, PiOS,
};

/// Returns a `PlatformNotAvailable` error when Raspberry Pi hardware is not detected.
fn platform_not_available(reason: &str) -> ToadStoolError {
    toadstool_common::error::SystemError::NotSupported {
        feature: "raspberry_pi".to_string(),
        reason: reason.to_string(),
    }
    .into()
}

/// Discover Raspberry Pi devices by probing hardware capabilities.
///
/// This platform is discovered at runtime by probing hardware capabilities.
/// Detects Raspberry Pi via `/proc/device-tree/model` and returns devices with
/// capabilities (GPIO, camera, compute) based on the detected model.
///
/// # Returns
///
/// - `Ok(devices)` when Raspberry Pi hardware is detected
/// - `Err(PlatformNotAvailable)` when the target hardware is not detected
pub fn discover_raspberry_pi_devices() -> ToadStoolResult<Vec<RaspberryPiDevice>> {
    let model_path = Path::new("/proc/device-tree/model");
    if !model_path.exists() {
        return Err(platform_not_available(
            "Raspberry Pi hardware not detected (no /proc/device-tree/model)",
        ));
    }

    let model_str = std::fs::read_to_string(model_path)
        .map_err(|e| {
            toadstool_common::error::SystemError::Io {
                reason: format!("Failed to read device tree model: {}", e),
            }
            .into()
        })?
        .trim_end_matches('\0')
        .to_string();

    if !model_str.contains("Raspberry Pi") {
        return Err(platform_not_available(
            format!(
                "Raspberry Pi hardware not detected (model: '{}')",
                model_str
            )
            .as_str(),
        ));
    }

    let (model, capabilities) = parse_model_and_capabilities(&model_str);
    let device = RaspberryPiDevice::from_detected_hardware(model, capabilities);
    Ok(vec![device])
}

fn parse_model_and_capabilities(model_str: &str) -> (PiModel, Vec<String>) {
    let model = if model_str.contains("Pi 5") {
        PiModel::Pi5
    } else if model_str.contains("Pi 4") {
        PiModel::Pi4
    } else if model_str.contains("Pi Zero 2") || model_str.contains("Zero 2") {
        PiModel::PiZero2W
    } else if model_str.contains("Pi Zero") {
        PiModel::PiZero
    } else if model_str.contains("Pi 3") {
        PiModel::Pi3
    } else if model_str.contains("Pi 2") {
        PiModel::Pi2
    } else if model_str.contains("Compute Module 4") || model_str.contains("CM4") {
        PiModel::Compute4
    } else if model_str.contains("Compute Module 3") || model_str.contains("CM3") {
        PiModel::Compute3
    } else if model_str.contains("Pi Pico W") {
        PiModel::PiPicoW
    } else if model_str.contains("Pi Pico") {
        PiModel::PiPico
    } else {
        PiModel::Pi4
    };

    let mut capabilities = vec![
        "gpio".to_string(),
        "compute".to_string(),
        "i2c".to_string(),
        "spi".to_string(),
        "uart".to_string(),
    ];
    if matches!(
        model,
        PiModel::Pi3 | PiModel::Pi4 | PiModel::Pi5 | PiModel::PiZero2W
    ) {
        capabilities.push("wifi".to_string());
        capabilities.push("bluetooth".to_string());
    }
    if matches!(model, PiModel::Pi4 | PiModel::Pi5) {
        capabilities.push("camera".to_string());
    }

    (model, capabilities)
}

/// Create a Raspberry Pi device from connection info.
///
/// Returns `Err(PlatformNotAvailable)` when the platform cannot create a device
/// (e.g., when not running on Raspberry Pi hardware).
pub fn create_raspberry_pi_device(
    _model: PiModel,
    _os: PiOS,
    _address: String,
    _port: Option<u16>,
) -> ToadStoolResult<RaspberryPiDevice> {
    Err(platform_not_available(
        "Raspberry Pi device creation requires discovery on target hardware first",
    ))
}

/// Raspberry Pi device — capability-based representation.
///
/// Represents a Raspberry Pi discovered via hardware capability probing.
/// Capabilities (GPIO, camera, compute) are determined from the detected model.
#[derive(Debug)]
pub struct RaspberryPiDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
}

impl RaspberryPiDevice {
    /// Create a device from detected hardware capabilities.
    pub(crate) fn from_detected_hardware(model: PiModel, capabilities: Vec<String>) -> Self {
        let id = Uuid::new_v4();
        let resources = Self::model_resources(&model);
        let info = EdgeDeviceInfo {
            id,
            name: format!("Raspberry Pi {:?}", model),
            platform: super::EdgePlatform::RaspberryPi {
                model: model.clone(),
                os: PiOS::RaspberryPiOS,
            },
            capabilities,
            resources,
            connection_info: ConnectionInfo {
                connection_type: ConnectionType::Network,
                address: toadstool_common::constants::network::DEFAULT_HOSTNAME.to_string(),
                port: None,
                protocol: "local".to_string(),
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

    fn model_resources(model: &PiModel) -> EdgeDeviceResources {
        let (cores, memory_mb, gpio) = match model {
            PiModel::Pi5 => (4, 4096, 40),
            PiModel::Pi4 => (4, 2048, 40),
            PiModel::Pi3 | PiModel::PiZero2W => (4, 1024, 40),
            PiModel::Pi2 => (4, 1024, 40),
            PiModel::Pi1 | PiModel::PiZero => (1, 512, 40),
            PiModel::Compute3 | PiModel::Compute4 => (4, 1024, 40),
            PiModel::PiPico | PiModel::PiPicoW => (1, 0, 26),
        };
        EdgeDeviceResources {
            cpu_cores: cores,
            cpu_frequency_mhz: 1500,
            memory_bytes: memory_mb * 1024 * 1024,
            storage_bytes: 0,
            network_interfaces: vec![],
            gpio_pins: gpio,
            analog_pins: 0,
            pwm_pins: 2,
            i2c_buses: 2,
            spi_buses: 2,
            uart_ports: 2,
        }
    }

    /// Create a device from connection parameters (for remote Pi).
    /// Returns `Err` when the platform cannot create a device.
    #[expect(
        dead_code,
        reason = "Raspberry Pi constructor; requires discovery on target hardware"
    )]
    pub fn new(_model: PiModel, _os: PiOS, _address: String) -> ToadStoolResult<Self> {
        Err(platform_not_available(
            "Raspberry Pi device creation requires discovery on target hardware first",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discover_raspberry_pi_probes_hardware() {
        let result = discover_raspberry_pi_devices();
        // On non-Pi systems, should return Err with meaningful message
        if result.is_err() {
            let err = result.unwrap_err();
            assert!(
                err.to_string().contains("Raspberry Pi")
                    || err.to_string().contains("raspberry_pi"),
                "Error should mention Raspberry Pi: {}",
                err
            );
        } else {
            let devices = result.unwrap();
            assert!(!devices.is_empty());
            assert!(
                devices[0]
                    .get_info()
                    .capabilities
                    .contains(&"gpio".to_string())
            );
        }
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

    #[test]
    fn test_parse_model_and_capabilities() {
        let (model, caps) = parse_model_and_capabilities("Raspberry Pi 4 Model B");
        assert!(matches!(model, PiModel::Pi4));
        assert!(caps.contains(&"gpio".to_string()));
        assert!(caps.contains(&"camera".to_string()));
    }
}
