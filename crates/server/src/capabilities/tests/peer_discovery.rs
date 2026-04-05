// SPDX-License-Identifier: AGPL-3.0-or-later
//! Announce, find_peer, find_all_peers, cleanup tests.
use super::*;

#[tokio::test]
async fn test_announce_success() {
    with_temp_discovery(|discovery_base| async move {
        let caps = PrimalCapabilities {
            primal_id: "announce-test-id".to_string(),
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
            socket_path: PathBuf::from("/tmp/test.sock"),
            metadata: HashMap::new(),
        };

        let result = caps.announce().await;
        assert!(result.is_ok());

        let file_path = discovery_base.join("announce-test-id.json");
        assert!(file_path.exists());

        let contents = tokio::fs::read_to_string(&file_path).await.unwrap();
        let parsed: PrimalCapabilities = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed.primal_id, "announce-test-id");
        assert!(parsed.capabilities.contains(&"gpu-nvidia".to_string()));
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_success() {
    with_temp_discovery(|discovery_base| async move {
        let peer = PrimalCapabilities {
            primal_id: "peer-gpu-123".to_string(),
            primal_type: primals::TOADSTOOL.to_string(),
            version: "0.1.0".to_string(),
            resources: SystemResources {
                cpu_cores: 4,
                total_memory_bytes: 16 * 1024 * 1024 * 1024,
                available_memory_bytes: 8 * 1024 * 1024 * 1024,
                gpu_devices: vec![GpuDevice {
                    device_id: 0,
                    name: "RTX 4090".to_string(),
                    vendor: "nvidia".to_string(),
                    memory_bytes: 24 * 1024 * 1024 * 1024,
                    compute_capability: None,
                    render_node: None,
                    driver: None,
                    arch: None,
                }],
                architecture: "x86_64".to_string(),
                os: "linux".to_string(),
            },
            capabilities: vec!["compute".to_string(), "gpu-nvidia".to_string()],
            socket_path: PathBuf::from("/tmp/peer.sock"),
            metadata: HashMap::new(),
        };

        let path = discovery_base.join("peer-gpu-123.json");
        tokio::fs::write(&path, serde_json::to_string_pretty(&peer).unwrap())
            .await
            .unwrap();

        let found = PrimalCapabilities::find_peer_with_in("gpu-nvidia", &discovery_base).await;
        assert!(found.is_ok());
        let found = found.unwrap();
        assert_eq!(found.primal_id, "peer-gpu-123");
        assert!(found.capabilities.iter().any(|c| c.contains("gpu-nvidia")));
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_partial_match() {
    with_temp_discovery(|discovery_base| async move {
        let peer = PrimalCapabilities {
            primal_id: "peer-partial".to_string(),
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
            capabilities: vec!["compute".to_string(), "gpu-nvidia-rtx".to_string()],
            socket_path: PathBuf::from("/tmp/peer.sock"),
            metadata: HashMap::new(),
        };

        let path = discovery_base.join("peer-partial.json");
        tokio::fs::write(&path, serde_json::to_string_pretty(&peer).unwrap())
            .await
            .unwrap();

        let found = PrimalCapabilities::find_peer_with_in("nvidia", &discovery_base).await;
        assert!(found.is_ok());
        assert!(
            found
                .unwrap()
                .capabilities
                .iter()
                .any(|c| c.contains("nvidia"))
        );
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_not_found() {
    with_temp_discovery(|discovery_base| async move {
        let result =
            PrimalCapabilities::find_peer_with_in("nonexistent-capability-xyz", &discovery_base)
                .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("No peer found"));
        assert!(err.contains("nonexistent-capability-xyz"));
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_empty_dir() {
    with_temp_discovery(|discovery_base| async move {
        let result = PrimalCapabilities::find_peer_with_in("compute", &discovery_base).await;
        assert!(result.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_nonexistent_discovery_dir() {
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_path_buf();
    let base_str = base.to_string_lossy().to_string();
    temp_env::async_with_vars([("XDG_RUNTIME_DIR", Some(base_str.as_str()))], async {
        let _keep = temp;
        let result = PrimalCapabilities::find_peer_with("compute").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to read discovery directory")
                || err.contains("No such file")
                || err.contains("not found"),
            "Expected dir-read error, got: {err}"
        );
    })
    .await;
}

#[tokio::test]
async fn test_find_all_peers_nonexistent_discovery_dir() {
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_path_buf();
    let base_str = base.to_string_lossy().to_string();
    temp_env::async_with_vars([("XDG_RUNTIME_DIR", Some(base_str.as_str()))], async {
        let _keep = temp;
        let result = PrimalCapabilities::find_all_peers().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to read discovery directory")
                || err.contains("No such file")
                || err.contains("not found"),
            "Expected dir-read error, got: {err}"
        );
    })
    .await;
}

#[tokio::test]
async fn test_find_all_peers_empty() {
    with_temp_discovery(|discovery_base| async move {
        let peers = PrimalCapabilities::find_all_peers_in(&discovery_base).await;
        assert!(peers.is_ok());
        assert!(peers.unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_find_all_peers_populated() {
    with_temp_discovery(|discovery_base| async move {
        let peer1 = PrimalCapabilities {
            primal_id: "peer-1".to_string(),
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
            socket_path: PathBuf::from("/tmp/p1.sock"),
            metadata: HashMap::new(),
        };
        let peer2 = PrimalCapabilities {
            primal_id: "peer-2".to_string(),
            primal_type: primals::LEGACY_SECURITY_LABEL.to_string(),
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
            socket_path: PathBuf::from("/tmp/p2.sock"),
            metadata: HashMap::new(),
        };

        tokio::fs::write(
            discovery_base.join("peer-1.json"),
            serde_json::to_string_pretty(&peer1).unwrap(),
        )
        .await
        .unwrap();
        tokio::fs::write(
            discovery_base.join("peer-2.json"),
            serde_json::to_string_pretty(&peer2).unwrap(),
        )
        .await
        .unwrap();

        let peers = PrimalCapabilities::find_all_peers_in(&discovery_base).await;
        assert!(peers.is_ok());
        let peers = peers.unwrap();
        assert_eq!(peers.len(), 2);
        let ids: Vec<_> = peers.iter().map(|p| p.primal_id.as_str()).collect();
        assert!(ids.contains(&"peer-1"));
        assert!(ids.contains(&"peer-2"));
    })
    .await;
}

#[tokio::test]
async fn test_find_all_peers_skips_non_json() {
    with_temp_discovery(|discovery_base| async move {
        tokio::fs::write(discovery_base.join("data.txt"), "not json")
            .await
            .unwrap();
        tokio::fs::write(discovery_base.join("config.toml"), "x = 1")
            .await
            .unwrap();

        let peers = PrimalCapabilities::find_all_peers_in(&discovery_base).await;
        assert!(peers.is_ok());
        assert!(peers.unwrap().is_empty());
    })
    .await;
}

#[tokio::test]
async fn test_cleanup_removes_file() {
    with_temp_discovery(|discovery_base| async move {
        let caps = PrimalCapabilities {
            primal_id: "cleanup-test-id".to_string(),
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
        assert!(discovery_base.join("cleanup-test-id.json").exists());

        let result = caps.cleanup().await;
        assert!(result.is_ok());
        assert!(!discovery_base.join("cleanup-test-id.json").exists());
    })
    .await;
}

#[tokio::test]
async fn test_cleanup_idempotent_no_file() {
    with_temp_discovery(|_| async {
        let caps = PrimalCapabilities {
            primal_id: "never-announced-id".to_string(),
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

        let result = caps.cleanup().await;
        assert!(result.is_ok());
    })
    .await;
}

#[tokio::test]
async fn test_announce_creates_discovery_dir() {
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_path_buf();
    let discovery_base = base.join("ecoPrimals").join("discovery");
    let base_str = base.to_string_lossy().to_string();
    temp_env::async_with_vars([("XDG_RUNTIME_DIR", Some(base_str.as_str()))], async move {
        let _keep = temp;
        let caps = PrimalCapabilities {
            primal_id: "dir-create-test".to_string(),
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
        let result = caps.announce().await;
        assert!(result.is_ok());
        assert!(discovery_base.exists());
        assert!(discovery_base.join("dir-create-test.json").exists());
    })
    .await;
}

// --- discovery_directory and default_socket_path (private helpers) ---
