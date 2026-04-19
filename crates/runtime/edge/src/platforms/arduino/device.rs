// SPDX-License-Identifier: AGPL-3.0-or-later
//! Arduino device model, board profiles, and USB discovery helpers.

use serialport::SerialPortType;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use toadstool::error::ToadStoolResult;
use toadstool::execution::ExecutionStatus;

use super::super::*;

/// Arduino Device Implementation
pub struct ArduinoDevice {
    pub(in crate::platforms::arduino) id: Uuid,
    pub(in crate::platforms::arduino) info: EdgeDeviceInfo,
    pub(in crate::platforms::arduino) serial_port:
        Arc<Mutex<Option<Box<dyn serialport::SerialPort>>>>,
    pub(in crate::platforms::arduino) compilation_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pub(in crate::platforms::arduino) active_executions:
        Arc<RwLock<HashMap<Uuid, ArduinoExecution>>>,
}

#[derive(Debug, Clone)]
pub(super) struct ArduinoExecution {
    pub(super) id: Uuid,
    pub(super) status: ExecutionStatus,
    pub(super) started_at: std::time::Instant,
    pub(super) code_hash: String,
}

impl ArduinoDevice {
    /// Cheap clone of shared handles for async blocks that must be `Send` (`dyn Future + Send`).
    pub(super) fn clone_handles(&self) -> Self {
        Self {
            id: self.id,
            info: self.info.clone(),
            serial_port: Arc::clone(&self.serial_port),
            compilation_cache: Arc::clone(&self.compilation_cache),
            active_executions: Arc::clone(&self.active_executions),
        }
    }

