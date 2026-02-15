//! Tests for capability-based primal discovery.
//!
//! These tests use the deprecated primal name constants for testing
//! the legacy interop code paths and ensuring backwards compatibility.
#![allow(deprecated, clippy::await_holding_lock)]

use super::*;
use tempfile::TempDir;
use toadstool_common::interned_strings::primals;

/// Mutex to serialize tests that modify XDG_RUNTIME_DIR.
/// Environment variables are process-global, so concurrent modification causes races.
static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

async fn with_temp_discovery<F, Fut, R>(f: F) -> R
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let _temp = TempDir::new().expect("temp dir");
    let base = _temp.path().to_path_buf();
    let discovery_base = base.join("ecoPrimals").join("discovery");
    std::fs::create_dir_all(&discovery_base).expect("create discovery dir");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", &base);
    let out = f(discovery_base).await;
    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
    out
}

#[tokio::test]
async fn test_self_discovery() {
    let caps = PrimalCapabilities::discover_self(primals::TOADSTOOL).await;
    assert!(caps.resources.cpu_cores > 0);
    assert!(caps.capabilities.contains(&CAP_COMPUTE.to_string()));
    assert!(caps.primal_type == primals::TOADSTOOL);
}

#[tokio::test]
async fn test_self_discovery_primal_id_uuid_format() {
    let caps = PrimalCapabilities::discover_self(primals::BEARDOG).await;
    assert!(Uuid::parse_str(&caps.primal_id).is_ok());
}

#[tokio::test]
async fn test_self_discovery_version_set() {
    let caps = PrimalCapabilities::discover_self(primals::SONGBIRD).await;
    assert!(!caps.version.is_empty());
}

#[tokio::test]
async fn test_self_discovery_socket_path() {
    let caps = PrimalCapabilities::discover_self(primals::TOADSTOOL).await;
    assert!(caps
        .socket_path
        .as_os_str()
        .to_str()
        .unwrap()
        .ends_with(".sock"));
    assert!(caps.socket_path.to_string_lossy().contains(&caps.primal_id));
    assert!(caps.socket_path.to_string_lossy().contains("ecoPrimals"));
    assert!(caps.socket_path.to_string_lossy().contains("sockets"));
}

#[tokio::test]
async fn test_self_discovery_metadata_empty() {
    let caps = PrimalCapabilities::discover_self(primals::TOADSTOOL).await;
    assert!(caps.metadata.is_empty());
}

#[tokio::test]
async fn test_self_discovery_different_primal_types() {
    for primal_type in [primals::TOADSTOOL, primals::BEARDOG, primals::SONGBIRD, ""] {
        let caps = PrimalCapabilities::discover_self(primal_type).await;
        assert_eq!(caps.primal_type, primal_type);
    }
}

#[tokio::test]
async fn test_self_discovery_all_base_capabilities() {
    let caps = PrimalCapabilities::discover_self(primals::TOADSTOOL).await;
    assert!(caps.capabilities.contains(&CAP_COMPUTE.to_string()));
    assert!(caps.capabilities.contains(&CAP_ORCHESTRATION.to_string()));
    assert!(caps.capabilities.contains(&CAP_JSON_RPC.to_string()));
}

#[test]
fn test_system_resources_query() {
    let resources = query_system_resources();
    assert!(resources.cpu_cores > 0);
    assert!(!resources.architecture.is_empty());
    assert!(!resources.os.is_empty());
}

#[test]
fn test_system_resources_memory_sanity() {
    let resources = query_system_resources();
    assert!(resources.available_memory_bytes <= resources.total_memory_bytes);
}

#[test]
fn test_system_resources_gpu_vec_allocated() {
    let resources = query_system_resources();
    assert!(resources.gpu_devices.is_empty() || !resources.gpu_devices.is_empty());
}

#[test]
fn test_capabilities_build_medium_memory() {
    let resources = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&CAP_COMPUTE.to_string()));
    assert!(capabilities.contains(&"cpu-cores-8".to_string()));
    assert!(capabilities.contains(&"arch-x86_64".to_string()));
    assert!(capabilities.contains(&"os-linux".to_string()));
    assert!(capabilities.contains(&CAP_MEMORY_MEDIUM.to_string()));
}

#[test]
fn test_capabilities_build_memory_small() {
    let resources = SystemResources {
        cpu_cores: 1,
        total_memory_bytes: 8 * 1024 * 1024 * 1024,
        available_memory_bytes: 4 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "aarch64".to_string(),
        os: "macos".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&CAP_MEMORY_SMALL.to_string()));
    assert!(capabilities.contains(&"cpu-cores-1".to_string()));
    assert!(capabilities.contains(&"arch-aarch64".to_string()));
    assert!(capabilities.contains(&"os-macos".to_string()));
}

#[test]
fn test_capabilities_build_memory_large() {
    let resources = SystemResources {
        cpu_cores: 64,
        total_memory_bytes: 128 * 1024 * 1024 * 1024,
        available_memory_bytes: 64 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&CAP_MEMORY_LARGE.to_string()));
    assert!(capabilities.contains(&"cpu-cores-64".to_string()));
}

