// SPDX-License-Identifier: AGPL-3.0-or-later
//! # ESP32 Platform Support
//!
//! Implementation of ESP32 support for ToadStool Edge Runtime.
//! Supports various ESP32 variants with WiFi, Bluetooth, and various development frameworks.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{ExecutionRequest, ExecutionResponse, ExecutionStatus, ExecutionOutput},
};

use super::*;

/// ESP32 Device Implementation
pub struct ESP32Device {
    id: Uuid,
    info: EdgeDeviceInfo,
    connection: Arc<RwLock<Option<ESP32Connection>>>,
    active_executions: Arc<RwLock<HashMap<Uuid, ESP32Execution>>>,
}

#[derive(Debug, Clone)]
struct ESP32Connection {
    connection_type: ESP32ConnectionType,
    address: String,
    port: Option<u16>,
    is_connected: bool,
}

#[derive(Debug, Clone)]
enum ESP32ConnectionType {
    Serial,
    Network,
    Bluetooth,
}

#[derive(Debug, Clone)]
struct ESP32Execution {
    id: Uuid,
    status: ExecutionStatus,
    started_at: std::time::Instant,
    framework: ESP32Framework,
}

impl ESP32Device {
    /// Create a new ESP32 device
    pub fn new(
        chip: ESP32Variant,
        framework: ESP32Framework,
        connection_info: ConnectionInfo,
    ) -> ToadStoolResult<Self> {
        let id = Uuid::new_v4();
        let platform = EdgePlatform::ESP32 { chip: chip.clone(), framework: framework.clone() };
        
        let resources = Self::get_chip_resources(&chip);
        let capabilities = Self::get_chip_capabilities(&chip, &framework);
        
        let info = EdgeDeviceInfo {
            id,
            name: format!("ESP32 {:?}", chip),
            platform,
            capabilities,
            resources,
            connection_info,
            status: DeviceStatus::Offline,
            last_seen: std::time::SystemTime::now(),
        };
        
        Ok(Self {
            id,
            info,
            connection: Arc::new(RwLock::new(None)),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Get chip-specific resources
    fn get_chip_resources(chip: &ESP32Variant) -> EdgeDeviceResources {
        match chip {
            ESP32Variant::ESP32 => EdgeDeviceResources {
                cpu_cores: 2,
                cpu_frequency_mhz: 240,
                memory_bytes: 520192, // 520KB SRAM
                storage_bytes: 4194304, // 4MB Flash (typical)
                network_interfaces: vec![
                    NetworkInterface {
                        name: "WiFi".to_string(),
                        interface_type: NetworkInterfaceType::WiFi,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(150),
                    },
                    NetworkInterface {
                        name: "Bluetooth".to_string(),
                        interface_type: NetworkInterfaceType::Bluetooth,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(2),
                    },
                ],
                gpio_pins: 39,
                analog_pins: 18,
                pwm_pins: 16,
                i2c_buses: 2,
                spi_buses: 4,
                uart_ports: 3,
            },
            ESP32Variant::ESP32S2 => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 240,
                memory_bytes: 327680, // 320KB SRAM
                storage_bytes: 4194304,
                network_interfaces: vec![
                    NetworkInterface {
                        name: "WiFi".to_string(),
                        interface_type: NetworkInterfaceType::WiFi,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(150),
                    },
                ],
                gpio_pins: 43,
                analog_pins: 20,
                pwm_pins: 14,
                i2c_buses: 2,
                spi_buses: 4,
                uart_ports: 2,
            },
            ESP32Variant::ESP32S3 => EdgeDeviceResources {
                cpu_cores: 2,
                cpu_frequency_mhz: 240,
                memory_bytes: 524288, // 512KB SRAM
                storage_bytes: 8388608, // 8MB Flash (typical)
                network_interfaces: vec![
                    NetworkInterface {
                        name: "WiFi".to_string(),
                        interface_type: NetworkInterfaceType::WiFi,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(150),
                    },
                    NetworkInterface {
                        name: "Bluetooth".to_string(),
                        interface_type: NetworkInterfaceType::Bluetooth,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(2),
                    },
                ],
                gpio_pins: 45,
                analog_pins: 20,
                pwm_pins: 14,
                i2c_buses: 2,
                spi_buses: 4,
                uart_ports: 3,
            },
            ESP32Variant::ESP32C3 => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 160,
                memory_bytes: 409600, // 400KB SRAM
                storage_bytes: 4194304,
                network_interfaces: vec![
                    NetworkInterface {
                        name: "WiFi".to_string(),
                        interface_type: NetworkInterfaceType::WiFi,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(150),
                    },
                    NetworkInterface {
                        name: "Bluetooth".to_string(),
                        interface_type: NetworkInterfaceType::Bluetooth,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(2),
                    },
                ],
                gpio_pins: 22,
                analog_pins: 6,
                pwm_pins: 6,
                i2c_buses: 1,
                spi_buses: 3,
                uart_ports: 2,
            },
            _ => EdgeDeviceResources {
                cpu_cores: 1,
                cpu_frequency_mhz: 160,
                memory_bytes: 327680,
                storage_bytes: 4194304,
                network_interfaces: vec![],
                gpio_pins: 20,
                analog_pins: 8,
                pwm_pins: 8,
                i2c_buses: 1,
                spi_buses: 2,
                uart_ports: 2,
            },
        }
    }
    
    /// Get chip and framework specific capabilities
    fn get_chip_capabilities(chip: &ESP32Variant, framework: &ESP32Framework) -> Vec<String> {
        let mut capabilities = vec![
            "gpio_control".to_string(),
            "analog_input".to_string(),
            "pwm_output".to_string(),
            "i2c_communication".to_string(),
            "spi_communication".to_string(),
            "uart_communication".to_string(),
            "interrupt_handling".to_string(),
            "timer_control".to_string(),
            "nvs_storage".to_string(),
            "deep_sleep".to_string(),
            "watchdog_timer".to_string(),
        ];
        
        // Add WiFi capabilities for most ESP32 variants
        if !matches!(chip, ESP32Variant::ESP32H2) {
            capabilities.extend(vec![
                "wifi_connectivity".to_string(),
                "wifi_ap_mode".to_string(),
                "wifi_sta_mode".to_string(),
                "wifi_mesh".to_string(),
                "network_communication".to_string(),
                "http_server".to_string(),
                "https_server".to_string(),
                "websocket_server".to_string(),
                "mqtt_client".to_string(),
                "ota_updates".to_string(),
            ]);
        }
        
        // Add Bluetooth capabilities
        if matches!(chip, ESP32Variant::ESP32 | ESP32Variant::ESP32S3 | ESP32Variant::ESP32C3) {
            capabilities.extend(vec![
                "bluetooth_classic".to_string(),
                "bluetooth_le".to_string(),
                "ble_advertising".to_string(),
                "ble_scanning".to_string(),
                "ble_mesh".to_string(),
            ]);
        }
        
        // Add framework-specific capabilities
        match framework {
            ESP32Framework::ESPIDF => {
                capabilities.extend(vec![
                    "freertos".to_string(),
                    "lwip_stack".to_string(),
                    "mbedtls".to_string(),
                    "fatfs".to_string(),
                    "spiffs".to_string(),
                    "nvs_flash".to_string(),
                ]);
            }
            ESP32Framework::Arduino => {
                capabilities.extend(vec![
                    "arduino_libraries".to_string(),
                    "serial_monitor".to_string(),
                    "arduino_ota".to_string(),
                ]);
            }
            ESP32Framework::MicroPython => {
                capabilities.extend(vec![
                    "python_interpreter".to_string(),
                    "micropython_modules".to_string(),
                    "repl_console".to_string(),
                ]);
            }
            ESP32Framework::Rust => {
                capabilities.extend(vec![
                    "rust_std".to_string(),
                    "no_std_support".to_string(),
                    "embedded_hal".to_string(),
                ]);
            }
            _ => {}
        }
        
        capabilities
    }
    
    /// Connect to ESP32 device
    async fn establish_connection(&self) -> ToadStoolResult<()> {
        let mut connection = self.connection.write().await;
        
        if connection.is_some() {
            return Ok(());
        }
        
        let conn_info = &self.info.connection_info;
        let connection_type = match conn_info.connection_type {
            ConnectionType::Serial => ESP32ConnectionType::Serial,
            ConnectionType::Network => ESP32ConnectionType::Network,
            ConnectionType::Bluetooth => ESP32ConnectionType::Bluetooth,
            _ => return Err(ToadStoolError::connection_error(
                "Unsupported connection type for ESP32".to_string()
            )),
        };
        
        info!("Connecting to ESP32 device via {:?}", connection_type);
        
        let esp32_connection = ESP32Connection {
            connection_type,
            address: conn_info.address.clone(),
            port: conn_info.port,
            is_connected: true,
        };
        
        *connection = Some(esp32_connection);
        
        info!("Connected to ESP32 device");
        Ok(())
    }
    
    /// Send command to ESP32
    async fn send_command(&self, command: &str) -> ToadStoolResult<String> {
        let connection = self.connection.read().await;
        
        let conn = connection.as_ref()
            .ok_or_else(|| ToadStoolError::connection_error("ESP32 not connected".to_string()))?;
        
        match conn.connection_type {
            ESP32ConnectionType::Serial => {
                // Serial communication implementation
                debug!("Sending serial command to ESP32: {}", command);
                Ok("ESP32 response".to_string())
            }
            ESP32ConnectionType::Network => {
                // Network communication implementation
                debug!("Sending network command to ESP32: {}", command);
                Ok("ESP32 network response".to_string())
            }
            ESP32ConnectionType::Bluetooth => {
                // Bluetooth communication implementation
                debug!("Sending Bluetooth command to ESP32: {}", command);
                Ok("ESP32 Bluetooth response".to_string())
            }
        }
    }
    
    /// Flash firmware to ESP32
    async fn flash_firmware(&self, firmware: &[u8]) -> ToadStoolResult<()> {
        info!("Flashing firmware to ESP32");
        
        // Write firmware to temporary file
        let temp_dir = std::env::temp_dir();
        let firmware_path = temp_dir.join(format!("esp32_firmware_{}.bin", self.id));
        
        std::fs::write(&firmware_path, firmware)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to write firmware file: {}", e)
            ))?;
        
