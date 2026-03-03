// SPDX-License-Identifier: AGPL-3.0-or-later
//! Toolchain implementations for embedded systems
//!
//! Error-returning stubs until cross-compilation toolchains are integrated.
//! All compile/link/disassemble operations return `not_supported`.

use async_trait::async_trait;
use std::path::PathBuf;

use toadstool::ToadStoolError;

use crate::{EmbeddedConfig, LegacyArchitecture, ToadStoolResult};

use super::types::{
    CompilationResult, EmbeddedToolchain, LinkResult, MemoryLayout, OutputFileType,
    SourceFile,
};

fn not_implemented(feature: impl Into<String>) -> ToadStoolError {
    ToadStoolError::not_supported(format!(
        "{} not yet implemented; requires cross-compilation toolchain integration",
        feature.into()
    ))
}

/// Toolchain for 6502 (stub until cc65/wdc816 integration)
pub struct Toolchain6502;

/// Toolchain for Z80 (stub until z88dk/SDCC integration)
pub struct ToolchainZ80;

/// Toolchain for 8080 (stub)
pub struct Toolchain8080;

/// Toolchain for 8051 (stub until SDCC integration)
pub struct Toolchain8051;

/// Toolchain for 8086 (stub until NASM/MASM integration)
pub struct Toolchain8086;

/// Toolchain for 68000 (stub until vasm/gcc-m68k integration)
pub struct Toolchain68000;

impl Toolchain6502 {
    pub fn new() -> Self {
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
        _output_path: &PathBuf,
    ) -> ToadStoolResult<CompilationResult> {
        Err(not_implemented("6502 compilation"))
    }

    async fn link(
        &self,
        _objects: &[PathBuf],
        _output_path: &PathBuf,
        _memory_layout: &MemoryLayout,
    ) -> ToadStoolResult<LinkResult> {
        Err(not_implemented("6502 linking"))
    }

    async fn generate_rom_image(
        &self,
        _executable: &PathBuf,
        _format: OutputFileType,
    ) -> ToadStoolResult<Vec<u8>> {
        Err(not_implemented("ROM image generation"))
    }

    async fn disassemble(&self, _binary: &[u8], _start_address: u32) -> ToadStoolResult<String> {
        Err(not_implemented("6502 disassembly"))
    }
}

impl ToolchainZ80 {
    /// Create a new Z80 toolchain
    pub fn new() -> Self {
        Self
    }
}

impl Toolchain8080 {
    /// Create a new 8080 toolchain
    pub fn new() -> Self {
        Self
    }
}

impl Toolchain8051 {
    /// Create a new 8051 toolchain
    pub fn new() -> Self {
        Self
    }
}

impl Toolchain8086 {
    /// Create a new 8086 toolchain
    pub fn new() -> Self {
        Self
    }
}

impl Toolchain68000 {
    /// Create a new 68000 toolchain
    pub fn new() -> Self {
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
                _output_path: &PathBuf,
            ) -> ToadStoolResult<CompilationResult> {
                Err(not_implemented(format!("{} compilation", $name)))
            }

            async fn link(
                &self,
                _objects: &[PathBuf],
                _output_path: &PathBuf,
                _memory_layout: &MemoryLayout,
            ) -> ToadStoolResult<LinkResult> {
                Err(not_implemented(format!("{} linking", $name)))
            }

            async fn generate_rom_image(
                &self,
                _executable: &PathBuf,
                _format: OutputFileType,
            ) -> ToadStoolResult<Vec<u8>> {
                Err(not_implemented("ROM image generation"))
            }

            async fn disassemble(&self, _binary: &[u8], _start_address: u32) -> ToadStoolResult<String> {
                Err(not_implemented(format!("{} disassembly", $name)))
            }
        }
    };
}

impl_toolchain_stub!(ToolchainZ80, "Z80 Toolchain", LegacyArchitecture::Zilog_Z80);
impl_toolchain_stub!(Toolchain8080, "8080 Toolchain", LegacyArchitecture::Intel_8080);
impl_toolchain_stub!(Toolchain8051, "8051 Toolchain", LegacyArchitecture::Intel_8051);
impl_toolchain_stub!(Toolchain8086, "8086 Toolchain", LegacyArchitecture::Intel_8086);
impl_toolchain_stub!(Toolchain68000, "68000 Toolchain", LegacyArchitecture::Motorola_68000);

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


