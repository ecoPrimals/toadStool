// SPDX-License-Identifier: AGPL-3.0-or-later
//! Specialized and experimental computing platforms
//!
//! Support for specialized architectures (AI/ML accelerators, GPUs, DSPs, etc.)
//! and experimental platforms (molecular computing, spintronics, etc.).

use serde::{Deserialize, Serialize};

/// Specialized computing architectures
///
/// Represents specialized hardware for specific computational tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum SpecializedArchitecture {
    /// Google TPU AI accelerator.
    TPU {
        /// TPU version.
        version: String,
        /// TOPS (trillion ops/sec).
        tops: f64,
        /// Memory in GB.
        memory_gb: u32,
    },
    /// Neural processing unit.
    NPU {
        /// Chip identifier.
        chip: String,
        /// TOPS.
        tops: f64,
        /// Supported frameworks.
        frameworks: Vec<String>,
    },
    /// Graphcore IPU.
    IPU {
        /// IPU generation.
        generation: String,
        /// Number of tiles.
        tiles: u32,
        /// Memory in GB.
        memory_gb: u32,
    },
    /// NVIDIA CUDA GPU.
    CUDA {
        /// CUDA version.
        version: String,
        /// Compute capability (e.g. 8.0).
        compute_capability: String,
        /// Memory in GB.
        memory_gb: u32,
    },
    /// AMD ROCm GPU.
    ROCm {
        /// ROCm version.
        version: String,
        /// GFX version.
        gfx_version: String,
        /// Memory in GB.
        memory_gb: u32,
    },
    /// OpenCL compute (serde-compatible; not surfaced by detection).
    #[deprecated(
        note = "DEPRECATED S198: OpenCL removed — use gpu.dispatch.opencl capability provider via IPC"
    )]
    OpenCL {
        /// OpenCL version.
        version: String,
        /// Device type (GPU, CPU, etc.).
        device_type: String,
        /// Compute units.
        compute_units: u32,
    },
    /// Vulkan GPU compute.
    Vulkan {
        /// Vulkan version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Apple Metal GPU.
    Metal {
        /// Metal version.
        version: String,
        /// Feature set.
        feature_set: String,
    },
    /// Digital signal processor.
    DSP {
        /// DSP family.
        family: String,
        /// MIPS.
        mips: f64,
        /// Special instruction sets.
        special_instructions: Vec<String>,
    },
    /// Data processing unit (SmartNIC).
    DPU {
        /// DPU chip.
        chip: String,
        /// Packet processing rate in Mpps.
        packet_processing_mpps: f64,
        /// Core count.
        cores: u32,
    },
    /// Application-specific integrated circuit.
    ASIC {
        /// Application domain.
        application: String,
        /// Performance metric name.
        performance_metric: String,
        /// Metric value.
        value: f64,
    },
    /// Photonic/optical processor.
    PhotonicProcessor {
        /// Wavelength count.
        wavelengths: u32,
        /// Switching speed in GHz.
        switching_speed_ghz: f64,
        /// Power consumption in watts.
        power_consumption_w: f64,
    },
    /// Analog computer.
    AnalogComputer {
        /// Analog computer type.
        type_name: String,
        /// Precision in bits.
        precision_bits: u8,
        /// Bandwidth in MHz.
        bandwidth_mhz: f64,
    },
}

