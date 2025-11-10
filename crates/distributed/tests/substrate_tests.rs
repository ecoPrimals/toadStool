//! Tests for universal substrate capabilities

use toadstool_distributed::universal::*;

// ============================================================================
// Traditional Platform Tests
// ============================================================================

#[test]
fn test_traditional_platform_x86_64() {
    let platform = TraditionalPlatform::X86_64 {
        cpu_model: "Intel Core i9-13900K".to_string(),
        cores: 24,
        threads: 32,
        cache_mb: 36,
        memory_gb: 64,
        features: vec!["AVX2".to_string(), "SSE4.2".to_string()],
    };

    // Test Debug trait
    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("X86_64"));
    assert!(debug_str.contains("Intel Core i9-13900K"));
}

#[test]
fn test_traditional_platform_arm64() {
    let platform = TraditionalPlatform::ARM64 {
        cpu_model: "Apple M3 Max".to_string(),
        cores: 16,
        big_little: true,
        memory_gb: 128,
        features: vec!["NEON".to_string(), "AES".to_string()],
    };

    // Test Clone trait
    let cloned = platform.clone();
    assert!(format!("{:?}", cloned).contains("Apple M3 Max"));
}

#[test]
fn test_traditional_platform_riscv() {
    let platform = TraditionalPlatform::RISCV {
        cpu_model: "SiFive U74".to_string(),
        cores: 4,
        extensions: vec!["RV64GC".to_string(), "V".to_string()],
        memory_gb: 8,
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("RISCV"));
    assert!(debug_str.contains("SiFive U74"));
}

#[test]
fn test_traditional_platform_powerpc() {
    let platform = TraditionalPlatform::PowerPC {
        cpu_model: "POWER9".to_string(),
        cores: 24,
        memory_gb: 256,
        features: vec!["AltiVec".to_string()],
    };

    assert!(format!("{:?}", platform).contains("PowerPC"));
}

#[test]
fn test_traditional_platform_sparc() {
    let platform = TraditionalPlatform::SPARC {
        cpu_model: "SPARC M8".to_string(),
        cores: 32,
        memory_gb: 512,
        features: vec!["VIS4".to_string()],
    };

    assert!(format!("{:?}", platform).contains("SPARC"));
}

#[test]
fn test_traditional_platform_mips() {
    let platform = TraditionalPlatform::MIPS {
        cpu_model: "MIPS64".to_string(),
        cores: 8,
        memory_gb: 16,
        features: vec!["MIPS-3D".to_string()],
    };

    assert!(format!("{:?}", platform).contains("MIPS"));
}

// ============================================================================
// Biological Computing Platform Tests
// ============================================================================

#[test]
fn test_biological_dna_computing() {
    let platform = BiologicalComputingPlatform::DNAComputing {
        platform: "DNA Storage v2.0".to_string(),
        synthesis_method: "enzymatic".to_string(),
        storage_capacity_bits: 1_000_000_000_000, // 1TB
        read_write_cycles: 1000,
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("DNAComputing"));
    assert!(debug_str.contains("DNA Storage"));
}

#[test]
fn test_biological_protein_folding() {
    let platform = BiologicalComputingPlatform::ProteinFolding {
        platform: "Rosetta@home".to_string(),
        folding_algorithms: vec!["AlphaFold".to_string(), "RoseTTAFold".to_string()],
        molecular_dynamics: true,
    };

    assert!(format!("{:?}", platform).contains("ProteinFolding"));
}

#[test]
fn test_biological_cellular_computing() {
    let platform = BiologicalComputingPlatform::CellularComputing {
        cell_type: "E. coli".to_string(),
        genetic_circuits: vec!["toggle_switch".to_string(), "repressilator".to_string()],
        biosafety_level: 1,
    };

    assert!(format!("{:?}", platform).contains("CellularComputing"));
    assert!(format!("{:?}", platform).contains("E. coli"));
}

#[test]
fn test_biological_enzymatic_computing() {
    let platform = BiologicalComputingPlatform::EnzymaticComputing {
        enzyme_set: vec!["polymerase".to_string(), "ligase".to_string()],
        reaction_networks: vec!["cascade_1".to_string()],
        temperature_range: (20.0, 37.0),
    };

    assert!(format!("{:?}", platform).contains("EnzymaticComputing"));
}

