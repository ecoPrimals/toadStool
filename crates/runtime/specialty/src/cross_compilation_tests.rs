// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::configs::StorageROMFormat;
use crate::types::cross_compilation::{
    CompilationResult, CrossCompilationToolchain, LinkResult, ToolchainConfig,
};
use crate::types::systems::LegacyArchitecture;

use super::{Toolchain6502, Toolchain68000, ToolchainZ80};

fn assert_json_round_trip<T>(value: &T)
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let json = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&json).unwrap();
    let v1: Value = serde_json::to_value(value).unwrap();
    let v2: Value = serde_json::to_value(&back).unwrap();
    assert_eq!(v1, v2);
}

fn round_trip_json_eq<T>(value: &T) -> T
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    let json = serde_json::to_string(value).unwrap();
    serde_json::from_str(&json).unwrap()
}

fn sample_toolchain_config() -> ToolchainConfig {
    let mut env = HashMap::new();
    env.insert("CC".to_string(), "cc65".to_string());
    ToolchainConfig {
        compiler_flags: vec!["-O2".to_string()],
        linker_flags: vec!["--gc-sections".to_string()],
        include_paths: vec![PathBuf::from("/opt/include")],
        library_paths: vec![PathBuf::from("/opt/lib")],
        environment: env,
    }
}

#[test]
fn toolchain_6502_default_matches_new_and_debug() {
    let a = Toolchain6502::default();
    let b = Toolchain6502::new();
    assert_eq!(a.name, b.name);
    assert_eq!(a.name, "6502 Cross-Compiler");
    let dbg = format!("{a:?}");
    assert!(dbg.contains("6502") || dbg.contains("Toolchain6502"));
}

#[test]
fn toolchain_z80_default_matches_new_and_debug() {
    let a = ToolchainZ80::default();
    let b = ToolchainZ80::new();
    assert_eq!(a.name, b.name);
    assert_eq!(a.name, "Z80 Cross-Compiler");
    let dbg = format!("{a:?}");
    assert!(dbg.contains("Z80") || dbg.contains("ToolchainZ80"));
}

#[test]
fn toolchain_68000_default_matches_new_and_debug() {
    let a = Toolchain68000::default();
    let b = Toolchain68000::new();
    assert_eq!(a.name, b.name);
    assert_eq!(a.name, "68000 Cross-Compiler");
    let dbg = format!("{a:?}");
    assert!(dbg.contains("68000") || dbg.contains("Toolchain68000"));
}

#[test]
fn toolchain_config_default_clone_debug_serde_round_trip() {
    let d = ToolchainConfig::default();
    let _ = format!("{d:?}");
    let c = d;
    assert_json_round_trip(&c);
    assert_json_round_trip(&sample_toolchain_config());
    assert_json_round_trip(&ToolchainConfig::default());
}

#[test]
fn compilation_result_clone_debug_serde_round_trip() {
    let r = CompilationResult {
        success: true,
        output_path: Some(PathBuf::from("out.o")),
        object_files: vec![PathBuf::from("a.o")],
        messages: vec!["ok".to_string()],
        warnings: vec!["w".to_string()],
        errors: vec![],
    };
    let _ = format!("{r:?}");
    let c = r.clone();
    assert_json_round_trip(&r);
    assert_json_round_trip(&c);
}

#[test]
fn link_result_clone_debug_serde_round_trip() {
    let r = LinkResult {
        success: true,
        executable_path: Some(PathBuf::from("a.out")),
        messages: vec!["linked".to_string()],
        warnings: vec![],
        errors: vec!["e".to_string()],
    };
    let _ = format!("{r:?}");
    assert_json_round_trip(&r);
    assert_json_round_trip(&r);
}

#[test]
fn legacy_architecture_clone_debug_serde_round_trip() {
    for arch in [
        LegacyArchitecture::MOS6502,
        LegacyArchitecture::ZilogZ80,
        LegacyArchitecture::Motorola68000,
    ] {
        let _ = format!("{arch:?}");
        assert_eq!(arch, arch.clone());
        assert_eq!(arch, round_trip_json_eq(&arch));
    }
}

