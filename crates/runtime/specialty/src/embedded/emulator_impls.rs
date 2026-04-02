// SPDX-License-Identifier: AGPL-3.0-only
//! Trait implementations for embedded emulators
//!
//! ## Planned / Future Implementation
//!
//! These emulator structs and trait implementations are **infrastructure placeholders**
//! for future CPU emulation support. They are registered in the embedded adapter
//! registry and satisfy the type system, but all operations (except no-op `stop` and
//! `clear_breakpoint`) return [`crate::SpecialtyRuntimeError::EmbeddedEmulatorPlaceholder`]
//! (mapped to [`toadstool::SystemError::NotSupported`]) until emulator cores exist.
//!
//! ## Feature Gates
//!
//! - **`embedded-placeholder-impls`** (current): compile these stub impls as placeholders.
//! - When `embedded-hw` is added (real emulator cores), this module will be gated behind
//!   `#[cfg(not(feature = "embedded-hw"))]` so the real impls take precedence.
//!
//! See DEBT.md `D-EMBEDDED-EMULATOR` for evolution tracking.
//!
//! ## Architecture Notes
//!
//! - **6502**: Planned cycle-accurate core; common in retro gaming (NES, C64, Apple II)
//! - **Z80**: Planned integration; used in ZX Spectrum, MSX, Game Boy
//!
//! Each emulator will require: CPU core, memory map, peripheral hooks, and debug
//! interface (breakpoints, register read/write, memory inspection).

use async_trait::async_trait;

use crate::{EmbeddedConfig, LegacyArchitecture, SpecialtyRuntimeError, ToadStoolResult};
use toadstool::ToadStoolError;

use super::emulators::{Emulator6502, EmulatorZ80};
use super::types::{CpuRegisters, EmbeddedEmulator as EmulatorTrait, EmulationStatus};

fn emulator_placeholder_err(feature_id: &'static str, operation: &'static str) -> ToadStoolError {
    SpecialtyRuntimeError::EmbeddedEmulatorPlaceholder {
        feature_id,
        operation,
    }
    .into()
}

