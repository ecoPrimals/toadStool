// SPDX-License-Identifier: AGPL-3.0-only
//! Compilation configuration types for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Target formats for compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TargetFormat {
    /// Executable program
    Executable,
    /// Object file
    Object,
    /// Library
    Library,
    /// ROM image
    ROMImage,
    /// Disk image
    DiskImage,
}

/// Optimization levels for compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationLevel {
    /// No optimization
    None,
    /// Basic optimization
    Basic,
    /// Standard optimization
    Standard,
    /// Maximum optimization
    Maximum,
}

/// Toolchain configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainConfig {
    /// Toolchain name
    pub name: String,
    /// Toolchain path
    pub path: PathBuf,
    /// Compiler executable
    pub compiler: String,
    /// Linker executable
    pub linker: String,
    /// Assembler executable
    pub assembler: String,
    /// Archiver executable
    pub archiver: String,
    /// Debugger executable
    pub debugger: Option<String>,
    /// Cross-compilation target
    pub target: String,
    /// Environment variables
    pub environment: HashMap<String, String>,
}
