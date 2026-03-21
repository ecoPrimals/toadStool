// SPDX-License-Identifier: AGPL-3.0-only
//! Trait implementations for embedded emulators
//!
//! ## Planned / Future Implementation
//!
//! These emulator structs and trait implementations are **infrastructure placeholders**
//! for future CPU emulation support. They are registered in the embedded adapter
//! registry and satisfy the type system, but all operations (except no-op `stop` and
//! `clear_breakpoint`) return `SystemError::NotSupported` until emulator cores exist.
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
use toadstool::{SystemError, ToadStoolError};

use super::emulators::{Emulator6502, EmulatorZ80};
use super::types::{CpuRegisters, EmbeddedEmulator as EmulatorTrait, EmulationStatus};

fn emulator_capability_unavailable(
    feature_id: &'static str,
    operation: &'static str,
) -> ToadStoolError {
    SystemError::NotSupported {
        feature: feature_id.to_string(),
        reason: format!("{operation}: embedded CPU emulator core not implemented"),
    }
    .into()
}

macro_rules! impl_emulator_stub {
    ($emulator:ty, $name:expr, $arch:expr, $feature_id:expr) => {
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
                Err(emulator_capability_unavailable($feature_id, "initialize"))
            }

            async fn load_rom(
                &mut self,
                _rom_data: &[u8],
                _load_address: u32,
            ) -> ToadStoolResult<()> {
                Err(emulator_capability_unavailable($feature_id, "load_rom"))
            }

            async fn start(&mut self) -> ToadStoolResult<()> {
                Err(emulator_capability_unavailable($feature_id, "start"))
            }

            async fn stop(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn step(&mut self) -> ToadStoolResult<()> {
                Err(emulator_capability_unavailable($feature_id, "step"))
            }

            async fn get_status(&self) -> ToadStoolResult<EmulationStatus> {
                Err(emulator_capability_unavailable($feature_id, "get_status"))
            }

            async fn read_memory(&self, _address: u32, _length: u32) -> ToadStoolResult<Vec<u8>> {
                Err(emulator_capability_unavailable($feature_id, "read_memory"))
            }

            async fn write_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<()> {
                Err(emulator_capability_unavailable($feature_id, "write_memory"))
            }

            async fn read_registers(&self) -> ToadStoolResult<CpuRegisters> {
                Err(emulator_capability_unavailable(
                    $feature_id,
                    "read_registers",
                ))
            }

            async fn write_registers(&mut self, _registers: &CpuRegisters) -> ToadStoolResult<()> {
                Err(emulator_capability_unavailable(
                    $feature_id,
                    "write_registers",
                ))
            }

            async fn set_breakpoint(&mut self, _address: u32) -> ToadStoolResult<()> {
                Err(emulator_capability_unavailable(
                    $feature_id,
                    "set_breakpoint",
                ))
            }

            async fn clear_breakpoint(&mut self, _address: u32) -> ToadStoolResult<()> {
                Ok(())
            }
        }
    };
}

impl_emulator_stub!(
    Emulator6502,
    "6502 Emulator",
    LegacyArchitecture::MOS6502,
    "embedded_emulator_mos6502"
);
impl_emulator_stub!(
    EmulatorZ80,
    "Z80 Emulator",
    LegacyArchitecture::ZilogZ80,
    "embedded_emulator_zilog_z80"
);
