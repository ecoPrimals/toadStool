// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Arduino Platform Support
//!
//! Implementation of Arduino board support for ToadStool Edge Runtime.
//! Supports various Arduino boards with serial communication and code deployment.

use async_trait::async_trait;
use serialport::{SerialPort, SerialPortType};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{ExecutionRequest, ExecutionResponse, ExecutionStatus, ExecutionOutput},
};

use super::*;

/// Arduino Device Implementation
pub struct ArduinoDevice {
    id: Uuid,
    info: EdgeDeviceInfo,
    serial_port: Arc<RwLock<Option<Box<dyn SerialPort>>>>,
    compilation_cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    active_executions: Arc<RwLock<HashMap<Uuid, ArduinoExecution>>>,
}

#[derive(Debug, Clone)]
struct ArduinoExecution {
    id: Uuid,
    status: ExecutionStatus,
    started_at: Instant,
    code_hash: String,
}

impl ArduinoDevice {
    /// Create a new Arduino device
    pub fn new(
        board: ArduinoBoard,
        version: String,
        port: String,
        baud_rate: u32,
    ) -> ToadStoolResult<Self> {
        let id = Uuid::new_v4();
        let platform = EdgePlatform::Arduino { board: board.clone(), version };
        
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
            serial_port: Arc::new(RwLock::new(None)),
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
                network_interfaces: vec![
                    NetworkInterface {
                        name: "WiFi".to_string(),
                        interface_type: NetworkInterfaceType::WiFi,
                        mac_address: None,
                        ip_address: None,
                        is_connected: false,
                        speed_mbps: Some(65),
                    },
                ],
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
    
    /// Open serial connection
    async fn open_serial_connection(&self) -> ToadStoolResult<()> {
        let mut port_guard = self.serial_port.write().await;
        
        if port_guard.is_some() {
            return Ok(());
        }
        
        let port_name = &self.info.connection_info.address;
        let baud_rate = 9600; // Default Arduino baud rate
        
        info!("Opening serial connection to Arduino on {}", port_name);
        
        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(1000))
            .open()
            .map_err(|e| ToadStoolError::connection_error(
                format!("Failed to open serial port {}: {}", port_name, e)
            ))?;
        
        *port_guard = Some(port);
        info!("Serial connection established to Arduino");
        
        Ok(())
    }
    
    /// Close serial connection
    async fn close_serial_connection(&self) -> ToadStoolResult<()> {
        let mut port_guard = self.serial_port.write().await;
        
        if let Some(mut port) = port_guard.take() {
            info!("Closing serial connection to Arduino");
            // Send any cleanup commands if needed
            let _ = port.write_all(b"RESET\n");
            let _ = port.flush();
        }
        
        Ok(())
    }
    
