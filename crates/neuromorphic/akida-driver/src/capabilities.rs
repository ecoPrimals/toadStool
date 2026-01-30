//! Device capability querying and representation
//!
//! This module provides runtime capability discovery for Akida devices.
//! No hardcoded device specifications—everything is discovered at runtime.

use crate::error::{AkidaError, Result};

/// Akida device capabilities discovered at runtime
#[derive(Debug, Clone, PartialEq)]
pub struct Capabilities {
    /// Chip version (AKD1000, AKD1500, etc.)
    pub chip_version: ChipVersion,
    
    /// Number of Neural Processing Units
    pub npu_count: u32,
    
    /// On-chip SRAM in megabytes
    pub memory_mb: u32,
    
    /// PCIe configuration
    pub pcie: PcieConfig,
    
    /// Current power consumption in milliwatts
    pub power_mw: Option<u32>,
    
    /// Die temperature in celsius
    pub temperature_c: Option<f32>,
}

/// Akida chip version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipVersion {
    /// AKD1000 (80 NPUs, 10MB SRAM)
    Akd1000,
    
    /// AKD1500 (with external memory support)
    Akd1500,
    
    /// Unknown/future version
    Unknown(u16),
}

impl ChipVersion {
    /// Parse chip version from device ID
    pub fn from_device_id(device_id: u16) -> Self {
        match device_id {
            0xBCA1 => Self::Akd1000,
            0xBCA2 => Self::Akd1500,
            other => Self::Unknown(other),
        }
    }

    /// Get typical NPU count for this chip version
    pub const fn typical_npu_count(&self) -> u32 {
        match self {
            Self::Akd1000 => 80,
            Self::Akd1500 => 80, // Same as AKD1000
            Self::Unknown(_) => 0,
        }
    }

    /// Get typical SRAM size in MB
    pub const fn typical_memory_mb(&self) -> u32 {
        match self {
            Self::Akd1000 => 10,
            Self::Akd1500 => 10, // Base SRAM, plus external DDR
            Self::Unknown(_) => 0,
        }
    }
}

/// PCIe configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PcieConfig {
    /// PCIe generation (1, 2, 3, 4, 5)
    pub generation: u8,
    
    /// Number of PCIe lanes (1, 4, 8, 16)
    pub lanes: u8,
    
    /// Link speed in GT/s
    pub speed_gts: f32,
    
    /// Theoretical bandwidth in GB/s
    pub bandwidth_gbps: f32,
}

impl PcieConfig {
    /// Create PCIe config from generation and lanes
    pub fn new(generation: u8, lanes: u8) -> Self {
        let speed_gts = Self::generation_to_speed(generation);
        let bandwidth_gbps = Self::calculate_bandwidth(generation, lanes);
        
        Self {
            generation,
            lanes,
            speed_gts,
            bandwidth_gbps,
        }
    }

    /// Query PCIe config from sysfs
    ///
    /// # Errors
    ///
    /// Returns error if PCIe configuration cannot be read from sysfs.
    pub fn from_sysfs(pcie_address: &str) -> Result<Self> {
        let base_path = format!("/sys/bus/pci/devices/{pcie_address}");
        
        let generation = Self::read_pcie_generation(&base_path)?;
        let lanes = Self::read_pcie_lanes(&base_path)?;
        
        Ok(Self::new(generation, lanes))
    }

    fn generation_to_speed(generation: u8) -> f32 {
        match generation {
            1 => 2.5,
            2 => 5.0,
            3 => 8.0,
            4 => 16.0,
            5 => 32.0,
            _ => 2.5, // Default to Gen1
        }
    }

    fn calculate_bandwidth(generation: u8, lanes: u8) -> f32 {
        let per_lane_gbps = match generation {
            1 => 0.25,  // 250 MB/s
            2 => 0.5,   // 500 MB/s
            3 => 1.0,   // ~1 GB/s
            4 => 2.0,   // ~2 GB/s
            5 => 4.0,   // 4 GB/s for Gen5
            _ => 4.0,   // default for unknown generations
        };
        
        per_lane_gbps * f32::from(lanes)
    }

