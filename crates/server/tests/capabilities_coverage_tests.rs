// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::future_not_send,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Integration coverage for [`toadstool_server::capabilities`]: peer discovery, capability
//! derivation, serialization, and filesystem-backed announcement paths. Public capability structs
//! implement `Clone`, `Debug`, and `serde` traits; they do not provide `std::default::Default`.

use std::collections::HashMap;
use std::path::PathBuf;

use tempfile::TempDir;
use toadstool_server::capabilities::*;

async fn with_temp_discovery<F, Fut, R>(f: F) -> R
where
    F: FnOnce(PathBuf) -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let temp = TempDir::new().expect("temp dir");
    let base = temp.path().to_path_buf();
    let discovery_base = base.join("ecoPrimals").join("discovery");
    std::fs::create_dir_all(&discovery_base).expect("create discovery dir");
    let base_str = base.to_string_lossy().to_string();
    temp_env::async_with_vars([("XDG_RUNTIME_DIR", Some(base_str.as_str()))], async move {
        let _keep = temp;
        f(discovery_base).await
    })
    .await
}

fn sample_resources() -> SystemResources {
    SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 16 * 1024 * 1024 * 1024,
        available_memory_bytes: 8 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    }
}

fn sample_peer(primal_id: &str, capabilities: Vec<String>) -> PrimalCapabilities {
    PrimalCapabilities {
        primal_id: primal_id.to_string(),
        primal_type: "toadstool".to_string(),
        version: "0.1.0".to_string(),
        resources: sample_resources(),
        capabilities,
        socket_path: PathBuf::from("/tmp/p.sock"),
        metadata: HashMap::new(),
    }
}

#[tokio::test]
async fn discover_self_yields_nonempty_identity_and_capabilities() {
    let caps = PrimalCapabilities::discover_self("integration-test").await;
    assert!(!caps.primal_id.is_empty());
    assert_eq!(caps.primal_type, "integration-test");
    assert!(!caps.version.is_empty());
    assert!(caps.resources.cpu_cores >= 1);
    assert!(caps.socket_path.to_string_lossy().contains(".sock"));
    assert!(!caps.capabilities.is_empty());
    assert!(caps.capabilities.iter().any(|c| c.contains("compute")));
}

#[tokio::test]
async fn query_system_resources_matches_host_constants() {
    let r = query_system_resources();
    assert!(r.cpu_cores >= 1);
    assert_eq!(r.architecture, std::env::consts::ARCH);
    assert_eq!(r.os, std::env::consts::OS);
}

