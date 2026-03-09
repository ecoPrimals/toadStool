// SPDX-License-Identifier: AGPL-3.0-only
//! Storage and media configuration types for legacy systems

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Paper tape formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaperTapeFormat {
    /// ASCII
    ASCII,
    /// Binary
    Binary,
    /// BASIC
    BASIC,
    /// Assembly
    Assembly,
    /// Custom format
    Custom { name: String },
}

/// ROM formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ROMFormat {
    /// Intel HEX
    IntelHex,
    /// Motorola S-record
    MotorolaS,
    /// Binary
    Binary,
    /// Custom format
    Custom { name: String },
}

/// ROM file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ROMFile {
    /// File name
    pub name: String,
    /// File path
    pub path: PathBuf,
    /// Load address
    pub load_address: u32,
    /// File size
    pub size: u64,
    /// Checksum
    pub checksum: String,
}

/// Disk image configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskImage {
    /// Image name
    pub name: String,
    /// Image path
    pub path: PathBuf,
    /// Image type
    pub image_type: DiskImageType,
    /// Image size
    pub size: u64,
    /// Read-only flag
    pub read_only: bool,
}

/// Disk image types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiskImageType {
    /// Raw disk image
    Raw,
    /// IMG file
    IMG,
    /// ISO file
    ISO,
    /// VDI file
    VDI,
    /// VMDK file
    VMDK,
    /// VHD file
    VHD,
    /// Custom format
    Custom(String),
}