impl SpecializedArchitecture {
    /// Get the architecture type name
    #[expect(deprecated, reason = "OpenCL arm for persisted specs (S198)")]
    pub const fn architecture_type(&self) -> &'static str {
        match self {
            Self::TPU { .. } => "TPU",
            Self::NPU { .. } => "NPU",
            Self::IPU { .. } => "IPU",
            Self::CUDA { .. } => "CUDA",
            Self::ROCm { .. } => "ROCm",
            // DEPRECATED S198
            Self::OpenCL { .. } => "OpenCL",
            Self::Vulkan { .. } => "Vulkan",
            Self::Metal { .. } => "Metal",
            Self::DSP { .. } => "DSP",
            Self::DPU { .. } => "DPU",
            Self::ASIC { .. } => "ASIC",
            Self::PhotonicProcessor { .. } => "Photonic Processor",
            Self::AnalogComputer { .. } => "Analog Computer",
        }
    }

    /// Check if architecture is for AI/ML workloads
    pub const fn is_ai_accelerator(&self) -> bool {
        matches!(self, Self::TPU { .. } | Self::NPU { .. } | Self::IPU { .. })
    }

    /// Check if architecture is a GPU compute platform
    #[expect(deprecated, reason = "OpenCL arm for persisted specs (S198)")]
    pub const fn is_gpu_compute(&self) -> bool {
        matches!(
            self,
            Self::CUDA { .. }
                | Self::ROCm { .. }
                // DEPRECATED S198: persisted OpenCL-shaped values only
                | Self::OpenCL { .. }
                | Self::Vulkan { .. }
                | Self::Metal { .. }
        )
    }

    /// Check if architecture is for network processing
    pub const fn is_network_processor(&self) -> bool {
        matches!(self, Self::DPU { .. })
    }

    /// Check if architecture is custom silicon
    pub const fn is_custom_silicon(&self) -> bool {
        matches!(self, Self::ASIC { .. })
    }

    /// Get performance in TOPS (if applicable)
    pub const fn tops_performance(&self) -> Option<f64> {
        match self {
            Self::TPU { tops, .. } | Self::NPU { tops, .. } => Some(*tops),
            _ => None,
        }
    }

    /// Get memory capacity in GB (if applicable)
    pub const fn memory_gb(&self) -> Option<u32> {
        match self {
            Self::TPU { memory_gb, .. }
            | Self::IPU { memory_gb, .. }
            | Self::CUDA { memory_gb, .. }
            | Self::ROCm { memory_gb, .. } => Some(*memory_gb),
            _ => None,
        }
    }
}

/// Experimental computing platforms
///
/// Represents cutting-edge and experimental computing technologies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum ExperimentalPlatform {
    /// Molecular computing.
    MolecularComputing {
        /// Platform identifier.
        platform: String,
        /// Molecular basis (DNA, RNA, etc.).
        molecular_basis: String,
        /// Operation temperature in Kelvin.
        operation_temperature_k: f64,
    },
    /// Biocomputing hybrids.
    CyborgSystems {
        /// Biological component.
        biological_component: String,
        /// Electronic component.
        electronic_component: String,
        /// Interface protocol.
        interface_protocol: String,
    },
    /// Metamaterial computing.
    MetamaterialProcessor {
        /// Material type.
        material: String,
        /// Frequency range in GHz.
        frequency_range_ghz: (f64, f64),
        /// Processing method.
        processing_method: String,
    },
    /// Spintronics.
    SpintronicsProcessor {
        /// Technology (MRAM, etc.).
        technology: String,
        /// Spin coherence time in ns.
        spin_coherence_time_ns: f64,
        /// Operating temperature in Kelvin.
        operating_temperature_k: f64,
    },
    /// Superconducting classical computers.
    SuperconductingClassical {
        /// Technology.
        technology: String,
        /// Operating temperature in Kelvin.
        operating_temperature_k: f64,
        /// Switching energy in joules.
        switching_energy_j: f64,
    },
    /// Reversible computing.
    ReversibleComputing {
        /// Platform.
        platform: String,
        /// Reversibility factor.
        reversibility_factor: f64,
        /// Energy efficiency.
        energy_efficiency: f64,
    },
    /// Crystalline computing.
    CrystallineComputing {
        /// Crystal structure.
        crystal_structure: String,
        /// Defect type.
        defect_type: String,
        /// Coherence time in ms.
        coherence_time_ms: f64,
    },
    /// Plasma computing.
    PlasmaComputing {
        /// Plasma type.
        plasma_type: String,
        /// Confinement method.
        confinement_method: String,
        /// Processing frequency in MHz.
        processing_frequency_mhz: f64,
    },
}

