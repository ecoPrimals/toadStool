//! Emulation type definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::LegacySystemType;

use crate::ToadStoolResult;

/// Legacy system emulator trait
#[async_trait::async_trait]
pub trait LegacyEmulator: Send + Sync {
    /// Get emulator name
    fn name(&self) -> &str;
    
    /// Get supported systems
    fn supported_systems(&self) -> Vec<LegacySystemType>;
    
    /// Initialize the emulator
    async fn initialize(&mut self, config: &EmulationConfig) -> ToadStoolResult<()>;
    
    /// Start the emulator
    async fn start(&mut self) -> ToadStoolResult<()>;
    
    /// Stop the emulator
    async fn stop(&mut self) -> ToadStoolResult<()>;
    
    /// Reset the emulator
    async fn reset(&mut self) -> ToadStoolResult<()>;
    
    /// Load disk/ROM image
    async fn load_image(&mut self, image: &PathBuf) -> ToadStoolResult<()>;
    
    /// Save emulator state
    async fn save_state(&mut self, path: &PathBuf) -> ToadStoolResult<()>;
    
    /// Load emulator state
    async fn load_state(&mut self, path: &PathBuf) -> ToadStoolResult<()>;
    
    /// Get emulator status
    async fn get_status(&self) -> ToadStoolResult<EmulationStatus>;
}

/// Emulation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulationConfig {
    /// CPU speed (MHz)
    pub cpu_speed_mhz: Option<f64>,
    /// Memory size (bytes)
    pub memory_size: usize,
    /// Enable debugging
    pub enable_debugging: bool,
    /// ROM paths
    pub rom_paths: Vec<PathBuf>,
    /// Peripheral configuration
    pub peripherals: HashMap<String, PeripheralConfig>,
}

impl Default for EmulationConfig {
    fn default() -> Self {
        Self {
            cpu_speed_mhz: None,
            memory_size: 65536, // 64KB default
            enable_debugging: false,
            rom_paths: Vec::new(),
            peripherals: HashMap::new(),
        }
    }
}

/// Peripheral configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeripheralConfig {
    /// Peripheral type
    pub peripheral_type: String,
    /// Configuration options
    pub options: HashMap<String, String>,
}

/// Emulation status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmulationStatus {
    /// Emulator is not initialized
    Uninitialized,
    /// Emulator is ready
    Ready,
    /// Emulation is running
    Running,
    /// Emulation is paused
    Paused,
    /// Emulation stopped
    Stopped,
    /// Emulation encountered an error
    Error(String),
}

impl Default for EmulationStatus {
    fn default() -> Self {
        EmulationStatus::Uninitialized
    }
}

