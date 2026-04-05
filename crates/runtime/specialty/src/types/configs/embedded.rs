// SPDX-License-Identifier: AGPL-3.0-or-later
//! Embedded systems configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::communication::ProgrammingInterface;
use crate::LegacyArchitecture;

/// Embedded system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedConfig {
    /// Target architecture
    pub architecture: LegacyArchitecture,
    /// Memory layout
    pub memory_layout: MemoryLayout,
    /// Peripheral configuration
    pub peripherals: Vec<PeripheralConfig>,
    /// Programming interface
    pub programming_interface: ProgrammingInterface,
}

/// Memory layout for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLayout {
    /// ROM/Flash regions
    pub rom_regions: Vec<MemoryRegion>,
    /// RAM regions
    pub ram_regions: Vec<MemoryRegion>,
    /// I/O regions
    pub io_regions: Vec<MemoryRegion>,
}

/// Memory region definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// Region name
    pub name: String,
    /// Start address
    pub start_address: u32,
    /// End address
    pub end_address: u32,
    /// Region type
    pub region_type: MemoryRegionType,
    /// Access permissions
    pub permissions: MemoryPermissions,
}

/// Memory region types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryRegionType {
    /// ROM
    ROM,
    /// Flash
    Flash,
    /// RAM
    RAM,
    /// I/O
    IO,
    /// Reserved
    Reserved,
}

/// Memory permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPermissions {
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
}

/// Peripheral configuration for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralConfig {
    /// Peripheral name
    pub name: String,
    /// Peripheral type
    pub peripheral_type: PeripheralType,
    /// Base address
    pub base_address: u32,
    /// Interrupt vector
    pub interrupt_vector: Option<u8>,
    /// Configuration parameters
    pub parameters: HashMap<String, String>,
}

/// Peripheral types for embedded systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeripheralType {
    /// UART
    UART,
    /// SPI
    SPI,
    /// I2C
    I2C,
    /// GPIO
    GPIO,
    /// Timer
    Timer,
    /// ADC
    ADC,
    /// DAC
    DAC,
    /// PWM
    PWM,
    /// CAN
    CAN,
    /// USB
    USB,
    /// Ethernet
    Ethernet,
    /// Custom peripheral type.
    Custom {
        /// Peripheral type name.
        name: String,
    },
}
