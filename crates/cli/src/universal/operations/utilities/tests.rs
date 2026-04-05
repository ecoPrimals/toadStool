// SPDX-License-Identifier: AGPL-3.0-or-later

use super::UtilityOps;
use toadstool_distributed::substrate_detection::PlatformType;

#[tokio::test]
async fn test_get_platform_id_linux() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Linux {
        distribution: "Ubuntu 22.04".to_string(),
        architecture: "x86_64".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "linux_ubuntu_22.04_x86_64");
}

#[tokio::test]
async fn test_get_platform_id_docker() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Docker;
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "docker");
}

#[tokio::test]
async fn test_get_platform_id_gpu() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::GPU {
        vendor: "NVIDIA".to_string(),
        framework: "CUDA".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "gpu_nvidia_cuda");
}

#[tokio::test]
async fn test_get_platform_id_wasm() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::WebAssembly {
        runtime: "Wasmtime".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "wasm_wasmtime");
}

#[tokio::test]
async fn test_get_platform_id_language() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Language {
        name: "Python".to_string(),
        command: "python3".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "lang_python");
}

#[tokio::test]
async fn test_get_platform_id_macos() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::MacOS {
        version: "14.0".to_string(),
        architecture: "arm64".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "macos_14_0_arm64");
}

#[tokio::test]
async fn test_get_platform_id_windows() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Windows {
        version: "11".to_string(),
        architecture: "x86_64".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "windows_11_x86_64");
}

#[tokio::test]
async fn test_get_platform_id_other() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Other {
        os: "FreeBSD".to_string(),
        architecture: "amd64".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "other_FreeBSD_amd64");
}

#[tokio::test]
async fn test_get_platform_id_edge_device() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::EdgeDevice {
        device_type: "Raspberry Pi".to_string(),
        architecture: "armv7l".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "edge_Raspberry Pi_armv7l");
}

#[tokio::test]
async fn test_get_platform_metadata_linux() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Linux {
        distribution: "Debian".to_string(),
        architecture: "aarch64".to_string(),
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("linux"));
    assert_eq!(meta.get("distribution").map(|s| s.as_ref()), Some("Debian"));
    assert_eq!(
        meta.get("architecture").map(|s| s.as_ref()),
        Some("aarch64")
    );
}

#[tokio::test]
async fn test_get_platform_metadata_docker() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Docker;
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("container"));
    assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("docker"));
}

#[tokio::test]
async fn test_get_platform_metadata_gpu() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::GPU {
        vendor: "AMD".to_string(),
        framework: "ROCm".to_string(),
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("gpu"));
    assert_eq!(meta.get("vendor").map(|s| s.as_ref()), Some("AMD"));
    assert_eq!(meta.get("framework").map(|s| s.as_ref()), Some("ROCm"));
}

#[tokio::test]
async fn test_get_platform_id_podman() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let id = manager.get_platform_id(&PlatformType::Podman);
    assert_eq!(id, "podman");
}

#[tokio::test]
async fn test_get_platform_id_containerd() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let id = manager.get_platform_id(&PlatformType::Containerd);
    assert_eq!(id, "containerd");
}

#[tokio::test]
async fn test_get_platform_metadata_podman() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let meta = manager.get_platform_metadata(&PlatformType::Podman);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("container"));
    assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("podman"));
}

#[tokio::test]
async fn test_get_platform_metadata_containerd() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let meta = manager.get_platform_metadata(&PlatformType::Containerd);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("container"));
    assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("containerd"));
}

#[tokio::test]
async fn test_get_platform_metadata_wasm() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::WebAssembly {
        runtime: "Wasmtime".to_string(),
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("wasm"));
    assert_eq!(meta.get("runtime").map(|s| s.as_ref()), Some("Wasmtime"));
}

#[tokio::test]
async fn test_get_platform_metadata_language() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Language {
        name: "Python".to_string(),
        command: "python3".to_string(),
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("language"));
    assert_eq!(meta.get("name").map(|s| s.as_ref()), Some("Python"));
    assert_eq!(meta.get("command").map(|s| s.as_ref()), Some("python3"));
}

#[tokio::test]
async fn test_get_platform_metadata_other() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Other {
        os: "FreeBSD".to_string(),
        architecture: "amd64".to_string(),
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("other"));
    assert_eq!(meta.get("os").map(|s| s.as_ref()), Some("FreeBSD"));
}

#[tokio::test]
async fn test_get_platform_metadata_edge_device() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::EdgeDevice {
        device_type: "Raspberry Pi".to_string(),
        architecture: "armv7l".to_string(),
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("edge_device"));
    assert_eq!(
        meta.get("device_type").map(|s| s.as_ref()),
        Some("Raspberry Pi")
    );
}

#[tokio::test]
async fn test_get_platform_id_mcu() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::MCUDevelopment {
        platform: "ESP32".to_string(),
        tool: "idf".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "mcu_ESP32_idf");
}

#[tokio::test]
async fn test_get_platform_id_biological() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::BiologicalComputing {
        platform: "DNA".to_string(),
        simulation: true,
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "bio_DNA_true");
}

#[tokio::test]
async fn test_get_platform_id_quantum() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Quantum {
        framework: "Qiskit".to_string(),
        simulator: true,
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "quantum_Qiskit_true");
}

#[tokio::test]
async fn test_get_platform_id_neuromorphic() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::NeuromorphicComputing {
        platform: "Loihi".to_string(),
        hardware: true,
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "neuro_Loihi_true");
}

#[tokio::test]
async fn test_get_system_hardware_info() {
    let manager = crate::universal::UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let info = manager.get_system_hardware_info().await;
    assert!(info.is_ok());
    let info = info.unwrap();
    assert!(info.cpu_cores > 0);
    assert!(info.memory_gb > 0.0);
}
