// SPDX-License-Identifier: AGPL-3.0-or-later
use super::common::serde_json_roundtrip;
use toadstool_distributed::substrate::{
    BiologicalComputingPlatform, ContainerPlatform, TraditionalPlatform,
};

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