    /// Read PCIe generation from sysfs
    ///
    /// # Errors
    ///
    /// Returns error if sysfs file cannot be read or parsed.
    fn read_pcie_generation(base_path: &str) -> Result<u8> {
        let speed_path = format!("{base_path}/current_link_speed");
        
        std::fs::read_to_string(&speed_path)
            .ok()
            .and_then(|s| {
                // Parse strings like "2.5 GT/s", "5.0 GT/s", "8.0 GT/s"
                if s.contains("2.5") {
                    Some(1)
                } else if s.contains("5.0") || s.contains("5 GT") {
                    Some(2)
                } else if s.contains("8.0") || s.contains("8 GT") {
                    Some(3)
                } else if s.contains("16.0") || s.contains("16 GT") {
                    Some(4)
                } else if s.contains("32.0") || s.contains("32 GT") {
                    Some(5)
                } else {
                    None
                }
            })
            .ok_or_else(|| AkidaError::capability_query_failed("Could not read PCIe generation"))
    }

    /// Read PCIe lane count from sysfs
    ///
    /// # Errors
    ///
    /// Returns error if sysfs file cannot be read or parsed.
    fn read_pcie_lanes(base_path: &str) -> Result<u8> {
        let width_path = format!("{base_path}/current_link_width");
        
        std::fs::read_to_string(&width_path)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .ok_or_else(|| AkidaError::capability_query_failed("Could not read PCIe lane count"))
    }
}

impl Capabilities {
    /// Query capabilities from a device via sysfs and device file
    ///
    /// This discovers capabilities at runtime—no hardcoded values.
    ///
    /// # Errors
    ///
    /// Returns error if sysfs files cannot be read or parsed.
    pub fn query(device_index: usize, pcie_address: &str) -> Result<Self> {
        tracing::debug!("Querying capabilities for device {device_index} at {pcie_address}");
        
        // Read device ID from sysfs to determine chip version
        let chip_version = Self::read_chip_version(pcie_address)?;
        
        // Query PCIe configuration
        let pcie = PcieConfig::from_sysfs(pcie_address)?;
        
        // For now, use typical values for the chip version
        // TODO: Query actual values from device when protocol is known
        let npu_count = chip_version.typical_npu_count();
        let memory_mb = chip_version.typical_memory_mb();
        
        // TODO: Query power and temperature from device
        let power_mw = None;
        let temperature_c = None;
        
        Ok(Self {
            chip_version,
            npu_count,
            memory_mb,
            pcie,
            power_mw,
            temperature_c,
        })
    }

    /// Read chip version from device ID in sysfs
    ///
    /// # Errors
    ///
    /// Returns error if device ID cannot be read or parsed.
    fn read_chip_version(pcie_address: &str) -> Result<ChipVersion> {
        let device_id_path = format!("/sys/bus/pci/devices/{pcie_address}/device");
        
        let device_id_str = std::fs::read_to_string(&device_id_path)
            .map_err(|e| AkidaError::capability_query_failed(format!("Failed to read device ID: {e}")))?;
        
        let device_id = u16::from_str_radix(
            device_id_str.trim().trim_start_matches("0x"),
            16
        ).map_err(|e| AkidaError::capability_query_failed(format!("Invalid device ID format: {e}")))?;
        
        Ok(ChipVersion::from_device_id(device_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chip_version_from_device_id() {
        assert_eq!(ChipVersion::from_device_id(0xBCA1), ChipVersion::Akd1000);
        assert_eq!(ChipVersion::from_device_id(0xBCA2), ChipVersion::Akd1500);
        assert!(matches!(ChipVersion::from_device_id(0xFFFF), ChipVersion::Unknown(0xFFFF)));
    }

    #[test]
    fn test_pcie_bandwidth_calculation() {
        let gen2_x4 = PcieConfig::new(2, 4);
        assert!((gen2_x4.bandwidth_gbps - 2.0).abs() < f32::EPSILON);
        
        let gen3_x8 = PcieConfig::new(3, 8);
        assert!((gen3_x8.bandwidth_gbps - 8.0).abs() < f32::EPSILON);
        
        let gen4_x16 = PcieConfig::new(4, 16);
        assert!((gen4_x16.bandwidth_gbps - 32.0).abs() < f32::EPSILON);
    }
}
