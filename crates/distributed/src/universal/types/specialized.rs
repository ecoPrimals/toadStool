// SPDX-License-Identifier: AGPL-3.0-only
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
    // AI/ML accelerators
    TPU {
        version: String,
        tops: f64,
        memory_gb: u32,
    },
    NPU {
        chip: String,
        tops: f64,
        frameworks: Vec<String>,
    },
    IPU {
        generation: String,
        tiles: u32,
        memory_gb: u32,
    },

    // Graphics processors
    CUDA {
        version: String,
        compute_capability: String,
        memory_gb: u32,
    },
    ROCm {
        version: String,
        gfx_version: String,
        memory_gb: u32,
    },
    OpenCL {
        version: String,
        device_type: String,
        compute_units: u32,
    },
    Vulkan {
        version: String,
        features: Vec<String>,
    },
    Metal {
        version: String,
        feature_set: String,
    },

    // Signal processors
    DSP {
        family: String,
        mips: f64,
        special_instructions: Vec<String>,
    },

    // Network processors
    DPU {
        chip: String,
        packet_processing_mpps: f64,
        cores: u32,
    },

    // Custom silicon
    ASIC {
        application: String,
        performance_metric: String,
        value: f64,
    },

    // Photonic processors
    PhotonicProcessor {
        wavelengths: u32,
        switching_speed_ghz: f64,
        power_consumption_w: f64,
    },

    // Analog computers
    AnalogComputer {
        type_name: String,
        precision_bits: u8,
        bandwidth_mhz: f64,
    },
}

impl SpecializedArchitecture {
    /// Get the architecture type name
    pub fn architecture_type(&self) -> &'static str {
        match self {
            Self::TPU { .. } => "TPU",
            Self::NPU { .. } => "NPU",
            Self::IPU { .. } => "IPU",
            Self::CUDA { .. } => "CUDA",
            Self::ROCm { .. } => "ROCm",
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
    pub const fn is_gpu_compute(&self) -> bool {
        matches!(
            self,
            Self::CUDA { .. }
                | Self::ROCm { .. }
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
    /// Molecular computing
    MolecularComputing {
        platform: String,
        molecular_basis: String,
        operation_temperature_k: f64,
    },

    /// Biocomputing hybrids
    CyborgSystems {
        biological_component: String,
        electronic_component: String,
        interface_protocol: String,
    },

    /// Metamaterial computing
    MetamaterialProcessor {
        material: String,
        frequency_range_ghz: (f64, f64),
        processing_method: String,
    },

    /// Spintronics
    SpintronicsProcessor {
        technology: String,
        spin_coherence_time_ns: f64,
        operating_temperature_k: f64,
    },

    /// Superconducting classical computers
    SuperconductingClassical {
        technology: String,
        operating_temperature_k: f64,
        switching_energy_j: f64,
    },

    /// Reversible computing
    ReversibleComputing {
        platform: String,
        reversibility_factor: f64,
        energy_efficiency: f64,
    },

    /// Crystalline computing
    CrystallineComputing {
        crystal_structure: String,
        defect_type: String,
        coherence_time_ms: f64,
    },

    /// Plasma computing
    PlasmaComputing {
        plasma_type: String,
        confinement_method: String,
        processing_frequency_mhz: f64,
    },
}

impl ExperimentalPlatform {
    /// Get the platform type name
    pub fn platform_type(&self) -> &'static str {
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
    pub fn operating_temperature_k(&self) -> Option<f64> {
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
