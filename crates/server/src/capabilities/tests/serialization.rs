// SPDX-License-Identifier: AGPL-3.0-only
//! Serialization roundtrip and clone tests.
use super::*;

#[test]
fn test_gpu_device_serialization() {
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

    let json = serde_json::to_string(&gpu).expect("serialize");
    let parsed: GpuDevice = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.device_id, gpu.device_id);
    assert_eq!(parsed.name, gpu.name);
    assert_eq!(parsed.vendor, gpu.vendor);
    assert_eq!(parsed.memory_bytes, gpu.memory_bytes);
    assert_eq!(parsed.compute_capability, gpu.compute_capability);
    assert_eq!(parsed.render_node, gpu.render_node);
    assert_eq!(parsed.driver, gpu.driver);
    assert_eq!(parsed.arch, gpu.arch);
}

#[test]
fn test_gpu_device_without_compute_capability() {
    let gpu = GpuDevice {
        device_id: 1,
        name: "Integrated".to_string(),
        vendor: "intel".to_string(),
        memory_bytes: 1024 * 1024 * 1024,
        compute_capability: None,
        render_node: None,
        driver: None,
        arch: None,
    };

    let json = serde_json::to_string(&gpu).expect("serialize");
    let parsed: GpuDevice = serde_json::from_str(&json).expect("deserialize");
    assert!(parsed.compute_capability.is_none());
    assert!(parsed.render_node.is_none());
    assert!(parsed.driver.is_none());
    assert!(parsed.arch.is_none());
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
        metadata: std::iter::once(("region".to_string(), "us-west".to_string())).collect(),
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
