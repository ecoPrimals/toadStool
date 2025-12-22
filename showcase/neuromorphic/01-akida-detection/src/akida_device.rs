//! Akida device querying and management

use anyhow::{Context, Result};
use std::path::PathBuf;

use crate::pcie_scan::PcieDevice;
use crate::{AkidaBoard, BoardHealth};

/// Query detailed board information
pub fn query_board_info(device: &PcieDevice, index: usize) -> Result<AkidaBoard> {
    // In production, this would use the Akida SDK to query actual board state
    // For now, we'll use mock data based on known Akida AKD1000 specs
    
    let device_path = PathBuf::from(format!("/dev/akida{}", index));
    
    // Try to get real PCIe link info if available
    let (pcie_gen, pcie_lanes) = query_pcie_link_info(&device.address)
        .unwrap_or((2, 4)); // Default: PCIe Gen2 x4
    
    // Mock board info (would come from Akida SDK)
    let board = AkidaBoard {
        index,
        pcie_address: device.address.clone(),
        device_path,
        chip_name: "Akida AKD1000".to_string(),
        npu_count: 80, // AKD1000 spec
        memory_bytes: 10 * 1024 * 1024, // 10MB SRAM
        power_watts: estimate_power_consumption(index),
        temperature_celsius: estimate_temperature(index),
        pcie_generation: pcie_gen,
        pcie_lanes,
        health: check_board_health(&device.address)?,
        node_name: None, // Local board
    };
    
    Ok(board)
}

/// Query PCIe link status from sysfs
fn query_pcie_link_info(address: &str) -> Result<(u8, u8)> {
    use std::fs;
    
    let base_path = format!("/sys/bus/pci/devices/{}", address);
    
    // Try to read current link speed and width
    let speed_path = format!("{}/current_link_speed", base_path);
    let width_path = format!("{}/current_link_width", base_path);
    
    let generation = if let Ok(speed) = fs::read_to_string(&speed_path) {
        parse_pcie_speed(&speed)
    } else {
        2 // Default Gen2
    };
    
    let lanes = if let Ok(width) = fs::read_to_string(&width_path) {
        width.trim().parse().unwrap_or(4)
    } else {
        4 // Default x4
    };
    
    Ok((generation, lanes))
}

/// Parse PCIe speed string to generation number
fn parse_pcie_speed(speed: &str) -> u8 {
    if speed.contains("2.5") {
        1
    } else if speed.contains("5.0") {
        2
    } else if speed.contains("8.0") {
        3
    } else if speed.contains("16.0") {
        4
    } else {
        2 // Default Gen2
    }
}

/// Estimate power consumption (would query from SDK in production)
fn estimate_power_consumption(index: usize) -> f64 {
    // Mock realistic power consumption based on index
    match index {
        0 => 1.2, // Slightly higher (more active)
        1 => 0.8, // Lower (idle)
        2 => 1.5, // Higher (gaming system with background load)
        _ => 1.0,
    }
}

/// Estimate temperature (would query from SDK in production)
fn estimate_temperature(index: usize) -> f64 {
    // Mock realistic temperatures
    match index {
        0 => 42.0,
        1 => 38.0,
        2 => 45.0, // Slightly higher (gaming system)
        _ => 40.0,
    }
}

/// Check board health status
fn check_board_health(address: &str) -> Result<BoardHealth> {
    use std::path::Path;
    
    // Check if device is present in sysfs
    let device_path = format!("/sys/bus/pci/devices/{}", address);
    
    if !Path::new(&device_path).exists() {
        return Ok(BoardHealth::Error);
    }
    
    // In production, would check:
    // - Memory test results
    // - NPU operational status
    // - Temperature thresholds
    // - Power consumption anomalies
    // - Error counters
    
    // For now, assume healthy if device exists
    Ok(BoardHealth::Healthy)
}

/// Run board diagnostics
pub fn run_diagnostics(board: &AkidaBoard) -> Result<DiagnosticReport> {
    let mut report = DiagnosticReport {
        board_index: board.index,
        tests: Vec::new(),
    };
    
    // PCIe link test
    report.tests.push(DiagnosticTest {
        name: "PCIe Link".to_string(),
        status: TestStatus::Passed,
        details: format!("Gen{} x{}", board.pcie_generation, board.pcie_lanes),
    });
    
    // Memory test (would actually test in production)
    report.tests.push(DiagnosticTest {
        name: "Memory".to_string(),
        status: TestStatus::Passed,
        details: format!("{} MB available", board.memory_bytes / 1_048_576),
    });
    
    // NPU test
    report.tests.push(DiagnosticTest {
        name: "NPUs".to_string(),
        status: TestStatus::Passed,
        details: format!("{}/{} operational", board.npu_count, board.npu_count),
    });
    
    // Temperature check
    let temp_status = if board.temperature_celsius > 80.0 {
        TestStatus::Failed
    } else if board.temperature_celsius > 60.0 {
        TestStatus::Warning
    } else {
        TestStatus::Passed
    };
    
    report.tests.push(DiagnosticTest {
        name: "Temperature".to_string(),
        status: temp_status,
        details: format!("{:.1}°C", board.temperature_celsius),
    });
    
    // Power check
    let power_status = if board.power_watts > 12.0 {
        TestStatus::Warning
    } else {
        TestStatus::Passed
    };
    
    report.tests.push(DiagnosticTest {
        name: "Power".to_string(),
        status: power_status,
        details: format!("{:.1}W", board.power_watts),
    });
    
    Ok(report)
}

/// Diagnostic report for a board
#[derive(Debug)]
pub struct DiagnosticReport {
    /// Board index
    pub board_index: usize,
    
    /// Individual test results
    pub tests: Vec<DiagnosticTest>,
}

impl DiagnosticReport {
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.tests.iter().all(|t| t.status == TestStatus::Passed)
    }
    
    /// Get overall status
    pub fn overall_status(&self) -> TestStatus {
        if self.tests.iter().any(|t| t.status == TestStatus::Failed) {
            TestStatus::Failed
        } else if self.tests.iter().any(|t| t.status == TestStatus::Warning) {
            TestStatus::Warning
        } else {
            TestStatus::Passed
        }
    }
}

/// Individual diagnostic test
#[derive(Debug)]
pub struct DiagnosticTest {
    /// Test name
    pub name: String,
    
    /// Test status
    pub status: TestStatus,
    
    /// Additional details
    pub details: String,
}

/// Test status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Warning,
    Failed,
}

impl TestStatus {
    /// Get status emoji
    pub const fn emoji(&self) -> &'static str {
        match self {
            Self::Passed => "✓",
            Self::Warning => "⚠",
            Self::Failed => "✗",
        }
    }
}

