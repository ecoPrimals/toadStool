// SPDX-License-Identifier: AGPL-3.0-only
//! Mainframe configuration types for legacy systems

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use super::communication::ConnectionSettings;
use crate::LegacySystemType;

/// Mainframe configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainframeConfig {
    /// Mainframe type
    pub system_type: LegacySystemType,
    /// Connection settings
    pub connection: ConnectionSettings,
    /// Dataset configuration
    pub datasets: HashMap<String, DatasetConfig>,
    /// JCL settings
    pub jcl_settings: JCLSettings,
    /// COBOL settings
    pub cobol_settings: COBOLSettings,
}

/// Dataset configuration for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// Dataset name
    pub name: String,
    /// Dataset type
    pub dataset_type: DatasetType,
    /// Record format
    pub record_format: RecordFormat,
    /// Record length
    pub record_length: u32,
    /// Block size
    pub block_size: u32,
    /// Space allocation
    pub space_allocation: SpaceAllocation,
}

/// Dataset types for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatasetType {
    /// Sequential dataset
    Sequential,
    /// Partitioned dataset
    Partitioned,
    /// Indexed dataset
    Indexed,
    /// Direct access dataset
    DirectAccess,
    /// VSAM dataset
    VSAM,
}

/// Record formats for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecordFormat {
    /// Fixed length
    Fixed,
    /// Variable length
    Variable,
    /// Fixed blocked
    FixedBlocked,
    /// Variable blocked
    VariableBlocked,
    /// Undefined
    Undefined,
}

/// Space allocation for datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpaceAllocation {
    /// Primary space
    pub primary: u64,
    /// Secondary space
    pub secondary: u64,
    /// Space unit
    pub unit: SpaceUnit,
}

/// Space units for datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpaceUnit {
    /// Tracks
    Tracks,
    /// Cylinders
    Cylinders,
    /// Blocks
    Blocks,
    /// Bytes
    Bytes,
}

/// JCL settings for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JCLSettings {
    /// Job class
    pub job_class: String,
    /// Message class
    pub message_class: String,
    /// Priority
    pub priority: u8,
    /// Time limit
    pub time_limit: Duration,
    /// Region size
    pub region_size: u64,
}

/// COBOL settings for mainframes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct COBOLSettings {
    /// COBOL compiler
    pub compiler: String,
    /// Compilation options
    pub compile_options: Vec<String>,
    /// Link options
    pub link_options: Vec<String>,
    /// Runtime options
    pub runtime_options: Vec<String>,
}