impl ExperimentalPlatform {
    /// Get the platform type name
    pub const fn platform_type(&self) -> &'static str {
        match self {
            Self::MolecularComputing { .. } => "Molecular Computing",
            Self::CyborgSystems { .. } => "Cyborg Systems",
            Self::MetamaterialProcessor { .. } => "Metamaterial Processor",
            Self::SpintronicsProcessor { .. } => "Spintronics Processor",
            Self::SuperconductingClassical { .. } => "Superconducting Classical",
            Self::ReversibleComputing { .. } => "Reversible Computing",
            Self::CrystallineComputing { .. } => "Crystalline Computing",
            Self::PlasmaComputing { .. } => "Plasma Computing",
        }
    }

    /// Check if platform requires cryogenic cooling
    pub fn requires_cryogenic(&self) -> bool {
        match self {
            Self::SuperconductingClassical {
                operating_temperature_k,
                ..
            }
            | Self::SpintronicsProcessor {
                operating_temperature_k,
                ..
            } => *operating_temperature_k < 77.0, // Below liquid nitrogen temperature
            _ => false,
        }
    }

    /// Check if platform is energy-efficient
    pub const fn is_energy_efficient(&self) -> bool {
        matches!(self, Self::ReversibleComputing { .. })
    }

    /// Check if platform involves biological components
    pub const fn has_biological_components(&self) -> bool {
        matches!(
            self,
            Self::MolecularComputing { .. } | Self::CyborgSystems { .. }
        )
    }

    /// Get operating temperature in Kelvin (if applicable)
    pub const fn operating_temperature_k(&self) -> Option<f64> {
        match self {
            Self::MolecularComputing {
                operation_temperature_k,
                ..
            } => Some(*operation_temperature_k),
            Self::SpintronicsProcessor {
                operating_temperature_k,
                ..
            } => Some(*operating_temperature_k),
            Self::SuperconductingClassical {
                operating_temperature_k,
                ..
            } => Some(*operating_temperature_k),
            _ => None,
        }
    }

    /// Check if platform is in early research stage
    pub const fn is_research_stage(&self) -> bool {
        true // All experimental platforms are considered research stage
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_accelerator() {
        let tpu = SpecializedArchitecture::TPU {
            version: "v4".to_string(),
            tops: 275.0,
            memory_gb: 32,
        };

        assert_eq!(tpu.architecture_type(), "TPU");
        assert!(tpu.is_ai_accelerator());
        assert_eq!(tpu.tops_performance(), Some(275.0));
        assert_eq!(tpu.memory_gb(), Some(32));
    }

    #[test]
    fn test_gpu_compute() {
        let cuda = SpecializedArchitecture::CUDA {
            version: "12.0".to_string(),
            compute_capability: "8.9".to_string(),
            memory_gb: 80,
        };

        assert!(cuda.is_gpu_compute());
        assert!(!cuda.is_ai_accelerator());
        assert_eq!(cuda.memory_gb(), Some(80));
    }

    #[test]
    fn test_network_processor() {
        let dpu = SpecializedArchitecture::DPU {
            chip: "BlueField-3".to_string(),
            packet_processing_mpps: 400.0,
            cores: 16,
        };

        assert!(dpu.is_network_processor());
    }

    #[test]
    fn test_experimental_cryogenic() {
        let superconducting = ExperimentalPlatform::SuperconductingClassical {
            technology: "SFQ".to_string(),
            operating_temperature_k: 4.2,
            switching_energy_j: 1e-19,
        };

        assert!(superconducting.requires_cryogenic());
        assert_eq!(superconducting.operating_temperature_k(), Some(4.2));
    }

    #[test]
    fn test_biological_experimental() {
        let cyborg = ExperimentalPlatform::CyborgSystems {
            biological_component: "Neurons".to_string(),
            electronic_component: "CMOS".to_string(),
            interface_protocol: "Multi-electrode array".to_string(),
        };

        assert!(cyborg.has_biological_components());
        assert!(cyborg.is_research_stage());
    }

    #[test]
    fn test_energy_efficient() {
        let reversible = ExperimentalPlatform::ReversibleComputing {
            platform: "Pendulum".to_string(),
            reversibility_factor: 0.95,
            energy_efficiency: 0.99,
        };

        assert!(reversible.is_energy_efficient());
    }

    #[test]
    fn test_serialization() {
        let platform = SpecializedArchitecture::PhotonicProcessor {
            wavelengths: 64,
            switching_speed_ghz: 100.0,
            power_consumption_w: 10.0,
        };

        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: SpecializedArchitecture = serde_json::from_str(&json).unwrap();

        assert_eq!(platform, deserialized);
    }
}
