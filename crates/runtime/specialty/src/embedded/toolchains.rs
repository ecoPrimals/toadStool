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

use std::future::{Future, ready};
use std::path::{Path, PathBuf};
use std::pin::Pin;

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

impl EmbeddedToolchain for Toolchain6502 {
    fn name(&self) -> &'static str {
        "6502 Toolchain"
    }

    fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
        vec![LegacyArchitecture::MOS6502]
    }

    fn initialize<'a>(
        &'a mut self,
        _config: &'a EmbeddedConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
        Box::pin(ready(Err(toolchain_unavailable("6502 initialization"))))
    }

    fn compile<'a>(
        &'a self,
        _sources: &'a [SourceFile],
        _output_path: &'a Path,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<CompilationResult>> + Send + 'a>> {
        Box::pin(ready(Err(toolchain_unavailable("6502 compilation"))))
    }

    fn link<'a>(
        &'a self,
        _objects: &'a [PathBuf],
        _output_path: &'a Path,
        _memory_layout: &'a MemoryLayout,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<LinkResult>> + Send + 'a>> {
        Box::pin(ready(Err(toolchain_unavailable("6502 linking"))))
    }

    fn generate_rom_image<'a>(
        &'a self,
        _executable: &'a Path,
        _format: OutputFileType,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a>> {
        Box::pin(ready(Err(toolchain_unavailable(
            "6502 ROM image generation",
        ))))
    }

    fn disassemble<'a>(
        &'a self,
        _binary: &'a [u8],
        _start_address: u32,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + 'a>> {
        Box::pin(ready(Err(toolchain_unavailable("6502 disassembly"))))
    }

    fn create_memory_map<'a>(
        &'a self,
        _executable: &'a Path,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<MemoryMap>> + Send + 'a>> {
        Box::pin(ready(Err(toolchain_unavailable(
            "6502 memory map creation",
        ))))
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
        impl EmbeddedToolchain for $toolchain {
            fn name(&self) -> &'static str {
                $name
            }

            fn supported_architectures(&self) -> Vec<LegacyArchitecture> {
                vec![$arch]
            }

            fn initialize<'a>(
                &'a mut self,
                _config: &'a EmbeddedConfig,
            ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + 'a>> {
                Box::pin(ready(Err(toolchain_unavailable(format!(
                    "{} initialization",
                    $name
                )))))
            }

            fn compile<'a>(
                &'a self,
                _sources: &'a [SourceFile],
                _output_path: &'a Path,
            ) -> Pin<Box<dyn Future<Output = ToadStoolResult<CompilationResult>> + Send + 'a>> {
                Box::pin(ready(Err(toolchain_unavailable(format!(
                    "{} compilation",
                    $name
                )))))
            }

            fn link<'a>(
                &'a self,
                _objects: &'a [PathBuf],
                _output_path: &'a Path,
                _memory_layout: &'a MemoryLayout,
            ) -> Pin<Box<dyn Future<Output = ToadStoolResult<LinkResult>> + Send + 'a>> {
                Box::pin(ready(Err(toolchain_unavailable(format!(
                    "{} linking",
                    $name
                )))))
            }

            fn generate_rom_image<'a>(
                &'a self,
                _executable: &'a Path,
                _format: OutputFileType,
            ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a>> {
                Box::pin(ready(Err(toolchain_unavailable(format!(
                    "{} ROM image generation",
                    $name
                )))))
            }

            fn disassemble<'a>(
                &'a self,
                _binary: &'a [u8],
                _start_address: u32,
            ) -> Pin<Box<dyn Future<Output = ToadStoolResult<String>> + Send + 'a>> {
                Box::pin(ready(Err(toolchain_unavailable(format!(
                    "{} disassembly",
                    $name
                )))))
            }

            fn create_memory_map<'a>(
                &'a self,
                _executable: &'a Path,
            ) -> Pin<Box<dyn Future<Output = ToadStoolResult<MemoryMap>> + Send + 'a>> {
                Box::pin(ready(Err(toolchain_unavailable(format!(
                    "{} memory map creation",
                    $name
                )))))
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
