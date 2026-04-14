// SPDX-License-Identifier: AGPL-3.0-or-later
//! Embedded cross-compilation toolchain trait implementations.
//!
//! Each struct satisfies the [`EmbeddedToolchain`] trait for a specific legacy
//! architecture.  Operations return [`ToadStoolError::not_supported`] until the
//! corresponding cross-compiler is discovered at runtime.
//!
//! ## Evolution Path
//!
//! | Arch   | Compiler         | Status       |
//! |--------|------------------|--------------|
//! | 6502   | cc65 / WDC816    | Pending — runtime discovery needed |
//! | Z80    | z88dk / SDCC     | Pending — runtime discovery needed |
//! | 8080   | z88dk (shared)   | Pending — runtime discovery needed |
//! | 8051   | SDCC             | Pending — runtime discovery needed |
//! | 8086   | NASM / MASM      | Pending — runtime discovery needed |
//! | 68000  | vasm / gcc-m68k  | Pending — runtime discovery needed |
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

fn toolchain_unavailable(feature: impl Into<String>) -> ToadStoolError {
    ToadStoolError::not_supported(format!(
        "{}: cross-compilation toolchain not discovered on this system",
        feature.into()
    ))
}

/// MOS 6502 cross-compilation toolchain (cc65 / WDC816).
///
/// Returns `not_supported` until the compiler is discovered at runtime.
#[derive(Debug)]
pub struct Toolchain6502;

/// Zilog Z80 cross-compilation toolchain (z88dk / SDCC).
///
/// Returns `not_supported` until the compiler is discovered at runtime.
#[derive(Debug)]
pub struct ToolchainZ80;

/// Intel 8080 cross-compilation toolchain (shared with Z80 tooling).
///
/// Returns `not_supported` until the compiler is discovered at runtime.
#[derive(Debug)]
pub struct Toolchain8080;

/// Intel 8051 cross-compilation toolchain (SDCC).
///
/// Returns `not_supported` until the compiler is discovered at runtime.
#[derive(Debug)]
pub struct Toolchain8051;

/// Intel 8086 cross-compilation toolchain (NASM / MASM).
///
/// Returns `not_supported` until the compiler is discovered at runtime.
#[derive(Debug)]
pub struct Toolchain8086;

/// Motorola 68000 cross-compilation toolchain (vasm / gcc-m68k).
///
/// Returns `not_supported` until the compiler is discovered at runtime.
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
    fn name(&self) -> &'static str {
        "6502 Toolchain"
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::MOS6502]
    }

    async fn initialize(&mut self, _config: &EmbeddedConfig) -> ToadStoolResult<()> {
        Err(toolchain_unavailable("6502 initialization"))
    }

    async fn compile(
        &self,
        _sources: &[SourceFile],
        _output_path: &Path,
    ) -> ToadStoolResult<CompilationResult> {
        Err(toolchain_unavailable("6502 compilation"))
    }

    async fn link(
        &self,
        _objects: &[PathBuf],
        _output_path: &Path,
        _memory_layout: &MemoryLayout,
    ) -> ToadStoolResult<LinkResult> {
        Err(toolchain_unavailable("6502 linking"))
    }

    async fn generate_rom_image(
        &self,
        _executable: &Path,
        _format: OutputFileType,
    ) -> ToadStoolResult<Vec<u8>> {
        Err(toolchain_unavailable("6502 ROM image generation"))
    }

    async fn disassemble(&self, _binary: &[u8], _start_address: u32) -> ToadStoolResult<String> {
        Err(toolchain_unavailable("6502 disassembly"))
    }

    async fn create_memory_map(&self, _executable: &Path) -> ToadStoolResult<MemoryMap> {
        Err(toolchain_unavailable("6502 memory map creation"))
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

macro_rules! impl_pending_toolchain {
    ($toolchain:ty, $name:expr, $arch:expr) => {
        // NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
        #[async_trait]
        impl EmbeddedToolchain for $toolchain {
            fn name(&self) -> &'static str {
                $name
            }

            fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
                vec![$arch]
            }

            async fn initialize(&mut self, _config: &EmbeddedConfig) -> ToadStoolResult<()> {
                Err(toolchain_unavailable(format!("{} initialization", $name)))
            }

            async fn compile(
                &self,
                _sources: &[SourceFile],
                _output_path: &Path,
            ) -> ToadStoolResult<CompilationResult> {
                Err(toolchain_unavailable(format!("{} compilation", $name)))
            }

            async fn link(
                &self,
                _objects: &[PathBuf],
                _output_path: &Path,
                _memory_layout: &MemoryLayout,
            ) -> ToadStoolResult<LinkResult> {
                Err(toolchain_unavailable(format!("{} linking", $name)))
            }

            async fn generate_rom_image(
                &self,
                _executable: &Path,
                _format: OutputFileType,
            ) -> ToadStoolResult<Vec<u8>> {
                Err(toolchain_unavailable(format!(
                    "{} ROM image generation",
                    $name
                )))
            }

            async fn disassemble(
                &self,
                _binary: &[u8],
                _start_address: u32,
            ) -> ToadStoolResult<String> {
                Err(toolchain_unavailable(format!("{} disassembly", $name)))
            }

            async fn create_memory_map(&self, _executable: &Path) -> ToadStoolResult<MemoryMap> {
                Err(toolchain_unavailable(format!(
                    "{} memory map creation",
                    $name
                )))
            }
        }
    };
}

impl_pending_toolchain!(ToolchainZ80, "Z80 Toolchain", LegacyArchitecture::ZilogZ80);
impl_pending_toolchain!(
    Toolchain8080,
    "8080 Toolchain",
    LegacyArchitecture::Intel8080
);
impl_pending_toolchain!(
    Toolchain8051,
    "8051 Toolchain",
    LegacyArchitecture::Intel8051
);
impl_pending_toolchain!(
    Toolchain8086,
    "8086 Toolchain",
    LegacyArchitecture::Intel8086
);
impl_pending_toolchain!(
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

#[cfg(test)]
#[path = "toolchains_tests.rs"]
mod tests;
