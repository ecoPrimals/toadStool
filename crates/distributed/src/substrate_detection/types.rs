// SPDX-License-Identifier: AGPL-3.0-or-later
//! Platform and capability type definitions for substrate detection.

use serde::{Deserialize, Serialize};

/// Platform type for substrate detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlatformType {
    /// Linux OS.
    Linux {
        /// Distribution (ubuntu, debian, etc.).
        distribution: String,
        /// Architecture (x86_64, aarch64, etc.).
        architecture: String,
    },
    /// Windows OS.
    Windows {
        /// Windows version.
        version: String,
        /// Architecture.
        architecture: String,
    },
    /// macOS.
    MacOS {
        /// macOS version.
        version: String,
        /// Architecture (arm64, x86_64).
        architecture: String,
    },
    /// Docker container runtime.
    Docker,
    /// Podman container runtime.
    Podman,
    /// Containerd container runtime.
    Containerd,
    /// Language runtime.
    Language {
        /// Language name.
        name: String,
        /// Command to invoke.
        command: String,
    },
    /// GPU platform.
    GPU {
        /// Vendor (nvidia, amd, etc.).
        vendor: String,
        /// Framework (cuda, rocm, etc.).
        framework: String,
    },
    /// WebAssembly runtime.
    WebAssembly {
        /// Runtime (wasmtime, wasmer, etc.).
        runtime: String,
    },
    /// Other/unknown platform.
    Other {
        /// OS identifier.
        os: String,
        /// Architecture.
        architecture: String,
    },
    /// Edge IoT device.
    EdgeDevice {
        /// Device type.
        device_type: String,
        /// Architecture.
        architecture: String,
    },
    /// MCU/embedded development.
    MCUDevelopment {
        /// Platform (stm32, esp32, etc.).
        platform: String,
        /// Tool (arduino, platformio, etc.).
        tool: String,
    },
    /// Biological computing substrate.
    BiologicalComputing {
        /// Platform identifier.
        platform: String,
        /// Whether simulation or real hardware.
        simulation: bool,
    },
    /// Quantum computing substrate.
    Quantum {
        /// Framework (qiskit, cirq, etc.).
        framework: String,
        /// Simulator vs real backend.
        simulator: bool,
    },
    /// Neuromorphic computing substrate.
    NeuromorphicComputing {
        /// Platform (akida, loihi, etc.).
        platform: String,
        /// Hardware vs simulation.
        hardware: bool,
    },
}

/// Comprehensive substrate capabilities for detected platforms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubstrateCapabilities {
    /// Traditional OS platforms (Linux, Windows, macOS).
    pub traditional_platforms: Vec<PlatformType>,
    /// Container runtimes (Docker, Podman, etc.).
    pub container_platforms: Vec<PlatformType>,
    /// Language runtimes.
    pub language_runtimes: Vec<PlatformType>,
    /// GPU platforms.
    pub gpu_platforms: Vec<PlatformType>,
    /// Specialized platforms (WASM, edge, etc.).
    pub specialized_platforms: Vec<PlatformType>,
    /// Experimental platforms (quantum, neuromorphic, biological).
    pub experimental_platforms: Vec<PlatformType>,
}

impl SubstrateCapabilities {
    /// Returns total count of detected platforms.
    #[must_use]
    pub const fn total_platforms(&self) -> usize {
        self.traditional_platforms.len()
            + self.container_platforms.len()
            + self.language_runtimes.len()
            + self.gpu_platforms.len()
            + self.specialized_platforms.len()
            + self.experimental_platforms.len()
    }

    /// Returns true if container platforms are detected.
    #[must_use]
    pub const fn has_containers(&self) -> bool {
        !self.container_platforms.is_empty()
    }

    /// Returns true if GPU platforms are detected.
    #[must_use]
    pub const fn has_gpu(&self) -> bool {
        !self.gpu_platforms.is_empty()
    }

    /// Returns true if WebAssembly platform is detected.
    #[must_use]
    pub fn has_wasm(&self) -> bool {
        self.specialized_platforms
            .iter()
            .any(|p| matches!(p, PlatformType::WebAssembly { .. }))
    }
}
