// SPDX-License-Identifier: AGPL-3.0-or-later

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

fn z80_embedded_config() -> EmbeddedConfig {
    EmbeddedConfig {
        architecture: LegacyArchitecture::ZilogZ80,
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
    let b = Emulator6502::default();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    let s = format!("{a:?}");
    assert!(s.contains("Emulator6502"), "{s}");
}

#[test]
fn emulator_z80_new_default_debug() {
    let a = EmulatorZ80::new();
    let b = EmulatorZ80::default();
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
async fn emulator_6502_runs_loaded_rom() {
    let mut e = Emulator6502::new();
    let cfg = minimal_embedded_config();
    e.initialize(&cfg).await.expect("init");
    e.load_rom(&[0xA9, 0x42, 0xEA], 0x0400).await.expect("load");
    e.cpu.mem[0xFFFC] = 0x00;
    e.cpu.mem[0xFFFD] = 0x04;
    e.cpu.reset();
    e.start().await.expect("start");
    e.step().await.expect("step");
    let regs = e.read_registers().await.expect("regs");
    assert_eq!(regs.general_purpose.get("A"), Some(&0x42));
}

#[tokio::test]
async fn emulator_6502_breakpoint_stops() {
    let mut e = Emulator6502::new();
    e.initialize(&minimal_embedded_config())
        .await
        .expect("init");
    e.load_rom(&[0xEA, 0xEA], 0x0400).await.expect("load");
    e.cpu.mem[0xFFFC] = 0x00;
    e.cpu.mem[0xFFFD] = 0x04;
    e.cpu.reset();
    e.set_breakpoint(0x0401).await.expect("bp");
    e.start().await.expect("start");
    e.step().await.expect("s1");
    let st = e.get_status().await.expect("st");
    assert!(matches!(
        st,
        EmulationStatus::Breakpoint { address: 0x0401 }
    ));
}

#[tokio::test]
async fn emulator_z80_step() {
    let mut e = EmulatorZ80::new();
    e.initialize(&z80_embedded_config()).await.expect("init");
    e.load_rom(&[0x3E, 0x07], 0x0000).await.expect("load");
    e.start().await.expect("start");
    e.step().await.expect("step");
    let regs = e.read_registers().await.expect("regs");
    assert_eq!(regs.general_purpose.get("A"), Some(&7));
}
