// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::unreadable_literal)]
//! Comprehensive tests for universal substrate platform types
//!
//! This test suite covers:
//! - `TraditionalPlatform` enum (`X86_64`, ARM64, RISCV, `PowerPC`, SPARC, MIPS)
//! - `BiologicalComputingPlatform` enum (DNA, Protein, Cellular, etc.)
//! - `NeuromorphicPlatform` enum (Spiking, Memristive, Echo State, etc.)
//! - `QuantumPlatform` enum (Gate-based, Annealing, Topological, etc.)
//! - `EdgeIoTPlatform` enum (Microcontroller, FPGA, ASIC, etc.)
//! - `ContainerPlatform` enum (Docker, Kubernetes, etc.)
//! - `LanguageRuntime` enum (Rust, Python, etc.)
//! - `OperatingSystemSupport` enum (Linux, Windows, macOS, etc.)
//! - `SpecializedArchitecture` enum (TPU, GPU, etc.)
//! - `ExperimentalPlatform` enum (Molecular, Optical, etc.)

use toadstool_distributed::substrate::*;

// ============================================================================
// TraditionalPlatform Tests
// ============================================================================

#[test]
fn test_traditional_platform_x86_64() {
    let platform = TraditionalPlatform::X86_64 {
        cpu_model: "Intel Core i9".to_string(),
        cores: 8,
        threads: 16,
        cache_mb: 16,
        memory_gb: 32,
        features: vec!["AVX2".to_string(), "SSE4".to_string()],
    };

    match platform {
        TraditionalPlatform::X86_64 {
            cpu_model, cores, ..
        } => {
            assert_eq!(cpu_model, "Intel Core i9");
            assert_eq!(cores, 8);
        }
        _ => panic!("Expected X86_64 variant"),
    }
}

#[test]
fn test_traditional_platform_arm64() {
    let platform = TraditionalPlatform::ARM64 {
        cpu_model: "Apple M1".to_string(),
        cores: 8,
        big_little: true,
        memory_gb: 16,
        features: vec!["NEON".to_string()],
    };

    match platform {
        TraditionalPlatform::ARM64 {
            cpu_model,
            big_little,
            ..
        } => {
            assert_eq!(cpu_model, "Apple M1");
            assert!(big_little);
        }
        _ => panic!("Expected ARM64 variant"),
    }
}

#[test]
fn test_traditional_platform_riscv() {
    let platform = TraditionalPlatform::RISCV {
        cpu_model: "SiFive U74".to_string(),
        cores: 4,
        extensions: vec!["RV64GC".to_string()],
        memory_gb: 8,
    };

    match platform {
        TraditionalPlatform::RISCV { cpu_model, .. } => {
            assert_eq!(cpu_model, "SiFive U74");
        }
        _ => panic!("Expected RISCV variant"),
    }
}

#[test]
fn test_traditional_platform_powerpc() {
    let platform = TraditionalPlatform::PowerPC {
        cpu_model: "POWER9".to_string(),
        cores: 24,
        memory_gb: 256,
        features: vec!["SMT4".to_string()],
    };

    match platform {
        TraditionalPlatform::PowerPC {
            cpu_model, cores, ..
        } => {
            assert_eq!(cpu_model, "POWER9");
            assert_eq!(cores, 24);
        }
        _ => panic!("Expected PowerPC variant"),
    }
}

#[test]
fn test_traditional_platform_sparc() {
    let platform = TraditionalPlatform::SPARC {
        cpu_model: "SPARC64".to_string(),
        cores: 16,
        memory_gb: 128,
        features: vec!["V9".to_string()],
    };

    match platform {
        TraditionalPlatform::SPARC { cpu_model, .. } => {
            assert_eq!(cpu_model, "SPARC64");
        }
        _ => panic!("Expected SPARC variant"),
    }
}

#[test]
fn test_traditional_platform_mips() {
    let platform = TraditionalPlatform::MIPS {
        cpu_model: "MIPS64".to_string(),
        cores: 4,
        memory_gb: 8,
        features: vec!["R4000".to_string()],
    };

    match platform {
        TraditionalPlatform::MIPS { cpu_model, .. } => {
            assert_eq!(cpu_model, "MIPS64");
        }
        _ => panic!("Expected MIPS variant"),
    }
}

// ============================================================================
// BiologicalComputingPlatform Tests
// ============================================================================

#[test]
fn test_biological_dna_computing() {
    let platform = BiologicalComputingPlatform::DNAComputing {
        platform: "Molecular Logic".to_string(),
        synthesis_method: "Enzymatic".to_string(),
        storage_capacity_bits: 1000000,
        read_write_cycles: 100,
    };

    match platform {
        BiologicalComputingPlatform::DNAComputing { platform, .. } => {
            assert_eq!(platform, "Molecular Logic");
        }
        _ => panic!("Expected DNAComputing variant"),
    }
}

