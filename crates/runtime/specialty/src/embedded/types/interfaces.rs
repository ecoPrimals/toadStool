// SPDX-License-Identifier: AGPL-3.0-or-later
//! Programmer, emulator, and peripheral hardware interface traits and status types.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::types::configs::embedded::{PeripheralConfig, PeripheralType};
use crate::{LegacyArchitecture, ProgrammingInterface, ProgrammingInterfaceType, ToadStoolResult};

/// Programmer interface trait
pub trait ProgrammerInterface: Send + Sync + std::fmt::Debug {
    /// Get programmer name
    fn name(&self) -> &'static str;

    /// Get supported interfaces
    fn supported_interfaces(&self) -> Vec<ProgrammingInterfaceType>;

    /// Initialize programmer
    fn initialize<'a>(
        &'a mut self,
        config: &'a ProgrammingInterface,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Connect to target
    fn connect(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Disconnect from target
    fn disconnect(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Read memory
    fn read_memory(
        &mut self,
        address: u32,
        length: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_>>;

    /// Write memory
    fn write_memory<'a>(
        &'a mut self,
        address: u32,
        data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Erase memory
    fn erase_memory(
        &mut self,
        address: u32,
        length: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Verify memory
    fn verify_memory<'a>(
        &'a mut self,
        address: u32,
        expected_data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<bool>> + Send + 'a>>;

    /// Get target information
    fn get_target_info(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<TargetInfo>> + Send + '_>>;
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
pub trait EmbeddedEmulator: Send + Sync + std::fmt::Debug {
    /// Get emulator name
    fn name(&self) -> &'static str;

    /// Get supported architectures
    fn supported_architectures(&self) -> Vec<LegacyArchitecture>;

    /// Initialize emulator
    fn initialize<'a>(
        &'a mut self,
        config: &'a crate::EmbeddedConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Load ROM image
    fn load_rom<'a>(
        &'a mut self,
        rom_data: &'a [u8],
        load_address: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Start emulation
    fn start(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Stop emulation
    fn stop(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Step instruction
    fn step(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Set breakpoint
    fn set_breakpoint(
        &mut self,
        address: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Clear breakpoint
    fn clear_breakpoint(
        &mut self,
        address: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Read CPU registers
    fn read_registers(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<CpuRegisters>> + Send + '_>>;

    /// Write CPU registers
    fn write_registers<'a>(
        &'a mut self,
        registers: &'a CpuRegisters,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Read memory
    fn read_memory(
        &self,
        address: u32,
        length: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + '_>>;

    /// Write memory
    fn write_memory<'a>(
        &'a mut self,
        address: u32,
        data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Get emulation status
    fn get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<EmulationStatus>> + Send + '_>>;
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
pub trait PeripheralInterface: Send + Sync + std::fmt::Debug {
    /// Get peripheral name
    fn name(&self) -> &'static str;

    /// Get peripheral type
    fn peripheral_type(&self) -> PeripheralType;

    /// Initialize peripheral
    fn initialize<'a>(
        &'a mut self,
        config: &'a PeripheralConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>>;

    /// Read from peripheral
    fn read(&self, address: u32)
    -> Pin<Box<dyn Future<Output = ToadStoolResult<u32>> + Send + '_>>;

    /// Write to peripheral
    fn write(
        &mut self,
        address: u32,
        value: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Reset peripheral
    fn reset(&mut self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Get peripheral status
    fn get_status(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<PeripheralStatus>> + Send + '_>>;
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
