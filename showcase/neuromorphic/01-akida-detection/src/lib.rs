// SPDX-License-Identifier: AGPL-3.0-or-later
//! Akida PCIe board detection and integration
//!
//! This library provides detection and management of BrainChip Akida neuromorphic
//! PCIe boards, integrating them with ToadStool's UniversalSubstrate.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub mod akida_device;
pub mod pcie_scan;
pub mod substrate_integration;

/// BrainChip vendor ID in PCIe space
pub const BRAINCHIP_VENDOR_ID: u16 = 0x1E7C;

/// Akida AKD1000 device ID
pub const AKIDA_AKD1000_DEVICE_ID: u16 = 0x0001;

/// Akida board information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AkidaBoard {
    /// Board index (0, 1, 2, ...)
    pub index: usize,

    /// PCIe bus address (e.g., "0000:01:00.0")
    pub pcie_address: String,

    /// Device file path (e.g., "/dev/akida0")
    pub device_path: PathBuf,

    /// Chip name
    pub chip_name: String,

    /// Number of Neural Processing Units
    pub npu_count: u32,

    /// On-chip SRAM in bytes
    pub memory_bytes: u64,

    /// Current power consumption in watts
    pub power_watts: f64,

    /// Board temperature in Celsius
    pub temperature_celsius: f64,

    /// PCIe generation (1, 2, 3, 4)
    pub pcie_generation: u8,

    /// PCIe lanes (1, 4, 8, 16)
    pub pcie_lanes: u8,

    /// Board health status
    pub health: BoardHealth,

    /// Node name (for distributed mesh)
    pub node_name: Option<String>,
}

/// Board health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoardHealth {
    /// Board is healthy and operational
    Healthy,

    /// Board is operational but with warnings
    Warning,

    /// Board has errors or is not operational
    Error,

    /// Board status unknown
    Unknown,
}

impl AkidaBoard {
    /// Check if board is locally attached (not remote)
    pub fn is_local(&self) -> bool {
        self.node_name.is_none()
    }

    /// Get bandwidth in GB/s based on PCIe configuration
    pub fn bandwidth_gbps(&self) -> f64 {
        // PCIe bandwidth per lane per generation (GB/s)
        let per_lane_bandwidth = match self.pcie_generation {
            1 => 0.25, // PCIe 1.0: 2.5 GT/s = 250 MB/s
            2 => 0.5,  // PCIe 2.0: 5.0 GT/s = 500 MB/s
            3 => 1.0,  // PCIe 3.0: 8.0 GT/s = ~1 GB/s
            4 => 2.0,  // PCIe 4.0: 16.0 GT/s = ~2 GB/s
            _ => 0.5,  // Default to Gen2
        };

        per_lane_bandwidth * f64::from(self.pcie_lanes)
    }

    /// Check if board is healthy
    pub const fn is_healthy(&self) -> bool {
        matches!(self.health, BoardHealth::Healthy)
    }
}

/// Collection of detected Akida boards
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AkidaMesh {
    /// All detected boards
    pub boards: Vec<AkidaBoard>,

    /// Total NPU count across all boards
    pub total_npus: u32,

    /// Total memory in bytes
    pub total_memory_bytes: u64,

    /// Total power consumption in watts
    pub total_power_watts: f64,
}

impl AkidaMesh {
    /// Create mesh from discovered boards
    pub fn from_boards(boards: Vec<AkidaBoard>) -> Self {
        let total_npus = boards.iter().map(|b| b.npu_count).sum();
        let total_memory_bytes = boards.iter().map(|b| b.memory_bytes).sum();
        let total_power_watts = boards.iter().map(|b| b.power_watts).sum();

        Self {
            boards,
            total_npus,
            total_memory_bytes,
            total_power_watts,
        }
    }

    /// Get all healthy boards
    pub fn healthy_boards(&self) -> Vec<&AkidaBoard> {
        self.boards.iter().filter(|b| b.is_healthy()).collect()
    }

    /// Get local boards only
    pub fn local_boards(&self) -> Vec<&AkidaBoard> {
        self.boards.iter().filter(|b| b.is_local()).collect()
    }

    /// Get remote boards only
    pub fn remote_boards(&self) -> Vec<&AkidaBoard> {
        self.boards.iter().filter(|b| !b.is_local()).collect()
    }
}

/// Detect all Akida boards on the system
pub async fn detect_all_boards() -> Result<AkidaMesh> {
    tracing::info!("Scanning for Akida boards...");

    // Scan PCIe bus for Akida devices
    let pcie_devices = pcie_scan::scan_for_akida().context("Failed to scan PCIe bus")?;

    let mut boards = Vec::new();

    for (index, device) in pcie_devices.into_iter().enumerate() {
        match akida_device::query_board_info(&device, index) {
            Ok(board) => {
                tracing::info!(
                    "Found Akida board {} at {} ({} NPUs, {:.1}W)",
                    index,
                    board.pcie_address,
                    board.npu_count,
                    board.power_watts
                );
                boards.push(board);
            }
            Err(e) => {
                tracing::warn!("Failed to query board {}: {}", index, e);
            }
        }
    }

    if boards.is_empty() {
        tracing::warn!("No Akida boards detected");
    }

    Ok(AkidaMesh::from_boards(boards))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandwidth_calculation() {
        let board = AkidaBoard {
            index: 0,
            pcie_address: "0000:01:00.0".to_string(),
            device_path: PathBuf::from("/dev/akida0"),
            chip_name: "Akida AKD1000".to_string(),
            npu_count: 80,
            memory_bytes: 10 * 1024 * 1024,
            power_watts: 1.2,
            temperature_celsius: 42.0,
            pcie_generation: 2,
            pcie_lanes: 4,
            health: BoardHealth::Healthy,
            node_name: None,
        };

        // PCIe Gen2 x4 = 0.5 GB/s per lane * 4 lanes = 2.0 GB/s
        assert_eq!(board.bandwidth_gbps(), 2.0);
    }

    #[test]
    fn test_mesh_aggregation() {
        let boards = vec![
            AkidaBoard {
                index: 0,
                pcie_address: "0000:01:00.0".to_string(),
                device_path: PathBuf::from("/dev/akida0"),
                chip_name: "Akida AKD1000".to_string(),
                npu_count: 80,
                memory_bytes: 10 * 1024 * 1024,
                power_watts: 1.2,
                temperature_celsius: 42.0,
                pcie_generation: 2,
                pcie_lanes: 4,
                health: BoardHealth::Healthy,
                node_name: None,
            },
            AkidaBoard {
                index: 1,
                pcie_address: "0000:02:00.0".to_string(),
                device_path: PathBuf::from("/dev/akida1"),
                chip_name: "Akida AKD1000".to_string(),
                npu_count: 80,
                memory_bytes: 10 * 1024 * 1024,
                power_watts: 0.8,
                temperature_celsius: 38.0,
                pcie_generation: 2,
                pcie_lanes: 4,
                health: BoardHealth::Healthy,
                node_name: None,
            },
        ];

        let mesh = AkidaMesh::from_boards(boards);

        assert_eq!(mesh.total_npus, 160);
        assert_eq!(mesh.total_memory_bytes, 20 * 1024 * 1024);
        assert_eq!(mesh.total_power_watts, 2.0);
        assert_eq!(mesh.boards.len(), 2);
    }
}
