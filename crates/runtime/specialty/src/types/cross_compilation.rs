//! Cross-compilation type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::LegacyArchitecture;
use crate::ToadStoolResult;
use std::future::Future;

/// Cross-compilation toolchain trait for legacy architectures
#[async_trait::async_trait]
pub trait CrossCompilationToolchain: Send + Sync {
    /// Get the toolchain name
    fn name(&self) -> &str;
    
    /// Get supported architectures
    fn supported_architectures(&self) -> Vec<LegacyArchitecture>;
    
    /// Initialize the toolchain
    async fn initialize(&mut self, config: &ToolchainConfig) -> ToadStoolResult<()>;
    
    /// Compile source code
    async fn compile(&self, source: PathBuf, target: LegacyArchitecture) -> ToadStoolResult<CompilationResult>;
    
    /// Link object files
    async fn link(&self, objects: Vec<PathBuf>, output: PathBuf) -> ToadStoolResult<LinkResult>;
    
    /// Create ROM image from executable
    async fn create_rom_image(&self, executable: &PathBuf, format: &super::configs::ROMFormat) -> ToadStoolResult<Vec<u8>>;
    
    /// Disassemble binary
    async fn disassemble(&self, binary: &[u8], architecture: &LegacyArchitecture) -> ToadStoolResult<String>;
}

/// Toolchain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainConfig {
    /// Compiler flags
    pub compiler_flags: Vec<String>,
    /// Linker flags
    pub linker_flags: Vec<String>,
    /// Include paths
    pub include_paths: Vec<PathBuf>,
    /// Library paths
    pub library_paths: Vec<PathBuf>,
    /// Environment variables
    pub environment: HashMap<String, String>,
}

impl Default for ToolchainConfig {
    fn default() -> Self {
        Self {
            compiler_flags: Vec::new(),
            linker_flags: Vec::new(),
            include_paths: Vec::new(),
            library_paths: Vec::new(),
            environment: HashMap::new(),
        }
    }
}

/// Compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    /// Whether compilation succeeded
    pub success: bool,
    /// Output file path
    pub output_path: Option<PathBuf>,
    /// Object files produced
    pub object_files: Vec<PathBuf>,
    /// Compilation messages
    pub messages: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Errors
    pub errors: Vec<String>,
}

/// Link result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkResult {
    /// Whether linking succeeded
    pub success: bool,
    /// Output executable path
    pub executable_path: Option<PathBuf>,
    /// Link messages
    pub messages: Vec<String>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Errors
    pub errors: Vec<String>,
}

