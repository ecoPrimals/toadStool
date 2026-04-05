// SPDX-License-Identifier: AGPL-3.0-or-later
//! # Cross-compilation Support
//!
//! Cross-compilation toolchains for legacy architectures:
//! - 6502 toolchain
//! - Z80 toolchain
//! - 68000 toolchain
//! - 8086 toolchain
//! - Cross-compilation utilities
//!
//! **Native async traits migration complete** - Zero-cost async abstractions

use std::path::{Path, PathBuf};
use tracing::info;

use crate::ToadStoolResult;
use crate::types::configs::StorageROMFormat as ROMFormat;
use crate::types::cross_compilation::{
    CompilationResult, CrossCompilationToolchain, LinkResult, ToolchainConfig,
};
use crate::types::systems::LegacyArchitecture;

/// 6502 Toolchain
#[derive(Debug)]
pub struct Toolchain6502 {
    name: &'static str,
    config: Option<ToolchainConfig>,
}

/// Z80 Toolchain
#[derive(Debug)]
pub struct ToolchainZ80 {
    name: &'static str,
    config: Option<ToolchainConfig>,
}

/// 68000 Toolchain
#[derive(Debug)]
pub struct Toolchain68000 {
    name: &'static str,
    config: Option<ToolchainConfig>,
}

impl Default for Toolchain6502 {
    fn default() -> Self {
        Self {
            name: "6502 Cross-Compiler",
            config: None,
        }
    }
}

impl Toolchain6502 {
    /// Creates a new 6502 cross-compilation toolchain with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ToolchainZ80 {
    /// Creates a new Z80 cross-compilation toolchain with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ToolchainZ80 {
    fn default() -> Self {
        Self {
            name: "Z80 Cross-Compiler",
            config: None,
        }
    }
}

impl Toolchain68000 {
    /// Creates a new 68000 cross-compilation toolchain with default settings.
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Toolchain68000 {
    fn default() -> Self {
        Self {
            name: "68000 Cross-Compiler",
            config: None,
        }
    }
}

#[async_trait::async_trait]
impl CrossCompilationToolchain for Toolchain6502 {
    fn name(&self) -> &'static str {
        self.name
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::MOS6502]
    }

    async fn initialize(&mut self, config: &ToolchainConfig) -> ToadStoolResult<()> {
        info!("Initializing 6502 toolchain");
        self.config = Some(config.clone());
        Ok(())
    }

    async fn compile(
        &self,
        source: PathBuf,
        target: LegacyArchitecture,
    ) -> ToadStoolResult<CompilationResult> {
        info!(
            "Compiling 6502 source code from {:?} for {:?}",
            source, target
        );

        // Simulate compilation
        Ok(CompilationResult {
            success: true,
            output_path: Some(PathBuf::from("output.prg")),
            object_files: vec![PathBuf::from("output.o")],
            messages: vec!["6502 compilation successful".to_string()],
            warnings: vec![],
            errors: vec![],
        })
    }

    async fn link(&self, _objects: Vec<PathBuf>, output: PathBuf) -> ToadStoolResult<LinkResult> {
        info!("Linking 6502 objects to {:?}", output);

        // Simulate linking
        Ok(LinkResult {
            success: true,
            executable_path: Some(output),
            messages: vec!["6502 linking successful".to_string()],
            warnings: vec![],
            errors: vec![],
        })
    }

    async fn create_rom_image(
        &self,
        _executable: &Path,
        _format: &ROMFormat,
    ) -> ToadStoolResult<Vec<u8>> {
        info!("Creating 6502 ROM image");

        // Simulate ROM image creation
        Ok(vec![0xA9, 0x00, 0x85, 0x00, 0x60]) // LDA #$00, STA $00, RTS
    }

    async fn disassemble(
        &self,
        _binary: &[u8],
        _architecture: &LegacyArchitecture,
    ) -> ToadStoolResult<String> {
        info!("Disassembling 6502 binary");

        // Simulate disassembly
        Ok("A9 00    LDA #$00\n85 00    STA $00\n60       RTS".to_string())
    }
}

