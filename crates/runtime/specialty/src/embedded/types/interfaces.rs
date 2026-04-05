// SPDX-License-Identifier: AGPL-3.0-or-later
//! Programmer, emulator, and peripheral hardware interface traits and status types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::configs::embedded::{PeripheralConfig, PeripheralType};
use crate::{LegacyArchitecture, ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult};

/// Programmer interface trait
#[async_trait::async_trait]
pub trait ProgrammerInterface: Send + Sync + std::fmt::Debug {
    /// Get programmer name
    fn name(&self) -> &'static str;

    /// Get supported interfaces
    fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType>;

    /// Initialize programmer
    async fn initialize(&mut self, config: &ProgrammingInterface) -> ToadStoolResult<()>;

    /// Connect to target
    async fn connect(&mut self) -> ToadStoolResult<()>;

    /// Disconnect from target
    async fn disconnect(&mut self) -> ToadStoolResult<()>;

    /// Read memory
    async fn read_memory(&mut self, address: u32, length: u32) -> ToadStoolResult<Vec<u8>>;

    /// Write memory
    async fn write_memory(&mut self, address: u32, data: &[u8]) -> ToadStoolResult<()>;

    /// Erase memory
    async fn erase_memory(&mut self, address: u32, length: u32) -> ToadStoolResult<()>;

    /// Verify memory
    async fn verify_memory(&mut self, address: u32, expected_data: &[u8]) -> ToadStoolResult<bool>;

    /// Get target information
    async fn get_target_info(&self) -> ToadStoolResult<TargetInfo>;
}

/// Target information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetInfo {
    /// Target name
    pub name: String,
    /// Target architecture
    pub architecture: LegacyArchitecture,
    /// Flash size
    pub flash_size: u32,
    /// RAM size
    pub ram_size: u32,
    /// EEPROM size
    pub eeprom_size: Option<u32>,
    /// CPU speed
    pub cpu_speed: u32,
    /// Supported features
    pub features: Vec<String>,
}

/// Embedded emulator trait
#[async_trait::async_trait]
pub trait EmbeddedEmulator: Send + Sync + std::fmt::Debug {
    /// Get emulator name
    fn name(&self) -> &'static str;

    /// Get supported architectures
    fn supported_architectures(&self) -> Vec<LegacyArchitecture>;

    /// Initialize emulator
    async fn initialize(&mut self, config: &crate::EmbeddedConfig) -> ToadStoolResult<()>;

    /// Load ROM image
    async fn load_rom(&mut self, rom_data: &[u8], load_address: u32) -> ToadStoolResult<()>;

    /// Start emulation
    async fn start(&mut self) -> ToadStoolResult<()>;

    /// Stop emulation
    async fn stop(&mut self) -> ToadStoolResult<()>;

    /// Step instruction
    async fn step(&mut self) -> ToadStoolResult<()>;

    /// Set breakpoint
    async fn set_breakpoint(&mut self, address: u32) -> ToadStoolResult<()>;

    /// Clear breakpoint
    async fn clear_breakpoint(&mut self, address: u32) -> ToadStoolResult<()>;

    /// Read CPU registers
    async fn read_registers(&self) -> ToadStoolResult<CpuRegisters>;

    /// Write CPU registers
    async fn write_registers(&mut self, registers: &CpuRegisters) -> ToadStoolResult<()>;

    /// Read memory
    async fn read_memory(&self, address: u32, length: u32) -> ToadStoolResult<Vec<u8>>;

    /// Write memory
    async fn write_memory(&mut self, address: u32, data: &[u8]) -> ToadStoolResult<()>;

    /// Get emulation status
    async fn get_status(&self) -> ToadStoolResult<EmulationStatus>;
}

/// CPU registers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuRegisters {
    /// General purpose registers
    pub general_purpose: HashMap<String, u32>,
    /// Program counter
    pub program_counter: u32,
    /// Stack pointer
    pub stack_pointer: u32,
    /// Status register
    pub status_register: u32,
    /// Special registers
    pub special: HashMap<String, u32>,
}

/// Emulation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulationStatus {
    /// Emulation is running
    Running,
    /// Emulation is stopped
    Stopped,
    /// Emulation is paused at breakpoint
    Breakpoint {
        /// Address where execution stopped.
        address: u32,
    },
    /// Emulation error
    Error {
        /// Error description.
        message: String,
    },
}

/// Peripheral interface trait
#[async_trait::async_trait]
pub trait PeripheralInterface: Send + Sync + std::fmt::Debug {
    /// Get peripheral name
    fn name(&self) -> &'static str;

    /// Get peripheral type
    fn peripheral_type(&self) -> PeripheralType;

    /// Initialize peripheral
    async fn initialize(&mut self, config: &PeripheralConfig) -> ToadStoolResult<()>;

    /// Read from peripheral
    async fn read(&self, address: u32) -> ToadStoolResult<u32>;

    /// Write to peripheral
    async fn write(&mut self, address: u32, value: u32) -> ToadStoolResult<()>;

    /// Reset peripheral
    async fn reset(&mut self) -> ToadStoolResult<()>;

    /// Get peripheral status
    async fn get_status(&self) -> ToadStoolResult<PeripheralStatus>;
}

/// Peripheral status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralStatus {
    /// Peripheral name
    pub name: String,
    /// Peripheral type
    pub peripheral_type: PeripheralType,
    /// Status
    pub status: String,
    /// Register values
    pub registers: HashMap<String, u32>,
    /// Interrupt status
    pub interrupt_status: bool,
}
