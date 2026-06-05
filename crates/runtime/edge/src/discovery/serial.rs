// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serial port discovery (Arduino, ESP32, etc.).
//!
//! Gated behind feature `serial-transport` (opt-in; not in `default`).

use std::time::Duration;

use super::DiscoveryMethod;

/// Serial Port Discovery Method
pub struct SerialPortDiscovery {
    #[expect(
        dead_code,
        reason = "stored from config; will iterate baud rates during auto-detect"
    )]
    pub(super) baud_rates: Vec<u32>,
    #[expect(dead_code, reason = "stored from config; will bound serial probe wait")]
    pub(super) timeout: Duration,
}

#[cfg(feature = "serial-transport")]
mod inner {
    use std::sync::Arc;
    use tracing::debug;

    use toadstool::error::ToadStoolError;

    use crate::platforms::*;

    use super::SerialPortDiscovery;
    use super::DiscoveryMethod;

    impl DiscoveryMethod for SerialPortDiscovery {
        fn get_name(&self) -> &str {
            "Serial Port Discovery"
        }

        fn discover(&self) -> super::super::DiscoveryFuture<'_> {
            Box::pin(async move {
                let mut devices = Vec::new();

                let ports = serialport::available_ports().map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to enumerate serial ports: {}", e))
                })?;

                for port in ports {
                    if let Some(device) = self.identify_serial_device(&port).await {
                        devices.push(device);
                    }
                }

                Ok(devices)
            })
        }

        fn is_available(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
            Box::pin(async { serialport::available_ports().is_ok() })
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
                if ArduinoDevice::is_arduino_device(usb_info.vid, usb_info.pid) {
                    let board = ArduinoDevice::detect_board_type(usb_info.vid, usb_info.pid);
                    if let Ok(device) =
                        ArduinoDevice::new(board, "1.0".to_string(), port.port_name.clone(), 9600)
                    {
                        return Some(Arc::new(device));
                    }
                }

                if self.is_esp32_device(usb_info.vid, usb_info.pid) {
                    debug!("Found ESP32 device on {}", port.port_name);
                }
            }

            None
        }

        fn is_esp32_device(&self, vid: u16, pid: u16) -> bool {
            match vid {
                0x10C4 => matches!(pid, 0xEA60),
                0x1A86 => matches!(pid, 0x7523),
                0x0403 => matches!(pid, 0x6001 | 0x6010 | 0x6011),
                _ => false,
            }
        }
    }
}

#[cfg(not(feature = "serial-transport"))]
mod feature_disabled {
    use toadstool::error::ToadStoolError;

    use crate::serial_transport::SERIAL_TRANSPORT_UNAVAILABLE;

    use super::SerialPortDiscovery;
    use super::DiscoveryMethod;

    impl DiscoveryMethod for SerialPortDiscovery {
        fn get_name(&self) -> &str {
            "Serial Port Discovery"
        }

        fn discover(&self) -> super::super::DiscoveryFuture<'_> {
            Box::pin(async move {
                Err(ToadStoolError::runtime(SERIAL_TRANSPORT_UNAVAILABLE.to_string()))
            })
        }

        fn is_available(
            &self,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + '_>> {
            Box::pin(async { false })
        }

        fn get_supported_types(&self) -> Vec<String> {
            vec![
                "Arduino".to_string(),
                "ESP32".to_string(),
                "Generic Serial".to_string(),
            ]
        }
    }
}
