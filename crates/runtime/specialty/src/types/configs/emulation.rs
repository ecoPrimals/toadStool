// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emulation configuration types for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::storage::{DiskImage, ROMFile};

/// Emulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulationConfig {
    /// Emulator type
    pub emulator_type: EmulatorType,
    /// Emulator path
    pub emulator_path: PathBuf,
    /// Emulator parameters
    pub parameters: HashMap<String, String>,
    /// ROM/BIOS files
    pub rom_files: Vec<ROMFile>,
    /// Disk images
    pub disk_images: Vec<DiskImage>,
}

/// Emulator types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmulatorType {
    /// SIMH emulator
    SIMH,
    /// MAME emulator
    MAME,
    /// MESS emulator
    MESS,
    /// Virtual machine
    VirtualMachine,
    /// Custom emulator
    Custom { name: String },
}
