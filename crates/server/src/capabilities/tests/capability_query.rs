// SPDX-License-Identifier: AGPL-3.0-only
//! Capability querying, matching, and negotiation tests.
use super::*;

#[test]
fn test_build_capabilities_base_caps() {
    let resources = SystemResources {
        cpu_cores: 2,
        total_memory_bytes: 8 * 1024 * 1024 * 1024,
        available_memory_bytes: 4 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&CAP_COMPUTE.to_string()));
    assert!(caps.contains(&CAP_ORCHESTRATION.to_string()));
    assert!(caps.contains(&CAP_JSON_RPC.to_string()));
    assert!(caps.contains(&"arch-x86_64".to_string()));
    assert!(caps.contains(&"os-linux".to_string()));
    assert!(caps.contains(&"cpu-cores-2".to_string()));
}

#[test]
fn test_build_capabilities_memory_small() {
    let resources = SystemResources {
        cpu_cores: 1,
        total_memory_bytes: 4 * 1024 * 1024 * 1024,
        available_memory_bytes: 2 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&CAP_MEMORY_SMALL.to_string()));
}

#[test]
fn test_build_capabilities_multiple_gpus() {
    let resources = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        gpu_devices: vec![
            GpuDevice {
                device_id: 0,
                name: "GPU A".to_string(),
                vendor: "nvidia".to_string(),
                memory_bytes: 8 * 1024 * 1024 * 1024,
                compute_capability: None,
                render_node: None,
                driver: None,
                arch: None,
            },
            GpuDevice {
                device_id: 1,
                name: "GPU B".to_string(),
                vendor: "nvidia".to_string(),
                memory_bytes: 8 * 1024 * 1024 * 1024,
                compute_capability: None,
                render_node: None,
                driver: None,
                arch: None,
            },
        ],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&"gpu-0".to_string()));
    assert!(caps.contains(&"gpu-1".to_string()));
    assert!(caps.contains(&"gpu-nvidia".to_string()));
}

#[test]
fn test_capability_constants() {
    assert_eq!(CAP_COMPUTE, "compute");
    assert_eq!(CAP_ORCHESTRATION, "orchestration");
    assert_eq!(CAP_JSON_RPC, "jsonrpc");
    assert_eq!(CAP_MEMORY_LARGE, "memory-large");
    assert_eq!(CAP_MEMORY_MEDIUM, "memory-medium");
    assert_eq!(CAP_MEMORY_SMALL, "memory-small");
}

#[tokio::test]
async fn test_discover_self_returns_valid() {
    let caps = PrimalCapabilities::discover_self("toadstool").await;
    assert!(!caps.primal_id.is_empty());
    assert_eq!(caps.primal_type, "toadstool");
    assert!(caps.resources.cpu_cores >= 1);
    assert!(caps.resources.total_memory_bytes > 0);
    assert!(!caps.capabilities.is_empty());
    assert_eq!(
        caps.socket_path.extension().and_then(|e| e.to_str()),
        Some("sock"),
        "socket path should have .sock extension"
    );
}

#[tokio::test]
async fn test_find_peer_with_in_empty_dir() {
    with_temp_discovery(|discovery_base| async move {
        let result = PrimalCapabilities::find_peer_with_in("compute", &discovery_base).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No peer found"));
    })
    .await;
}

#[tokio::test]
async fn test_find_all_peers_in_empty_dir() {
    with_temp_discovery(|discovery_base| async move {
        let peers = PrimalCapabilities::find_all_peers_in(&discovery_base)
            .await
            .unwrap();
        assert_eq!(peers.len(), 0);
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_capability_contains_match() {
    with_temp_discovery(|discovery_base| async move {
        let peer = PrimalCapabilities {
            primal_id: "contain-match".to_string(),
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
            capabilities: vec!["gpu-nvidia-rtx4090".to_string()],
            socket_path: PathBuf::from("/tmp/p.sock"),
            metadata: HashMap::new(),
        };

        tokio::fs::write(
            discovery_base.join("contain-match.json"),
            serde_json::to_string_pretty(&peer).unwrap(),
        )
        .await
        .unwrap();

        let found = PrimalCapabilities::find_peer_with_in("gpu-nvidia", &discovery_base)
            .await
            .unwrap();
        assert_eq!(found.primal_id, "contain-match");
    })
    .await;
}

#[tokio::test]
async fn test_cleanup_when_file_not_exists() {
    with_temp_discovery(|_discovery_base| async move {
        let caps = PrimalCapabilities {
            primal_id: "never-announced".to_string(),
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
            socket_path: PathBuf::from("/tmp/nonexistent.sock"),
            metadata: HashMap::new(),
        };
        let result = caps.cleanup().await;
        assert!(result.is_ok());
    })
    .await;
}

#[test]
fn test_query_system_resources_returns_valid() {
    let resources = query_system_resources();
    assert!(resources.cpu_cores >= 1);
    assert!(resources.total_memory_bytes > 0);
    assert_eq!(resources.architecture, std::env::consts::ARCH);
    assert_eq!(resources.os, std::env::consts::OS);
}

#[tokio::test]
async fn test_find_all_peers_in_returns_multiple() {
    with_temp_discovery(|discovery_base| async move {
        for i in 0..3 {
            let peer = PrimalCapabilities {
                primal_id: format!("multi-{i}"),
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
                discovery_base.join(format!("multi-{i}.json")),
                serde_json::to_string_pretty(&peer).unwrap(),
            )
            .await
            .unwrap();
        }

        let peers = PrimalCapabilities::find_all_peers_in(&discovery_base)
            .await
            .unwrap();
        assert_eq!(peers.len(), 3);
    })
    .await;
}