#[tokio::test]
async fn build_capabilities_includes_base_and_arch_os_cpu_memory_small_tier() {
    let resources = SystemResources {
        cpu_cores: 2,
        total_memory_bytes: 4 * 1024 * 1024 * 1024,
        available_memory_bytes: 2 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "aarch64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&resources);
    assert!(caps.contains(&"compute".to_string()));
    assert!(caps.contains(&"orchestration".to_string()));
    assert!(caps.contains(&"jsonrpc".to_string()));
    assert!(caps.contains(&"arch-aarch64".to_string()));
    assert!(caps.contains(&"os-linux".to_string()));
    assert!(caps.contains(&"cpu-cores-2".to_string()));
    assert!(caps.contains(&"memory-small".to_string()));
}

#[tokio::test]
async fn build_capabilities_memory_medium_and_large_tiers() {
    let medium = SystemResources {
        cpu_cores: 4,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    assert!(build_capabilities(&medium).contains(&"memory-medium".to_string()));

    let large = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 128 * 1024 * 1024 * 1024,
        available_memory_bytes: 64 * 1024 * 1024 * 1024,
        gpu_devices: vec![],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    assert!(build_capabilities(&large).contains(&"memory-large".to_string()));
}

#[tokio::test]
async fn build_capabilities_adds_gpu_dispatch_when_gpus_present() {
    let gpu = GpuDevice {
        device_id: 0,
        name: "Test GPU".to_string(),
        vendor: "nvidia".to_string(),
        memory_bytes: 8 * 1024 * 1024 * 1024,
        compute_capability: Some("8.0".to_string()),
        render_node: Some("/dev/dri/renderD128".to_string()),
        driver: Some("nvidia".to_string()),
        arch: Some("sm_80".to_string()),
    };
    let with_gpu = SystemResources {
        cpu_cores: 8,
        total_memory_bytes: 32 * 1024 * 1024 * 1024,
        available_memory_bytes: 16 * 1024 * 1024 * 1024,
        gpu_devices: vec![gpu],
        architecture: "x86_64".to_string(),
        os: "linux".to_string(),
    };
    let caps = build_capabilities(&with_gpu);
    assert!(caps.contains(&"gpu.dispatch".to_string()));
    assert!(caps.contains(&"science.gpu.dispatch".to_string()));
    assert!(caps.contains(&"gpu-0".to_string()));
    assert!(caps.contains(&"gpu-nvidia".to_string()));
    assert!(caps.contains(&"gpu-nvidia-Test GPU".to_string()));

    let no_gpu = SystemResources {
        gpu_devices: vec![],
        ..with_gpu
    };
    let caps_no = build_capabilities(&no_gpu);
    assert!(!caps_no.contains(&"gpu.dispatch".to_string()));
}

#[tokio::test]
async fn primal_capabilities_clone_debug_serde_roundtrip() {
    let mut meta = HashMap::new();
    meta.insert("k".to_string(), "v".to_string());
    let caps = PrimalCapabilities {
        primal_id: "id-1".to_string(),
        primal_type: "t".to_string(),
        version: "1.0.0".to_string(),
        resources: sample_resources(),
        capabilities: vec!["compute".to_string()],
        socket_path: PathBuf::from("/x/y.sock"),
        metadata: meta,
    };
    let dbg = format!("{caps:?}");
    assert!(dbg.contains("PrimalCapabilities") && dbg.contains("id-1"));
    let cloned = caps.clone();
    assert_eq!(cloned.primal_id, caps.primal_id);
    let json = serde_json::to_string(&caps).expect("serialize");
    let back: PrimalCapabilities = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.primal_id, caps.primal_id);
    assert_eq!(back.metadata.get("k"), Some(&"v".to_string()));
}

#[tokio::test]
async fn system_resources_clone_debug_serde_roundtrip() {
    let res = sample_resources();
    let dbg = format!("{res:?}");
    assert!(dbg.contains("SystemResources"));
    let cloned = res.clone();
    assert_eq!(cloned.cpu_cores, res.cpu_cores);
    let json = serde_json::to_string(&res).expect("serialize");
    let back: SystemResources = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.cpu_cores, res.cpu_cores);
}

#[tokio::test]
async fn gpu_device_clone_debug_serde_roundtrip_and_optional_omission() {
    let gpu = GpuDevice {
        device_id: 0,
        name: "G".to_string(),
        vendor: "amd".to_string(),
        memory_bytes: 1024,
        compute_capability: None,
        render_node: None,
        driver: None,
        arch: None,
    };
    let json = serde_json::to_string(&gpu).expect("serialize");
    assert!(!json.contains("render_node"));
    let back: GpuDevice = serde_json::from_str(&json).expect("deserialize");
    assert!(back.render_node.is_none());
    let dbg = format!("{gpu:?}");
    assert!(dbg.contains("GpuDevice"));
    let c = gpu.clone();
    assert_eq!(c.vendor, gpu.vendor);
}

#[tokio::test]
async fn gpu_device_deserializes_when_optional_fields_absent() {
    let json = r#"{"device_id":0,"name":"X","vendor":"intel","memory_bytes":1}"#;
    let g: GpuDevice = serde_json::from_str(json).expect("deserialize");
    assert!(g.compute_capability.is_none());
    assert!(g.render_node.is_none());
}

