// SPDX-License-Identifier: AGPL-3.0-or-later
//! build_capabilities edge cases, Debug format, Clone tests.
use super::*;

// --- build_capabilities edge cases ---

#[test]
fn test_build_capabilities_memory_exactly_16gb() {
    let resources = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&CAP_MEMORY_MEDIUM.to_string()));
}

#[test]
fn test_build_capabilities_memory_exactly_64gb() {
    let resources = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 64 * 1024 * 1024 * 1024,
        available_memory_bytes: 32 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&CAP_MEMORY_LARGE.to_string()));
}

#[test]
fn test_build_capabilities_memory_one_byte_under_16gb() {
    let resources = SystemResources {
        cpu_cores: 2,
        total_memory_bytes: 16 * 1024 * 1024 * 1024 - 1,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&CAP_MEMORY_SMALL.to_string()));
}

#[test]
fn test_build_capabilities_memory_one_byte_under_64gb() {
    let resources = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 64 * 1024 * 1024 * 1024 - 1,
        available_memory_bytes: 32 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&CAP_MEMORY_MEDIUM.to_string()));
}

#[test]
fn test_build_capabilities_apple_gpu() {
    let resources = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        gpu_devices: vec![GpuDevice {
            device_id: 0,
            name: "Apple M2".to_string(),
            vendor: "apple".to_string(),
            memory_bytes: 8 * 1024 * 1024 * 1024,
            compute_capability: None,
            render_node: None,
            driver: Some("metal".to_string()),
            arch: None,
        }],
        architecture: "aarch64".to_string(),
        os: "macos".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&"gpu-0".to_string()));
    assert!(caps.contains(&"gpu-apple".to_string()));
    assert!(caps.contains(&"gpu-apple-Apple M2".to_string()));
}

#[test]
fn test_build_capabilities_intel_gpu() {
    let resources = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![GpuDevice {
            device_id: 0,
            name: "Intel UHD 770".to_string(),
            vendor: "intel".to_string(),
            memory_bytes: 256 * 1024 * 1024,
            compute_capability: None,
            render_node: None,
            driver: Some("i915".to_string()),
            arch: None,
        }],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&"gpu-intel".to_string()));
    assert!(caps.contains(&"gpu-intel-Intel UHD 770".to_string()));
}

#[test]
fn test_build_capabilities_max_cpu_cores() {
    let resources = SystemResources {
        cpu_cores: 1024,
        total_memory_bytes: 128 * 1024 * 1024 * 1024,
        available_memory_bytes: 64 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&"cpu-cores-1024".to_string()));
}

// --- Struct serialization, Debug, Clone ---

#[test]
fn test_system_resources_serialization_roundtrip() {
    let res = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        gpu_devices: vec![GpuDevice {
            device_id: 0,
            name: "Test".to_string(),
            vendor: "nvidia".to_string(),
            memory_bytes: 8 * 1024 * 1024 * 1024,
            compute_capability: Some("8.0".to_string()),
            render_node: None,
            driver: None,
            arch: None,
        }],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let json = serde_json::to_string(&res).expect("serialize");
    let parsed: SystemResources = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.cpu_cores, res.cpu_cores);
    assert_eq!(parsed.total_memory_bytes, res.total_memory_bytes);
    assert_eq!(parsed.gpu_devices.len(), res.gpu_devices.len());
    assert_eq!(parsed.architecture, res.architecture);
}

#[test]
fn test_gpu_device_clone() {
    let gpu = GpuDevice {
        device_id: 1,
        name: "RTX 4080".to_string(),
        vendor: "nvidia".to_string(),
        memory_bytes: 16 * 1024 * 1024 * 1024,
        compute_capability: Some("8.9".to_string()),
        render_node: Some("/dev/dri/renderD128".to_string()),
        driver: Some("nvidia".to_string()),
        arch: Some("sm_89".to_string()),
    };
    let cloned = gpu.clone();
    assert_eq!(cloned.device_id, gpu.device_id);
    assert_eq!(cloned.name, gpu.name);
    assert_eq!(cloned.vendor, gpu.vendor);
    assert_eq!(cloned.render_node, gpu.render_node);
    assert_eq!(cloned.driver, gpu.driver);
    assert_eq!(cloned.arch, gpu.arch);
}

#[test]
fn test_primal_capabilities_debug_format() {
    let caps = PrimalCapabilities {
        primal_id: "debug-test".to_string(),
        primal_type: primals::TOADSTOOL.to_string(),
        version: "0.1.0".to_string(),
        resources: SystemResources {
            cpu_cores: 4,
            total_memory_bytes: 16 * 1024 * 1024 * 1024,
            available_memory_bytes: 8 * 1024 * 1024 * 1024,
            gpu_devices: vec![],
            architecture: "x86_64".to_string(),
            os: "linux".to_string(),
        },
        capabilities: vec!["compute".to_string()],
        socket_path: PathBuf::from("/tmp/test.sock"),
        metadata: HashMap::new(),
    };
    let formatted = format!("{caps:?}");
    assert!(formatted.contains("debug-test"));
    assert!(formatted.contains(primals::TOADSTOOL));
    assert!(formatted.contains("PrimalCapabilities"));
}

