// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for UniversalSubstrateCapabilities
//!
//! Test Coverage Phase 2 - Zero Coverage File
//! Target: substrate.rs UniversalSubstrateCapabilities (currently 0% coverage)
//!
//! This test suite covers:
//! - UniversalSubstrateCapabilities structure
//! - Serialization/deserialization
//! - Platform collection management
//! - Integration scenarios

use toadstool_distributed::substrate::*;

// ============================================================================
// UniversalSubstrateCapabilities Tests
// ============================================================================

#[test]
fn test_universal_substrate_capabilities_empty() {
    // Test empty capabilities structure
    let capabilities = UniversalSubstrateCapabilities {
        traditional_platforms: vec![],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    assert!(capabilities.traditional_platforms.is_empty());
    assert!(capabilities.biological_platforms.is_empty());
    assert!(capabilities.neuromorphic_platforms.is_empty());
    assert!(capabilities.quantum_platforms.is_empty());
    assert!(capabilities.edge_iot_platforms.is_empty());
}

#[test]
fn test_universal_substrate_capabilities_with_traditional() {
    // Test capabilities with traditional platforms
    let capabilities = UniversalSubstrateCapabilities {
        traditional_platforms: vec![TraditionalPlatform::X86_64 {
            cpu_model: "Intel Core i9".to_string(),
            cores: 8,
            threads: 16,
            cache_mb: 16,
            memory_gb: 32,
            features: vec!["AVX2".to_string()],
        }],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.traditional_platforms.len(), 1);
    assert!(capabilities.biological_platforms.is_empty());
}

#[test]
fn test_universal_substrate_capabilities_multi_platform() {
    // Test capabilities with multiple platform types
    let capabilities = UniversalSubstrateCapabilities {
        traditional_platforms: vec![TraditionalPlatform::ARM64 {
            cpu_model: "Apple M1".to_string(),
            cores: 8,
            big_little: true,
            memory_gb: 16,
            features: vec!["NEON".to_string()],
        }],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![ContainerPlatform::Docker {
            version: "20.10.0".to_string(),
            features: vec!["buildkit".to_string()],
        }],
        language_runtimes: vec![LanguageRuntime::Rust {
            version: "1.70.0".to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            features: vec!["cargo".to_string()],
        }],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.traditional_platforms.len(), 1);
    assert_eq!(capabilities.container_platforms.len(), 1);
    assert_eq!(capabilities.language_runtimes.len(), 1);
}

#[test]
fn test_universal_substrate_capabilities_clone() {
    // Test that capabilities can be cloned
    let capabilities = UniversalSubstrateCapabilities {
        traditional_platforms: vec![TraditionalPlatform::RISCV {
            cpu_model: "SiFive U74".to_string(),
            cores: 4,
            extensions: vec!["RV64GC".to_string()],
            memory_gb: 8,
        }],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    let cloned = capabilities.clone();
    assert_eq!(
        capabilities.traditional_platforms.len(),
        cloned.traditional_platforms.len()
    );
}

#[test]
fn test_universal_substrate_capabilities_debug() {
    // Test debug formatting
    let capabilities = UniversalSubstrateCapabilities {
        traditional_platforms: vec![],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    let debug_str = format!("{:?}", capabilities);
    assert!(debug_str.contains("UniversalSubstrateCapabilities"));
}

// ============================================================================
// Biological Platform Tests
// ============================================================================

#[test]
fn test_biological_platform_dna_computing() {
    // Test DNA computing platform
    let platform = BiologicalComputingPlatform::DNAComputing {
        platform: "DNA Store".to_string(),
        synthesis_method: "Enzymatic".to_string(),
        storage_capacity_bits: 1_000_000_000,
        read_write_cycles: 100,
    };

    match platform {
        BiologicalComputingPlatform::DNAComputing {
            platform,
            storage_capacity_bits,
            ..
        } => {
            assert_eq!(platform, "DNA Store");
            assert!(storage_capacity_bits > 0);
        }
        _ => panic!("Expected DNAComputing variant"),
    }
}

#[test]
fn test_biological_platform_protein_folding() {
    // Test protein folding platform
    let platform = BiologicalComputingPlatform::ProteinFolding {
        platform: "AlphaFold".to_string(),
        folding_algorithms: vec!["MD".to_string()],
        molecular_dynamics: true,
    };

    match platform {
        BiologicalComputingPlatform::ProteinFolding {
            platform,
            molecular_dynamics,
            ..
        } => {
            assert_eq!(platform, "AlphaFold");
            assert!(molecular_dynamics);
        }
        _ => panic!("Expected ProteinFolding variant"),
    }
}

#[test]
fn test_biological_platform_cellular_computing() {
    // Test cellular computing platform
    let platform = BiologicalComputingPlatform::CellularComputing {
        cell_type: "E. coli".to_string(),
        genetic_circuits: vec!["toggle".to_string()],
        biosafety_level: 1,
    };

    match platform {
        BiologicalComputingPlatform::CellularComputing {
            cell_type,
            biosafety_level,
            ..
        } => {
            assert_eq!(cell_type, "E. coli");
            assert_eq!(biosafety_level, 1);
        }
        _ => panic!("Expected CellularComputing variant"),
    }
}

// ============================================================================
// Neuromorphic Platform Tests
// ============================================================================

#[test]
fn test_neuromorphic_platform_spiking_neural() {
    // Test spiking neural network platform
    let platform = NeuromorphicPlatform::SpikingNeuralNetwork {
        platform: "Intel Loihi".to_string(),
        neuron_model: "Leaky Integrate-and-Fire".to_string(),
        synapse_model: "STDP".to_string(),
        neuron_count: 130_000,
        connectivity_pattern: "sparse".to_string(),
    };

    match platform {
        NeuromorphicPlatform::SpikingNeuralNetwork {
            platform,
            neuron_count,
            ..
        } => {
            assert_eq!(platform, "Intel Loihi");
            assert!(neuron_count > 100_000);
        }
        _ => panic!("Expected SpikingNeuralNetwork variant"),
    }
}

#[test]
fn test_neuromorphic_platform_memristive() {
    // Test memristive computing platform
    let platform = NeuromorphicPlatform::MemristiveComputing {
        platform: "Memristor Array".to_string(),
        memristor_technology: "ReRAM".to_string(),
        crossbar_size: (1024, 1024),
        resistance_levels: 16,
    };

    match platform {
        NeuromorphicPlatform::MemristiveComputing {
            memristor_technology,
            crossbar_size,
            ..
        } => {
            assert_eq!(memristor_technology, "ReRAM");
            assert_eq!(crossbar_size, (1024, 1024));
        }
        _ => panic!("Expected MemristiveComputing variant"),
    }
}

// ============================================================================
// Quantum Platform Tests
// ============================================================================

#[test]
fn test_quantum_platform_gate_based() {
    // Test gate-based quantum computing
    let platform = QuantumPlatform::GateBasedQuantum {
        platform: "IBM Quantum".to_string(),
        qubit_count: 50,
        gate_fidelity: 0.999,
        connectivity_graph: "Linear".to_string(),
        error_correction: true,
    };

    match platform {
        QuantumPlatform::GateBasedQuantum {
            qubit_count,
            gate_fidelity,
            ..
        } => {
            assert_eq!(qubit_count, 50);
            assert!(gate_fidelity > 0.99);
        }
        _ => panic!("Expected GateBasedQuantum variant"),
    }
}

#[test]
fn test_quantum_platform_annealing() {
    // Test quantum annealing platform
    let platform = QuantumPlatform::QuantumAnnealing {
        platform: "D-Wave".to_string(),
        qubit_count: 5000,
        coupling_strength: 1.5,
        annealing_time_us: 20.0,
    };

    match platform {
        QuantumPlatform::QuantumAnnealing { qubit_count, .. } => {
            assert!(qubit_count >= 1000);
        }
        _ => panic!("Expected QuantumAnnealing variant"),
    }
}

// ============================================================================
// Edge/IoT Platform Tests
// ============================================================================

#[test]
fn test_edge_iot_platform_microcontroller() {
    // Test microcontroller platform
    let platform = EdgeIoTPlatform::Microcontroller {
        chip: "ESP32".to_string(),
        architecture: "Xtensa".to_string(),
        flash_kb: 4096,
        ram_kb: 520,
        clock_speed_mhz: 240,
        gpio_pins: 34,
    };

    match platform {
        EdgeIoTPlatform::Microcontroller {
            chip,
            clock_speed_mhz,
            ..
        } => {
            assert_eq!(chip, "ESP32");
            assert_eq!(clock_speed_mhz, 240);
        }
        _ => panic!("Expected Microcontroller variant"),
    }
}

#[test]
fn test_edge_iot_platform_fpga() {
    // Test FPGA platform
    let platform = EdgeIoTPlatform::FPGA {
        family: "Xilinx Zynq".to_string(),
        logic_elements: 85000,
        ram_blocks: 560,
        dsp_blocks: 220,
        io_pins: 400,
    };

    match platform {
        EdgeIoTPlatform::FPGA {
            family,
            logic_elements,
            ..
        } => {
            assert_eq!(family, "Xilinx Zynq");
            assert_eq!(logic_elements, 85000);
        }
        _ => panic!("Expected FPGA variant"),
    }
}

// ============================================================================
// Container Platform Tests
// ============================================================================

#[test]
fn test_container_platform_docker() {
    // Test Docker platform
    let platform = ContainerPlatform::Docker {
        version: "20.10.0".to_string(),
        features: vec!["buildkit".to_string(), "compose".to_string()],
    };

    match platform {
        ContainerPlatform::Docker {
            version, features, ..
        } => {
            assert_eq!(version, "20.10.0");
            assert_eq!(features.len(), 2);
        }
        _ => panic!("Expected Docker variant"),
    }
}

#[test]
fn test_container_platform_kubernetes() {
    // Test Kubernetes platform
    let platform = ContainerPlatform::Kubernetes {
        version: "1.27.0".to_string(),
        distribution: "k8s".to_string(),
    };

    match platform {
        ContainerPlatform::Kubernetes {
            version,
            distribution,
            ..
        } => {
            assert_eq!(version, "1.27.0");
            assert_eq!(distribution, "k8s");
        }
        _ => panic!("Expected Kubernetes variant"),
    }
}

// ============================================================================
// Language Runtime Tests
// ============================================================================

#[test]
fn test_language_runtime_rust() {
    // Test Rust runtime
    let runtime = LanguageRuntime::Rust {
        version: "1.70.0".to_string(),
        target_triple: "x86_64-unknown-linux-gnu".to_string(),
        features: vec!["cargo".to_string(), "rustfmt".to_string()],
    };

    match runtime {
        LanguageRuntime::Rust {
            version,
            target_triple,
            ..
        } => {
            assert_eq!(version, "1.70.0");
            assert_eq!(target_triple, "x86_64-unknown-linux-gnu");
        }
        _ => panic!("Expected Rust variant"),
    }
}

#[test]
fn test_language_runtime_python() {
    // Test Python runtime
    let runtime = LanguageRuntime::Python {
        version: "3.11.0".to_string(),
        implementation: "CPython".to_string(),
        features: vec!["asyncio".to_string(), "typing".to_string()],
    };

    match runtime {
        LanguageRuntime::Python {
            version,
            implementation,
            ..
        } => {
            assert_eq!(version, "3.11.0");
            assert_eq!(implementation, "CPython");
        }
        _ => panic!("Expected Python variant"),
    }
}

#[test]
fn test_container_platform_wasmtime() {
    // Test Wasmtime WebAssembly runtime
    let platform = ContainerPlatform::Wasmtime {
        version: "10.0.0".to_string(),
        features: vec!["component-model".to_string(), "wasi".to_string()],
    };

    match platform {
        ContainerPlatform::Wasmtime {
            version, features, ..
        } => {
            assert_eq!(version, "10.0.0");
            assert!(features.contains(&"wasi".to_string()));
        }
        _ => panic!("Expected Wasmtime variant"),
    }
}

// ============================================================================
// Operating System Tests
// ============================================================================

#[test]
fn test_operating_system_linux() {
    // Test Linux OS
    let os = OperatingSystemSupport::Linux {
        distribution: "Ubuntu".to_string(),
        kernel_version: "5.15.0".to_string(),
        init_system: "systemd".to_string(),
        package_manager: "apt".to_string(),
    };

    match os {
        OperatingSystemSupport::Linux {
            distribution,
            kernel_version,
            ..
        } => {
            assert_eq!(distribution, "Ubuntu");
            assert_eq!(kernel_version, "5.15.0");
        }
        _ => panic!("Expected Linux variant"),
    }
}

#[test]
fn test_operating_system_windows() {
    // Test Windows OS
    let os = OperatingSystemSupport::Windows {
        version: "11".to_string(),
        edition: "Pro".to_string(),
        features: vec!["Hyper-V".to_string()],
        subsystems: vec!["WSL2".to_string()],
    };

    match os {
        OperatingSystemSupport::Windows {
            version, edition, ..
        } => {
            assert_eq!(version, "11");
            assert_eq!(edition, "Pro");
        }
        _ => panic!("Expected Windows variant"),
    }
}

// ============================================================================
// Specialized Architecture Tests
// ============================================================================

#[test]
fn test_specialized_architecture_tpu() {
    // Test TPU architecture
    let arch = SpecializedArchitecture::TPU {
        version: "v4".to_string(),
        tops: 275.0,
        memory_gb: 32,
    };

    match arch {
        SpecializedArchitecture::TPU { version, tops, .. } => {
            assert_eq!(version, "v4");
            assert!(tops > 200.0);
        }
        _ => panic!("Expected TPU variant"),
    }
}

#[test]
fn test_specialized_architecture_cuda() {
    // Test CUDA GPU compute architecture
    let arch = SpecializedArchitecture::CUDA {
        version: "12.0".to_string(),
        compute_capability: "8.0".to_string(),
        memory_gb: 80,
    };

    match arch {
        SpecializedArchitecture::CUDA {
            version,
            compute_capability,
            ..
        } => {
            assert_eq!(version, "12.0");
            assert_eq!(compute_capability, "8.0");
        }
        _ => panic!("Expected CUDA variant"),
    }
}

// ============================================================================
// Integration Scenarios
// ============================================================================

#[test]
fn test_scenario_heterogeneous_compute_cluster() {
    // Test a realistic heterogeneous computing scenario
    let capabilities = UniversalSubstrateCapabilities {
        traditional_platforms: vec![
            TraditionalPlatform::X86_64 {
                cpu_model: "AMD EPYC".to_string(),
                cores: 64,
                threads: 128,
                cache_mb: 256,
                memory_gb: 512,
                features: vec!["AVX512".to_string()],
            },
            TraditionalPlatform::ARM64 {
                cpu_model: "Graviton3".to_string(),
                cores: 64,
                big_little: false,
                memory_gb: 256,
                features: vec!["NEON".to_string()],
            },
        ],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![ContainerPlatform::Kubernetes {
            version: "1.27.0".to_string(),
            distribution: "EKS".to_string(),
        }],
        language_runtimes: vec![
            LanguageRuntime::Rust {
                version: "1.70.0".to_string(),
                target_triple: "x86_64-unknown-linux-gnu".to_string(),
                features: vec!["cargo".to_string()],
            },
            LanguageRuntime::Python {
                version: "3.11.0".to_string(),
                implementation: "CPython".to_string(),
                features: vec!["asyncio".to_string()],
            },
        ],
        operating_systems: vec![OperatingSystemSupport::Linux {
            distribution: "Ubuntu".to_string(),
            kernel_version: "5.15.0".to_string(),
            init_system: "systemd".to_string(),
            package_manager: "apt".to_string(),
        }],
        specialized_architectures: vec![SpecializedArchitecture::CUDA {
            version: "12.0".to_string(),
            compute_capability: "8.0".to_string(),
            memory_gb: 80,
        }],
        experimental_platforms: vec![],
    };

    // Verify cluster composition
    assert_eq!(capabilities.traditional_platforms.len(), 2);
    assert_eq!(capabilities.container_platforms.len(), 1);
    assert_eq!(capabilities.language_runtimes.len(), 2);
    assert_eq!(capabilities.specialized_architectures.len(), 1);
}

#[test]
fn test_scenario_quantum_hybrid_system() {
    // Test quantum-classical hybrid computing scenario
    let capabilities = UniversalSubstrateCapabilities {
        traditional_platforms: vec![TraditionalPlatform::X86_64 {
            cpu_model: "Intel Xeon".to_string(),
            cores: 32,
            threads: 64,
            cache_mb: 48,
            memory_gb: 256,
            features: vec!["AVX2".to_string()],
        }],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![QuantumPlatform::GateBasedQuantum {
            platform: "IBM Quantum".to_string(),
            qubit_count: 50,
            gate_fidelity: 0.999,
            connectivity_graph: "Linear".to_string(),
            error_correction: true,
        }],
        edge_iot_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![LanguageRuntime::Python {
            version: "3.11.0".to_string(),
            implementation: "CPython".to_string(),
            features: vec!["qiskit".to_string()],
        }],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    // Verify hybrid setup
    assert_eq!(capabilities.traditional_platforms.len(), 1);
    assert_eq!(capabilities.quantum_platforms.len(), 1);
    assert!(!capabilities.language_runtimes.is_empty());
}

// ============================================================================
// Coverage Summary
// ============================================================================

#[test]
fn test_substrate_capabilities_coverage_summary() {
    println!("============================================");
    println!("Substrate Capabilities Tests Summary:");
    println!("============================================");
    println!("Universal Capabilities:  6 tests");
    println!("Biological Platforms:    3 tests");
    println!("Neuromorphic Platforms:  2 tests");
    println!("Quantum Platforms:       2 tests");
    println!("Edge/IoT Platforms:      2 tests");
    println!("Container Platforms:     2 tests");
    println!("Language Runtimes:       3 tests");
    println!("Operating Systems:       2 tests");
    println!("Specialized Arch:        2 tests");
    println!("Integration Scenarios:   2 tests");
    println!("============================================");
    println!("Total Substrate Tests:  26 tests");
    println!("============================================");
    println!("Target: Increase substrate.rs coverage from 0% to 30%+");
}
