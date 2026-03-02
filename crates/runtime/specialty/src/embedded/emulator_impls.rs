//! Trait implementations for embedded emulators
//!
//! Error-returning stubs until CPU emulation cores are implemented.
//! All emulation operations return `not_supported` to clearly communicate status.

use async_trait::async_trait;

use crate::{EmbeddedConfig, LegacyArchitecture, ToadStoolResult};
use toadstool::ToadStoolError;

use super::emulators::{Emulator6502, EmulatorZ80};
use super::types::{EmbeddedEmulator as EmulatorTrait, EmulationStatus};

fn not_implemented(feature: &str) -> ToadStoolError {
    ToadStoolError::not_supported(format!(
        "{feature} not yet implemented; requires emulator core implementation"
    ))
}

macro_rules! impl_emulator_stub {
    ($emulator:ty, $name:expr, $arch:expr) => {
        // TODO(afit): Migrate when trait_variant stabilizes (used as dyn)
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

            async fn load_rom(&mut self, _rom_data: &[u8], _load_address: u32) -> ToadStoolResult<()> {
                Err(not_implemented("ROM load"))
            }

            async fn start(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator start"))
            }

            async fn stop(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn pause(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator pause"))
            }

            async fn resume(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator resume"))
            }

            async fn step(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator step"))
            }

            async fn reset(&mut self) -> ToadStoolResult<()> {
                Err(not_implemented("Emulator reset"))
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

            async fn get_registers(&self) -> ToadStoolResult<Vec<(String, u32)>> {
                Err(not_implemented("Register read"))
            }

            async fn set_register(&mut self, _name: &str, _value: u32) -> ToadStoolResult<()> {
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
impl_emulator_stub!(EmulatorZ80, "Z80 Emulator", LegacyArchitecture::Zilog_Z80);

