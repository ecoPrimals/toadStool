// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantum computing platforms
//!
//! Support for quantum computing architectures including gate-based,
//! annealing, photonic, trapped ion, and superconducting systems.

use serde::{Deserialize, Serialize};

/// Quantum computing platforms
///
/// Represents various quantum computing architectures and simulators.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum QuantumPlatform {
    /// Gate-based quantum computers
    GateBasedQuantum {
        /// Platform name/vendor
        platform: String,
        /// Number of qubits
        qubit_count: u32,
        /// Gate operation fidelity (0.0-1.0)
        gate_fidelity: f64,
        /// Qubit connectivity topology
        connectivity_graph: String,
        /// Error correction enabled
        error_correction: bool,
    },

    /// Annealing quantum computers
    QuantumAnnealing {
        /// Platform name/vendor
        platform: String,
        /// Number of qubits
        qubit_count: u32,
        /// Qubit-qubit coupling strength
        coupling_strength: f64,
        /// Annealing time in microseconds
        annealing_time_us: f64,
    },

    /// Photonic quantum computers
    PhotonicQuantum {
        /// Platform name/vendor
        platform: String,
        /// Number of photon sources
        photon_sources: u32,
        /// Number of beam splitters
        beam_splitters: u32,
        /// Number of detectors
        detectors: u32,
        /// Squeezing level in decibels
        squeezing_level_db: f64,
    },

    /// Trapped ion quantum computers
    TrappedIonQuantum {
        /// Platform name/vendor
        platform: String,
        /// Ion species used
        ion_species: String,
        /// Trap frequency in MHz
        trap_frequency_mhz: f64,
        /// Laser cooling enabled
        laser_cooling: bool,
    },

    /// Superconducting quantum computers
    SuperconductingQuantum {
        /// Platform name/vendor
        platform: String,
        /// Qubit type (transmon, flux, etc.)
        qubit_type: String,
        /// Operating temperature in millikelvin
        operating_temperature_mk: f64,
        /// Coherence time in microseconds
        coherence_time_us: f64,
    },

    /// Quantum simulators
    QuantumSimulator {
        /// Platform name
        platform: String,
        /// Type of simulation
        simulation_type: String,
        /// Number of qubits that can be simulated
        classical_qubits_simulated: u32,
    },
}

impl QuantumPlatform {
    /// Get the platform type name
    pub fn platform_type(&self) -> &'static str {
        match self {
            Self::GateBasedQuantum { .. } => "Gate-Based Quantum",
            Self::QuantumAnnealing { .. } => "Quantum Annealing",
            Self::PhotonicQuantum { .. } => "Photonic Quantum",
            Self::TrappedIonQuantum { .. } => "Trapped Ion Quantum",
            Self::SuperconductingQuantum { .. } => "Superconducting Quantum",
            Self::QuantumSimulator { .. } => "Quantum Simulator",
        }
    }

    /// Get qubit count
    pub const fn qubit_count(&self) -> Option<u32> {
        match self {
            Self::GateBasedQuantum { qubit_count, .. }
            | Self::QuantumAnnealing { qubit_count, .. } => Some(*qubit_count),
            Self::QuantumSimulator {
                classical_qubits_simulated,
                ..
            } => Some(*classical_qubits_simulated),
            _ => None,
        }
    }

    /// Check if platform supports universal quantum computation
    pub const fn is_universal(&self) -> bool {
        matches!(
            self,
            Self::GateBasedQuantum { .. }
                | Self::TrappedIonQuantum { .. }
                | Self::SuperconductingQuantum { .. }
                | Self::PhotonicQuantum { .. }
        )
    }

    /// Check if platform is hardware (not simulator)
    pub const fn is_hardware(&self) -> bool {
        !matches!(self, Self::QuantumSimulator { .. })
    }

    /// Check if platform requires cryogenic cooling
    pub const fn requires_cryogenic_cooling(&self) -> bool {
        matches!(
            self,
            Self::SuperconductingQuantum { .. } | Self::TrappedIonQuantum { .. }
        )
    }

    /// Get operating temperature in millikelvin (if applicable)
    pub const fn operating_temperature_mk(&self) -> Option<f64> {
        match self {
            Self::SuperconductingQuantum {
                operating_temperature_mk,
                ..
            } => Some(*operating_temperature_mk),
            _ => None,
        }
    }

    /// Check if platform supports error correction
    pub const fn supports_error_correction(&self) -> bool {
        match self {
            Self::GateBasedQuantum {
                error_correction, ..
            } => *error_correction,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_type() {
        let gate_based = QuantumPlatform::GateBasedQuantum {
            platform: "IBM Quantum".to_string(),
            qubit_count: 127,
            gate_fidelity: 0.999,
            connectivity_graph: "heavy-hex".to_string(),
            error_correction: true,
        };

        assert_eq!(gate_based.platform_type(), "Gate-Based Quantum");
        assert_eq!(gate_based.qubit_count(), Some(127));
        assert!(gate_based.is_universal());
        assert!(gate_based.supports_error_correction());
    }

    #[test]
    fn test_superconducting() {
        let superconducting = QuantumPlatform::SuperconductingQuantum {
            platform: "Google Sycamore".to_string(),
            qubit_type: "transmon".to_string(),
            operating_temperature_mk: 15.0,
            coherence_time_us: 20.0,
        };

        assert!(superconducting.requires_cryogenic_cooling());
        assert_eq!(superconducting.operating_temperature_mk(), Some(15.0));
    }

    #[test]
    fn test_quantum_annealing() {
        let annealing = QuantumPlatform::QuantumAnnealing {
            platform: "D-Wave".to_string(),
            qubit_count: 5000,
            coupling_strength: 1.0,
            annealing_time_us: 20.0,
        };

        assert!(!annealing.is_universal());
        assert!(annealing.is_hardware());
        assert_eq!(annealing.qubit_count(), Some(5000));
    }

    #[test]
    fn test_quantum_simulator() {
        let simulator = QuantumPlatform::QuantumSimulator {
            platform: "Qiskit Aer".to_string(),
            simulation_type: "statevector".to_string(),
            classical_qubits_simulated: 32,
        };

        assert!(!simulator.is_hardware());
        assert_eq!(simulator.qubit_count(), Some(32));
    }

    #[test]
    fn test_serialization() {
        let platform = QuantumPlatform::PhotonicQuantum {
            platform: "Xanadu".to_string(),
            photon_sources: 8,
            beam_splitters: 16,
            detectors: 8,
            squeezing_level_db: 15.0,
        };

        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: QuantumPlatform = serde_json::from_str(&json).unwrap();

        assert_eq!(platform, deserialized);
    }
}
