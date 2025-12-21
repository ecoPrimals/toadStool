//! Trait implementations for emulator placeholders
//!
//! Modern, idiomatic placeholder implementations for future embedded systems support.

use async_trait::async_trait;
use crate::{ToadStoolResult, ToadStoolError, LegacyArchitecture, EmbeddedConfig};
use super::types::{EmbeddedEmulator as EmulatorTrait, EmulationStatus};
use super::emulators::{Emulator6502, EmulatorZ80};

/// Macro for implementing placeholder emulators (modern, DRY approach)
macro_rules! impl_placeholder_emulator {
    ($emulator:ty, $name:expr, $arch:expr) => {
        #[async_trait]
        impl EmulatorTrait for $emulator {
            fn name(&self) -> &str {
                $name
            }

            fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
                vec![$arch]
            }

            async fn initialize(&mut self, _config: &EmbeddedConfig) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn load_rom(&mut self, _rom_data: &[u8], _load_address: u32) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn start(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn stop(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn pause(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn resume(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn step(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn reset(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn get_status(&self) -> ToadStoolResult<EmulationStatus> {
                // Placeholder: return stopped status
                Ok(EmulationStatus::Stopped)
            }

            async fn read_memory(&self, _address: u32, length: u32) -> ToadStoolResult<Vec<u8>> {
                // Placeholder: return zeros
                Ok(vec![0u8; length as usize])
            }

            async fn write_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn get_registers(&self) -> ToadStoolResult<Vec<(String, u32)>> {
                // Placeholder: return empty register list
                Ok(vec![])
            }

            async fn set_register(&mut self, _name: &str, _value: u32) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn set_breakpoint(&mut self, _address: u32) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn clear_breakpoint(&mut self, _address: u32) -> ToadStoolResult<()> {
                Ok(())
            }
        }
    };
}

// Implement for all emulator types
impl_placeholder_emulator!(Emulator6502, "6502 Emulator (Placeholder)", LegacyArchitecture::MOS6502);
impl_placeholder_emulator!(EmulatorZ80, "Z80 Emulator (Placeholder)", LegacyArchitecture::Zilog_Z80);

