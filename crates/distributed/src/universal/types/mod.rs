// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal substrate platform types
//!
//! This module provides comprehensive type definitions for all supported computing platforms,
//! from traditional CPU architectures to cutting-edge experimental systems.
//!
//! # Organization
//!
//! Types are organized by platform category:
//! - **Traditional**: x86_64, ARM64, RISC-V, PowerPC, SPARC, MIPS
//! - **Biological**: DNA computing, protein folding, cellular computing
//! - **Neuromorphic**: Spiking neural networks, memristive computing
//! - **Quantum**: Gate-based, annealing, photonic, trapped ion
//! - **Edge/IoT**: Microcontrollers, SBCs, sensors, FPGAs, NPUs
//! - **Container**: Docker, Podman, WASM runtimes, serverless, K8s
//! - **Language**: Runtime environments for 40+ programming languages
//! - **Operating System**: Unix-like, Windows, mobile, embedded, real-time
//! - **Specialized**: AI/ML accelerators, GPUs, DSPs, DPUs, ASICs
//! - **Experimental**: Molecular computing, spintronics, plasma computing
//!
//! # Example
//!
//! ```no_run
//! use toadstool_distributed::universal::*;
//!
//! let platform = TraditionalPlatform::X86_64 {
//!     cpu_model: "Intel Core i9".to_string(),
//!     cores: 16,
//!     threads: 32,
//!     cache_mb: 64,
//!     memory_gb: 64,
//!     features: vec!["AVX512".to_string()],
//! };
//!
//! assert_eq!(platform.architecture_name(), "x86_64");
//! assert!(platform.is_high_performance());
//! ```

use serde::{Deserialize, Serialize};

mod biological;
mod container;
mod edge_iot;
mod language;
#[cfg(test)]
mod language_tests;
mod neuromorphic;
mod operating_system;
mod quantum;
mod specialized;
mod traditional;

// Re-export all platform types
pub use biological::BiologicalComputingPlatform;
pub use container::ContainerPlatform;
pub use edge_iot::EdgeIoTPlatform;
pub use language::LanguageRuntime;
pub use neuromorphic::NeuromorphicPlatform;
pub use operating_system::OperatingSystemSupport;
pub use quantum::QuantumPlatform;
pub use specialized::{ExperimentalPlatform, SpecializedArchitecture};
pub use traditional::TraditionalPlatform;

/// Universal substrate capabilities for all computing platforms
///
/// This is the top-level container for all detected platform capabilities.
/// It aggregates all platform types into a single, queryable structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct UniversalSubstrateCapabilities {
    /// Traditional computing platforms
    pub traditional_platforms: Vec<TraditionalPlatform>,

    /// Biological computing platforms
    pub biological_platforms: Vec<BiologicalComputingPlatform>,

    /// Neuromorphic computing platforms
    pub neuromorphic_platforms: Vec<NeuromorphicPlatform>,

    /// Quantum computing platforms
    pub quantum_platforms: Vec<QuantumPlatform>,

    /// Edge/IoT platforms
    pub edge_iot_platforms: Vec<EdgeIoTPlatform>,

    /// Container platforms
    pub container_platforms: Vec<ContainerPlatform>,

    /// Language runtimes
    pub language_runtimes: Vec<LanguageRuntime>,

    /// Operating system support
    pub operating_systems: Vec<OperatingSystemSupport>,

    /// Specialized architectures
    pub specialized_architectures: Vec<SpecializedArchitecture>,

    /// Experimental platforms
    pub experimental_platforms: Vec<ExperimentalPlatform>,
}

impl UniversalSubstrateCapabilities {
    /// Create a new empty capabilities structure
    pub const fn new() -> Self {
        Self {
            traditional_platforms: Vec::new(),
            biological_platforms: Vec::new(),
            neuromorphic_platforms: Vec::new(),
            quantum_platforms: Vec::new(),
            edge_iot_platforms: Vec::new(),
            container_platforms: Vec::new(),
            language_runtimes: Vec::new(),
            operating_systems: Vec::new(),
            specialized_architectures: Vec::new(),
            experimental_platforms: Vec::new(),
        }
    }

    /// Get total number of detected platforms
    pub const fn total_platforms(&self) -> usize {
        self.traditional_platforms.len()
            + self.biological_platforms.len()
            + self.neuromorphic_platforms.len()
            + self.quantum_platforms.len()
            + self.edge_iot_platforms.len()
            + self.container_platforms.len()
            + self.language_runtimes.len()
            + self.operating_systems.len()
            + self.specialized_architectures.len()
            + self.experimental_platforms.len()
    }

    /// Check if any platforms were detected
    pub const fn is_empty(&self) -> bool {
        self.total_platforms() == 0
    }

    /// Check if traditional platforms are available
    pub const fn has_traditional_platforms(&self) -> bool {
        !self.traditional_platforms.is_empty()
    }

    /// Check if container platforms are available
    pub const fn has_container_platforms(&self) -> bool {
        !self.container_platforms.is_empty()
    }

    /// Check if language runtimes are available
    pub const fn has_language_runtimes(&self) -> bool {
        !self.language_runtimes.is_empty()
    }

    /// Check if operating systems are available
    pub const fn has_operating_systems(&self) -> bool {
        !self.operating_systems.is_empty()
    }

    /// Check if AI/ML accelerators are available
    pub fn has_ai_accelerators(&self) -> bool {
        self.specialized_architectures
            .iter()
            .any(SpecializedArchitecture::is_ai_accelerator)
    }

    /// Check if quantum platforms are available
    pub const fn has_quantum_platforms(&self) -> bool {
        !self.quantum_platforms.is_empty()
    }

    /// Check if experimental platforms are available
    pub const fn has_experimental_platforms(&self) -> bool {
        !self.experimental_platforms.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_capabilities() {
        let caps = UniversalSubstrateCapabilities::new();

        assert!(caps.is_empty());
        assert_eq!(caps.total_platforms(), 0);
        assert!(!caps.has_traditional_platforms());
    }

    #[test]
    fn test_capabilities_with_platforms() {
        let mut caps = UniversalSubstrateCapabilities::new();

        caps.traditional_platforms
            .push(TraditionalPlatform::X86_64 {
                cpu_model: "Test CPU".to_string(),
                cores: 8,
                threads: 16,
                cache_mb: 16,
                memory_gb: 32,
                features: vec![],
            });

        assert!(!caps.is_empty());
        assert_eq!(caps.total_platforms(), 1);
        assert!(caps.has_traditional_platforms());
    }

    #[test]
    fn test_ai_accelerator_detection() {
        let mut caps = UniversalSubstrateCapabilities::new();

        caps.specialized_architectures
            .push(SpecializedArchitecture::TPU {
                version: "v4".to_string(),
                tops: 275.0,
                memory_gb: 32,
            });

        assert!(caps.has_ai_accelerators());
    }

    #[test]
    fn test_default() {
        let caps = UniversalSubstrateCapabilities::default();
        assert!(caps.is_empty());
    }

    #[test]
    fn test_serialization() {
        let caps = UniversalSubstrateCapabilities::new();
        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: UniversalSubstrateCapabilities = serde_json::from_str(&json).unwrap();

        assert_eq!(caps, deserialized);
    }
}