#[test]
fn test_system_resources_debug_format() {
    let res = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 1024,
        available_memory_bytes: 512,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let formatted = format!("{res:?}");
    assert!(formatted.contains("SystemResources"));
    assert!(formatted.contains('4'));
}

#[test]
fn test_gpu_device_debug_format() {
    let gpu = GpuDevice {
        device_id: 0,
        name: "Test GPU".to_string(),
        vendor: "amd".to_string(),
        memory_bytes: 1024,
        compute_capability: None,
        render_node: None,
        driver: Some("amdgpu".to_string()),
        arch: Some("rdna2".to_string()),
    };
    let formatted = format!("{gpu:?}");
    assert!(formatted.contains("GpuDevice"));
    assert!(formatted.contains("Test GPU"));
}

#[test]
fn test_system_resources_clone() {
    let res = SystemResources {
        cpu_cores: 2,
        total_memory_bytes: 8 * 1024 * 1024 * 1024,
        available_memory_bytes: 4 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "aarch64".to_string(),
        os: "macos".to_string(),
    };
    let cloned = res.clone();
    assert_eq!(cloned.cpu_cores, res.cpu_cores);
    assert_eq!(cloned.architecture, res.architecture);
}

// --- Announce, find_peer, find_all_peers, cleanup ---

#[tokio::test]
async fn test_announce_preserves_metadata() {
    with_temp_discovery(|discovery_base| async move {
        let mut metadata = HashMap::new();
        metadata.insert("region".to_string(), "eu-west".to_string());
        metadata.insert("tier".to_string(), "premium".to_string());

        let caps = PrimalCapabilities {
            primal_id: "metadata-test".to_string(),
            primal_type: primals::TOADSTOOL.to_string(),
            version: "0.1.0".to_string(),
            resources: SystemResources {
                cpu_cores: 4,
                total_memory_bytes: 16 * 1024 * 1024 * 1024,
                available_memory_bytes: 8 * 1024 * 1024 * 1024,
                gpu_devices: vec![],
                architecture: "x86_64".to_string(),
                os: "linux".to_string(),
            },
            capabilities: vec!["compute".to_string()],
            socket_path: PathBuf::from("/tmp/test.sock"),
            metadata,
        };

        caps.announce().await.unwrap();
        let contents = tokio::fs::read_to_string(discovery_base.join("metadata-test.json"))
            .await
            .unwrap();
        let parsed: PrimalCapabilities = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.metadata.get("region"), Some(&"eu-west".to_string()));
        assert_eq!(parsed.metadata.get("tier"), Some(&"premium".to_string()));
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_skips_non_json() {
    with_temp_discovery(|discovery_base| async move {
        tokio::fs::write(discovery_base.join("notes.txt"), "not json")
            .await
            .unwrap();
        tokio::fs::write(discovery_base.join("config.yaml"), "key: value")
            .await
            .unwrap();

        let result = PrimalCapabilities::find_peer_with("compute").await;
        assert!(result.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_multiple_peers_returns_first_match() {
    with_temp_discovery(|discovery_base| async move {
        let peer1 = PrimalCapabilities {
            primal_id: "first-match".to_string(),
            primal_type: primals::TOADSTOOL.to_string(),
            version: "0.1.0".to_string(),
            resources: SystemResources {
                cpu_cores: 2,
                total_memory_bytes: 8 * 1024 * 1024 * 1024,
                available_memory_bytes: 4 * 1024 * 1024 * 1024,
                gpu_devices: vec![],
                architecture: "x86_64".to_string(),
                os: "linux".to_string(),
            },
            capabilities: vec!["compute".to_string(), "gpu-nvidia".to_string()],
            socket_path: PathBuf::from("/tmp/p1.sock"),
            metadata: HashMap::new(),
        };
        let peer2 = PrimalCapabilities {
            primal_id: "second-match".to_string(),
            primal_type: primals::TOADSTOOL.to_string(),
            version: "0.1.0".to_string(),
            resources: SystemResources {
                cpu_cores: 4,
                total_memory_bytes: 16 * 1024 * 1024 * 1024,
                available_memory_bytes: 8 * 1024 * 1024 * 1024,
                gpu_devices: vec![],
                architecture: "x86_64".to_string(),
                os: "linux".to_string(),
            },
            capabilities: vec!["compute".to_string(), "gpu-nvidia".to_string()],
            socket_path: PathBuf::from("/tmp/p2.sock"),
            metadata: HashMap::new(),
        };

        tokio::fs::write(
            discovery_base.join("first-match.json"),
            serde_json::to_string_pretty(&peer1).unwrap(),
        )
        .await
        .unwrap();
        tokio::fs::write(
            discovery_base.join("second-match.json"),
            serde_json::to_string_pretty(&peer2).unwrap(),
        )
        .await
        .unwrap();

        let found = PrimalCapabilities::find_peer_with("gpu-nvidia")
            .await
            .unwrap();
        assert!(
            found.primal_id == "first-match" || found.primal_id == "second-match",
            "Should find one of the matching peers"
        );
        assert!(found.capabilities.iter().any(|c| c.contains("gpu-nvidia")));
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_invalid_json_fails() {
    with_temp_discovery(|discovery_base| async move {
        tokio::fs::write(discovery_base.join("bad-peer.json"), "{ invalid json }")
            .await
            .unwrap();

        let result = PrimalCapabilities::find_peer_with("compute").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Failed to parse") || err.contains("parse"));
    })
    .await;
}

#[tokio::test]
async fn test_find_all_peers_mixed_valid_invalid_json() {
    with_temp_discovery(|discovery_base| async move {
        let valid_peer = PrimalCapabilities {
            primal_id: "valid-peer".to_string(),
            primal_type: primals::TOADSTOOL.to_string(),
            version: "0.1.0".to_string(),
            resources: SystemResources {
                cpu_cores: 2,
                total_memory_bytes: 8 * 1024 * 1024 * 1024,
                available_memory_bytes: 4 * 1024 * 1024 * 1024,
                gpu_devices: vec![],
                architecture: "x86_64".to_string(),
                os: "linux".to_string(),
            },
            capabilities: vec!["compute".to_string()],
            socket_path: PathBuf::from("/tmp/p.sock"),
            metadata: HashMap::new(),
        };

        tokio::fs::write(
            discovery_base.join("valid-peer.json"),
            serde_json::to_string_pretty(&valid_peer).unwrap(),
        )
        .await
        .unwrap();
        tokio::fs::write(discovery_base.join("invalid.json"), "{ broken }")
            .await
            .unwrap();

        let peers = PrimalCapabilities::find_all_peers().await.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].primal_id, "valid-peer");
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_empty_capability_match() {
    with_temp_discovery(|discovery_base| async move {
        let peer = PrimalCapabilities {
            primal_id: "empty-cap-peer".to_string(),
            primal_type: primals::TOADSTOOL.to_string(),
            version: "0.1.0".to_string(),
            resources: SystemResources {
                cpu_cores: 2,
                total_memory_bytes: 8 * 1024 * 1024 * 1024,
                available_memory_bytes: 4 * 1024 * 1024 * 1024,
                gpu_devices: vec![],
                architecture: "x86_64".to_string(),
                os: "linux".to_string(),
            },
            capabilities: vec!["compute".to_string(), "arch-x86_64".to_string()],
            socket_path: PathBuf::from("/tmp/p.sock"),
            metadata: HashMap::new(),
        };

        tokio::fs::write(
            discovery_base.join("empty-cap-peer.json"),
            serde_json::to_string_pretty(&peer).unwrap(),
        )
        .await
        .unwrap();

        let found = PrimalCapabilities::find_peer_with("arch").await;
        assert!(found.is_ok());
        assert!(
            found
                .unwrap()
                .capabilities
                .iter()
                .any(|c| c.contains("arch"))
        );
    })
    .await;
}

#[tokio::test]
async fn test_cleanup_twice_idempotent() {
    with_temp_discovery(|discovery_base| async move {
        let caps = PrimalCapabilities {
            primal_id: "double-cleanup-id".to_string(),
            primal_type: primals::TOADSTOOL.to_string(),
            version: "0.1.0".to_string(),
            resources: SystemResources {
                cpu_cores: 2,
                total_memory_bytes: 8 * 1024 * 1024 * 1024,
                available_memory_bytes: 4 * 1024 * 1024 * 1024,
                gpu_devices: vec![],
                architecture: "x86_64".to_string(),
                os: "linux".to_string(),
            },
            capabilities: vec!["compute".to_string()],
            socket_path: PathBuf::from("/tmp/test.sock"),
            metadata: HashMap::new(),
        };

        caps.announce().await.unwrap();
        assert!(discovery_base.join("double-cleanup-id.json").exists());

        let r1 = caps.cleanup().await;
        let r2 = caps.cleanup().await;
        assert!(r1.is_ok());
        assert!(r2.is_ok());
        assert!(!discovery_base.join("double-cleanup-id.json").exists());
    })
    .await;
}