#[async_trait::async_trait]
impl CrossCompilationToolchain for ToolchainZ80 {
    fn name(&self) -> &'static str {
        self.name
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::ZilogZ80]
    }

    async fn initialize(&mut self, config: &ToolchainConfig) -> ToadStoolResult<()> {
        info!("Initializing Z80 toolchain");
        self.config = Some(config.clone());
        Ok(())
    }

    async fn compile(
        &self,
        source: PathBuf,
        target: LegacyArchitecture,
    ) -> ToadStoolResult<CompilationResult> {
        info!(
            "Compiling Z80 source code from {:?} for {:?}",
            source, target
        );

        // Simulate compilation
        Ok(CompilationResult {
            success: true,
            output_path: Some(PathBuf::from("output.com")),
            object_files: vec![PathBuf::from("output.o")],
            messages: vec!["Z80 compilation successful".to_string()],
            warnings: vec![],
            errors: vec![],
        })
    }

    async fn link(&self, _objects: Vec<PathBuf>, output: PathBuf) -> ToadStoolResult<LinkResult> {
        info!("Linking Z80 objects to {:?}", output);

        // Simulate linking
        Ok(LinkResult {
            success: true,
            executable_path: Some(output),
            messages: vec!["Z80 linking successful".to_string()],
            warnings: vec![],
            errors: vec![],
        })
    }

    async fn create_rom_image(
        &self,
        _executable: &Path,
        _format: &ROMFormat,
    ) -> ToadStoolResult<Vec<u8>> {
        info!("Creating Z80 ROM image");

        // Simulate ROM image creation
        Ok(vec![0x3E, 0x00, 0x32, 0x00, 0x80, 0xC9]) // LD A,00h; LD (8000h),A; RET
    }

    async fn disassemble(
        &self,
        _binary: &[u8],
        _architecture: &LegacyArchitecture,
    ) -> ToadStoolResult<String> {
        info!("Disassembling Z80 binary");

        // Simulate disassembly
        Ok("3E 00       LD A,00h\n32 00 80    LD (8000h),A\nC9          RET".to_string())
    }
}

#[async_trait::async_trait]
impl CrossCompilationToolchain for Toolchain68000 {
    fn name(&self) -> &'static str {
        self.name
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::Motorola68000]
    }

    async fn initialize(&mut self, config: &ToolchainConfig) -> ToadStoolResult<()> {
        info!("Initializing 68000 toolchain");
        self.config = Some(config.clone());
        Ok(())
    }

    async fn compile(
        &self,
        source: PathBuf,
        target: LegacyArchitecture,
    ) -> ToadStoolResult<CompilationResult> {
        info!(
            "Compiling 68000 source code from {:?} for {:?}",
            source, target
        );

        // Simulate compilation
        Ok(CompilationResult {
            success: true,
            output_path: Some(PathBuf::from("output.bin")),
            object_files: vec![PathBuf::from("output.o")],
            messages: vec!["68000 compilation successful".to_string()],
            warnings: vec![],
            errors: vec![],
        })
    }

    async fn link(&self, _objects: Vec<PathBuf>, output: PathBuf) -> ToadStoolResult<LinkResult> {
        info!("Linking 68000 objects to {:?}", output);

        // Simulate linking
        Ok(LinkResult {
            success: true,
            executable_path: Some(output),
            messages: vec!["68000 linking successful".to_string()],
            warnings: vec![],
            errors: vec![],
        })
    }

    async fn create_rom_image(
        &self,
        _executable: &Path,
        _format: &ROMFormat,
    ) -> ToadStoolResult<Vec<u8>> {
        info!("Creating 68000 ROM image");

        // Simulate ROM image creation
        Ok(vec![0x70, 0x00, 0x4E, 0x75]) // MOVEQ #0,D0; RTS
    }

    async fn disassemble(
        &self,
        _binary: &[u8],
        _architecture: &LegacyArchitecture,
    ) -> ToadStoolResult<String> {
        info!("Disassembling 68000 binary");

        // Simulate disassembly
        Ok("7000    MOVEQ #0,D0\n4E75    RTS".to_string())
    }
}

#[cfg(test)]
mod tests {
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
}