/// Generates [`EmbeddedEmulator`] impls that return structured placeholder errors (no panics).
///
/// See DEBT.md `D-EMBEDDED-EMULATOR` and module-level docs for evolution plan.
macro_rules! impl_emulator_stub {
    ($emulator:ty, $name:expr, $arch:expr, $feature_id:expr) => {
        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl EmulatorTrait for $emulator {
            fn name(&self) -> &'static str {
                $name
            }

            fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
                vec![$arch]
            }

            async fn initialize(&mut self, _config: &EmbeddedConfig) -> ToadStoolResult<()> {
                Err(emulator_placeholder_err($feature_id, "initialize"))
            }

            async fn load_rom(
                &mut self,
                _rom_data: &[u8],
                _load_address: u32,
            ) -> ToadStoolResult<()> {
                Err(emulator_placeholder_err($feature_id, "load_rom"))
            }

            async fn start(&mut self) -> ToadStoolResult<()> {
                Err(emulator_placeholder_err($feature_id, "start"))
            }

            async fn stop(&mut self) -> ToadStoolResult<()> {
                Ok(())
            }

            async fn step(&mut self) -> ToadStoolResult<()> {
                Err(emulator_placeholder_err($feature_id, "step"))
            }

            async fn get_status(&self) -> ToadStoolResult<EmulationStatus> {
                Err(emulator_placeholder_err($feature_id, "get_status"))
            }

            async fn read_memory(&self, _address: u32, _length: u32) -> ToadStoolResult<Vec<u8>> {
                Err(emulator_placeholder_err($feature_id, "read_memory"))
            }

            async fn write_memory(&mut self, _address: u32, _data: &[u8]) -> ToadStoolResult<()> {
                Err(emulator_placeholder_err($feature_id, "write_memory"))
            }

            async fn read_registers(&self) -> ToadStoolResult<CpuRegisters> {
                Err(emulator_placeholder_err($feature_id, "read_registers"))
            }

            async fn write_registers(&mut self, _registers: &CpuRegisters) -> ToadStoolResult<()> {
                Err(emulator_placeholder_err($feature_id, "write_registers"))
            }

            async fn set_breakpoint(&mut self, _address: u32) -> ToadStoolResult<()> {
                Err(emulator_placeholder_err($feature_id, "set_breakpoint"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EmbeddedConfig, LegacyArchitecture, MemoryLayout, ProgrammingInterface,
        ProgrammingInterfaceType,
    };
    use std::collections::HashMap;

    use crate::embedded::emulators::{Emulator6502, EmulatorZ80};
    use crate::embedded::types::{CpuRegisters, EmbeddedEmulator, EmulationStatus};

    fn minimal_embedded_config() -> EmbeddedConfig {
        EmbeddedConfig {
            architecture: LegacyArchitecture::MOS6502,
            memory_layout: MemoryLayout {
                rom_regions: vec![],
                ram_regions: vec![],
                io_regions: vec![],
            },
            peripherals: vec![],
            programming_interface: ProgrammingInterface {
                interface_type: ProgrammingInterfaceType::ISP,
                connection_params: HashMap::new(),
            },
        }
    }

    fn assert_not_supported_emulator(err: &ToadStoolError) {
        let msg = err.to_string();
        assert!(
            msg.contains("not supported"),
            "expected not-supported wording, got: {msg}"
        );
        assert!(
            msg.contains("emulator"),
            "expected emulator feature or reason, got: {msg}"
        );
    }

    fn assert_serde_json_stable<T>(value: &T)
    where
        T: serde::Serialize + serde::de::DeserializeOwned + std::fmt::Debug,
    {
        let json = serde_json::to_string(value).expect("serde_json serialize");
        let back: T = serde_json::from_str(&json).expect("serde_json deserialize");
        let json_again = serde_json::to_string(&back).expect("serde_json re-serialize");
        assert_eq!(json, json_again);
    }

    #[test]
    fn emulator_6502_new_default_debug() {
        let a = Emulator6502::new();
        let b = Emulator6502;
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let s = format!("{a:?}");
        assert!(s.contains("Emulator6502"), "{s}");
    }

    #[test]
    fn emulator_z80_new_default_debug() {
        let a = EmulatorZ80::new();
        let b = EmulatorZ80;
        assert_eq!(format!("{a:?}"), format!("{b:?}"));
        let s = format!("{a:?}");
        assert!(s.contains("EmulatorZ80"), "{s}");
    }

    #[test]
    fn serde_roundtrip_types_used_by_emulator_trait() {
        let cfg = minimal_embedded_config();
        assert_serde_json_stable(&cfg);
        let pi = ProgrammingInterface {
            interface_type: ProgrammingInterfaceType::ISP,
            connection_params: HashMap::from([("port".to_string(), "/dev/ttyUSB0".to_string())]),
        };
        assert_serde_json_stable(&pi);
        assert_serde_json_stable(&ProgrammingInterfaceType::Parallel);
        let regs = CpuRegisters {
            general_purpose: HashMap::from([("A".to_string(), 0x42)]),
            program_counter: 0x8000,
            stack_pointer: 0x100,
            status_register: 0,
            special: HashMap::new(),
        };
        assert_serde_json_stable(&regs);
        assert_serde_json_stable(&EmulationStatus::Running);
        assert_serde_json_stable(&EmulationStatus::Stopped);
        assert_serde_json_stable(&EmulationStatus::Breakpoint { address: 0x2000 });
        assert_serde_json_stable(&EmulationStatus::Error {
            message: "boom".to_string(),
        });
    }

    #[test]
    fn emulator_6502_trait_name_and_architectures() {
        let e = Emulator6502::new();
        assert_eq!(EmbeddedEmulator::name(&e), "6502 Emulator");
        assert_eq!(
            EmbeddedEmulator::supported_architectures(&e),
            vec![LegacyArchitecture::MOS6502]
        );
    }

    #[test]
    fn emulator_z80_trait_name_and_architectures() {
        let e = EmulatorZ80::new();
        assert_eq!(EmbeddedEmulator::name(&e), "Z80 Emulator");
        assert_eq!(
            EmbeddedEmulator::supported_architectures(&e),
            vec![LegacyArchitecture::ZilogZ80]
        );
    }

    #[tokio::test]
    async fn emulator_6502_stub_returns_not_supported_except_noops() {
        let mut e = Emulator6502::new();
        let cfg = minimal_embedded_config();
        assert_not_supported_emulator(&e.initialize(&cfg).await.expect_err("initialize"));
        assert_not_supported_emulator(&e.load_rom(&[], 0).await.expect_err("load_rom"));
        assert_not_supported_emulator(&e.start().await.expect_err("start"));
        e.stop().await.expect("stop");
        assert_not_supported_emulator(&e.step().await.expect_err("step"));
        assert_not_supported_emulator(&e.get_status().await.expect_err("get_status"));
        assert_not_supported_emulator(&e.read_memory(0, 4).await.expect_err("read_memory"));
        assert_not_supported_emulator(&e.write_memory(0, &[1]).await.expect_err("write_memory"));
        let regs = CpuRegisters {
            general_purpose: HashMap::new(),
            program_counter: 0,
            stack_pointer: 0,
            status_register: 0,
            special: HashMap::new(),
        };
        assert_not_supported_emulator(&e.read_registers().await.expect_err("read_registers"));
        assert_not_supported_emulator(
            &e.write_registers(&regs).await.expect_err("write_registers"),
        );
        assert_not_supported_emulator(&e.set_breakpoint(0).await.expect_err("set_breakpoint"));
        e.clear_breakpoint(0).await.expect("clear_breakpoint");
    }

    #[tokio::test]
    async fn emulator_z80_stub_returns_not_supported_except_noops() {
        let mut e = EmulatorZ80::new();
        let cfg = minimal_embedded_config();
        assert_not_supported_emulator(&e.initialize(&cfg).await.expect_err("initialize"));
        assert_not_supported_emulator(&e.load_rom(&[], 0).await.expect_err("load_rom"));
        assert_not_supported_emulator(&e.start().await.expect_err("start"));
        e.stop().await.expect("stop");
        assert_not_supported_emulator(&e.step().await.expect_err("step"));
        assert_not_supported_emulator(&e.get_status().await.expect_err("get_status"));
        assert_not_supported_emulator(&e.read_memory(0, 4).await.expect_err("read_memory"));
        assert_not_supported_emulator(&e.write_memory(0, &[1]).await.expect_err("write_memory"));
        let regs = CpuRegisters {
            general_purpose: HashMap::new(),
            program_counter: 0,
            stack_pointer: 0,
            status_register: 0,
            special: HashMap::new(),
        };
        assert_not_supported_emulator(&e.read_registers().await.expect_err("read_registers"));
        assert_not_supported_emulator(
            &e.write_registers(&regs).await.expect_err("write_registers"),
        );
        assert_not_supported_emulator(&e.set_breakpoint(0).await.expect_err("set_breakpoint"));
        e.clear_breakpoint(0).await.expect("clear_breakpoint");
    }
}
