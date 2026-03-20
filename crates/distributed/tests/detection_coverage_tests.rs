// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests targeting `universal::detection` behavior and substrate types produced by detection.
//!
//! Exercises `UniversalSubstrateCapabilities::detect_all` and serde, `Debug`, `Clone`, and helper APIs on
//! substrate enums and `UniversalSubstrateCapabilities` used by the detection pipeline.

#![allow(clippy::expect_used, clippy::float_cmp, clippy::too_many_lines)]

use serde::{Serialize, de::DeserializeOwned};
use toadstool_distributed::substrate::{
    BiologicalComputingPlatform, ContainerPlatform, EdgeIoTPlatform, ExperimentalPlatform,
    LanguageRuntime, NeuromorphicPlatform, OperatingSystemSupport, QuantumPlatform,
    SpecializedArchitecture, TraditionalPlatform, UniversalSubstrateCapabilities,
};

fn serde_json_roundtrip<T>(value: &T) -> T
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let json = serde_json::to_string(value).expect("serialize");
    serde_json::from_str(&json).expect("deserialize")
}

#[tokio::test]
async fn detect_all_succeeds_and_populates_known_subsystems() {
    let result = UniversalSubstrateCapabilities::detect_all().await;
    assert!(
        result.is_ok(),
        "detect_all returns Ok in normal environments"
    );
    let caps = result.expect("detect_all ok");

    match std::env::consts::ARCH {
        "x86_64" | "aarch64" => assert!(
            !caps.traditional_platforms.is_empty(),
            "traditional CPU family expected on this host"
        ),
        _ => {}
    }

    match std::env::consts::OS {
        "linux" | "macos" | "windows" => assert!(
            !caps.operating_systems.is_empty(),
            "OS entry expected for supported host OS"
        ),
        _ => {}
    }

    assert!(
        caps.total_platforms() > 0,
        "at least one subsystem should report on typical hosts"
    );
}

#[tokio::test]
async fn detect_all_result_is_cloneable_and_debuggable() {
    let caps = UniversalSubstrateCapabilities::detect_all()
        .await
        .expect("detect_all ok");
    let cloned = caps.clone();
    assert_eq!(caps, cloned);
    let dbg = format!("{caps:?}");
    assert!(dbg.contains("UniversalSubstrateCapabilities"));
}

#[test]
fn universal_substrate_capabilities_default_new_and_empty() {
    let a = UniversalSubstrateCapabilities::default();
    let b = UniversalSubstrateCapabilities::new();
    assert_eq!(a, b);
    assert!(a.is_empty());
    assert_eq!(a.total_platforms(), 0);
    assert!(!a.has_traditional_platforms());
    assert!(!a.has_container_platforms());
    assert!(!a.has_language_runtimes());
    assert!(!a.has_operating_systems());
    assert!(!a.has_ai_accelerators());
    assert!(!a.has_quantum_platforms());
    assert!(!a.has_experimental_platforms());
}

#[test]
fn empty_capabilities_serde_roundtrip() {
    let empty = UniversalSubstrateCapabilities::new();
    let back = serde_json_roundtrip(&empty);
    assert_eq!(empty, back);
}

#[test]
fn serde_rejects_invalid_json_for_capabilities() {
    let err = serde_json::from_str::<UniversalSubstrateCapabilities>("not json");
    assert!(err.is_err());
}

