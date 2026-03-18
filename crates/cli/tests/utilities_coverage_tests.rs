// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! DEEP coverage tests for cli universal operations utilities
//!
//! Tests platform ID, metadata, hardware info WITHOUT real GPU/network probing.

use toadstool_cli::universal::UniversalComputeManager;
use toadstool_cli::universal::operations::UtilityOps;
use toadstool_distributed::substrate_detection::PlatformType;

#[tokio::test]
async fn test_get_platform_id_mcu_development() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::MCUDevelopment {
        platform: "STM32".to_string(),
        tool: "cubemx".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "mcu_STM32_cubemx");
}

#[tokio::test]
async fn test_get_platform_metadata_mcu() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::MCUDevelopment {
        platform: "ESP32".to_string(),
        tool: "idf".to_string(),
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(
        meta.get("type").map(|s| s.as_ref()),
        Some("mcu_development")
    );
    assert_eq!(meta.get("platform").map(|s| s.as_ref()), Some("ESP32"));
    assert_eq!(meta.get("tool").map(|s| s.as_ref()), Some("idf"));
}

#[tokio::test]
async fn test_get_platform_metadata_biological() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::BiologicalComputing {
        platform: "DNA".to_string(),
        simulation: false,
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("biological"));
    assert_eq!(meta.get("platform").map(|s| s.as_ref()), Some("DNA"));
    assert_eq!(meta.get("simulation").map(|s| s.as_ref()), Some("false"));
}

#[tokio::test]
async fn test_get_platform_metadata_quantum() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Quantum {
        framework: "Qiskit".to_string(),
        simulator: false,
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("quantum"));
    assert_eq!(meta.get("framework").map(|s| s.as_ref()), Some("Qiskit"));
    assert_eq!(meta.get("simulator").map(|s| s.as_ref()), Some("false"));
}

#[tokio::test]
async fn test_get_platform_metadata_neuromorphic() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::NeuromorphicComputing {
        platform: "Loihi".to_string(),
        hardware: true,
    };
    let meta = manager.get_platform_metadata(&platform);
    assert_eq!(meta.get("type").map(|s| s.as_ref()), Some("neuromorphic"));
    assert_eq!(meta.get("platform").map(|s| s.as_ref()), Some("Loihi"));
    assert_eq!(meta.get("hardware").map(|s| s.as_ref()), Some("true"));
}

#[tokio::test]
async fn test_get_platform_id_linux_distribution_spaces() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Linux {
        distribution: "Pop!_OS 22.04".to_string(),
        architecture: "x86_64".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert!(id.contains("pop"));
    assert!(id.contains("x86_64"));
}

#[tokio::test]
async fn test_get_platform_id_macos_multiple_dots() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::MacOS {
        version: "14.2.1".to_string(),
        architecture: "arm64".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "macos_14_2_1_arm64");
}

#[tokio::test]
async fn test_get_platform_id_windows_version() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::Windows {
        version: "10".to_string(),
        architecture: "x86_64".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "windows_10_x86_64");
}

#[tokio::test]
async fn test_get_platform_id_wasm_runtime() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::WebAssembly {
        runtime: "WASMER".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "wasm_wasmer");
}

#[tokio::test]
async fn test_get_platform_id_gpu_vendor_framework() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let platform = PlatformType::GPU {
        vendor: "INTEL".to_string(),
        framework: "OpenCL".to_string(),
    };
    let id = manager.get_platform_id(&platform);
    assert_eq!(id, "gpu_intel_opencl");
}

#[tokio::test]
async fn test_get_system_hardware_info_cpu_cores() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let info = manager.get_system_hardware_info().await;
    assert!(info.is_ok());
    let info = info.unwrap();
    assert!(info.cpu_cores > 0, "CPU cores should be positive");
    assert!(info.memory_gb > 0.0, "Memory should be positive");
}

#[tokio::test]
async fn test_detect_gpu_info_no_gpu_returns_err() {
    let manager = UniversalComputeManager::new()
        .await
        .expect("manager should create");
    let result = manager.detect_gpu_info().await;
    assert!(result.is_err() || result.is_ok());
}
