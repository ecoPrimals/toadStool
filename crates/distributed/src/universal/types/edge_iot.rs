//! Edge and IoT platform support
//!
//! Support for edge computing and IoT devices including microcontrollers,
//! single-board computers, sensors, FPGAs, and neural processing units.

use serde::{Deserialize, Serialize};

/// Edge and IoT platforms
///
/// Represents various edge computing and IoT devices from microcontrollers to smart devices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum EdgeIoTPlatform {
    /// Microcontrollers
    Microcontroller {
        /// Chip model
        chip: String,
        /// CPU architecture (ARM Cortex-M, RISC-V, etc.)
        architecture: String,
        /// Flash memory in kilobytes
        flash_kb: u32,
        /// RAM in kilobytes
        ram_kb: u32,
        /// Clock speed in MHz
        clock_speed_mhz: u32,
        /// Number of GPIO pins
        gpio_pins: u32,
    },

    /// Single board computers
    SingleBoardComputer {
        /// Board name (Raspberry Pi, etc.)
        board: String,
        /// System-on-Chip model
        soc: String,
        /// RAM in megabytes
        ram_mb: u32,
        /// Storage type (SD card, eMMC, etc.)
        storage_type: String,
        /// Connectivity options (WiFi, Bluetooth, Ethernet)
        connectivity: Vec<String>,
    },

    /// IoT sensors
    IoTSensor {
        /// Type of sensor (temperature, humidity, etc.)
        sensor_type: String,
        /// Measurement range description
        measurement_range: String,
        /// Power consumption in microwatts
        power_consumption_uw: f64,
        /// Communication protocol (I2C, SPI, UART, etc.)
        communication_protocol: String,
    },

    /// Smart devices
    SmartDevice {
        /// Device type (speaker, display, etc.)
        device_type: String,
        /// Device capabilities
        capabilities: Vec<String>,
        /// Connectivity options
        connectivity: Vec<String>,
        /// Has AI acceleration
        ai_acceleration: bool,
    },

    /// FPGA platforms
    FPGA {
        /// FPGA family
        family: String,
        /// Number of logic elements
        logic_elements: u32,
        /// RAM blocks available
        ram_blocks: u32,
        /// DSP blocks available
        dsp_blocks: u32,
        /// Number of I/O pins
        io_pins: u32,
    },

    /// Neural processing units
    NPU {
        /// NPU chip model
        chip: String,
        /// Performance in TOPS (trillion operations per second)
        tops_performance: f64,
        /// Power efficiency in TOPS/Watt
        power_efficiency_tops_per_watt: f64,
        /// Supported ML frameworks
        supported_frameworks: Vec<String>,
    },
}

impl EdgeIoTPlatform {
    /// Get the platform type name
    pub fn platform_type(&self) -> &'static str {
        match self {
            Self::Microcontroller { .. } => "Microcontroller",
            Self::SingleBoardComputer { .. } => "Single Board Computer",
            Self::IoTSensor { .. } => "IoT Sensor",
            Self::SmartDevice { .. } => "Smart Device",
            Self::FPGA { .. } => "FPGA",
            Self::NPU { .. } => "Neural Processing Unit",
        }
    }

    /// Check if platform has AI capabilities
    pub const fn has_ai_capability(&self) -> bool {
        matches!(
            self,
            Self::SmartDevice {
                ai_acceleration: true,
                ..
            } | Self::NPU { .. }
        )
    }

    /// Check if platform is low-power
    pub fn is_low_power(&self) -> bool {
        match self {
            Self::IoTSensor {
                power_consumption_uw,
                ..
            } => *power_consumption_uw < 1000.0,
            Self::Microcontroller { .. } => true,
            _ => false,
        }
    }

    /// Get memory capacity in bytes (if applicable)
    pub fn memory_bytes(&self) -> Option<u64> {
        match self {
            Self::Microcontroller { ram_kb, .. } => Some(u64::from(*ram_kb) * 1024),
            Self::SingleBoardComputer { ram_mb, .. } => Some(u64::from(*ram_mb) * 1024 * 1024),
            _ => None,
        }
    }

    /// Check if platform supports wireless connectivity
    pub fn has_wireless(&self) -> bool {
        match self {
            Self::SingleBoardComputer { connectivity, .. }
            | Self::SmartDevice { connectivity, .. } => connectivity.iter().any(|c| {
                c.to_lowercase().contains("wifi")
                    || c.to_lowercase().contains("bluetooth")
                    || c.to_lowercase().contains("zigbee")
                    || c.to_lowercase().contains("lora")
            }),
            _ => false,
        }
    }

    /// Check if platform is programmable
    pub const fn is_programmable(&self) -> bool {
        matches!(
            self,
            Self::Microcontroller { .. } | Self::SingleBoardComputer { .. } | Self::FPGA { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_microcontroller() {
        let mcu = EdgeIoTPlatform::Microcontroller {
            chip: "ESP32".to_string(),
            architecture: "Xtensa LX6".to_string(),
            flash_kb: 4096,
            ram_kb: 520,
            clock_speed_mhz: 240,
            gpio_pins: 34,
        };

        assert_eq!(mcu.platform_type(), "Microcontroller");
        assert!(mcu.is_low_power());
        assert!(mcu.is_programmable());
        assert_eq!(mcu.memory_bytes(), Some(520 * 1024));
    }

    #[test]
    fn test_single_board_computer() {
        let sbc = EdgeIoTPlatform::SingleBoardComputer {
            board: "Raspberry Pi 4".to_string(),
            soc: "BCM2711".to_string(),
            ram_mb: 4096,
            storage_type: "microSD".to_string(),
            connectivity: vec![
                "WiFi".to_string(),
                "Bluetooth".to_string(),
                "Ethernet".to_string(),
            ],
        };

        assert!(sbc.has_wireless());
        assert!(sbc.is_programmable());
        assert_eq!(sbc.memory_bytes(), Some(4096 * 1024 * 1024));
    }

    #[test]
    fn test_npu() {
        let npu = EdgeIoTPlatform::NPU {
            chip: "Google Edge TPU".to_string(),
            tops_performance: 4.0,
            power_efficiency_tops_per_watt: 2.0,
            supported_frameworks: vec!["TensorFlow Lite".to_string()],
        };

        assert!(npu.has_ai_capability());
        assert_eq!(npu.platform_type(), "Neural Processing Unit");
    }

    #[test]
    fn test_iot_sensor() {
        let sensor = EdgeIoTPlatform::IoTSensor {
            sensor_type: "Temperature".to_string(),
            measurement_range: "-40°C to 125°C".to_string(),
            power_consumption_uw: 500.0,
            communication_protocol: "I2C".to_string(),
        };

        assert!(sensor.is_low_power());
        assert!(!sensor.is_programmable());
    }

    #[test]
    fn test_serialization() {
        let platform = EdgeIoTPlatform::FPGA {
            family: "Xilinx Artix-7".to_string(),
            logic_elements: 33280,
            ram_blocks: 120,
            dsp_blocks: 90,
            io_pins: 250,
        };

        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: EdgeIoTPlatform = serde_json::from_str(&json).unwrap();

        assert_eq!(platform, deserialized);
    }
}
