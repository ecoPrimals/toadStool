// SPDX-License-Identifier: AGPL-3.0-or-later
//! Trait implementations for embedded emulators
//!
//! ## Planned / Future Implementation
//!
//! These emulator structs and trait implementations are **infrastructure placeholders**
//! for future CPU emulation support. They are registered in the embedded adapter
//! registry and satisfy the type system, but all operations (except no-op `stop` and
//! `clear_breakpoint`) return `not_supported` until emulator cores are implemented.
//!
//! ## Architecture Notes
//!
//! - **6502**: Planned cycle-accurate core; common in retro gaming (NES, C64, Apple II)
//! - **Z80**: Planned integration; used in ZX Spectrum, MSX, Game Boy
//!
//! Each emulator will require: CPU core, memory map, peripheral hooks, and debug
//! interface (breakpoints, register read/write, memory inspection).

use async_trait::async_trait;

use crate::{EmbeddedConfig, LegacyArchitecture, ToadStoolResult};
use toadstool::ToadStoolError;

use super::emulators::{Emulator6502, EmulatorZ80};
use super::types::{CpuRegisters, EmbeddedEmulator as EmulatorTrait, EmulationStatus};

fn not_implemented(feature: &str) -> ToadStoolError {
    ToadStoolError::not_supported(format!(
        "{feature} not yet implemented; requires emulator core implementation"
    ))
}

macro_rules! impl_emulator_stub {
    ($emulator:ty, $name:expr, $arch:expr) => {
        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl EmulatorTrait for $emulator {
            fn name(&self) -> &str {
                $name
            }

            fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
                vec![$arch]
            }

            async fn initialize(&mut self, _config: &EmbeddedConfig) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator initialization"))
            }

            async fn load_rom(
                &mut self,
                _rom_data: &[u8],
                _load_address: u32,
            ) -> ToadStoolResult<()> {
                Err(not_implemented("ROM load"))
            }

            async fn start(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator start"))
            }

            async fn stop(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn step(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator step"))
            }

            async fn get_status(&self) -> ToadStoolResult<EmulationStatus> {
                Ok(EmulationStatus::Stopped)
            }

            async fn read_memory(&self, _address: u32, _length: u32) -> ToadStoolResult<Vec<u8>> {
                Err(not_implemented("Memory read"))
            }

            async fn write_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<()> {
                Err(not_implemented("Memory write"))
            }

            async fn read_registers(&self) -> ToadStoolResult<CpuRegisters> {
                Err(not_implemented("Register read"))
            }

            async fn write_registers(&mut self, _registers: &CpuRegisters) -> ToadStoolResult<()> {
                Err(not_implemented("Register write"))
            }

            async fn set_breakpoint(&mut self, _address: u32) -> ToadStoolResult<()> {
                Err(not_implemented("Breakpoint set"))
            }

            async fn clear_breakpoint(&mut self, _address: u32) -> ToadStoolResult<()> {
                Ok(())
            }
        }
    };
}

impl_emulator_stub!(Emulator6502, "6502 Emulator", LegacyArchitecture::MOS6502);
impl_emulator_stub!(EmulatorZ80, "Z80 Emulator", LegacyArchitecture::ZilogZ80);
