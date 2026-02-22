//! # Edge Platform Implementations
//!
//! Platform-specific adapters for various edge computing devices and IoT platforms.

pub mod arduino;
pub mod esp32;
pub mod linux_edge;
pub mod raspberry_pi;
pub mod industrial;
pub mod microcontroller;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use toadstool::{
    error::{ToadStoolError, ToadStoolResult},
    execution::{ExecutionRequest, ExecutionResponse},
};

pub use arduino::*;
pub use esp32::*;
pub use linux_edge::*;
pub use raspberry_pi::*;
pub use industrial::*;
pub use microcontroller::*;

/// Edge Platform Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgePlatform {
    /// Arduino boards
    Arduino {
        board: ArduinoBoard,
        version: String,
    },
    /// ESP32 variants
    ESP32 {
        chip: ESP32Variant,
        framework: ESP32Framework,
    },
    /// Raspberry Pi models
    RaspberryPi {
        model: PiModel,
        os: PiOS,
    },
    /// BeagleBone variants
    BeagleBone {
        variant: BeagleBoneVariant,
    },
    /// Industrial control systems
    Industrial {
        system_type: IndustrialSystemType,
        protocol: IndustrialProtocol,
    },
    /// Generic microcontrollers
    Microcontroller {
        architecture: MicrocontrollerArch,
        vendor: String,
        model: String,
    },
    /// Generic Linux-based edge devices
    LinuxEdge {
        architecture: String,
        kernel_version: String,
    },
}

/// Arduino Board Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ArduinoBoard {
    Uno,
    Nano,
    Mega2560,
    Leonardo,
    Micro,
    Due,
    MKR1000,
    MKRZero,
    Portenta,
    Nano33IoT,
    Nano33BLE,
    MKRWiFi1010,
}

/// ESP32 Variants
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ESP32Variant {
    ESP32,
    ESP32S2,
    ESP32S3,
    ESP32C3,
    ESP32C6,
    ESP32H2,
    ESP32P4,
}

/// ESP32 Development Frameworks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ESP32Framework {
    ESPIDF,
    Arduino,
    PlatformIO,
    MicroPython,
    Rust,
}

/// Raspberry Pi Models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PiModel {
    Pi1,
    Pi2,
    Pi3,
    Pi4,
    Pi5,
    PiZero,
    PiZero2W,
    PiPico,
    PiPicoW,
    Compute3,
    Compute4,
}

/// Raspberry Pi Operating Systems
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PiOS {
    RaspberryPiOS,
    Ubuntu,
    BuildRoot,
    Yocto,
    CustomLinux,
}

/// BeagleBone Variants
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BeagleBoneVariant {
    Black,
    Green,
    Blue,
    AI,
    X15,
}

/// Industrial System Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndustrialSystemType {
    PLC,
    SCADA,
    HMI,
    DCS,
    RTU,
    IED,
}

/// Industrial Communication Protocols
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IndustrialProtocol {
    Modbus,
    Profibus,
    Profinet,
    EtherCAT,
    DeviceNet,
    CANopen,
    EtherNetIP,
    Foundation,
    Hart,
}

/// Microcontroller Architectures
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MicrocontrollerArch {
    ARM,
    AVR,
    PIC,
    MSP430,
    RISCV,
    x86,
    Z80,
    M68K,
    PowerPC,
}

/// Device Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDeviceInfo {
    pub id: Uuid,
    pub name: String,
    pub platform: EdgePlatform,
    pub capabilities: Vec<String>,
    pub resources: EdgeDeviceResources,
    pub connection_info: ConnectionInfo,
    pub status: DeviceStatus,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Device Resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDeviceResources {
    pub cpu_cores: u32,
    pub cpu_frequency_mhz: u32,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_interfaces: Vec<NetworkInterface>,
    pub gpio_pins: u32,
    pub analog_pins: u32,
    pub pwm_pins: u32,
    pub i2c_buses: u32,
    pub spi_buses: u32,
    pub uart_ports: u32,
}