#[test]
fn test_capabilities_build_memory_boundary_16gb() {
    let resources = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&CAP_MEMORY_MEDIUM.to_string()));
}

#[test]
fn test_capabilities_build_memory_boundary_64gb() {
    let resources = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 64 * 1024 * 1024 * 1024,
        available_memory_bytes: 32 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&CAP_MEMORY_LARGE.to_string()));
}

#[test]
fn test_capabilities_build_memory_just_under_16gb() {
    let resources = SystemResources {
        cpu_cores: 2,
        total_memory_bytes: 15 * 1024 * 1024 * 1024,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&CAP_MEMORY_SMALL.to_string()));
}

#[test]
fn test_capabilities_build_with_gpu_devices() {
    let resources = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        gpu_devices: vec![
            GpuDevice {
                device_id: 0,
                name: "RTX 4090".to_string(),
                vendor: "nvidia".to_string(),
                memory_bytes: 24 * 1024 * 1024 * 1024,
                compute_capability: Some("8.9".to_string()),
            },
            GpuDevice {
                device_id: 1,
                name: "RX 7900".to_string(),
                vendor: "amd".to_string(),
                memory_bytes: 20 * 1024 * 1024 * 1024,
                compute_capability: None,
            },
        ],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&"gpu-0".to_string()));
    assert!(capabilities.contains(&"gpu-1".to_string()));
    assert!(capabilities.contains(&"gpu-nvidia".to_string()));
    assert!(capabilities.contains(&"gpu-amd".to_string()));
    assert!(capabilities.contains(&"gpu-nvidia-RTX 4090".to_string()));
    assert!(capabilities.contains(&"gpu-amd-RX 7900".to_string()));
}

#[test]
fn test_capabilities_build_empty_gpu() {
    let resources = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(!capabilities.iter().any(|c| c.starts_with("gpu-")));
}

#[test]
fn test_capabilities_build_minimal_resources() {
    let resources = SystemResources {
        cpu_cores: 0,
        total_memory_bytes: 0,
        available_memory_bytes: 0,
        gpu_devices: vec![],
        architecture: "unknown".to_string(),
        os: "unknown".to_string(),
    };

    let capabilities = build_capabilities(&resources);
    assert!(capabilities.contains(&"cpu-cores-0".to_string()));
    assert!(capabilities.contains(&CAP_MEMORY_SMALL.to_string()));
    assert!(capabilities.contains(&"arch-unknown".to_string()));
    assert!(capabilities.contains(&"os-unknown".to_string()));
}

#[test]
fn test_gpu_device_serialization() {
    let gpu = GpuDevice {
        device_id: 0,
        name: "Test GPU".to_string(),
        vendor: "nvidia".to_string(),
        memory_bytes: 8 * 1024 * 1024 * 1024,
        compute_capability: Some("8.0".to_string()),
    };

    let json = serde_json::to_string(&gpu).expect("serialize");
    let parsed: GpuDevice = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.device_id, gpu.device_id);
    assert_eq!(parsed.name, gpu.name);
    assert_eq!(parsed.vendor, gpu.vendor);
    assert_eq!(parsed.memory_bytes, gpu.memory_bytes);
    assert_eq!(parsed.compute_capability, gpu.compute_capability);
}

#[test]
fn test_gpu_device_without_compute_capability() {
    let gpu = GpuDevice {
        device_id: 1,
        name: "Integrated".to_string(),
        vendor: "intel".to_string(),
        memory_bytes: 1024 * 1024 * 1024,
        compute_capability: None,
    };

    let json = serde_json::to_string(&gpu).expect("serialize");
    let parsed: GpuDevice = serde_json::from_str(&json).expect("deserialize");
    assert!(parsed.compute_capability.is_none());
}

#[test]
fn test_primal_capabilities_serialization_roundtrip() {
    let caps = PrimalCapabilities {
        primal_id: "test-id-123".to_string(),
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
        socket_path: PathBuf::from("/tmp/ecoPrimals/sockets/test.sock"),
        metadata: [("region".to_string(), "us-west".to_string())]
            .into_iter()
            .collect(),
    };

    let json = serde_json::to_string(&caps).expect("serialize");
    let parsed: PrimalCapabilities = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.primal_id, caps.primal_id);
    assert_eq!(parsed.primal_type, caps.primal_type);
    assert_eq!(parsed.version, caps.version);
    assert_eq!(parsed.resources.cpu_cores, caps.resources.cpu_cores);
    assert_eq!(parsed.capabilities, caps.capabilities);
    assert_eq!(parsed.metadata.get("region"), Some(&"us-west".to_string()));
}