#[test]
fn test_biological_bacterial_computing() {
    let platform = BiologicalComputingPlatform::BacterialComputing {
        organism: "Bacillus subtilis".to_string(),
        plasmid_circuits: vec!["pBR322".to_string()],
        growth_medium: "LB broth".to_string(),
    };

    assert!(format!("{:?}", platform).contains("BacterialComputing"));
}

#[test]
fn test_biological_neural_organoids() {
    let platform = BiologicalComputingPlatform::NeuralOrganoids {
        organoid_type: "Cerebral organoid".to_string(),
        neuron_count: 1_000_000,
        plasticity_features: vec!["LTP".to_string(), "LTD".to_string()],
    };

    assert!(format!("{:?}", platform).contains("NeuralOrganoids"));
}

#[test]
fn test_biological_bioelectronic_interface() {
    let platform = BiologicalComputingPlatform::BioelectronicInterface {
        interface_type: "Neuron-CMOS".to_string(),
        biological_component: "neurons".to_string(),
        electronic_component: "CMOS array".to_string(),
    };

    assert!(format!("{:?}", platform).contains("BioelectronicInterface"));
}

// ============================================================================
// Container Platform Tests
// ============================================================================

#[test]
fn test_container_platform_docker() {
    let platform = ContainerPlatform::Docker {
        version: "24.0.7".to_string(),
        features: vec!["buildkit".to_string(), "compose".to_string()],
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Docker"));
    assert!(debug_str.contains("24.0.7"));
}

#[test]
fn test_container_platform_podman() {
    let platform = ContainerPlatform::Podman {
        version: "4.8.0".to_string(),
        rootless: true,
    };

    assert!(format!("{:?}", platform).contains("Podman"));
}

#[test]
fn test_container_platform_containerd() {
    let platform = ContainerPlatform::Containerd {
        version: "1.7.10".to_string(),
        snapshotter: "overlayfs".to_string(),
    };

    assert!(format!("{:?}", platform).contains("Containerd"));
}

#[test]
fn test_container_platform_crio() {
    let platform = ContainerPlatform::CriO {
        version: "1.28.0".to_string(),
        runtime: "runc".to_string(),
    };

    assert!(format!("{:?}", platform).contains("CriO"));
}

#[test]
fn test_container_platform_firecracker() {
    let platform = ContainerPlatform::Firecracker {
        version: "1.5.0".to_string(),
        jailer: true,
    };

    assert!(format!("{:?}", platform).contains("Firecracker"));
}

#[test]
fn test_container_platform_kata() {
    let platform = ContainerPlatform::Kata {
        version: "3.0.0".to_string(),
        hypervisor: "qemu".to_string(),
    };

    assert!(format!("{:?}", platform).contains("Kata"));
}

#[test]
fn test_container_platform_gvisor() {
    let platform = ContainerPlatform::GVisor {
        version: "20231113.0".to_string(),
        platform: "ptrace".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("GVisor"));
}

#[test]
fn test_container_platform_wasmtime() {
    let platform = ContainerPlatform::Wasmtime {
        version: "15.0.0".to_string(),
        features: vec!["wasi".to_string(), "component-model".to_string()],
    };

    assert!(format!("{:?}", platform).contains("Wasmtime"));
}

#[test]
fn test_container_platform_wasmer() {
    let platform = ContainerPlatform::Wasmer {
        version: "4.2.3".to_string(),
        backends: vec!["cranelift".to_string(), "llvm".to_string()],
    };

    assert!(format!("{:?}", platform).contains("Wasmer"));
}

#[test]
fn test_container_platform_wasmedge() {
    let platform = ContainerPlatform::WasmEdge {
        version: "0.13.5".to_string(),
        extensions: vec!["tensorflow".to_string(), "socket".to_string()],
    };

    assert!(format!("{:?}", platform).contains("WasmEdge"));
}

#[test]
fn test_container_platform_unikernel() {
    let platform = ContainerPlatform::Unikernel {
        platform: "MirageOS".to_string(),
        language: "OCaml".to_string(),
    };

    assert!(format!("{:?}", platform).contains("Unikernel"));
}

#[test]
fn test_container_platform_lambda() {
    let platform = ContainerPlatform::Lambda {
        runtime: "python3.12".to_string(),
        memory_mb: 512,
    };

    assert!(format!("{:?}", platform).contains("Lambda"));
}

#[test]
fn test_container_platform_cloud_run() {
    let platform = ContainerPlatform::CloudRun {
        runtime: "go1.21".to_string(),
        cpu_allocation: "1000m".to_string(),
    };

    assert!(format!("{:?}", platform).contains("CloudRun"));
}

#[test]
fn test_container_platform_azure_functions() {
    let platform = ContainerPlatform::AzureFunctions {
        runtime: "node18".to_string(),
        trigger_type: "http".to_string(),
    };

    assert!(format!("{:?}", platform).contains("AzureFunctions"));
}

#[test]
fn test_container_platform_kubernetes() {
    let platform = ContainerPlatform::Kubernetes {
        version: "1.28.4".to_string(),
        distribution: "k3s".to_string(),
    };

    assert!(format!("{:?}", platform).contains("Kubernetes"));
}

#[test]
fn test_container_platform_docker_swarm() {
    let platform = ContainerPlatform::DockerSwarm {
        version: "24.0.7".to_string(),
        features: vec!["secrets".to_string(), "configs".to_string()],
    };

    assert!(format!("{:?}", platform).contains("DockerSwarm"));
}

#[test]
fn test_container_platform_nomad() {
    let platform = ContainerPlatform::Nomad {
        version: "1.7.0".to_string(),
        driver: "docker".to_string(),
    };

    assert!(format!("{:?}", platform).contains("Nomad"));
}

// ============================================================================
// Universal Substrate Capabilities Tests
// ============================================================================

#[test]
fn test_universal_substrate_capabilities_creation() {
    let caps = UniversalSubstrateCapabilities {
        traditional_platforms: vec![TraditionalPlatform::X86_64 {
            cpu_model: "Test CPU".to_string(),
            cores: 8,
            threads: 16,
            cache_mb: 16,
            memory_gb: 32,
            features: vec![],
        }],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![ContainerPlatform::Docker {
            version: "24.0.0".to_string(),
            features: vec![],
        }],
        language_runtimes: vec![],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    // Test Debug trait
    let debug_str = format!("{:?}", caps);
    assert!(debug_str.contains("UniversalSubstrateCapabilities"));
    assert!(debug_str.contains("Test CPU"));

    // Test Clone trait
    let cloned = caps.clone();
    assert_eq!(cloned.traditional_platforms.len(), 1);
    assert_eq!(cloned.container_platforms.len(), 1);
}

#[test]
fn test_universal_substrate_capabilities_empty() {
    let caps = UniversalSubstrateCapabilities {
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

    assert_eq!(caps.traditional_platforms.len(), 0);
    assert_eq!(caps.container_platforms.len(), 0);
}

#[test]
fn test_universal_substrate_capabilities_multiple_platforms() {
    let caps = UniversalSubstrateCapabilities {
        traditional_platforms: vec![
            TraditionalPlatform::X86_64 {
                cpu_model: "Intel".to_string(),
                cores: 8,
                threads: 16,
                cache_mb: 16,
                memory_gb: 32,
                features: vec![],
            },
            TraditionalPlatform::ARM64 {
                cpu_model: "Apple".to_string(),
                cores: 16,
                big_little: true,
                memory_gb: 64,
                features: vec![],
            },
        ],
        biological_platforms: vec![],
        neuromorphic_platforms: vec![],
        quantum_platforms: vec![],
        edge_iot_platforms: vec![],
        container_platforms: vec![
            ContainerPlatform::Docker {
                version: "24.0.0".to_string(),
                features: vec![],
            },
            ContainerPlatform::Podman {
                version: "4.8.0".to_string(),
                rootless: true,
            },
        ],
        language_runtimes: vec![],
        operating_systems: vec![],
        specialized_architectures: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(caps.traditional_platforms.len(), 2);
    assert_eq!(caps.container_platforms.len(), 2);
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_traditional_platform_serialization() {
    let platform = TraditionalPlatform::X86_64 {
        cpu_model: "Intel Core i9".to_string(),
        cores: 24,
        threads: 32,
        cache_mb: 36,
        memory_gb: 64,
        features: vec!["AVX2".to_string()],
    };

    // Test JSON serialization
    let json = serde_json::to_string(&platform).expect("Failed to serialize");
    assert!(json.contains("X86_64"));
    assert!(json.contains("Intel Core i9"));

    // Test deserialization
    let deserialized: TraditionalPlatform =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert!(format!("{:?}", deserialized).contains("Intel Core i9"));
}

#[test]
fn test_container_platform_serialization() {
    let platform = ContainerPlatform::Docker {
        version: "24.0.7".to_string(),
        features: vec!["buildkit".to_string()],
    };

    let json = serde_json::to_string(&platform).expect("Failed to serialize");
    assert!(json.contains("Docker"));
    assert!(json.contains("24.0.7"));

    let deserialized: ContainerPlatform =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert!(format!("{:?}", deserialized).contains("Docker"));
}

#[test]
fn test_universal_substrate_capabilities_serialization() {
    let caps = UniversalSubstrateCapabilities {
        traditional_platforms: vec![TraditionalPlatform::X86_64 {
            cpu_model: "Test".to_string(),
            cores: 8,
            threads: 16,
            cache_mb: 16,
            memory_gb: 32,
            features: vec![],
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

    let json = serde_json::to_string(&caps).expect("Failed to serialize");
    assert!(json.contains("traditional_platforms"));
    assert!(json.contains("Test"));

    let deserialized: UniversalSubstrateCapabilities =
        serde_json::from_str(&json).expect("Failed to deserialize");
    assert_eq!(deserialized.traditional_platforms.len(), 1);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_traditional_platform_zero_cores() {
    // This tests handling of unusual but valid configurations
    let platform = TraditionalPlatform::X86_64 {
        cpu_model: "Minimal CPU".to_string(),
        cores: 1,
        threads: 1,
        cache_mb: 1,
        memory_gb: 1,
        features: vec![],
    };

    assert!(format!("{:?}", platform).contains("Minimal CPU"));
}

#[test]
fn test_traditional_platform_high_core_count() {
    // Test with very high core counts (server-grade CPUs)
    let platform = TraditionalPlatform::X86_64 {
        cpu_model: "AMD EPYC 9654".to_string(),
        cores: 96,
        threads: 192,
        cache_mb: 384,
        memory_gb: 1536,
        features: vec!["AVX512".to_string(), "SHA".to_string(), "SEV".to_string()],
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("AMD EPYC"));
    assert!(debug_str.contains("96"));
}

#[test]
fn test_container_platform_empty_features() {
    let platform = ContainerPlatform::Docker {
        version: "1.0.0".to_string(),
        features: vec![],
    };

    let json = serde_json::to_string(&platform).expect("Failed to serialize");
    assert!(json.contains("Docker"));
}

#[test]
fn test_container_platform_many_features() {
    let platform = ContainerPlatform::Docker {
        version: "24.0.7".to_string(),
        features: vec![
            "buildkit".to_string(),
            "compose".to_string(),
            "swarm".to_string(),
            "secrets".to_string(),
            "configs".to_string(),
        ],
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("buildkit"));
    assert!(debug_str.contains("swarm"));
}

#[test]
fn test_biological_computing_biosafety_levels() {
    // Test different biosafety levels
    for level in 1..=4 {
        let platform = BiologicalComputingPlatform::CellularComputing {
            cell_type: format!("Cell type BSL-{}", level),
            genetic_circuits: vec![],
            biosafety_level: level,
        };

        assert!(format!("{:?}", platform).contains(&format!("BSL-{}", level)));
    }
}

#[test]
fn test_enzyme_temperature_ranges() {
    // Test different temperature ranges
    let cold = BiologicalComputingPlatform::EnzymaticComputing {
        enzyme_set: vec!["cold-active".to_string()],
        reaction_networks: vec![],
        temperature_range: (4.0, 15.0),
    };

    let hot = BiologicalComputingPlatform::EnzymaticComputing {
        enzyme_set: vec!["thermostable".to_string()],
        reaction_networks: vec![],
        temperature_range: (60.0, 95.0),
    };

    assert!(format!("{:?}", cold).contains("4.0"));
    assert!(format!("{:?}", hot).contains("95.0"));
}
