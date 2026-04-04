// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::types::configs::{
    CompilationToolchainConfig as ToolchainConfig, ConfigEmulationConfig, EmbeddedConfig,
    IndustrialConfig, MainframeConfig, RealtimeConfig,
};
use crate::types::systems::{LegacyArchitecture, LegacySystemType};

/// Configuration for the specialty hardware runtime engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "bool fields map directly to hardware flags"
)]
pub struct SpecialtyRuntimeConfig {
    /// Whether mainframe system support is enabled.
    pub mainframe_enabled: bool,
    /// Whether embedded system support is enabled.
    pub embedded_enabled: bool,
    /// Whether industrial control system support is enabled.
    pub industrial_enabled: bool,
    /// Whether real-time OS support is enabled.
    pub realtime_enabled: bool,
    /// Whether cross-compilation toolchains are available.
    pub cross_compilation_enabled: bool,
    /// Whether emulation of legacy architectures is enabled.
    pub emulation_enabled: bool,
    /// Maximum number of jobs that can run concurrently.
    pub max_concurrent_jobs: usize,
    /// Timeout for individual job execution.
    pub job_timeout: Duration,
    /// Timeout for inter-process communication.
    pub communication_timeout: Duration,
    /// List of legacy system types this runtime supports.
    pub supported_systems: Vec<LegacySystemType>,
    /// Toolchain configs keyed by legacy architecture.
    pub toolchain_configs: HashMap<LegacyArchitecture, ToolchainConfig>,
    /// Mainframe-specific configurations by name.
    pub mainframe_configs: HashMap<String, MainframeConfig>,
    /// Embedded system configurations by name.
    pub embedded_configs: HashMap<String, EmbeddedConfig>,
    /// Industrial system configurations by name.
    pub industrial_configs: HashMap<String, IndustrialConfig>,
    /// Real-time system configurations by name.
    pub realtime_configs: HashMap<String, RealtimeConfig>,
    /// Emulation configs keyed by legacy system type.
    pub emulation_configs: HashMap<LegacySystemType, ConfigEmulationConfig>,
}

impl Default for SpecialtyRuntimeConfig {
    fn default() -> Self {
        Self {
            mainframe_enabled: true,
            embedded_enabled: true,
            industrial_enabled: true,
            realtime_enabled: true,
            cross_compilation_enabled: true,
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
