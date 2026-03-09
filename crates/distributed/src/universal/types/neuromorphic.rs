// SPDX-License-Identifier: AGPL-3.0-only
//! Neuromorphic computing platforms
//!
//! Support for brain-inspired computing including spiking neural networks,
//! memristive computing, and specialized neuromorphic chips.

use serde::{Deserialize, Serialize};

/// Neuromorphic computing platforms
///
/// Represents brain-inspired computing architectures that mimic biological neural systems.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum NeuromorphicPlatform {
    /// Spiking neural networks
    SpikingNeuralNetwork {
        /// Platform name
        platform: String,
        /// Neuron model (LIF, Izhikevich, etc.)
        neuron_model: String,
        /// Synapse model (STDP, etc.)
        synapse_model: String,
        /// Total neuron count
        neuron_count: u64,
        /// Network connectivity pattern
        connectivity_pattern: String,
    },

    /// Memristive computing
    MemristiveComputing {
        /// Platform name
        platform: String,
        /// Memristor technology type
        memristor_technology: String,
        /// Crossbar array dimensions (rows, columns)
        crossbar_size: (u32, u32),
        /// Number of resistance levels
        resistance_levels: u32,
    },

    /// Echo state networks
    EchoStateNetwork {
        /// Platform name
        platform: String,
        /// Number of reservoir neurons
        reservoir_size: u32,
        /// Connection density (0.0-1.0)
        connectivity_density: f64,
        /// Spectral radius for stability
        spectral_radius: f64,
        /// Input scaling factor
        input_scaling: f64,
        /// Leak rate parameter
        leak_rate: f64,
    },

    /// Liquid state machines
    LiquidStateMachine {
        /// Platform name
        platform: String,
        /// Number of liquid (reservoir) neurons
        liquid_neuron_count: u32,
        /// Number of readout neurons
        readout_neuron_count: u32,
        /// Temporal dynamics description
        temporal_dynamics: String,
    },

    /// Neuromorphic chips
    NeuromorphicChip {
        /// Chip model name
        chip_name: String,
        /// Manufacturer
        manufacturer: String,
        /// Number of cores
        core_count: u32,
        /// Neurons per core
        neuron_count_per_core: u32,
        /// Synapses per core
        synapse_count_per_core: u64,
        /// Power consumption in milliwatts
        power_consumption_mw: f64,
    },

    /// Optical neural networks
    OpticalNeuralNetwork {
        /// Platform name
        platform: String,
        /// Number of wavelength channels
        wavelength_channels: u32,
        /// Number of photonic neurons
        photonic_neurons: u32,
        /// Number of optical switches
        optical_switches: u32,
    },

    /// Analog neural networks
    AnalogNeuralNetwork {
        /// Platform name
        platform: String,
        /// Number of analog neurons
        analog_neurons: u32,
        /// Precision in bits
        precision_bits: u8,
        /// Noise characteristics description
        noise_characteristics: String,
    },
}

impl NeuromorphicPlatform {
    /// Get the platform type name
    pub fn platform_type(&self) -> &'static str {
        match self {
            Self::SpikingNeuralNetwork { .. } => "Spiking Neural Network",
            Self::MemristiveComputing { .. } => "Memristive Computing",
            Self::EchoStateNetwork { .. } => "Echo State Network",
            Self::LiquidStateMachine { .. } => "Liquid State Machine",
            Self::NeuromorphicChip { .. } => "Neuromorphic Chip",
            Self::OpticalNeuralNetwork { .. } => "Optical Neural Network",
            Self::AnalogNeuralNetwork { .. } => "Analog Neural Network",
        }
    }

    /// Check if platform is hardware-based
    pub const fn is_hardware(&self) -> bool {
        matches!(
            self,
            Self::NeuromorphicChip { .. }
                | Self::MemristiveComputing { .. }
                | Self::OpticalNeuralNetwork { .. }
        )
    }

    /// Check if platform uses spiking dynamics
    pub const fn uses_spikes(&self) -> bool {
        matches!(self, Self::SpikingNeuralNetwork { .. })
    }

    /// Get power efficiency (if applicable)
    pub fn power_consumption_mw(&self) -> Option<f64> {
        match self {
            Self::NeuromorphicChip {
                power_consumption_mw,
                ..
            } => Some(*power_consumption_mw),
            _ => None,
        }
    }

    /// Check if platform is suitable for real-time processing
    pub const fn is_realtime_capable(&self) -> bool {
        matches!(
            self,
            Self::NeuromorphicChip { .. } | Self::SpikingNeuralNetwork { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_type() {
        let snn = NeuromorphicPlatform::SpikingNeuralNetwork {
            platform: "BrainScaleS".to_string(),
            neuron_model: "LIF".to_string(),
            synapse_model: "STDP".to_string(),
            neuron_count: 200_000,
            connectivity_pattern: "Small-world".to_string(),
        };

        assert_eq!(snn.platform_type(), "Spiking Neural Network");
        assert!(snn.uses_spikes());
        assert!(snn.is_realtime_capable());
    }

    #[test]
    fn test_neuromorphic_chip() {
        let chip = NeuromorphicPlatform::NeuromorphicChip {
            chip_name: "TrueNorth".to_string(),
            manufacturer: "IBM".to_string(),
            core_count: 4096,
            neuron_count_per_core: 256,
            synapse_count_per_core: 256_000,
            power_consumption_mw: 70.0,
        };

        assert!(chip.is_hardware());
        assert_eq!(chip.power_consumption_mw(), Some(70.0));
    }

    #[test]
    fn test_echo_state_network() {
        let esn = NeuromorphicPlatform::EchoStateNetwork {
            platform: "Custom ESN".to_string(),
            reservoir_size: 1000,
            connectivity_density: 0.1,
            spectral_radius: 0.9,
            input_scaling: 1.0,
            leak_rate: 0.3,
        };

        assert!(!esn.is_hardware());
        assert_eq!(esn.platform_type(), "Echo State Network");
    }

    #[test]
    fn test_serialization() {
        let platform = NeuromorphicPlatform::MemristiveComputing {
            platform: "Memristor Array".to_string(),
            memristor_technology: "TiO2".to_string(),
            crossbar_size: (128, 128),
            resistance_levels: 16,
        };

        let json = serde_json::to_string(&platform).unwrap();
        let deserialized: NeuromorphicPlatform = serde_json::from_str(&json).unwrap();

        assert_eq!(platform, deserialized);
    }
}
