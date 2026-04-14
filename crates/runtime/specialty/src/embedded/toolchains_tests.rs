// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::{Path, PathBuf};

use super::*;
use crate::embedded::types::{EmbeddedToolchain, OutputFileType, SourceFile};
use crate::{
    EmbeddedConfig, LegacyArchitecture, MemoryLayout, ProgrammingInterface,
    ProgrammingInterfaceType,
};
use toadstool::ToadStoolError;

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
            connection_params: std::collections::HashMap::new(),
        },
    }
}

fn assert_not_supported_toolchain_error(err: &ToadStoolError) {
    let msg = err.to_string();
    assert!(
        msg.contains("not supported"),
        "expected platform not-supported wording, got: {msg}"
    );
    assert!(
        msg.contains("cross-compilation toolchain not discovered"),
        "expected toolchain discovery message, got: {msg}"
    );
}

#[tokio::test]
async fn toolchain_6502_returns_not_supported_for_initialization() {
    let mut t = Toolchain6502::new();
    let err = t
        .initialize(&minimal_embedded_config())
        .await
        .expect_err("6502 initialize should fail until toolchain is installed");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_6502_returns_not_supported_for_compile_link_and_rom() {
    let t = Toolchain6502::new();
    let err = t
        .compile(&[], Path::new("out.o"))
        .await
        .expect_err("compile");
    assert_not_supported_toolchain_error(&err);

    let err = t
        .link(
            &[],
            Path::new("out.bin"),
            &minimal_embedded_config().memory_layout,
        )
        .await
        .expect_err("link");
    assert_not_supported_toolchain_error(&err);

    let err = t
        .generate_rom_image(Path::new("a.bin"), OutputFileType::Binary)
        .await
        .expect_err("rom");
    assert_not_supported_toolchain_error(&err);

    let err = t
        .disassemble(&[0xea], 0x8000)
        .await
        .expect_err("disassemble");
    assert_not_supported_toolchain_error(&err);

    let err = t
        .create_memory_map(Path::new("a.bin"))
        .await
        .expect_err("memory map");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_6502_name_and_architectures() {
    let t = Toolchain6502;
    assert_eq!(t.name(), "6502 Toolchain");
    assert_eq!(
        t.supported_architectures(),
        vec![LegacyArchitecture::MOS6502]
    );
}

#[tokio::test]
async fn toolchain_z80_returns_not_supported_for_initialization() {
    let mut t = ToolchainZ80;
    let err = t
        .initialize(&minimal_embedded_config())
        .await
        .expect_err("init");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_z80_returns_not_supported_for_compile() {
    let t = ToolchainZ80::new();
    let err = t
        .compile(&[], Path::new("out.o"))
        .await
        .expect_err("compile");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_8080_returns_not_supported_for_initialization() {
    let mut t = Toolchain8080;
    let err = t
        .initialize(&minimal_embedded_config())
        .await
        .expect_err("init");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_8051_returns_not_supported_for_initialization() {
    let mut t = Toolchain8051;
    let err = t
        .initialize(&minimal_embedded_config())
        .await
        .expect_err("init");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_8086_returns_not_supported_for_initialization() {
    let mut t = Toolchain8086;
    let err = t
        .initialize(&minimal_embedded_config())
        .await
        .expect_err("init");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_68000_returns_not_supported_for_initialization() {
    let mut t = Toolchain68000;
    let err = t
        .initialize(&minimal_embedded_config())
        .await
        .expect_err("init");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn pending_toolchains_macro_impl_returns_not_supported_for_all_operations() {
    let t = ToolchainZ80::new();
    let cfg = minimal_embedded_config();
    let mem = &cfg.memory_layout;

    let err = t
        .compile(&[], Path::new("out.o"))
        .await
        .expect_err("compile");
    assert_not_supported_toolchain_error(&err);

    let err = t
        .link(&[], Path::new("linked.bin"), mem)
        .await
        .expect_err("link");
    assert_not_supported_toolchain_error(&err);

    let err = t
        .generate_rom_image(Path::new("rom.bin"), OutputFileType::IntelHex)
        .await
        .expect_err("rom");
    assert_not_supported_toolchain_error(&err);

    let err = t.disassemble(&[], 0).await.expect_err("disassemble");
    assert_not_supported_toolchain_error(&err);

    let err = t
        .create_memory_map(Path::new("a.out"))
        .await
        .expect_err("map");
    assert_not_supported_toolchain_error(&err);
}

#[tokio::test]
async fn toolchain_constructors_match_default() {
    assert_eq!(
        format!("{:?}", Toolchain6502::new()),
        format!("{:?}", Toolchain6502)
    );
    assert_eq!(
        format!("{:?}", ToolchainZ80::new()),
        format!("{:?}", ToolchainZ80)
    );
    assert_eq!(
        format!("{:?}", Toolchain8080::new()),
        format!("{:?}", Toolchain8080)
    );
    assert_eq!(
        format!("{:?}", Toolchain8051::new()),
        format!("{:?}", Toolchain8051)
    );
    assert_eq!(
        format!("{:?}", Toolchain8086::new()),
        format!("{:?}", Toolchain8086)
    );
    assert_eq!(
        format!("{:?}", Toolchain68000::new()),
        format!("{:?}", Toolchain68000)
    );
}

#[tokio::test]
async fn toolchain_6502_compile_mentions_feature_in_error_message() {
    let t = Toolchain6502::new();
    let err = t
        .compile(
            &[SourceFile {
                path: PathBuf::from("main.asm"),
                file_type: crate::embedded::types::SourceFileType::Assembly,
                content: String::new(),
                include_paths: vec![],
                defines: std::collections::HashMap::new(),
            }],
            Path::new("out.o"),
        )
        .await
        .expect_err("compile");
    let msg = err.to_string();
    assert!(msg.contains("6502 compilation"), "msg: {msg}");
}