/// Network Interface Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub interface_type: NetworkInterfaceType,
    pub mac_address: Option<String>,
    pub ip_address: Option<String>,
    pub is_connected: bool,
    pub speed_mbps: Option<u32>,
}

/// Network Interface Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkInterfaceType {
    Ethernet,
    WiFi,
    Bluetooth,
    LoRa,
    Zigbee,
    CAN,
    Serial,
    USB,
}

/// Device Connection Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub connection_type: ConnectionType,
    pub address: String,
    pub port: Option<u16>,
    pub protocol: String,
    pub authentication: Option<AuthenticationInfo>,
    pub encryption: Option<EncryptionInfo>,
}

/// Connection Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Serial,
    Network,
    USB,
    Bluetooth,
    WiFi,
    LoRa,
    CAN,
}

/// Authentication Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationInfo {
    pub method: AuthenticationMethod,
    pub username: Option<String>,
    pub key_path: Option<String>,
    pub certificate_path: Option<String>,
}

/// Authentication Methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthenticationMethod {
    None,
    Password,
    Key,
    Certificate,
    Token,
}

/// Encryption Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionInfo {
    pub algorithm: EncryptionAlgorithm,
    pub key_size: u32,
    pub mode: EncryptionMode,
}

/// Encryption Algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    None,
    AES,
    ChaCha20,
    RSA,
    ECC,
}

/// Encryption Modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionMode {
    None,
    GCM,
    CBC,
    CTR,
    ECB,
}

/// Device Status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Busy,
    Error,
    Maintenance,
    Unknown,
}

/// Edge Device Trait
#[async_trait]
pub trait EdgeDevice: Send + Sync {
    /// Get device ID
    fn get_id(&self) -> Uuid;
    
    /// Get device information
    fn get_info(&self) -> EdgeDeviceInfo;
    
    /// Get device platform
    fn get_platform(&self) -> &EdgePlatform;
    
    /// Get device capabilities
    fn get_capabilities(&self) -> Vec<String>;
    
    /// Check if device is connected
    async fn is_connected(&self) -> bool;
    
    /// Connect to device
    async fn connect(&self) -> ToadStoolResult<()>;
    
    /// Disconnect from device
    async fn disconnect(&self) -> ToadStoolResult<()>;
    
    /// Execute code on device
    async fn execute(&self, request: &ExecutionRequest) -> ToadStoolResult<ExecutionResponse>;
    
    /// Deploy code to device
    async fn deploy(&self, code: &[u8]) -> ToadStoolResult<String>;
    
    /// Stop execution on device
    async fn stop_execution(&self, execution_id: Uuid) -> ToadStoolResult<()>;
    
    /// Get device status
    async fn get_status(&self) -> ToadStoolResult<DeviceStatus>;
    
    /// Get resource usage
    async fn get_resource_usage(&self) -> ToadStoolResult<HashMap<String, f64>>;
    
    /// Upload file to device
    async fn upload_file(&self, path: &str, content: &[u8]) -> ToadStoolResult<()>;
    
    /// Download file from device
    async fn download_file(&self, path: &str) -> ToadStoolResult<Vec<u8>>;
    
    /// Execute shell command on device
    async fn execute_command(&self, command: &str) -> ToadStoolResult<String>;
    
    /// Get device logs
    async fn get_logs(&self, lines: Option<usize>) -> ToadStoolResult<String>;
    
    /// Restart device
    async fn restart(&self) -> ToadStoolResult<()>;
    
    /// Update device firmware
    async fn update_firmware(&self, firmware: &[u8]) -> ToadStoolResult<()>;
    
    /// Get device sensors data
    async fn get_sensors(&self) -> ToadStoolResult<HashMap<String, f64>>;
    
    /// Control device actuators
    async fn control_actuators(&self, commands: HashMap<String, f64>) -> ToadStoolResult<()>;
} 