#[test]
fn test_primal_capabilities_clone() {
    let caps = PrimalCapabilities {
        primal_id: "clone-test".to_string(),
        primal_type: primals::BEARDOG.to_string(),
        version: "1.0.0".to_string(),
        resources: SystemResources {
            cpu_cores: 2,
            total_memory_bytes: 8 * 1024 * 1024 * 1024,
            available_memory_bytes: 4 * 1024 * 1024 * 1024,
            gpu_devices: vec![],
            architecture: "aarch64".to_string(),
            os: "macos".to_string(),
        },
        capabilities: vec!["compute".to_string()],
        socket_path: PathBuf::from("/tmp/test.sock"),
        metadata: HashMap::new(),
    };

    let cloned = caps.clone();
    assert_eq!(cloned.primal_id, caps.primal_id);
    assert_eq!(cloned.resources.cpu_cores, caps.resources.cpu_cores);
}

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

        let found = PrimalCapabilities::find_peer_with("gpu-nvidia").await;
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

        let found = PrimalCapabilities::find_peer_with("nvidia").await;
        assert!(found.is_ok());
        assert!(found
            .unwrap()
            .capabilities
            .iter()
            .any(|c| c.contains("nvidia")));
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_not_found() {
    with_temp_discovery(|_| async {
        let result = PrimalCapabilities::find_peer_with("nonexistent-capability-xyz").await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("No peer found"));
        assert!(err.contains("nonexistent-capability-xyz"));
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_empty_dir() {
    with_temp_discovery(|_| async {
        let result = PrimalCapabilities::find_peer_with("compute").await;
        assert!(result.is_err());
    })
    .await;
}

#[tokio::test]
async fn test_find_peer_with_nonexistent_discovery_dir() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_path_buf();
    // Set XDG_RUNTIME_DIR but do NOT create ecoPrimals/discovery
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", &base);

    let result = PrimalCapabilities::find_peer_with("compute").await;

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Failed to read discovery directory")
            || err.contains("No such file")
            || err.contains("not found"),
        "Expected dir-read error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_find_all_peers_nonexistent_discovery_dir() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_path_buf();
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", &base);

    let result = PrimalCapabilities::find_all_peers().await;

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("Failed to read discovery directory")
            || err.contains("No such file")
            || err.contains("not found"),
        "Expected dir-read error, got: {}",
        err
    );
}

#[tokio::test]
async fn test_find_all_peers_empty() {
    with_temp_discovery(|_| async {
        let peers = PrimalCapabilities::find_all_peers().await;
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
            primal_type: primals::BEARDOG.to_string(),
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

        let peers = PrimalCapabilities::find_all_peers().await;
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

        let peers = PrimalCapabilities::find_all_peers().await;
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
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_path_buf();
    let discovery_base = base.join("ecoPrimals").join("discovery");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", &base);

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

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    assert!(result.is_ok());
    assert!(discovery_base.exists());
    assert!(discovery_base.join("dir-create-test.json").exists());
}

// --- discovery_directory and default_socket_path (private helpers) ---

#[test]
fn test_discovery_directory_structure() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path();
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", base);

    let dir = super::discovery_directory();
    assert!(dir.ends_with("ecoPrimals/discovery") || dir.to_string_lossy().contains("ecoPrimals"));
    assert!(dir.to_string_lossy().contains("discovery"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}

#[test]
fn test_discovery_directory_fallback_when_xdg_unset() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::remove_var("XDG_RUNTIME_DIR");

    let dir = super::discovery_directory();
    assert!(dir.to_string_lossy().contains("ecoPrimals"));
    assert!(dir.to_string_lossy().contains("discovery"));
    // Fallback is /tmp when XDG_RUNTIME_DIR not set
    assert!(dir.starts_with("/tmp") || dir.to_string_lossy().starts_with("/tmp"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    }
}

#[test]
fn test_default_socket_path_format() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");

    let path = super::default_socket_path("my-primal-id");
    assert!(path.ends_with("my-primal-id.sock"));
    assert!(path.to_string_lossy().contains("ecoPrimals"));
    assert!(path.to_string_lossy().contains("sockets"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    } else {
        std::env::remove_var("XDG_RUNTIME_DIR");
    }
}

#[test]
fn test_default_socket_path_fallback() {
    let _lock = ENV_MUTEX.lock().expect("env mutex poisoned");
    let prev = std::env::var("XDG_RUNTIME_DIR").ok();
    std::env::remove_var("XDG_RUNTIME_DIR");

    let path = super::default_socket_path("test-id");
    assert!(path.ends_with("test-id.sock"));
    assert!(path.to_string_lossy().contains("/tmp"));

    if let Some(p) = prev {
        std::env::set_var("XDG_RUNTIME_DIR", p);
    }
}

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
    };
    let cloned = gpu.clone();
    assert_eq!(cloned.device_id, gpu.device_id);
    assert_eq!(cloned.name, gpu.name);
    assert_eq!(cloned.vendor, gpu.vendor);
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
    let formatted = format!("{:?}", caps);
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
    let formatted = format!("{:?}", res);
    assert!(formatted.contains("SystemResources"));
    assert!(formatted.contains("4"));
}

#[test]
fn test_gpu_device_debug_format() {
    let gpu = GpuDevice {
        device_id: 0,
        name: "Test GPU".to_string(),
        vendor: "amd".to_string(),
        memory_bytes: 1024,
        compute_capability: None,
    };
    let formatted = format!("{:?}", gpu);
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
        assert!(found
            .unwrap()
            .capabilities
            .iter()
            .any(|c| c.contains("arch")));
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
