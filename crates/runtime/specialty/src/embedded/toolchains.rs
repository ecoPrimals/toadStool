// SPDX-License-Identifier: AGPL-3.0-only
//! Toolchain implementations for embedded systems
//!
//! ## Planned / Future Implementation
//!
//! These toolchain structs and trait implementations are **infrastructure placeholders**
//! for future embedded cross-compilation support. They are registered in the embedded
//! adapter registry and satisfy the type system, but all operations return
//! `not_supported` until real toolchains are integrated.
//!
//! ## Architecture Notes
//!
//! - **6502**: Planned integration with cc65 or WDC816 toolchain
//! - **Z80**: Planned integration with z88dk or SDCC
//! - **8080**: Legacy Intel; may share tooling with Z80
//! - **8051**: Planned SDCC integration
//! - **8086**: Planned NASM/MASM integration for x86 real mode
//! - **68000**: Planned vasm or gcc-m68k integration
//!
//! Each toolchain will require: cross-compiler binary discovery, platform-specific
//! linker scripts, and ROM image format generation (e.g., .nes, .sms, raw binary).

use async_trait::async_trait;
use std::path::{Path, PathBuf};

use toadstool::ToadStoolError;

use crate::{EmbeddedConfig, LegacyArchitecture, MemoryLayout, ToadStoolResult};

use super::types::{
    CompilationResult, EmbeddedToolchain, LinkResult, MemoryMap, OutputFileType, SourceFile,
};

fn not_implemented(feature: impl Into<String>) -> ToadStoolError {
    ToadStoolError::not_supported(format!(
        "{} not yet implemented; requires cross-compilation toolchain integration",
        feature.into()
    ))
}

/// Toolchain for 6502 (planned: cc65/wdc816 integration).
/// Stub until cross-compilation toolchain is integrated.
#[derive(Debug)]
pub struct Toolchain6502;

/// Toolchain for Z80 (planned: z88dk/SDCC integration).
/// Stub until cross-compilation toolchain is integrated.
#[derive(Debug)]
pub struct ToolchainZ80;

/// Toolchain for 8080 (planned: may share tooling with Z80).
/// Stub until cross-compilation toolchain is integrated.
#[derive(Debug)]
pub struct Toolchain8080;

/// Toolchain for 8051 (planned: SDCC integration).
/// Stub until cross-compilation toolchain is integrated.
#[derive(Debug)]
pub struct Toolchain8051;

/// Toolchain for 8086 (planned: NASM/MASM integration).
/// Stub until cross-compilation toolchain is integrated.
#[derive(Debug)]
pub struct Toolchain8086;

/// Toolchain for 68000 (planned: vasm/gcc-m68k integration).
/// Stub until cross-compilation toolchain is integrated.
#[derive(Debug)]
pub struct Toolchain68000;

impl Toolchain6502 {
    /// Creates a new 6502 embedded toolchain instance.
    pub const fn new() -> Self {
        Self
    }
}

// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl EmbeddedToolchain for Toolchain6502 {
    fn name(&self) -> &str {
        "6502 Toolchain"
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::MOS6502]
    }

    async fn initialize(&mut self, _config: &EmbeddedConfig) -> ToadStoolResult<()> {
        Err(not_implemented("Toolchain initialization"))
    }

    async fn compile(
        &self,
        _sources: &[SourceFile],
        _output_path: &Path,
    ) -> ToadStoolResult<CompilationResult> {
        Err(not_implemented("6502 compilation"))
    }

    async fn link(
        &self,
        _objects: &[PathBuf],
        _output_path: &Path,
        _memory_layout: &MemoryLayout,
    ) -> ToadStoolResult<LinkResult> {
        Err(not_implemented("6502 linking"))
    }

    async fn generate_rom_image(
        &self,
        _executable: &Path,
        _format: OutputFileType,
    ) -> ToadStoolResult<Vec<u8>> {
        Err(not_implemented("ROM image generation"))
    }

    async fn disassemble(&self, _binary: &[u8], _start_address: u32) -> ToadStoolResult<String> {
        Err(not_implemented("6502 disassembly"))
    }

    async fn create_memory_map(&self, _executable: &Path) -> ToadStoolResult<MemoryMap> {
        Err(not_implemented("Memory map creation"))
    }
}

impl ToolchainZ80 {
    /// Create a new Z80 toolchain
    pub const fn new() -> Self {
        Self
    }
}

impl Toolchain8080 {
    /// Create a new 8080 toolchain
    pub const fn new() -> Self {
        Self
    }
}

impl Toolchain8051 {
    /// Create a new 8051 toolchain
    pub const fn new() -> Self {
        Self
    }
}

impl Toolchain8086 {
    /// Create a new 8086 toolchain
    pub const fn new() -> Self {
        Self
    }
}

impl Toolchain68000 {
    /// Create a new 68000 toolchain
    pub const fn new() -> Self {
        Self
    }
}

macro_rules! impl_toolchain_stub {
    ($toolchain:ty, $name:expr, $arch:expr) => {
        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl EmbeddedToolchain for $toolchain {
            fn name(&self) -> &str {
                $name
            }

            fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
                vec![$arch]
            }

            async fn initialize(&mut self, _config: &EmbeddedConfig) -> ToadStoolResult<()> {
                Err(not_implemented("Toolchain initialization"))
            }

            async fn compile(
                &self,
                _sources: &[SourceFile],
                _output_path: &Path,
            ) -> ToadStoolResult<CompilationResult> {
                Err(not_implemented(format!("{} compilation", $name)))
            }

            async fn link(
                &self,
                _objects: &[PathBuf],
                _output_path: &Path,
                _memory_layout: &MemoryLayout,
            ) -> ToadStoolResult<LinkResult> {
                Err(not_implemented(format!("{} linking", $name)))
            }

            async fn generate_rom_image(
                &self,
                _executable: &Path,
                _format: OutputFileType,
            ) -> ToadStoolResult<Vec<u8>> {
                Err(not_implemented("ROM image generation"))
            }

            async fn disassemble(
                &self,
                _binary: &[u8],
                _start_address: u32,
            ) -> ToadStoolResult<String> {
                Err(not_implemented(format!("{} disassembly", $name)))
            }

            async fn create_memory_map(&self, _executable: &Path) -> ToadStoolResult<MemoryMap> {
                Err(not_implemented("Memory map creation"))
            }
        }
    };
}

impl_toolchain_stub!(ToolchainZ80, "Z80 Toolchain", LegacyArchitecture::ZilogZ80);
impl_toolchain_stub!(
    Toolchain8080,
    "8080 Toolchain",
    LegacyArchitecture::Intel8080
);
impl_toolchain_stub!(
    Toolchain8051,
    "8051 Toolchain",
    LegacyArchitecture::Intel8051
);
impl_toolchain_stub!(
    Toolchain8086,
    "8086 Toolchain",
    LegacyArchitecture::Intel8086
);
impl_toolchain_stub!(
    Toolchain68000,
    "68000 Toolchain",
    LegacyArchitecture::Motorola68000
);

impl Default for Toolchain6502 {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ToolchainZ80 {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Toolchain8080 {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Toolchain8051 {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Toolchain8086 {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Toolchain68000 {
    fn default() -> Self {
        Self::new()
    }
}
