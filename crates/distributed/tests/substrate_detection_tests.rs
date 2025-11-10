//! Comprehensive tests for substrate detection module

use toadstool_distributed::substrate_detection::*;

#[test]
fn test_substrate_detector_new() {
    let _detector = SubstrateDetector::new();
    // Should create without panicking
}

#[test]
fn test_substrate_detector_default() {
    let _detector = SubstrateDetector;
    // Should create without panicking
}

#[test]
fn test_substrate_detector_creation_consistency() {
    let _detector1 = SubstrateDetector::new();
    let _detector2 = SubstrateDetector;

    // Both should create valid instances
}

#[test]
fn test_substrate_capabilities_creation() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    assert!(capabilities.traditional_platforms.is_empty());
    assert!(capabilities.container_platforms.is_empty());
    assert!(capabilities.language_runtimes.is_empty());
}

#[test]
fn test_substrate_capabilities_debug() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    let debug_str = format!("{:?}", capabilities);
    assert!(debug_str.contains("SubstrateCapabilities"));
}

#[test]
fn test_substrate_capabilities_clone() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    let cloned = capabilities.clone();
    assert_eq!(
        cloned.traditional_platforms.len(),
        capabilities.traditional_platforms.len()
    );
    assert_eq!(
        cloned.container_platforms.len(),
        capabilities.container_platforms.len()
    );
}

#[test]
fn test_platform_type_linux() {
    let platform = PlatformType::Linux {
        distribution: "Ubuntu".to_string(),
        architecture: "x86_64".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Linux"));
}

#[test]
fn test_platform_type_macos() {
    let platform = PlatformType::MacOS {
        version: "14.0".to_string(),
        architecture: "aarch64".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("MacOS"));
}

#[test]
fn test_platform_type_windows() {
    let platform = PlatformType::Windows {
        version: "11".to_string(),
        architecture: "x86_64".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Windows"));
}

#[test]
fn test_platform_type_docker() {
    let platform = PlatformType::Docker;

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Docker"));
}

#[test]
fn test_platform_type_podman() {
    let platform = PlatformType::Podman;

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Podman"));
}

#[test]
fn test_platform_type_containerd() {
    let platform = PlatformType::Containerd;

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Containerd"));
}

#[test]
fn test_platform_type_language() {
    let platform = PlatformType::Language {
        name: "Python".to_string(),
        command: "python3".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Language"));
}

#[test]
fn test_platform_type_language_rust() {
    let platform = PlatformType::Language {
        name: "Rust".to_string(),
        command: "cargo".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Rust"));
}

#[test]
fn test_platform_type_gpu() {
    let platform = PlatformType::GPU {
        vendor: "NVIDIA".to_string(),
        framework: "CUDA".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("GPU"));
}

#[test]
fn test_platform_type_gpu_amd() {
    let platform = PlatformType::GPU {
        vendor: "AMD".to_string(),
        framework: "ROCm".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("AMD"));
}

#[test]
fn test_platform_type_webassembly() {
    let platform = PlatformType::WebAssembly {
        runtime: "wasmtime".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("WebAssembly"));
}

#[test]
fn test_platform_type_webassembly_wasmer() {
    let platform = PlatformType::WebAssembly {
        runtime: "wasmer".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("wasmer"));
}

#[test]
fn test_platform_type_other() {
    let platform = PlatformType::Other {
        os: "FreeBSD".to_string(),
        architecture: "x86_64".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Other"));
}

#[test]
fn test_platform_type_mcu_development() {
    let platform = PlatformType::MCUDevelopment {
        platform: "Arduino".to_string(),
        tool: "arduino-cli".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("MCUDevelopment"));
}

#[test]
fn test_platform_type_biological_computing() {
    let platform = PlatformType::BiologicalComputing {
        platform: "DNA Storage".to_string(),
        simulation: true,
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("BiologicalComputing"));
}

#[test]
fn test_platform_type_neuromorphic() {
    let platform = PlatformType::NeuromorphicComputing {
        platform: "Intel Loihi".to_string(),
        hardware: true,
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("NeuromorphicComputing"));
}

#[test]
fn test_platform_type_quantum() {
    let platform = PlatformType::Quantum {
        framework: "IBM Quantum".to_string(),
        simulator: false,
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("Quantum"));
}

#[test]
fn test_platform_type_quantum_simulator() {
    let platform = PlatformType::Quantum {
        framework: "Qiskit".to_string(),
        simulator: true,
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("simulator"));
}

#[test]
fn test_platform_type_edge_device() {
    let platform = PlatformType::EdgeDevice {
        device_type: "Raspberry Pi".to_string(),
        architecture: "aarch64".to_string(),
    };

    let debug_str = format!("{:?}", platform);
    assert!(debug_str.contains("EdgeDevice"));
}

#[test]
fn test_substrate_capabilities_has_wasm() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![PlatformType::WebAssembly {
            runtime: "wasmtime".to_string(),
        }],
        experimental_platforms: vec![],
    };

    assert!(capabilities.has_wasm());
}

#[test]
fn test_substrate_capabilities_no_wasm() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    assert!(!capabilities.has_wasm());
}

#[test]
fn test_substrate_capabilities_multiple_platforms() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![PlatformType::Linux {
            distribution: "Ubuntu".to_string(),
            architecture: "x86_64".to_string(),
        }],
        container_platforms: vec![PlatformType::Docker],
        language_runtimes: vec![PlatformType::Language {
            name: "Python".to_string(),
            command: "python3".to_string(),
        }],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.traditional_platforms.len(), 1);
    assert_eq!(capabilities.container_platforms.len(), 1);
    assert_eq!(capabilities.language_runtimes.len(), 1);
}