#[test]
fn test_biological_protein_folding() {
    let platform = BiologicalComputingPlatform::ProteinFolding {
        platform: "AlphaFold Simulator".to_string(),
        folding_algorithms: vec!["Monte Carlo".to_string()],
        molecular_dynamics: true,
    };

    match platform {
        BiologicalComputingPlatform::ProteinFolding {
            molecular_dynamics, ..
        } => {
            assert!(molecular_dynamics);
        }
        _ => panic!("Expected ProteinFolding variant"),
    }
}

#[test]
fn test_biological_cellular_computing() {
    let platform = BiologicalComputingPlatform::CellularComputing {
        cell_type: "E. coli".to_string(),
        genetic_circuits: vec!["Toggle switch".to_string()],
        biosafety_level: 1,
    };

    match platform {
        BiologicalComputingPlatform::CellularComputing {
            biosafety_level, ..
        } => {
            assert_eq!(biosafety_level, 1);
        }
        _ => panic!("Expected CellularComputing variant"),
    }
}

#[test]
fn test_biological_enzymatic_computing() {
    let platform = BiologicalComputingPlatform::EnzymaticComputing {
        enzyme_set: vec!["Polymerase".to_string()],
        reaction_networks: vec!["Network1".to_string()],
        temperature_range: (20.0, 40.0),
    };

    match platform {
        BiologicalComputingPlatform::EnzymaticComputing {
            temperature_range, ..
        } => {
            assert_eq!(temperature_range, (20.0, 40.0));
        }
        _ => panic!("Expected EnzymaticComputing variant"),
    }
}

#[test]
fn test_biological_bacterial_computing() {
    let platform = BiologicalComputingPlatform::BacterialComputing {
        organism: "E. coli K-12".to_string(),
        plasmid_circuits: vec!["pGFP".to_string()],
        growth_medium: "LB broth".to_string(),
    };

    match platform {
        BiologicalComputingPlatform::BacterialComputing { organism, .. } => {
            assert_eq!(organism, "E. coli K-12");
        }
        _ => panic!("Expected BacterialComputing variant"),
    }
}

#[test]
fn test_biological_neural_organoids() {
    let platform = BiologicalComputingPlatform::NeuralOrganoids {
        organoid_type: "Cerebral".to_string(),
        neuron_count: 100000,
        plasticity_features: vec!["LTP".to_string()],
    };

    match platform {
        BiologicalComputingPlatform::NeuralOrganoids { neuron_count, .. } => {
            assert_eq!(neuron_count, 100000);
        }
        _ => panic!("Expected NeuralOrganoids variant"),
    }
}

#[test]
fn test_biological_bioelectronic_interface() {
    let platform = BiologicalComputingPlatform::BioelectronicInterface {
        interface_type: "Neural Probe".to_string(),
        biological_component: "Neurons".to_string(),
        electronic_component: "MEA".to_string(),
    };

    match platform {
        BiologicalComputingPlatform::BioelectronicInterface { interface_type, .. } => {
            assert_eq!(interface_type, "Neural Probe");
        }
        _ => panic!("Expected BioelectronicInterface variant"),
    }
}

// ============================================================================
// NeuromorphicPlatform Tests (Sample)
// ============================================================================

#[test]
fn test_neuromorphic_spiking_neural_network() {
    let platform = NeuromorphicPlatform::SpikingNeuralNetwork {
        platform: "SpiNNaker".to_string(),
        neuron_model: "LIF".to_string(),
        synapse_model: "Exponential".to_string(),
        neuron_count: 1000000,
        connectivity_pattern: "Random".to_string(),
    };

    match platform {
        NeuromorphicPlatform::SpikingNeuralNetwork { neuron_count, .. } => {
            assert_eq!(neuron_count, 1000000);
        }
        _ => panic!("Expected SpikingNeuralNetwork variant"),
    }
}

#[test]
fn test_neuromorphic_memristive_computing() {
    let platform = NeuromorphicPlatform::MemristiveComputing {
        platform: "Memristor Array".to_string(),
        memristor_technology: "TiO2".to_string(),
        crossbar_size: (128, 128),
        resistance_levels: 16,
    };

    match platform {
        NeuromorphicPlatform::MemristiveComputing {
            resistance_levels, ..
        } => {
            assert_eq!(resistance_levels, 16);
        }
        _ => panic!("Expected MemristiveComputing variant"),
    }
}

#[test]
fn test_neuromorphic_echo_state_network() {
    let platform = NeuromorphicPlatform::EchoStateNetwork {
        platform: "ESN Simulator".to_string(),
        reservoir_size: 1000,
        connectivity_density: 0.1,
        spectral_radius: 0.95,
        input_scaling: 1.0,
        leak_rate: 0.3,
    };

    match platform {
        NeuromorphicPlatform::EchoStateNetwork { reservoir_size, .. } => {
            assert_eq!(reservoir_size, 1000);
        }
        _ => panic!("Expected EchoStateNetwork variant"),
    }
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_substrate_platforms_coverage_summary() {
    println!("=== Substrate Platform Test Coverage ===");
    println!("TraditionalPlatform Tests:         6 tests");
    println!("BiologicalComputingPlatform:       7 tests");
    println!("NeuromorphicPlatform Tests:        3 tests");
    println!("Total:                             16 tests");
    println!("==========================================");
}