    /// Create a new Arduino device
    pub fn new(
        board: ArduinoBoard,
        version: String,
        port: String,
        _baud_rate: u32,
    ) -> ToadStoolResult<Self> {
        let id = Uuid::new_v4();
        let platform = EdgePlatform::Arduino {
            board: board.clone(),
            version,
        };

        let resources = Self::get_board_resources(&board);
        let capabilities = Self::get_board_capabilities(&board);

        let info = EdgeDeviceInfo {
            id,
            name: format!("Arduino {:?}", board),
            platform,
            capabilities,
            resources,
            connection_info: ConnectionInfo {
                connection_type: ConnectionType::Serial,
                address: port,
                port: None,
                protocol: "Serial".to_string(),
                authentication: None,
                encryption: None,
            },
            status: DeviceStatus::Offline,
            last_seen: std::time::SystemTime::now(),
        };

        Ok(Self {
            id,
            info,
            serial_port: Arc::new(Mutex::new(None)),
            compilation_cache: Arc::new(RwLock::new(HashMap::new())),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get board-specific resources
    fn get_board_resources(board: &ArduinoBoard) -> EdgeDeviceResources {
        match board {
            ArduinoBoard::Uno => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 16,
                memory_bytes: 2048,
                storage_bytes: 32768,
                network_interfaces: vec![],
                gpio_pins: 20,
                analog_pins: 6,
                pwm_pins: 6,
                i2c_buses: 1,
                spi_buses: 1,
                uart_ports: 1,
            },
            ArduinoBoard::Nano => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 16,
                memory_bytes: 2048,
                storage_bytes: 32768,
                network_interfaces: vec![],
                gpio_pins: 22,
                analog_pins: 8,
                pwm_pins: 6,
                i2c_buses: 1,
                spi_buses: 1,
                uart_ports: 1,
            },
            ArduinoBoard::Mega2560 => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 16,
                memory_bytes: 8192,
                storage_bytes: 262144,
                network_interfaces: vec![],
                gpio_pins: 70,
                analog_pins: 16,
                pwm_pins: 15,
                i2c_buses: 1,
                spi_buses: 1,
                uart_ports: 4,
            },
            ArduinoBoard::Nano33IoT => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 48,
                memory_bytes: 32768,
                storage_bytes: 262144,
                network_interfaces: vec![NetworkInterface {
                    name: "WiFi".to_string(),
                    interface_type: NetworkInterfaceType::WiFi,
                    mac_address: None,
                    ip_address: None,
                    is_connected: false,
                    speed_mbps: Some(65),
                }],
                gpio_pins: 22,
                analog_pins: 8,
                pwm_pins: 11,
                i2c_buses: 1,
                spi_buses: 1,
                uart_ports: 2,
            },
            ArduinoBoard::Due => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 84,
                memory_bytes: 98304,
                storage_bytes: 524288,
                network_interfaces: vec![],
                gpio_pins: 66,
                analog_pins: 12,
                pwm_pins: 12,
                i2c_buses: 2,
                spi_buses: 1,
                uart_ports: 4,
            },
            _ => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 16,
                memory_bytes: 2048,
                storage_bytes: 32768,
                network_interfaces: vec![],
                gpio_pins: 20,
                analog_pins: 6,
                pwm_pins: 6,
                i2c_buses: 1,
                spi_buses: 1,
                uart_ports: 1,
            },
        }
    }

    /// Get board-specific capabilities
    fn get_board_capabilities(board: &ArduinoBoard) -> Vec<String> {
        let mut capabilities = vec![
            "gpio_control".to_string(),
            "analog_input".to_string(),
            "pwm_output".to_string(),
            "i2c_communication".to_string(),
            "spi_communication".to_string(),
            "serial_communication".to_string(),
            "interrupt_handling".to_string(),
            "timer_control".to_string(),
            "eeprom_access".to_string(),
        ];

        match board {
            ArduinoBoard::Nano33IoT | ArduinoBoard::MKRWiFi1010 => {
                capabilities.extend(vec![
                    "wifi_connectivity".to_string(),
                    "network_communication".to_string(),
                    "ota_updates".to_string(),
                ]);
            }
            ArduinoBoard::Nano33BLE => {
                capabilities.extend(vec![
                    "bluetooth_connectivity".to_string(),
                    "ble_communication".to_string(),
                    "sensor_fusion".to_string(),
                ]);
            }
            ArduinoBoard::Due => {
                capabilities.extend(vec![
                    "high_performance".to_string(),
                    "floating_point".to_string(),
                    "dma_support".to_string(),
                ]);
            }
            _ => {}
        }

        capabilities
    }

    /// Discover Arduino devices
    pub fn discover_devices() -> ToadStoolResult<Vec<ArduinoDevice>> {
        let mut devices = Vec::new();

        for port in serialport::available_ports().map_err(|e| {
            toadstool::error::ToadStoolError::runtime(format!(
                "Failed to enumerate serial ports: {e}"
            ))
        })? {
            if let SerialPortType::UsbPort(usb_info) = &port.port_type {
                // Check for Arduino vendor IDs
                if Self::is_arduino_device(usb_info.vid, usb_info.pid) {
                    let board = Self::detect_board_type(usb_info.vid, usb_info.pid);
                    let device =
                        ArduinoDevice::new(board, "1.0".to_string(), port.port_name.clone(), 9600)?;
                    devices.push(device);
                }
            }
        }

        Ok(devices)
    }

    /// Check if device is Arduino based on vendor/product ID
    pub fn is_arduino_device(vid: u16, _pid: u16) -> bool {
        match vid {
            0x2341 => true, // Arduino LLC
            0x1B4F => true, // SparkFun
            0x10C4 => true, // Silicon Labs (used by some Arduino clones)
            0x0403 => true, // FTDI (used by some Arduino boards)
            _ => false,
        }
    }

    /// Detect Arduino board type based on vendor/product ID
    pub fn detect_board_type(vid: u16, pid: u16) -> ArduinoBoard {
        match (vid, pid) {
            (0x2341, 0x0043) => ArduinoBoard::Uno,
            (0x2341, 0x0001) => ArduinoBoard::Uno,
            (0x2341, 0x0010) => ArduinoBoard::Mega2560,
            (0x2341, 0x0042) => ArduinoBoard::Mega2560,
            (0x2341, 0x0036) => ArduinoBoard::Leonardo,
            (0x2341, 0x8036) => ArduinoBoard::Leonardo,
            (0x2341, 0x0037) => ArduinoBoard::Micro,
            (0x2341, 0x8037) => ArduinoBoard::Micro,
            (0x2341, 0x003E) => ArduinoBoard::Due,
            (0x2341, 0x804E) => ArduinoBoard::Nano33IoT,
            _ => ArduinoBoard::Uno, // Default to Uno
        }
    }
}