#[tokio::test]
async fn find_peer_with_in_ignores_nonmatching_peer_files() {
    let dir = TempDir::new().expect("temp");
    let peer_a = sample_peer("a", vec!["other".to_string()]);
    let peer_b = sample_peer("b", vec!["gpu-nvidia-rtx".to_string()]);
    tokio::fs::write(
        dir.path().join("a.json"),
        serde_json::to_string_pretty(&peer_a).expect("json"),
    )
    .await
    .expect("write");
    tokio::fs::write(
        dir.path().join("b.json"),
        serde_json::to_string_pretty(&peer_b).expect("json"),
    )
    .await
    .expect("write");
    let found = PrimalCapabilities::find_peer_with_in("nvidia", dir.path())
        .await
        .expect("found");
    assert_eq!(found.primal_id, "b");
}

#[tokio::test]
async fn find_peer_with_in_skips_non_json_then_matches() {
    let dir = TempDir::new().expect("temp");
    tokio::fs::write(dir.path().join("readme.md"), "# x")
        .await
        .expect("write");
    let peer = sample_peer("p1", vec!["compute".to_string()]);
    tokio::fs::write(
        dir.path().join("p1.json"),
        serde_json::to_string_pretty(&peer).expect("json"),
    )
    .await
    .expect("write");
    let found = PrimalCapabilities::find_peer_with_in("compute", dir.path())
        .await
        .expect("found");
    assert_eq!(found.primal_id, "p1");
}

#[tokio::test]
async fn find_peer_with_in_errors_when_no_peer_matches() {
    let dir = TempDir::new().expect("temp");
    let peer = sample_peer("z", vec!["only-this".to_string()]);
    tokio::fs::write(
        dir.path().join("z.json"),
        serde_json::to_string_pretty(&peer).expect("json"),
    )
    .await
    .expect("write");
    let err = PrimalCapabilities::find_peer_with_in("missing-cap", dir.path())
        .await
        .unwrap_err();
    assert!(err.contains("No peer found"));
}

#[tokio::test]
async fn find_peer_with_in_errors_when_capabilities_empty() {
    let dir = TempDir::new().expect("temp");
    let peer = sample_peer("empty-caps", vec![]);
    tokio::fs::write(
        dir.path().join("empty-caps.json"),
        serde_json::to_string_pretty(&peer).expect("json"),
    )
    .await
    .expect("write");
    let err = PrimalCapabilities::find_peer_with_in("compute", dir.path())
        .await
        .unwrap_err();
    assert!(err.contains("No peer found"));
}

#[tokio::test]
async fn find_peer_with_in_errors_on_invalid_json() {
    let dir = TempDir::new().expect("temp");
    tokio::fs::write(dir.path().join("bad.json"), "{ not json }")
        .await
        .expect("write");
    let err = PrimalCapabilities::find_peer_with_in("compute", dir.path())
        .await
        .unwrap_err();
    assert!(err.contains("Failed to parse"));
}

#[tokio::test]
async fn find_peer_with_in_errors_when_discovery_dir_missing() {
    let dir = TempDir::new().expect("temp");
    let missing = dir.path().join("nope");
    let err = PrimalCapabilities::find_peer_with_in("x", &missing)
        .await
        .unwrap_err();
    assert!(err.contains("Failed to read discovery directory"));
}

#[tokio::test]
async fn find_all_peers_in_empty_directory_returns_empty_vec() {
    let dir = TempDir::new().expect("temp");
    let peers = PrimalCapabilities::find_all_peers_in(dir.path())
        .await
        .expect("ok");
    assert!(peers.is_empty());
}

#[tokio::test]
async fn find_all_peers_in_collects_valid_json_and_skips_invalid() {
    let dir = TempDir::new().expect("temp");
    let good = sample_peer("good", vec!["compute".to_string()]);
    tokio::fs::write(
        dir.path().join("good.json"),
        serde_json::to_string_pretty(&good).expect("json"),
    )
    .await
    .expect("write");
    tokio::fs::write(dir.path().join("bad.json"), "{")
        .await
        .expect("write");
    let peers = PrimalCapabilities::find_all_peers_in(dir.path())
        .await
        .expect("ok");
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].primal_id, "good");
}