#[test]
fn storage_rom_format_clone_debug_serde_round_trip() {
    for fmt in [
        StorageROMFormat::IntelHex,
        StorageROMFormat::MotorolaS,
        StorageROMFormat::Binary,
        StorageROMFormat::Custom {
            name: "nes".to_string(),
        },
    ] {
        let _ = format!("{fmt:?}");
        assert_eq!(fmt, fmt.clone());
        assert_eq!(fmt, round_trip_json_eq(&fmt));
    }
}

#[tokio::test]
async fn toolchain_6502_trait_methods_succeed() {
    let mut t = Toolchain6502::new();
    assert_eq!(t.name(), "6502 Cross-Compiler");
    assert_eq!(
        t.supported_architectures(),
        vec![LegacyArchitecture::MOS6502]
    );
    t.initialize(&sample_toolchain_config()).await.unwrap();

    let cr = t
        .compile(PathBuf::from("src.asm"), LegacyArchitecture::MOS6502)
        .await
        .unwrap();
    assert!(cr.success);
    assert_eq!(cr.output_path, Some(PathBuf::from("output.prg")));

    let lr = t
        .link(vec![PathBuf::from("a.o")], PathBuf::from("game.prg"))
        .await
        .unwrap();
    assert!(lr.success);
    assert_eq!(lr.executable_path, Some(PathBuf::from("game.prg")));

    let rom = t
        .create_rom_image(Path::new("game.prg"), &StorageROMFormat::Binary)
        .await
        .unwrap();
    assert_eq!(rom, vec![0xA9, 0x00, 0x85, 0x00, 0x60]);

    let asm = t
        .disassemble(&rom, &LegacyArchitecture::MOS6502)
        .await
        .unwrap();
    assert!(asm.contains("LDA"));
}

#[tokio::test]
async fn toolchain_z80_trait_methods_succeed() {
    let mut t = ToolchainZ80::default();
    assert_eq!(t.name(), "Z80 Cross-Compiler");
    assert_eq!(
        t.supported_architectures(),
        vec![LegacyArchitecture::ZilogZ80]
    );
    t.initialize(&ToolchainConfig::default()).await.unwrap();

    let cr = t
        .compile(PathBuf::from("main.z80"), LegacyArchitecture::ZilogZ80)
        .await
        .unwrap();
    assert!(cr.success);
    assert_eq!(cr.output_path, Some(PathBuf::from("output.com")));

    let lr = t.link(vec![], PathBuf::from("out.com")).await.unwrap();
    assert_eq!(lr.executable_path, Some(PathBuf::from("out.com")));

    let rom = t
        .create_rom_image(Path::new("out.com"), &StorageROMFormat::IntelHex)
        .await
        .unwrap();
    assert_eq!(rom, vec![0x3E, 0x00, 0x32, 0x00, 0x80, 0xC9]);

    let s = t
        .disassemble(&[0x00], &LegacyArchitecture::ZilogZ80)
        .await
        .unwrap();
    assert!(s.contains("LD A"));
}

#[tokio::test]
async fn toolchain_68000_trait_methods_succeed() {
    let mut t = Toolchain68000::new();
    assert_eq!(t.name(), "68000 Cross-Compiler");
    assert_eq!(
        t.supported_architectures(),
        vec![LegacyArchitecture::Motorola68000]
    );
    t.initialize(&sample_toolchain_config()).await.unwrap();

    let cr = t
        .compile(PathBuf::from("main.s"), LegacyArchitecture::Motorola68000)
        .await
        .unwrap();
    assert!(cr.messages.iter().any(|m| m.contains("68000")));

    let lr = t
        .link(vec![PathBuf::from("x.o")], PathBuf::from("prog.bin"))
        .await
        .unwrap();
    assert!(lr.success);

    let rom = t
        .create_rom_image(
            Path::new("prog.bin"),
            &StorageROMFormat::Custom {
                name: "raw".to_string(),
            },
        )
        .await
        .unwrap();
    assert_eq!(rom, vec![0x70, 0x00, 0x4E, 0x75]);

    let s = t
        .disassemble(&rom, &LegacyArchitecture::Motorola68000)
        .await
        .unwrap();
    assert!(s.contains("MOVEQ") || s.contains("RTS"));
}
