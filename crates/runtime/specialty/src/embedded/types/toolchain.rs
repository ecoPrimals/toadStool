// SPDX-License-Identifier: AGPL-3.0-or-later
//! Toolchain trait, compile/link results, diagnostics, and memory map metadata.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::{
    LegacyArchitecture, MemoryLayout, MemoryPermissions, MemoryRegionType, ToadStoolResult,
};

use super::job::{OutputFile, OutputFileType, SourceFile};

/// Embedded toolchain trait
pub trait EmbeddedToolchain: Send + Sync + std::fmt::Debug {
    /// Get toolchain name
    fn name(&self) -> &'static str;

    /// Get supported architectures
    fn supported_architectures(&self) -> Vec<LegacyArchitecture>;

    /// Initialize toolchain
    fn initialize<'a>(
        &'a mut self,
        config: &'a crate::EmbeddedConfig,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Compile source code
    fn compile<'a>(
        &'a self,
        sources: &'a [SourceFile],
        output_path: &'a Path,
    ) -> impl Future<Output = ToadStoolResult<CompilationResult>> + Send + 'a;

    /// Link object files
    fn link<'a>(
        &'a self,
        objects: &'a [PathBuf],
        output_path: &'a Path,
        memory_layout: &'a MemoryLayout,
    ) -> impl Future<Output = ToadStoolResult<LinkResult>> + Send + 'a;

    /// Generate ROM image
    fn generate_rom_image<'a>(
        &'a self,
        executable: &'a Path,
        format: OutputFileType,
    ) -> impl Future<Output = ToadStoolResult<Vec<u8>>> + Send + 'a;

    /// Disassemble binary
    fn disassemble<'a>(
        &'a self,
        binary: &'a [u8],
        start_address: u32,
    ) -> impl Future<Output = ToadStoolResult<String>> + Send + 'a;

    /// Create memory map
    fn create_memory_map<'a>(
        &'a self,
        executable: &'a Path,
    ) -> impl Future<Output = ToadStoolResult<MemoryMap>> + Send + 'a;
}

/// Compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    /// Success flag
    pub success: bool,
    /// Output files
    pub output_files: Vec<OutputFile>,
    /// Compiler messages
    pub messages: Vec<CompilerMessage>,
    /// Compilation time
    pub compilation_time: Duration,
    /// Memory usage
    pub memory_usage: MemoryUsage,
}

/// Link result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkResult {
    /// Success flag
    pub success: bool,
    /// Output executable
    pub executable: Option<PathBuf>,
    /// Memory map
    pub memory_map: Option<MemoryMap>,
    /// Linker messages
    pub messages: Vec<LinkerMessage>,
    /// Link time
    pub link_time: Duration,
}

/// Compiler message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerMessage {
    /// Message type
    pub message_type: MessageType,
    /// Source file
    pub source_file: Option<PathBuf>,
    /// Line number
    pub line_number: Option<u32>,
    /// Column number
    pub column_number: Option<u32>,
    /// Message text
    pub message: String,
}

/// Linker message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkerMessage {
    /// Message type
    pub message_type: MessageType,
    /// Section name
    pub section: Option<String>,
    /// Symbol name
    pub symbol: Option<String>,
    /// Message text
    pub message: String,
}

/// Message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    /// Error message
    Error,
    /// Warning message
    Warning,
    /// Information message
    Info,
    /// Debug message
    Debug,
}

/// Memory usage information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// ROM/Flash usage
    pub rom_usage: RegionUsage,
    /// RAM usage
    pub ram_usage: RegionUsage,
    /// EEPROM usage
    pub eeprom_usage: Option<RegionUsage>,
}

/// Memory region usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionUsage {
    /// Used bytes
    pub used: u32,
    /// Total bytes
    pub total: u32,
    /// Usage percentage
    pub percentage: f32,
}

impl Default for RegionUsage {
    fn default() -> Self {
        Self {
            used: 0,
            total: 0,
            percentage: 0.0,
        }
    }
}

/// Memory map
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMap {
    /// Memory regions
    pub regions: Vec<MemoryMapRegion>,
    /// Symbols
    pub symbols: Vec<Symbol>,
    /// Sections
    pub sections: Vec<Section>,
}

/// Memory map region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMapRegion {
    /// Region name
    pub name: String,
    /// Start address
    pub start_address: u32,
    /// End address
    pub end_address: u32,
    /// Size
    pub size: u32,
    /// Region type
    pub region_type: MemoryRegionType,
    /// Permissions
    pub permissions: MemoryPermissions,
}

/// Symbol definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol address
    pub address: u32,
    /// Symbol size
    pub size: u32,
    /// Symbol type
    pub symbol_type: SymbolType,
    /// Symbol section
    pub section: Option<String>,
}

/// Symbol types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolType {
    /// Function symbol
    Function,
    /// Variable symbol
    Variable,
    /// Constant symbol
    Constant,
    /// Label symbol
    Label,
    /// Section symbol
    Section,
}

/// Section definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Section name
    pub name: String,
    /// Start address
    pub start_address: u32,
    /// Size
    pub size: u32,
    /// Section type
    pub section_type: SectionType,
    /// Alignment
    pub alignment: u32,
}

/// Section types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SectionType {
    /// Code section
    Code,
    /// Data section
    Data,
    /// BSS section
    BSS,
    /// Read-only data section
    ReadOnlyData,
    /// Stack section
    Stack,
    /// Heap section
    Heap,
    /// Custom section with user-defined name.
    Custom {
        /// Section name.
        name: String,
    },
}
