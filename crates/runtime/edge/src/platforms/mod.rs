// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Edge Platform Implementations
//!
//! Platform-specific adapters for various edge computing devices and IoT platforms.

pub mod arduino;
pub mod esp32;
pub mod industrial;
pub mod linux_edge;
pub mod microcontroller;
pub mod raspberry_pi;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

type DeviceFuture<'a, T> = Pin<Box<dyn Future<Output = ToadStoolResult<T>> + Send + 'a>>;
type MetricsFuture<'a> = DeviceFuture<'a, HashMap<String, f64>>;

use toadstool::{
    error::ToadStoolResult,
    execution::{ExecutionRequest, ExecutionResponse},
};

pub use arduino::*;
pub use esp32::*;
pub use industrial::*;
pub use linux_edge::*;
pub use microcontroller::*;
pub use raspberry_pi::*;

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
    RaspberryPi { model: PiModel, os: PiOS },
    /// BeagleBone variants
    BeagleBone { variant: BeagleBoneVariant },
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

impl std::fmt::Display for EdgePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arduino { board, version } => write!(f, "Arduino({board:?} v{version})"),
            Self::ESP32 { chip, framework } => write!(f, "ESP32({chip:?}/{framework:?})"),
            Self::RaspberryPi { model, os } => write!(f, "RaspberryPi({model:?}/{os:?})"),
            Self::BeagleBone { variant } => write!(f, "BeagleBone({variant:?})"),
            Self::Industrial {
                system_type,
                protocol,
            } => write!(f, "Industrial({system_type:?}/{protocol:?})"),
            Self::Microcontroller {
                architecture,
                vendor,
                model,
            } => write!(f, "MCU({architecture:?}/{vendor}/{model})"),
            Self::LinuxEdge {
                architecture,
                kernel_version,
            } => write!(f, "LinuxEdge({architecture}/kernel-{kernel_version})"),
        }
    }
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
    X86,
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub last_seen: std::time::SystemTime,
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
///
/// Async methods use `Pin<Box<dyn Future<...>>>` so the trait remains object-safe for `dyn EdgeDevice`.
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
    fn is_connected(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>>;

    /// Connect to device
    fn connect(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Disconnect from device
    fn disconnect(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Execute code on device
    fn execute(
        &self,
        request: &ExecutionRequest,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<ExecutionResponse>> + Send + '_>>;

    /// Deploy code to device
    fn deploy(
        &self,
        code: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>>;

    /// Stop execution on device
    fn stop_execution(
        &self,
        execution_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Get device status
    fn get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<DeviceStatus>> + Send + '_>>;

    /// Get resource usage
    fn get_resource_usage(&self) -> MetricsFuture<'_>;

    /// Upload file to device
    fn upload_file(
        &self,
        path: &str,
        content: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Download file from device
    fn download_file(
        &self,
        path: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_>>;

    /// Execute shell command on device
    fn execute_command(
        &self,
        command: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>>;

    /// Get device logs
    fn get_logs(
        &self,
        lines: Option<usize>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + '_>>;

    /// Restart device
    fn restart(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Update device firmware
    fn update_firmware(
        &self,
        firmware: &[u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Get device sensors data
    fn get_sensors(&self) -> MetricsFuture<'_>;

    /// Control device actuators
    fn control_actuators(
        &self,
        commands: HashMap<String, f64>,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;
}