#[test]
fn test_substrate_capabilities_all_platform_types() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![PlatformType::Linux {
            distribution: "Debian".to_string(),
            architecture: "x86_64".to_string(),
        }],
        container_platforms: vec![PlatformType::Podman],
        language_runtimes: vec![PlatformType::WebAssembly {
            runtime: "wasmer".to_string(),
        }],
        gpu_platforms: vec![PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        }],
        specialized_platforms: vec![PlatformType::Quantum {
            framework: "IBM Q".to_string(),
            simulator: false,
        }],
        experimental_platforms: vec![PlatformType::BiologicalComputing {
            platform: "DNA".to_string(),
            simulation: true,
        }],
    };

    assert_eq!(capabilities.traditional_platforms.len(), 1);
    assert_eq!(capabilities.container_platforms.len(), 1);
    assert_eq!(capabilities.language_runtimes.len(), 1);
    assert_eq!(capabilities.gpu_platforms.len(), 1);
    assert_eq!(capabilities.specialized_platforms.len(), 1);
    assert_eq!(capabilities.experimental_platforms.len(), 1);
}

#[test]
fn test_platform_type_serialize_deserialize() {
    let platform = PlatformType::Docker;

    let serialized = serde_json::to_string(&platform).expect("Failed to serialize");
    let deserialized: PlatformType =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    let debug1 = format!("{:?}", platform);
    let debug2 = format!("{:?}", deserialized);
    assert_eq!(debug1, debug2);
}

#[test]
fn test_substrate_capabilities_serialize_deserialize() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    let serialized = serde_json::to_string(&capabilities).expect("Failed to serialize");
    let deserialized: SubstrateCapabilities =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(
        deserialized.traditional_platforms.len(),
        capabilities.traditional_platforms.len()
    );
}

#[test]
fn test_edge_device_with_architecture() {
    let platform = PlatformType::EdgeDevice {
        device_type: "Arduino".to_string(),
        architecture: "avr".to_string(),
    };

    if let PlatformType::EdgeDevice { architecture, .. } = platform {
        assert_eq!(architecture, "avr");
    } else {
        panic!("Expected EdgeDevice platform type");
    }
}

#[test]
fn test_edge_device_arm_architecture() {
    let platform = PlatformType::EdgeDevice {
        device_type: "Raspberry Pi Pico".to_string(),
        architecture: "arm".to_string(),
    };

    if let PlatformType::EdgeDevice { architecture, .. } = platform {
        assert_eq!(architecture, "arm");
    } else {
        panic!("Expected EdgeDevice platform type");
    }
}

#[test]
fn test_quantum_platform_simulator() {
    let platform = PlatformType::Quantum {
        framework: "Qiskit".to_string(),
        simulator: true,
    };

    if let PlatformType::Quantum { simulator, .. } = platform {
        assert!(simulator);
    } else {
        panic!("Expected Quantum platform type");
    }
}

#[test]
fn test_biological_computing_real_hardware() {
    let platform = PlatformType::BiologicalComputing {
        platform: "DNA Computer".to_string(),
        simulation: false,
    };

    if let PlatformType::BiologicalComputing { simulation, .. } = platform {
        assert!(!simulation);
    } else {
        panic!("Expected BiologicalComputing platform type");
    }
}

#[test]
fn test_language_different_runtimes() {
    let python = PlatformType::Language {
        name: "Python".to_string(),
        command: "python3".to_string(),
    };

    let node = PlatformType::Language {
        name: "Node.js".to_string(),
        command: "node".to_string(),
    };

    let debug_python = format!("{:?}", python);
    let debug_node = format!("{:?}", node);

    assert!(debug_python.contains("Python"));
    assert!(debug_node.contains("Node"));
}

#[test]
fn test_gpu_different_vendors() {
    let nvidia = PlatformType::GPU {
        vendor: "NVIDIA".to_string(),
        framework: "CUDA".to_string(),
    };

    let amd = PlatformType::GPU {
        vendor: "AMD".to_string(),
        framework: "ROCm".to_string(),
    };

    let debug_nvidia = format!("{:?}", nvidia);
    let debug_amd = format!("{:?}", amd);

    assert!(debug_nvidia.contains("NVIDIA"));
    assert!(debug_amd.contains("AMD"));
}

#[test]
fn test_architecture_variants() {
    let x86_platform = PlatformType::Linux {
        distribution: "Ubuntu".to_string(),
        architecture: "x86_64".to_string(),
    };

    let arm_platform = PlatformType::Linux {
        distribution: "Ubuntu".to_string(),
        architecture: "aarch64".to_string(),
    };

    let debug_x86 = format!("{:?}", x86_platform);
    let debug_arm = format!("{:?}", arm_platform);

    assert!(debug_x86.contains("x86_64"));
    assert!(debug_arm.contains("aarch64"));
}
