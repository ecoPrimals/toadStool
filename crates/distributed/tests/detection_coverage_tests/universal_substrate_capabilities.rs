// SPDX-License-Identifier: AGPL-3.0-or-later
use super::common::serde_json_roundtrip;
use toadstool_distributed::substrate::{
    BiologicalComputingPlatform, ContainerPlatform, EdgeIoTPlatform, ExperimentalPlatform,
    LanguageRuntime, NeuromorphicPlatform, OperatingSystemSupport, QuantumPlatform,
    SpecializedArchitecture, TraditionalPlatform, UniversalSubstrateCapabilities,
};

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
