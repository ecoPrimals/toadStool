// SPDX-License-Identifier: AGPL-3.0-only

use crate::songbird_integration::types::*;

#[test]
fn test_node_type_all_variants_serde() {
    for nt in [
        NodeType::ToadStool,
        NodeType::NestGate,
        NodeType::BearDog,
        NodeType::Songbird,
        NodeType::Custom("my-type".to_string()),
    ] {
        let json = serde_json::to_string(&nt).unwrap();
        let _: NodeType = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_node_registration_serde() {
    let reg = NodeRegistration {
        node_id: "node-1".to_string(),
        node_type: NodeType::ToadStool,
        capabilities: NodeCapabilities {
            cpu_cores: 4.0,
            memory_gb: 8.0,
            storage_gb: 100.0,
            gpu_count: 0,
            specialized_hardware: vec![],
            software_capabilities: vec![],
        },
        endpoints: vec!["http://localhost:8080".to_string()],
        protocols: vec!["http".to_string()],
        metadata: NodeMetadata {
            version: "1.0".to_string(),
            build_info: "test".to_string(),
            capabilities: NodeCapabilities {
                cpu_cores: 4.0,
                memory_gb: 8.0,
                storage_gb: 100.0,
                gpu_count: 0,
                specialized_hardware: vec![],
                software_capabilities: vec![],
            },
        },
    };
    let json = serde_json::to_string(&reg).unwrap();
    let parsed: NodeRegistration = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.node_id, reg.node_id);
}

#[test]
fn test_node_metadata_serde() {
    let meta = NodeMetadata {
        version: "2.0".to_string(),
        build_info: "release".to_string(),
        capabilities: NodeCapabilities {
            cpu_cores: 8.0,
            memory_gb: 16.0,
            storage_gb: 200.0,
            gpu_count: 1,
            specialized_hardware: vec!["cuda".to_string()],
            software_capabilities: vec!["wasm".to_string()],
        },
    };
    let json = serde_json::to_string(&meta).unwrap();
    let parsed: NodeMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.version, "2.0");
}

#[test]
fn test_network_requirements_serde() {
    let req = NetworkRequirements {
        bandwidth_mbps: Some(1000),
        latency_ms: Some(50),
        reliability_percent: Some(99.9),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: NetworkRequirements = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.bandwidth_mbps, Some(1000));
}

#[test]
fn test_load_metric_constructor() {
    let metric = LoadMetric {
        cpu_load: 0.5,
        memory_load: 0.3,
        network_load: 0.1,
    };
    assert!((metric.cpu_load - 0.5).abs() < 0.001);
}
