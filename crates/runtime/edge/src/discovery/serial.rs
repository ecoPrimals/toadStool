// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serial port discovery (Arduino, ESP32, etc.).

use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::platforms::*;

use super::DiscoveryMethod;

/// Serial Port Discovery Method
pub struct SerialPortDiscovery {
    pub(super) baud_rates: Vec<u32>,
    pub(super) timeout: Duration,
}

#[async_trait::async_trait]
impl DiscoveryMethod for SerialPortDiscovery {
    fn get_name(&self) -> &str {
        "Serial Port Discovery"
    }

    async fn discover(&self) -> ToadStoolResult<Vec<Arc<dyn EdgeDevice>>> {
        let mut devices = Vec::new();

        // Get available serial ports
        let ports = serialport::available_ports().map_err(|e| {
            ToadStoolError::discovery_error(format!("Failed to enumerate serial ports: {}", e))
        })?;

        for port in ports {
            // Try to identify device type
            if let Some(device) = self.identify_serial_device(&port).await {
                devices.push(device);
            }
        }

        Ok(devices)
    }

    async fn is_available(&self) -> bool {
        serialport::available_ports().is_ok()
    }

    fn get_supported_types(&self) -> Vec<String> {
        vec![
            "Arduino".to_string(),
            "ESP32".to_string(),
            "Generic Serial".to_string(),
        ]
    }
}

impl SerialPortDiscovery {
    async fn identify_serial_device(
        &self,
        port: &serialport::SerialPortInfo,
    ) -> Option<Arc<dyn EdgeDevice>> {
        if let serialport::SerialPortType::UsbPort(usb_info) = &port.port_type {
            // Check for Arduino devices
            if ArduinoDevice::is_arduino_device(usb_info.vid, usb_info.pid) {
                let board = ArduinoDevice::detect_board_type(usb_info.vid, usb_info.pid);
                if let Ok(device) = ArduinoDevice::new(
                    board,
                    "1.0".to_string(),
                    port.port_name.clone(),
                    9600,
                ) {
                    return Some(Arc::new(device));
                }
            }

            // Check for ESP32 devices
            if self.is_esp32_device(usb_info.vid, usb_info.pid) {
                // Create ESP32 device (implementation needed)
                // For now, we'll skip ESP32 creation
                debug!("Found ESP32 device on {}", port.port_name);
            }
        }

        None
    }

    fn is_esp32_device(&self, vid: u16, pid: u16) -> bool {
        match vid {
            0x10C4 => matches!(pid, 0xEA60), // Silicon Labs CP210x
            0x1A86 => matches!(pid, 0x7523), // CH340
            0x0403 => matches!(pid, 0x6001 | 0x6010 | 0x6011), // FTDI
            _ => false,
        }
    }
}