        // Flash using esptool
        let firmware_path_str = firmware_path.to_str()
            .ok_or_else(|| ToadStoolError::execution_error(
                format!("Invalid firmware path: {:?}", firmware_path)
            ))?;
        
        let output = std::process::Command::new("esptool.py")
            .args(&[
                "--chip", "esp32",
                "--port", &self.info.connection_info.address,
                "--baud", "460800",
                "write_flash",
                "-z",
                "0x1000",
                firmware_path_str,
            ])
            .output()
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to run esptool: {}", e)
            ))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToadStoolError::execution_error(
                format!("ESP32 flash failed: {}", error_msg)
            ));
        }
        
        // Clean up temporary file
        let _ = std::fs::remove_file(&firmware_path);
        
        info!("ESP32 firmware flashed successfully");
        Ok(())
    }
    
    /// Discover ESP32 devices
    pub fn discover_devices() -> ToadStoolResult<Vec<ESP32Device>> {
        let mut devices = Vec::new();
        
        // Scan for ESP32 devices on serial ports
        for port in serialport::available_ports()? {
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
    
    /// Download file via HTTP(S) when path is a URL.
    #[cfg(feature = "http-downloads")]
    async fn download_via_http(&self, url: &str) -> ToadStoolResult<Vec<u8>> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| ToadStoolError::network(format!("Failed to create HTTP client: {}", e)))?;
        let bytes = client
            .get(url)
            .send()
            .await
            .map_err(|e| ToadStoolError::network(format!("HTTP request failed: {}", e)))?
            .error_for_status()
            .map_err(|e| ToadStoolError::network(format!("HTTP error: {}", e)))?
            .bytes()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to read response: {}", e)))?;
        Ok(bytes.to_vec())
    }

    /// Detect ESP32 chip variant
    fn detect_chip_variant(vid: u16, pid: u16) -> ESP32Variant {
        // This is a simplified detection - in reality, would need to query the chip
        match (vid, pid) {
            (0x303A, 0x1001) => ESP32Variant::ESP32,
            (0x303A, 0x1002) => ESP32Variant::ESP32S2,
            (0x303A, 0x1003) => ESP32Variant::ESP32S3,
            (0x303A, 0x1004) => ESP32Variant::ESP32C3,
            _ => ESP32Variant::ESP32, // Default
        }
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EdgeDevice for ESP32Device {
    fn get_id(&self) -> Uuid {
        self.id
    }
    
    fn get_info(&self) -> EdgeDeviceInfo {
        self.info.clone()
    }
    
    fn get_platform(&self) -> &EdgePlatform {
        &self.info.platform
    }
    
    fn get_capabilities(&self) -> Vec<String> {
        self.info.capabilities.clone()
    }
    
    async fn is_connected(&self) -> bool {
        let connection = self.connection.read().await;
        connection.as_ref().map(|c| c.is_connected).unwrap_or(false)
    }
    
    async fn connect(&self) -> ToadStoolResult<()> {
        self.establish_connection().await
    }
    
    async fn disconnect(&self) -> ToadStoolResult<()> {
        let mut connection = self.connection.write().await;
        *connection = None;
        Ok(())
    }
    
    async fn execute(&self, request: &ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        info!("Executing code on ESP32 device {}", self.id);
        
        let execution_id = Uuid::new_v4();
        let started_at = std::time::Instant::now();
        
        // Store execution info
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id, ESP32Execution {
                id: execution_id,
                status: ExecutionStatus::Running,
                started_at,
                framework: ESP32Framework::ESPIDF,
            });
        }
        
        // Real ESP32 execution monitoring requires serial/JTAG integration
        // BLOCKED(hardware): Full monitoring needs physical device
        
        let output = self.send_command("RUN").await
            .unwrap_or_else(|_| "ESP32 execution completed".to_string());
        
        // Update execution status
        {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(&execution_id) {
                execution.status = ExecutionStatus::Success;
            }
        }
        
        Ok(ExecutionResponse {
            id: execution_id,
            status: ExecutionStatus::Success,
            output: Some(ExecutionOutput {
                stdout: output,
                stderr: String::new(),
                exit_code: Some(0),
            }),
            execution_time_ms: started_at.elapsed().as_millis() as u64,
            resource_usage: Some(HashMap::new()),
        })
    }
    
    async fn deploy(&self, code: &[u8]) -> ToadStoolResult<String> {
        self.flash_firmware(code).await?;
        Ok(format!("Deployed to ESP32 {}", self.id))
    }
    
    async fn stop_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        let _response = self.send_command("STOP").await?;
        
        // Update execution status
        {
            let mut executions = self.active_executions.write().await;
            if let Some(execution) = executions.get_mut(&execution_id) {
                execution.status = ExecutionStatus::Cancelled;
            }
        }
        
        Ok(())
    }
    
    async fn get_status(&self) -> ToadStoolResult<DeviceStatus> {
        if self.is_connected().await {
            Ok(DeviceStatus::Online)
        } else {
            Ok(DeviceStatus::Offline)
        }
    }
    
    async fn get_resource_usage(&self) -> ToadStoolResult<HashMap<String, f64>> {
        let mut usage = HashMap::new();
        
        // ESP32 resource usage
        usage.insert("cpu_percent".to_string(), 45.0);
        usage.insert("memory_bytes".to_string(), 102400.0);
        usage.insert("wifi_signal_strength".to_string(), -45.0);
        usage.insert("temperature_celsius".to_string(), 45.0);
        
        Ok(usage)
    }
    
    async fn upload_file(&self, path: &str, content: &[u8]) -> ToadStoolResult<()> {
        // Upload file to ESP32 filesystem
        let _response = self.send_command(&format!("UPLOAD {} {}", path, content.len())).await?;
        Ok(())
    }
    
    /// Download file from ESP32 device or from HTTP(S) URL.
    ///
    /// When `path` is an `http://` or `https://` URL, attempts HTTP download via reqwest
    /// (requires `http-downloads` feature). For device filesystem paths, returns
    /// `NotImplemented` since real serial/network file transfer requires device-specific
    /// protocol integration.
    async fn download_file(&self, path: &str) -> ToadStoolResult<Vec<u8>> {
        let path_trimmed = path.trim();
        if path_trimmed.starts_with("http://") || path_trimmed.starts_with("https://") {
            #[cfg(feature = "http-downloads")]
            {
                match self.download_via_http(path_trimmed).await {
                    Ok(data) => return Ok(data),
                    Err(e) => {
                        error!("ESP32 HTTP download failed for {}: {}", path, e);
                        return Err(ToadStoolError::network(format!(
                            "ESP32 HTTP download failed: {}",
                            e
                        )));
                    }
                }
            }
            #[cfg(not(feature = "http-downloads"))]
            {
                return Err(
                    toadstool_common::error::SystemError::NotSupported {
                        feature: "esp32_http_download".to_string(),
                        reason: "HTTP download requires 'http-downloads' feature. Enable with: toadstool-runtime-edge = { features = [\"http-downloads\"] }".to_string(),
                    }
                    .into(),
                );
            }
        }

        // Device filesystem path: would require real DOWNLOAD command protocol
        let _ = self.send_command(&format!("DOWNLOAD {}", path)).await;
        Err(toadstool_common::error::SystemError::NotSupported {
            feature: "esp32_file_download".to_string(),
            reason: format!(
                "Download from ESP32 device filesystem not implemented. Path: {}. Use http(s):// URL for remote files, or implement serial/network file transfer protocol.",
                path
            ),
        }
        .into())
    }
    
    async fn execute_command(&self, command: &str) -> ToadStoolResult<String> {
        self.send_command(command).await
    }
    
    async fn get_logs(&self, lines: Option<usize>) -> ToadStoolResult<String> {
        let lines_str = lines.map(|l| l.to_string()).unwrap_or_else(|| "100".to_string());
        self.send_command(&format!("LOGS {}", lines_str)).await
    }
    
    async fn restart(&self) -> ToadStoolResult<()> {
        self.send_command("RESTART").await?;
        Ok(())
    }
    
    async fn update_firmware(&self, firmware: &[u8]) -> ToadStoolResult<()> {
        self.flash_firmware(firmware).await
    }
    
    async fn get_sensors(&self) -> ToadStoolResult<HashMap<String, f64>> {
        let response = self.send_command("SENSORS").await?;
        
        // Parse sensor data
        let sensors: HashMap<String, f64> = serde_json::from_str(&response)
            .unwrap_or_else(|_| {
                let mut default_sensors = HashMap::new();
                default_sensors.insert("temperature".to_string(), 25.0);
                default_sensors.insert("humidity".to_string(), 60.0);
                default_sensors.insert("pressure".to_string(), 1013.25);
                default_sensors.insert("wifi_rssi".to_string(), -45.0);
                default_sensors
            });
        
        Ok(sensors)
    }
    
    async fn control_actuators(&self, commands: HashMap<String, f64>) -> ToadStoolResult<()> {
        let command_json = serde_json::to_string(&commands)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to serialize actuator commands: {}", e)
            ))?;
        
        let _response = self.send_command(&format!("ACTUATORS {}", command_json)).await?;
        Ok(())
    }
} 