#[test]
fn serde_rejects_wrong_shape_for_capabilities() {
    let err = serde_json::from_str::<UniversalSubstrateCapabilities>(r#"{"unexpected":true}"#);
    assert!(err.is_err());
}

#[test]
fn full_capabilities_payload_serde_roundtrip() {
    let caps = UniversalSubstrateCapabilities {
        traditional_platforms: vec![
            TraditionalPlatform::X86_64 {
                cpu_model: "m".into(),
                cores: 4,
                threads: 8,
                cache_mb: 8,
                memory_gb: 16,
                features: vec!["avx2".into()],
            },
            TraditionalPlatform::ARM64 {
                cpu_model: "m".into(),
                cores: 4,
                big_little: false,
                memory_gb: 8,
                features: vec![],
            },
            TraditionalPlatform::RISCV {
                cpu_model: "m".into(),
                cores: 2,
                extensions: vec!["m".into()],
                memory_gb: 4,
            },
            TraditionalPlatform::PowerPC {
                cpu_model: "m".into(),
                cores: 2,
                memory_gb: 4,
                features: vec![],
            },
            TraditionalPlatform::SPARC {
                cpu_model: "m".into(),
                cores: 2,
                memory_gb: 4,
                features: vec![],
            },
            TraditionalPlatform::MIPS {
                cpu_model: "m".into(),
                cores: 2,
                memory_gb: 4,
                features: vec![],
            },
        ],
        biological_platforms: vec![BiologicalComputingPlatform::DNAComputing {
            platform: "p".into(),
            synthesis_method: "s".into(),
            storage_capacity_bits: 1,
            read_write_cycles: 2,
        }],
        neuromorphic_platforms: vec![NeuromorphicPlatform::NeuromorphicChip {
            chip_name: "c".into(),
            manufacturer: "m".into(),
            core_count: 1,
            neuron_count_per_core: 2,
            synapse_count_per_core: 3,
            power_consumption_mw: 1.0,
        }],
        quantum_platforms: vec![QuantumPlatform::QuantumSimulator {
            platform: "p".into(),
            simulation_type: "t".into(),
            classical_qubits_simulated: 4,
        }],
        edge_iot_platforms: vec![EdgeIoTPlatform::SmartDevice {
            device_type: "d".into(),
            capabilities: vec![],
            connectivity: vec![],
            ai_acceleration: false,
        }],
        container_platforms: vec![ContainerPlatform::Nomad {
            version: "1".into(),
            driver: "d".into(),
        }],
        language_runtimes: vec![LanguageRuntime::Whitespace {
            interpreter: "i".into(),
        }],
        operating_systems: vec![OperatingSystemSupport::TempleOS {
            version: "5".into(),
        }],
        specialized_architectures: vec![SpecializedArchitecture::AnalogComputer {
            type_name: "t".into(),
            precision_bits: 8,
            bandwidth_mhz: 1.0,
        }],
        experimental_platforms: vec![ExperimentalPlatform::PlasmaComputing {
            plasma_type: "p".into(),
            confinement_method: "c".into(),
            processing_frequency_mhz: 1.0,
        }],
    };

    let back = serde_json_roundtrip(&caps);
    assert_eq!(caps, back);
    assert!(caps.has_ai_accelerators() == back.has_ai_accelerators());
}

#[test]
fn traditional_platform_methods_and_debug_clone() {
    let p = TraditionalPlatform::X86_64 {
        cpu_model: "Intel".into(),
        cores: 16,
        threads: 32,
        cache_mb: 32,
        memory_gb: 64,
        features: vec!["AVX2".into()],
    };
    let _ = format!("{p:?}");
    assert_eq!(p.clone(), p);
    assert_eq!(p.architecture_name(), "x86_64");
    assert_eq!(p.cores(), 16);
    assert_eq!(p.memory_gb(), 64);
    assert!(p.has_feature("avx2"));
    assert!(p.is_high_performance());
}

#[test]
fn container_platform_methods_and_serde() {
    let p = ContainerPlatform::GVisor {
        version: "1".into(),
        platform: "linux/amd64".into(),
    };
    assert_eq!(p.platform_type(), "gVisor");
    assert!(p.is_vm_based());
    let q = serde_json_roundtrip(&p);
    assert_eq!(p, q);
}

#[test]
fn biological_platform_serde_and_helpers() {
    let p = BiologicalComputingPlatform::EnzymaticComputing {
        enzyme_set: vec!["e".into()],
        reaction_networks: vec![],
        temperature_range: (0.0, 100.0),
    };
    assert!(!p.requires_biosafety());
    let q = serde_json_roundtrip(&p);
    assert_eq!(p, q);
}

#[test]
fn neuromorphic_platform_variants_roundtrip() {
    let samples = [
        NeuromorphicPlatform::SpikingNeuralNetwork {
            platform: "p".into(),
            neuron_model: "LIF".into(),
            synapse_model: "STDP".into(),
            neuron_count: 1,
            connectivity_pattern: "c".into(),
        },
        NeuromorphicPlatform::LiquidStateMachine {
            platform: "p".into(),
            liquid_neuron_count: 1,
            readout_neuron_count: 1,
            temporal_dynamics: "t".into(),
        },
        NeuromorphicPlatform::OpticalNeuralNetwork {
            platform: "p".into(),
            wavelength_channels: 1,
            photonic_neurons: 1,
            optical_switches: 1,
        },
        NeuromorphicPlatform::AnalogNeuralNetwork {
            platform: "p".into(),
            analog_neurons: 1,
            precision_bits: 8,
            noise_characteristics: "n".into(),
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn quantum_platform_variants_roundtrip() {
    let samples = [
        QuantumPlatform::GateBasedQuantum {
            platform: "p".into(),
            qubit_count: 2,
            gate_fidelity: 0.9,
            connectivity_graph: "g".into(),
            error_correction: false,
        },
        QuantumPlatform::PhotonicQuantum {
            platform: "p".into(),
            photon_sources: 1,
            beam_splitters: 1,
            detectors: 1,
            squeezing_level_db: 1.0,
        },
        QuantumPlatform::TrappedIonQuantum {
            platform: "p".into(),
            ion_species: "Yb".into(),
            trap_frequency_mhz: 1.0,
            laser_cooling: true,
        },
        QuantumPlatform::SuperconductingQuantum {
            platform: "p".into(),
            qubit_type: "transmon".into(),
            operating_temperature_mk: 15.0,
            coherence_time_us: 10.0,
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn edge_iot_platform_variants_roundtrip() {
    let samples = [
        EdgeIoTPlatform::IoTSensor {
            sensor_type: "t".into(),
            measurement_range: "r".into(),
            power_consumption_uw: 500.0,
            communication_protocol: "I2C".into(),
        },
        EdgeIoTPlatform::FPGA {
            family: "f".into(),
            logic_elements: 1,
            ram_blocks: 1,
            dsp_blocks: 1,
            io_pins: 1,
        },
        EdgeIoTPlatform::NPU {
            chip: "c".into(),
            tops_performance: 1.0,
            power_efficiency_tops_per_watt: 1.0,
            supported_frameworks: vec![],
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn language_runtime_many_variants_roundtrip() {
    let samples = vec![
        LanguageRuntime::Rust {
            version: "1".into(),
            target_triple: "t".into(),
            features: vec![],
        },
        LanguageRuntime::Cpp {
            compiler: "c++".into(),
            standard: "20".into(),
            features: vec![],
        },
        LanguageRuntime::Zig {
            version: "1".into(),
            target: "t".into(),
            mode: "Release".into(),
        },
        LanguageRuntime::CSharp {
            version: "1".into(),
            runtime: "dotnet".into(),
            framework: "net8".into(),
        },
        LanguageRuntime::Ruby {
            version: "3".into(),
            implementation: "mri".into(),
        },
        LanguageRuntime::Kotlin {
            version: "1".into(),
            target: "jvm".into(),
        },
        LanguageRuntime::Scala {
            version: "1".into(),
            platform: "jvm".into(),
        },
        LanguageRuntime::OCaml {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Elixir {
            version: "1".into(),
            otp_version: "26".into(),
        },
        LanguageRuntime::FSharp {
            version: "1".into(),
            runtime: "dotnet".into(),
        },
        LanguageRuntime::Lisp {
            dialect: "common".into(),
            implementation: "sbcl".into(),
        },
        LanguageRuntime::PowerShell {
            version: "1".into(),
            platform: "core".into(),
        },
        LanguageRuntime::Lua {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Perl {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::R {
            version: "1".into(),
            packages: vec![],
        },
        LanguageRuntime::Matlab {
            version: "1".into(),
            toolboxes: vec![],
        },
        LanguageRuntime::Mathematica {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Julia {
            version: "1".into(),
            packages: vec![],
        },
        LanguageRuntime::Mojo {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Carbon {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Gleam {
            version: "1".into(),
            target: "erlang".into(),
        },
        LanguageRuntime::Crystal {
            version: "1".into(),
            features: vec![],
        },
        LanguageRuntime::Assembly {
            architecture: "x86".into(),
            assembler: "nasm".into(),
            format: "elf".into(),
        },
        LanguageRuntime::Brainfuck {
            interpreter: "bf".into(),
        },
        LanguageRuntime::Shakespeare {
            interpreter: "s".into(),
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
        let _ = p.language_name();
    }
}

#[test]
fn operating_system_support_variants_roundtrip() {
    let samples = vec![
        OperatingSystemSupport::BSD {
            variant: "freebsd".into(),
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Android {
            version: "14".into(),
            api_level: 34,
            security_patch: "p".into(),
        },
        OperatingSystemSupport::IOS {
            version: "17".into(),
            device_family: "iPhone".into(),
            capabilities: vec![],
        },
        OperatingSystemSupport::FreeRTOS {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Zephyr {
            version: "1".into(),
            boards: vec![],
        },
        OperatingSystemSupport::VxWorks {
            version: "1".into(),
            bsp: "b".into(),
        },
        OperatingSystemSupport::QNX {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::RTLinux {
            version: "1".into(),
            latency_us: 1.0,
        },
        OperatingSystemSupport::Xenomai {
            version: "1".into(),
            skin: "posix".into(),
        },
        OperatingSystemSupport::Xen {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::VMware {
            product: "ESXi".into(),
            version: "1".into(),
        },
        OperatingSystemSupport::HyperV {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::KVM {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Plan9 {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Inferno {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::MenuetOS {
            version: "1".into(),
        },
        OperatingSystemSupport::KolibriOS {
            version: "1".into(),
        },
        OperatingSystemSupport::MSDOS {
            version: "6".into(),
        },
        OperatingSystemSupport::OS2 {
            version: "1".into(),
        },
        OperatingSystemSupport::BeOS {
            version: "1".into(),
        },
        OperatingSystemSupport::AmigaOS {
            version: "1".into(),
        },
        OperatingSystemSupport::AtariTOS {
            version: "1".into(),
        },
        OperatingSystemSupport::ZOS {
            version: "1".into(),
            subsystems: vec![],
        },
        OperatingSystemSupport::OpenVMS {
            version: "1".into(),
            clustering: false,
        },
        OperatingSystemSupport::UNICOS {
            version: "1".into(),
            features: vec![],
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn specialized_architecture_remaining_variants_roundtrip() {
    let samples = [
        SpecializedArchitecture::Vulkan {
            version: "1".into(),
            features: vec![],
        },
        SpecializedArchitecture::Metal {
            version: "1".into(),
            feature_set: "f".into(),
        },
        SpecializedArchitecture::PhotonicProcessor {
            wavelengths: 1,
            switching_speed_ghz: 1.0,
            power_consumption_w: 1.0,
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn experimental_platform_remaining_variants_roundtrip() {
    let samples = [
        ExperimentalPlatform::MolecularComputing {
            platform: "p".into(),
            molecular_basis: "DNA".into(),
            operation_temperature_k: 300.0,
        },
        ExperimentalPlatform::MetamaterialProcessor {
            material: "m".into(),
            frequency_range_ghz: (1.0, 2.0),
            processing_method: "p".into(),
        },
        ExperimentalPlatform::CrystallineComputing {
            crystal_structure: "c".into(),
            defect_type: "d".into(),
            coherence_time_ms: 1.0,
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn has_ai_accelerators_true_when_tpu_present() {
    let mut caps = UniversalSubstrateCapabilities::new();
    caps.specialized_architectures
        .push(SpecializedArchitecture::TPU {
            version: "v4".into(),
            tops: 1.0,
            memory_gb: 8,
        });
    assert!(caps.has_ai_accelerators());
}

#[test]
fn biological_platform_all_variants_roundtrip() {
    let samples = [
        BiologicalComputingPlatform::ProteinFolding {
            platform: "p".into(),
            folding_algorithms: vec![],
            molecular_dynamics: false,
        },
        BiologicalComputingPlatform::CellularComputing {
            cell_type: "c".into(),
            genetic_circuits: vec![],
            biosafety_level: 1,
        },
        BiologicalComputingPlatform::BacterialComputing {
            organism: "o".into(),
            plasmid_circuits: vec![],
            growth_medium: "m".into(),
        },
        BiologicalComputingPlatform::NeuralOrganoids {
            organoid_type: "o".into(),
            neuron_count: 1,
            plasticity_features: vec![],
        },
        BiologicalComputingPlatform::BioelectronicInterface {
            interface_type: "i".into(),
            biological_component: "b".into(),
            electronic_component: "e".into(),
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn neuromorphic_memristive_and_echo_state_roundtrip() {
    let m = NeuromorphicPlatform::MemristiveComputing {
        platform: "p".into(),
        memristor_technology: "t".into(),
        crossbar_size: (2, 2),
        resistance_levels: 4,
    };
    assert_eq!(m, serde_json_roundtrip(&m));
    let e = NeuromorphicPlatform::EchoStateNetwork {
        platform: "p".into(),
        reservoir_size: 10,
        connectivity_density: 0.1,
        spectral_radius: 0.9,
        input_scaling: 1.0,
        leak_rate: 0.1,
    };
    assert_eq!(e, serde_json_roundtrip(&e));
}

#[test]
fn edge_iot_microcontroller_and_sbc_roundtrip() {
    let m = EdgeIoTPlatform::Microcontroller {
        chip: "c".into(),
        architecture: "a".into(),
        flash_kb: 1,
        ram_kb: 2,
        clock_speed_mhz: 3,
        gpio_pins: 4,
    };
    assert_eq!(m, serde_json_roundtrip(&m));
    let s = EdgeIoTPlatform::SingleBoardComputer {
        board: "b".into(),
        soc: "s".into(),
        ram_mb: 512,
        storage_type: "sd".into(),
        connectivity: vec!["GPIO".into()],
    };
    assert_eq!(s, serde_json_roundtrip(&s));
}

#[test]
fn quantum_annealing_roundtrip() {
    let p = QuantumPlatform::QuantumAnnealing {
        platform: "p".into(),
        qubit_count: 8,
        coupling_strength: 1.0,
        annealing_time_us: 5.0,
    };
    assert_eq!(p, serde_json_roundtrip(&p));
}

#[test]
fn container_platform_docker_k8s_lambda_roundtrip() {
    let samples = [
        ContainerPlatform::Docker {
            version: "v".into(),
            features: vec![],
        },
        ContainerPlatform::Kubernetes {
            version: "v".into(),
            distribution: "vanilla".into(),
        },
        ContainerPlatform::Lambda {
            runtime: "r".into(),
            memory_mb: 128,
        },
        ContainerPlatform::Containerd {
            version: "v".into(),
            snapshotter: "overlay".into(),
        },
    ];
    for p in samples {
        assert_eq!(p, serde_json_roundtrip(&p));
    }
}

#[test]
fn language_runtime_c_go_python_java_js_erlang_haskell_roundtrip() {
    let samples = vec![
        LanguageRuntime::C {
            compiler: "gcc".into(),
            standard: "c17".into(),
            optimizations: vec!["O2".into()],
        },
        LanguageRuntime::Go {
            version: "1.22".into(),
            goos: "linux".into(),
            goarch: "amd64".into(),
        },
        LanguageRuntime::Python {
            version: "3.12".into(),
            implementation: "CPython".into(),
            features: vec![],
        },
        LanguageRuntime::Java {
            version: "21".into(),
            vm: "OpenJDK".into(),
            gc: "G1".into(),
        },
        LanguageRuntime::JavaScript {
            engine: "V8".into(),
            version: "20".into(),
            features: vec![],
        },
        LanguageRuntime::Erlang {
            version: "26".into(),
            otp_version: "26".into(),
        },
        LanguageRuntime::Haskell {
            compiler: "ghc".into(),
            version: "9.6".into(),
            extensions: vec![],
        },
        LanguageRuntime::Bash {
            version: "5".into(),
            features: vec![],
        },
    ];
    for p in samples {
        assert_eq!(p, serde_json_roundtrip(&p));
    }
}

#[test]
fn specialized_architecture_npu_cuda_rocm_opencl_dpu_asic_dsp_ipu_roundtrip() {
    let samples = [
        SpecializedArchitecture::NPU {
            chip: "c".into(),
            tops: 2.0,
            frameworks: vec![],
        },
        SpecializedArchitecture::IPU {
            generation: "g".into(),
            tiles: 4,
            memory_gb: 8,
        },
        SpecializedArchitecture::CUDA {
            version: "12".into(),
            compute_capability: "8.0".into(),
            memory_gb: 16,
        },
        SpecializedArchitecture::ROCm {
            version: "5".into(),
            gfx_version: "gfx1030".into(),
            memory_gb: 8,
        },
        SpecializedArchitecture::OpenCL {
            version: "3".into(),
            device_type: "GPU".into(),
            compute_units: 32,
        },
        SpecializedArchitecture::DSP {
            family: "f".into(),
            mips: 100.0,
            special_instructions: vec![],
        },
        SpecializedArchitecture::DPU {
            chip: "c".into(),
            packet_processing_mpps: 1.0,
            cores: 2,
        },
        SpecializedArchitecture::ASIC {
            application: "a".into(),
            performance_metric: "m".into(),
            value: 1.0,
        },
    ];
    for p in samples {
        assert_eq!(p, serde_json_roundtrip(&p));
    }
}

#[test]
fn experimental_platform_cyborg_spin_superconducting_reversible_roundtrip() {
    let samples = [
        ExperimentalPlatform::CyborgSystems {
            biological_component: "b".into(),
            electronic_component: "e".into(),
            interface_protocol: "i".into(),
        },
        ExperimentalPlatform::SpintronicsProcessor {
            technology: "MRAM".into(),
            spin_coherence_time_ns: 1.0,
            operating_temperature_k: 300.0,
        },
        ExperimentalPlatform::SuperconductingClassical {
            technology: "SFQ".into(),
            operating_temperature_k: 4.2,
            switching_energy_j: 1e-18,
        },
        ExperimentalPlatform::ReversibleComputing {
            platform: "p".into(),
            reversibility_factor: 0.9,
            energy_efficiency: 0.99,
        },
    ];
    for p in samples {
        assert_eq!(p, serde_json_roundtrip(&p));
    }
}

#[test]
fn operating_system_linux_macos_windows_roundtrip() {
    let linux = OperatingSystemSupport::Linux {
        distribution: "d".into(),
        kernel_version: "k".into(),
        init_system: "systemd".into(),
        package_manager: "apt".into(),
    };
    assert_eq!(linux, serde_json_roundtrip(&linux));
    let macos = OperatingSystemSupport::MacOS {
        version: "14".into(),
        architecture: "arm64".into(),
        frameworks: vec![],
    };
    assert_eq!(macos, serde_json_roundtrip(&macos));
    let win = OperatingSystemSupport::Windows {
        version: "11".into(),
        edition: "Pro".into(),
        features: vec![],
        subsystems: vec![],
    };
    assert_eq!(win, serde_json_roundtrip(&win));
}
