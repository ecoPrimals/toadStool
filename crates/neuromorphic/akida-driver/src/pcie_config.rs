// SPDX-License-Identifier: AGPL-3.0-or-later
//! PCIe configuration discovery from sysfs.

use crate::error::{AkidaError, Result};

#[derive(Debug, Clone, PartialEq)]
/// PCIe link configuration discovered from sysfs.
pub struct PcieConfig {
    /// `PCIe` generation (1, 2, 3, 4, 5)
    pub generation: u8,

    /// Number of `PCIe` lanes (1, 4, 8, 16)
    pub lanes: u8,

    /// Link speed in GT/s
    pub speed_gts: f32,

    /// Theoretical bandwidth in GB/s
    pub bandwidth_gbps: f32,
}

impl PcieConfig {
    /// Create `PCIe` config from generation and lanes
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

    /// Query `PCIe` config from sysfs
    ///
    /// # Errors
    ///
    /// Returns error if `PCIe` configuration cannot be read from sysfs.
    pub fn from_sysfs(pcie_address: &str) -> Result<Self> {
        let base_path = format!("/sys/bus/pci/devices/{pcie_address}");

        let generation = Self::read_pcie_generation(&base_path)?;
        let lanes = Self::read_pcie_lanes(&base_path)?;

        Ok(Self::new(generation, lanes))
    }

    const fn generation_to_speed(generation: u8) -> f32 {
        match generation {
            2 => 5.0,
            3 => 8.0,
            4 => 16.0,
            5 => 32.0,
            _ => 2.5, // Gen1 and unknown generations
        }
    }

    fn calculate_bandwidth(generation: u8, lanes: u8) -> f32 {
        let per_lane_gbps = match generation {
            1 => 0.25, // 250 MB/s
            2 => 0.5,  // 500 MB/s
            3 => 1.0,  // ~1 GB/s
            4 => 2.0,  // ~2 GB/s
            _ => 4.0,  // Gen5 and unknown: use highest per-lane rate in this table
        };

        per_lane_gbps * f32::from(lanes)
    }

    /// Read `PCIe` generation from sysfs
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

    /// Read `PCIe` lane count from sysfs
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