#[tokio::test]
async fn find_all_peers_in_duplicate_files_same_primal_id_appear_twice() {
    let dir = TempDir::new().expect("temp");
    let peer = sample_peer("dup-id", vec!["compute".to_string()]);
    let json = serde_json::to_string_pretty(&peer).expect("json");
    tokio::fs::write(dir.path().join("one.json"), &json)
        .await
        .expect("write");
    tokio::fs::write(dir.path().join("two.json"), &json)
        .await
        .expect("write");
    let peers = PrimalCapabilities::find_all_peers_in(dir.path())
        .await
        .expect("ok");
    assert_eq!(peers.len(), 2);
}

#[tokio::test]
async fn find_all_peers_in_errors_when_directory_unreadable() {
    let dir = TempDir::new().expect("temp");
    let missing = dir.path().join("missing");
    let err = PrimalCapabilities::find_all_peers_in(&missing)
        .await
        .unwrap_err();
    assert!(err.contains("Failed to read discovery directory"));
}

#[tokio::test]
async fn announce_writes_canonical_and_compat_json_files() {
    with_temp_discovery(|discovery_base| async move {
        let caps = sample_peer("announce-dual", vec!["compute".to_string()]);
        caps.announce().await.expect("announce");
        let eco_root = discovery_base.parent().expect("ecoPrimals root");
        assert!(discovery_base.join("announce-dual.json").exists());
        assert!(eco_root.join("announce-dual.json").exists());
        let a = tokio::fs::read_to_string(discovery_base.join("announce-dual.json"))
            .await
            .expect("read");
        let b = tokio::fs::read_to_string(eco_root.join("announce-dual.json"))
            .await
            .expect("read");
        assert_eq!(a, b);
    })
    .await;
}

#[tokio::test]
async fn cleanup_removes_both_announcement_files() {
    with_temp_discovery(|discovery_base| async move {
        let caps = sample_peer("clean-me", vec!["compute".to_string()]);
        caps.announce().await.expect("announce");
        let eco_root = discovery_base.parent().expect("ecoPrimals root");
        assert!(discovery_base.join("clean-me.json").exists());
        caps.cleanup().await.expect("cleanup");
        assert!(!discovery_base.join("clean-me.json").exists());
        assert!(!eco_root.join("clean-me.json").exists());
    })
    .await;
}

#[tokio::test]
async fn cleanup_succeeds_when_announcement_files_absent() {
    with_temp_discovery(|_| async move {
        let caps = sample_peer("never-announced", vec!["compute".to_string()]);
        caps.cleanup().await.expect("cleanup");
    })
    .await;
}

#[tokio::test]
async fn find_peer_with_uses_global_discovery_directory_under_xdg() {
    with_temp_discovery(|discovery_base| async move {
        let peer = sample_peer("global-find", vec!["science.gpu.dispatch".to_string()]);
        tokio::fs::write(
            discovery_base.join("global-find.json"),
            serde_json::to_string_pretty(&peer).expect("json"),
        )
        .await
        .expect("write");
        let found = PrimalCapabilities::find_peer_with("science.gpu")
            .await
            .expect("peer");
        assert_eq!(found.primal_id, "global-find");
    })
    .await;
}

#[tokio::test]
async fn find_all_peers_global_collects_from_xdg_discovery() {
    with_temp_discovery(|discovery_base| async move {
        let p = sample_peer("all-global", vec!["compute".to_string()]);
        tokio::fs::write(
            discovery_base.join("all-global.json"),
            serde_json::to_string_pretty(&p).expect("json"),
        )
        .await
        .expect("write");
        let peers = PrimalCapabilities::find_all_peers().await.expect("peers");
        assert!(peers.iter().any(|x| x.primal_id == "all-global"));
    })
    .await;
}

#[tokio::test]
async fn find_peer_with_errors_when_xdg_discovery_missing() {
    let temp = TempDir::new().expect("temp");
    let base_str = temp.path().to_string_lossy().to_string();
    temp_env::async_with_vars([("XDG_RUNTIME_DIR", Some(base_str.as_str()))], async {
        let _keep = temp;
        let err = PrimalCapabilities::find_peer_with("compute")
            .await
            .unwrap_err();
        assert!(
            err.contains("Failed to read discovery directory")
                || err.contains("No such file")
                || err.contains("not found")
        );
    })
    .await;
}
