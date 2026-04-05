// SPDX-License-Identifier: AGPL-3.0-or-later
//! USB VID/PID detection and ESP32 device discovery.

use toadstool::error::ToadStoolResult;

use super::super::{
    ConnectionInfo, ConnectionType, ESP32Framework, ESP32Variant,
};
use super::ESP32Device;

impl ESP32Device {
    /// Discover ESP32 devices
    pub fn discover_devices() -> ToadStoolResult<Vec<ESP32Device>> {
        let mut devices = Vec::new();

        for port in serialport::available_ports().map_err(|e| {
            toadstool::error::ToadStoolError::io(e.to_string())
        })? {
            if let serialport::SerialPortType::UsbPort(usb_info) = &port.port_type {
                if Self::is_esp32_device(usb_info.vid, usb_info.pid) {
                    let chip = Self::detect_chip_variant(usb_info.vid, usb_info.pid);
                    let device = ESP32Device::new(
                        chip,
                        ESP32Framework::ESPIDF,
                        ConnectionInfo {
                            connection_type: ConnectionType::Serial,
                            address: port.port_name.clone(),
                            port: None,
                            protocol: "Serial".to_string(),
                            authentication: None,
                            encryption: None,
                        },
                    )?;
                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }

    /// Check if device is ESP32
    fn is_esp32_device(vid: u16, pid: u16) -> bool {
        match vid {
            0x10C4 => matches!(pid, 0xEA60), // Silicon Labs CP210x
            0x1A86 => matches!(pid, 0x7523), // CH340
            0x0403 => matches!(pid, 0x6001 | 0x6010 | 0x6011), // FTDI
            0x303A => true, // Espressif
            _ => false,
        }
    }

    /// Detect ESP32 chip variant
    fn detect_chip_variant(vid: u16, pid: u16) -> ESP32Variant {
        match (vid, pid) {
            (0x303A, 0x1001) => ESP32Variant::ESP32,
            (0x303A, 0x1002) => ESP32Variant::ESP32S2,
            (0x303A, 0x1003) => ESP32Variant::ESP32S3,
            (0x303A, 0x1004) => ESP32Variant::ESP32C3,
            _ => ESP32Variant::ESP32,
        }
    }
}
