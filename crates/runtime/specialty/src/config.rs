// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::types::configs::{CompilationToolchainConfig as ToolchainConfig, EmbeddedConfig, IndustrialConfig, MainframeConfig, RealtimeConfig};
use crate::types::emulation::EmulationConfig;
use crate::types::systems::{LegacyArchitecture, LegacySystemType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialtyRuntimeConfig {
    pub mainframe_enabled: bool,
    pub embedded_enabled: bool,
    pub industrial_enabled: bool,
    pub realtime_enabled: bool,
    pub cross_compilation_enabled: bool,
    pub legacy_networking_enabled: bool,
    pub emulation_enabled: bool,
    pub max_concurrent_jobs: usize,
    pub job_timeout: Duration,
    pub communication_timeout: Duration,
    pub supported_systems: Vec<LegacySystemType>,
    pub toolchain_configs: HashMap<LegacyArchitecture, ToolchainConfig>,
    pub mainframe_configs: HashMap<String, MainframeConfig>,
    pub embedded_configs: HashMap<String, EmbeddedConfig>,
    pub industrial_configs: HashMap<String, IndustrialConfig>,
    pub realtime_configs: HashMap<String, RealtimeConfig>,
    pub emulation_configs: HashMap<LegacySystemType, EmulationConfig>,
}

impl Default for SpecialtyRuntimeConfig {
    fn default() -> Self {
        Self {
            mainframe_enabled: true,
            embedded_enabled: true,
            industrial_enabled: true,
            realtime_enabled: true,
            cross_compilation_enabled: true,
            legacy_networking_enabled: true,
            emulation_enabled: true,
            max_concurrent_jobs: 10,
            job_timeout: Duration::from_secs(3600),
            communication_timeout: Duration::from_secs(30),
            supported_systems: Vec::new(),
            toolchain_configs: HashMap::new(),
            mainframe_configs: HashMap::new(),
            embedded_configs: HashMap::new(),
            industrial_configs: HashMap::new(),
            realtime_configs: HashMap::new(),
            emulation_configs: HashMap::new(),
        }
    }
}
