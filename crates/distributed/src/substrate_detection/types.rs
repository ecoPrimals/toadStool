// SPDX-License-Identifier: AGPL-3.0-only
//! Platform and capability type definitions for substrate detection.

use serde::{Deserialize, Serialize};

/// Platform type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformType {
    Linux {
        distribution: String,
        architecture: String,
    },
    Windows {
        version: String,
        architecture: String,
    },
    MacOS {
        version: String,
        architecture: String,
    },
    Docker,
    Podman,
    Containerd,
    Language {
        name: String,
        command: String,
    },
    GPU {
        vendor: String,
        framework: String,
    },
    WebAssembly {
        runtime: String,
    },
    Other {
        os: String,
        architecture: String,
    },
    EdgeDevice {
        device_type: String,
        architecture: String,
    },
    MCUDevelopment {
        platform: String,
        tool: String,
    },
    BiologicalComputing {
        platform: String,
        simulation: bool,
    },
    Quantum {
        framework: String,
        simulator: bool,
    },
    NeuromorphicComputing {
        platform: String,
        hardware: bool,
    },
}

/// Comprehensive substrate capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    pub traditional_platforms: Vec<PlatformType>,
    pub container_platforms: Vec<PlatformType>,
    pub language_runtimes: Vec<PlatformType>,
    pub gpu_platforms: Vec<PlatformType>,
    pub specialized_platforms: Vec<PlatformType>,
    pub experimental_platforms: Vec<PlatformType>,
}

impl SubstrateCapabilities {
    #[must_use]
    pub const fn total_platforms(&self) -> usize {
        self.traditional_platforms.len()
            + self.container_platforms.len()
            + self.language_runtimes.len()
            + self.gpu_platforms.len()
            + self.specialized_platforms.len()
            + self.experimental_platforms.len()
    }

    #[must_use]
    pub const fn has_containers(&self) -> bool {
        !self.container_platforms.is_empty()
    }

    #[must_use]
    pub const fn has_gpu(&self) -> bool {
        !self.gpu_platforms.is_empty()
    }

    #[must_use]
    pub fn has_wasm(&self) -> bool {
        self.specialized_platforms
            .iter()
            .any(|p| matches!(p, PlatformType::WebAssembly { .. }))
    }
}
