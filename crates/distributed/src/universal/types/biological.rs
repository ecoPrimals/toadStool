//! Biological computing platforms
//!
//! Support for bio-based computing including DNA, protein folding,
//! cellular, enzymatic, bacterial, neural organoids, and bioelectronic interfaces.

use serde::{Deserialize, Serialize};

/// Biological computing platforms
///
/// Represents various forms of biological computation from molecular to cellular levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum BiologicalComputingPlatform {
    /// DNA computing systems
    DNAComputing {
        /// Platform name/vendor
        platform: String,
        /// Method used for DNA synthesis
        synthesis_method: String,
        /// Storage capacity in bits
        storage_capacity_bits: u64,
        /// Number of read/write cycles supported
        read_write_cycles: u32,
    },

    /// Protein folding computers
    ProteinFolding {
        /// Platform name/vendor
        platform: String,
        /// Algorithms used for folding prediction
        folding_algorithms: Vec<String>,
        /// Supports molecular dynamics simulation
        molecular_dynamics: bool,
    },

    /// Cellular computing
    CellularComputing {
        /// Type of cells used
        cell_type: String,
        /// Genetic circuits implemented
        genetic_circuits: Vec<String>,
        /// Biosafety level (1-4)
        biosafety_level: u8,
    },

    /// Enzymatic computing
    EnzymaticComputing {
        /// Set of enzymes used
        enzyme_set: Vec<String>,
        /// Chemical reaction networks
        reaction_networks: Vec<String>,
        /// Operating temperature range (min, max) in Celsius
        temperature_range: (f64, f64),
    },

    /// Bacterial computing
    BacterialComputing {
        /// Bacterial organism species
        organism: String,
        /// Plasmid-based genetic circuits
        plasmid_circuits: Vec<String>,
        /// Growth medium requirements
        growth_medium: String,
    },

    /// Neural organoids
    NeuralOrganoids {
        /// Type of organoid (cortical, hippocampal, etc.)
        organoid_type: String,
        /// Estimated neuron count
        neuron_count: u64,
        /// Plasticity and learning features
        plasticity_features: Vec<String>,
    },

    /// Bioelectronic interfaces
    BioelectronicInterface {
        /// Type of interface
        interface_type: String,
        /// Biological component description
        biological_component: String,
        /// Electronic component description
        electronic_component: String,
    },
}

impl BiologicalComputingPlatform {
    /// Get the platform type name
    pub fn platform_type(&self) -> &'static str {
        match self {
            Self::DNAComputing { .. } => "DNA Computing",
            Self::ProteinFolding { .. } => "Protein Folding",
            Self::CellularComputing { .. } => "Cellular Computing",
            Self::EnzymaticComputing { .. } => "Enzymatic Computing",
            Self::BacterialComputing { .. } => "Bacterial Computing",
            Self::NeuralOrganoids { .. } => "Neural Organoids",
            Self::BioelectronicInterface { .. } => "Bioelectronic Interface",
        }
    }

    /// Check if platform requires biosafety containment
    pub const fn requires_biosafety(&self) -> bool {
        matches!(
            self,
            Self::CellularComputing { .. }
                | Self::BacterialComputing { .. }
                | Self::NeuralOrganoids { .. }
        )
    }

    /// Get biosafety level (returns 0 if not applicable)
    pub const fn biosafety_level(&self) -> u8 {
        match self {
            Self::CellularComputing {
                biosafety_level, ..
            } => *biosafety_level,
            Self::BacterialComputing { .. } | Self::NeuralOrganoids { .. } => 2, // Default BSL-2
            _ => 0,
        }
    }

    /// Check if platform is suitable for storage applications
    pub const fn is_storage_capable(&self) -> bool {
        matches!(self, Self::DNAComputing { .. })
    }

    /// Check if platform is suitable for computation
    pub const fn is_computational(&self) -> bool {
        !matches!(self, Self::DNAComputing { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_type() {
        let dna = BiologicalComputingPlatform::DNAComputing {
            platform: "DNA Storage System".to_string(),
            synthesis_method: "Enzymatic".to_string(),
            storage_capacity_bits: 1_000_000,
            read_write_cycles: 100,
        };

        assert_eq!(dna.platform_type(), "DNA Computing");
        assert!(dna.is_storage_capable());
    }

    #[test]
    fn test_biosafety() {
        let cellular = BiologicalComputingPlatform::CellularComputing {
            cell_type: "E. coli".to_string(),
            genetic_circuits: vec!["Toggle switch".to_string()],
            biosafety_level: 2,
        };

        assert!(cellular.requires_biosafety());
        assert_eq!(cellular.biosafety_level(), 2);
    }

    #[test]
    fn test_bacterial_computing() {
        let bacterial = BiologicalComputingPlatform::BacterialComputing {
            organism: "B. subtilis".to_string(),
            plasmid_circuits: vec!["Logic gate".to_string()],
            growth_medium: "LB broth".to_string(),
        };

        assert!(bacterial.requires_biosafety());
        assert!(bacterial.is_computational());
    }

    #[test]
    fn test_serialization() {
        let platform = BiologicalComputingPlatform::ProteinFolding {
            platform: "AlphaFold".to_string(),
            folding_algorithms: vec!["Deep learning".to_string()],
            molecular_dynamics: true,
        };

        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: BiologicalComputingPlatform = serde_json::from_str(&json).unwrap();

        assert_eq!(platform, deserialized);
    }
}
