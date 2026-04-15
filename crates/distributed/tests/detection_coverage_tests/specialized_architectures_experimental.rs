// SPDX-License-Identifier: AGPL-3.0-or-later
use super::common::serde_json_roundtrip;
use toadstool_distributed::substrate::{ExperimentalPlatform, SpecializedArchitecture};

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