    /// Send command to Arduino
    async fn send_command(&self, command: &str) -> ToadStoolResult<String> {
        let mut port_guard = self.serial_port.write().await;
        
        let port = port_guard.as_mut()
            .ok_or_else(|| ToadStoolError::connection_error("Serial port not connected".to_string()))?;
        
        // Send command
        let command_bytes = format!("{}\n", command).into_bytes();
        port.write_all(&command_bytes)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to send command: {}", e)
            ))?;
        
        port.flush()
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to flush serial port: {}", e)
            ))?;
        
        // Read response
        let mut buffer = vec![0; 1024];
        let bytes_read = port.read(&mut buffer)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to read response: {}", e)
            ))?;
        
        let response = String::from_utf8_lossy(&buffer[..bytes_read]).to_string();
        debug!("Arduino response: {}", response);
        
        Ok(response)
    }

    /// Read serial output with a timeout.
    ///
    /// Collects all bytes available on the serial port within `timeout`,
    /// returning whatever the Arduino has written back.
    async fn read_serial_output(&self, timeout: Duration) -> ToadStoolResult<String> {
        let mut port_guard = self.serial_port.write().await;

        let port = port_guard.as_mut().ok_or_else(|| {
            ToadStoolError::connection_error("Serial port not connected".to_string())
        })?;

        port.set_timeout(timeout).map_err(|e| {
            ToadStoolError::execution_error(format!("Failed to set serial timeout: {e}"))
        })?;

        let mut collected = Vec::with_capacity(4096);
        let mut buf = [0u8; 1024];
        let deadline = Instant::now() + timeout;

        while Instant::now() < deadline {
            match port.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => collected.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(e) => {
                    return Err(ToadStoolError::execution_error(format!(
                        "Serial read error: {e}"
                    )));
                }
            }
        }

        Ok(String::from_utf8_lossy(&collected).into_owned())
    }

    /// Compile Arduino code
    async fn compile_code(&self, code: &str) -> ToadStoolResult<Vec<u8>> {
        let code_hash = format!("{:x}", Sha256::digest(code.as_bytes()));
        
        // Check cache first
        {
            let cache = self.compilation_cache.read().await;
            if let Some(compiled) = cache.get(&code_hash) {
                debug!("Using cached compilation for Arduino code");
                return Ok(compiled.clone());
            }
        }
        
        info!("Compiling Arduino code");
        
        // Write code to temporary file
        let temp_dir = std::env::temp_dir();
        let sketch_path = temp_dir.join(format!("arduino_sketch_{}.ino", code_hash));
        
        std::fs::write(&sketch_path, code)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to write sketch file: {}", e)
            ))?;
        
        // Compile using Arduino CLI
        let sketch_path_str = sketch_path.to_str()
            .ok_or_else(|| ToadStoolError::execution_error(
                format!("Invalid sketch path: {:?}", sketch_path)
            ))?;
        
        let output = std::process::Command::new("arduino-cli")
            .args(&[
                "compile",
                "--fqbn", "arduino:avr:uno", // Default to Uno
                sketch_path_str,
            ])
            .output()
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to run Arduino CLI: {}", e)
            ))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToadStoolError::execution_error(
                format!("Arduino compilation failed: {}", error_msg)
            ));
        }
        
        // Read compiled binary
        let hex_path = sketch_path.with_extension("hex");
        let compiled_code = std::fs::read(&hex_path)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to read compiled binary: {}", e)
            ))?;
        
        // Cache compiled code
        {
            let mut cache = self.compilation_cache.write().await;
            cache.insert(code_hash, compiled_code.clone());
        }
        
        // Clean up temporary files
        let _ = std::fs::remove_file(&sketch_path);
        let _ = std::fs::remove_file(&hex_path);
        
        info!("Arduino code compiled successfully");
        Ok(compiled_code)
    }
    
    /// Upload compiled code to Arduino
    async fn upload_code(&self, compiled_code: &[u8]) -> ToadStoolResult<()> {
        info!("Uploading code to Arduino");
        
        // Write binary to temporary file
        let temp_dir = std::env::temp_dir();
        let hex_path = temp_dir.join(format!("arduino_upload_{}.hex", Uuid::new_v4()));
        
        std::fs::write(&hex_path, compiled_code)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to write hex file: {}", e)
            ))?;
        
        // Upload using Arduino CLI
        let hex_path_str = hex_path.to_str()
            .ok_or_else(|| ToadStoolError::execution_error(
                format!("Invalid hex file path: {:?}", hex_path)
            ))?;
        
        let output = std::process::Command::new("arduino-cli")
            .args(&[
                "upload",
                "--fqbn", "arduino:avr:uno",
                "--port", &self.info.connection_info.address,
                "--input-file", hex_path_str,
            ])
            .output()
            .map_err(|e| ToadStoolError::execution_error(
                format!("Failed to run Arduino CLI upload: {}", e)
            ))?;
        
        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(ToadStoolError::execution_error(
                format!("Arduino upload failed: {}", error_msg)
            ));
        }
        
        // Clean up temporary file
        let _ = std::fs::remove_file(&hex_path);
        
        info!("Code uploaded to Arduino successfully");
        Ok(())
    }
    
    /// Discover Arduino devices
    pub fn discover_devices() -> ToadStoolResult<Vec<ArduinoDevice>> {
        let mut devices = Vec::new();
        
        for port in serialport::available_ports()? {
            if let SerialPortType::UsbPort(usb_info) = &port.port_type {
                // Check for Arduino vendor IDs
                if Self::is_arduino_device(usb_info.vid, usb_info.pid) {
                    let board = Self::detect_board_type(usb_info.vid, usb_info.pid);
                    let device = ArduinoDevice::new(
                        board,
                        "1.0".to_string(),
                        port.port_name.clone(),
                        9600,
                    )?;
                    devices.push(device);
                }
            }
        }
        
        Ok(devices)
    }
    
    /// Check if device is Arduino based on vendor/product ID
    pub fn is_arduino_device(vid: u16, pid: u16) -> bool {
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

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EdgeDevice for ArduinoDevice {
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
        self.serial_port.read().await.is_some()
    }
    
    async fn connect(&self) -> ToadStoolResult<()> {
        self.open_serial_connection().await
    }
    
    async fn disconnect(&self) -> ToadStoolResult<()> {
        self.close_serial_connection().await
    }
    
    async fn execute(&self, request: &ExecutionRequest) -> ToadStoolResult<ExecutionResponse> {
        info!("Executing code on Arduino device {}", self.id);
        
        // Extract code from request
        let code = std::str::from_utf8(&request.code)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Invalid UTF-8 in Arduino code: {}", e)
            ))?;
        
        let execution_id = Uuid::new_v4();
        let started_at = Instant::now();
        
        // Store execution info
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(execution_id, ArduinoExecution {
                id: execution_id,
                status: ExecutionStatus::Running,
                started_at,
                code_hash: format!("{:x}", Sha256::digest(code.as_bytes())),
            });
        }
        
        // Compile and upload code
        let compiled_code = self.compile_code(code).await?;
        self.upload_code(&compiled_code).await?;

        // Read serial output after upload completes.
        // Arduino boards run continuously; we collect whatever the board
        // has written back to serial since the upload finished.
        let output = match self.read_serial_output(Duration::from_secs(2)).await {
            Ok(serial_out) if !serial_out.is_empty() => serial_out,
            Ok(_) => "Deployed — no serial output within timeout".to_string(),
            Err(_) => "Deployed — serial monitor unavailable".to_string(),
        };
        
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
        let code_str = std::str::from_utf8(code)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Invalid UTF-8 in Arduino code: {}", e)
            ))?;
        
        let compiled_code = self.compile_code(code_str).await?;
        self.upload_code(&compiled_code).await?;
        
        Ok(format!("Deployed to Arduino {}", self.id))
    }
    
    async fn stop_execution(&self, execution_id: Uuid) -> ToadStoolResult<()> {
        // Send reset command to Arduino
        let _response = self.send_command("RESET").await?;
        
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
        
        // Arduino resource usage is typically minimal
        usage.insert("cpu_percent".to_string(), 50.0); // Estimated
        usage.insert("memory_bytes".to_string(), 512.0); // Estimated
        usage.insert("flash_usage_percent".to_string(), 25.0); // Estimated
        
        Ok(usage)
    }
    
    async fn upload_file(&self, _path: &str, _content: &[u8]) -> ToadStoolResult<()> {
        Err(ToadStoolError::not_supported("File upload not supported on Arduino".to_string()))
    }
    
    async fn download_file(&self, _path: &str) -> ToadStoolResult<Vec<u8>> {
        Err(ToadStoolError::not_supported("File download not supported on Arduino".to_string()))
    }
    
    async fn execute_command(&self, command: &str) -> ToadStoolResult<String> {
        self.send_command(command).await
    }
    
    async fn get_logs(&self, _lines: Option<usize>) -> ToadStoolResult<String> {
        // Read from serial monitor
        self.send_command("LOGS").await
    }
    
    async fn restart(&self) -> ToadStoolResult<()> {
        self.send_command("RESTART").await?;
        Ok(())
    }
    
    async fn update_firmware(&self, firmware: &[u8]) -> ToadStoolResult<()> {
        // For Arduino, firmware update is essentially code upload
        let firmware_str = std::str::from_utf8(firmware)
            .map_err(|e| ToadStoolError::execution_error(
                format!("Invalid UTF-8 in Arduino firmware: {}", e)
            ))?;
        
        let compiled_code = self.compile_code(firmware_str).await?;
        self.upload_code(&compiled_code).await?;
        
        Ok(())
    }
    
    async fn get_sensors(&self) -> ToadStoolResult<HashMap<String, f64>> {
        let response = self.send_command("SENSORS").await?;
        
        // Parse sensor data (assuming JSON format)
        let sensors: HashMap<String, f64> = serde_json::from_str(&response)
            .unwrap_or_else(|_| HashMap::new());
        
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