//! Comprehensive tests for substrate detection module
//!
//! Tests cover platform detection, capabilities tracking, and substrate identification.

use toadstool_distributed::substrate_detection::*;

// ============================================================================
// SubstrateDetector Tests
// ============================================================================

#[tokio::test]
async fn test_substrate_detector_new() {
    let _detector = SubstrateDetector::new();
    // Creation should not panic
}

#[tokio::test]
async fn test_substrate_detector_default() {
    let _detector = SubstrateDetector::default();
    // Default constructor should work
}

// ============================================================================
// PlatformType Tests
// ============================================================================

#[test]
fn test_platform_type_linux() {
    let platform = PlatformType::Linux {
        distribution: "Ubuntu".to_string(),
        architecture: "x86_64".to_string(),
    };

    match platform {
        PlatformType::Linux {
            distribution,
            architecture,
        } => {
            assert_eq!(distribution, "Ubuntu");
            assert_eq!(architecture, "x86_64");
        }
        _ => panic!("Expected Linux platform type"),
    }
}

#[test]
fn test_platform_type_windows() {
    let platform = PlatformType::Windows {
        version: "11".to_string(),
        architecture: "x86_64".to_string(),
    };

    match platform {
        PlatformType::Windows {
            version,
            architecture,
        } => {
            assert_eq!(version, "11");
            assert_eq!(architecture, "x86_64");
        }
        _ => panic!("Expected Windows platform type"),
    }
}

#[test]
fn test_platform_type_macos() {
    let platform = PlatformType::MacOS {
        version: "14.0".to_string(),
        architecture: "arm64".to_string(),
    };

    match platform {
        PlatformType::MacOS {
            version,
            architecture,
        } => {
            assert_eq!(version, "14.0");
            assert_eq!(architecture, "arm64");
        }
        _ => panic!("Expected MacOS platform type"),
    }
}

#[test]
fn test_platform_type_docker() {
    let platform = PlatformType::Docker;
    match platform {
        PlatformType::Docker => {}
        _ => panic!("Expected Docker platform type"),
    }
}

#[test]
fn test_platform_type_podman() {
    let platform = PlatformType::Podman;
    match platform {
        PlatformType::Podman => {}
        _ => panic!("Expected Podman platform type"),
    }
}

#[test]
fn test_platform_type_containerd() {
    let platform = PlatformType::Containerd;
    match platform {
        PlatformType::Containerd => {}
        _ => panic!("Expected Containerd platform type"),
    }
}

#[test]
fn test_platform_type_language() {
    let platform = PlatformType::Language {
        name: "Python".to_string(),
        command: "python3".to_string(),
    };

    match platform {
        PlatformType::Language { name, command } => {
            assert_eq!(name, "Python");
            assert_eq!(command, "python3");
        }
        _ => panic!("Expected Language platform type"),
    }
}

#[test]
fn test_platform_type_gpu() {
    let platform = PlatformType::GPU {
        vendor: "NVIDIA".to_string(),
        framework: "CUDA".to_string(),
    };

    match platform {
        PlatformType::GPU { vendor, framework } => {
            assert_eq!(vendor, "NVIDIA");
            assert_eq!(framework, "CUDA");
        }
        _ => panic!("Expected GPU platform type"),
    }
}

#[test]
fn test_platform_type_webassembly() {
    let platform = PlatformType::WebAssembly {
        runtime: "wasmtime".to_string(),
    };

    match platform {
        PlatformType::WebAssembly { runtime } => {
            assert_eq!(runtime, "wasmtime");
        }
        _ => panic!("Expected WebAssembly platform type"),
    }
}

#[test]
fn test_platform_type_other() {
    let platform = PlatformType::Other {
        os: "FreeBSD".to_string(),
        architecture: "amd64".to_string(),
    };

    match platform {
        PlatformType::Other { os, architecture } => {
            assert_eq!(os, "FreeBSD");
            assert_eq!(architecture, "amd64");
        }
        _ => panic!("Expected Other platform type"),
    }
}

#[test]
fn test_platform_type_edge_device() {
    let platform = PlatformType::EdgeDevice {
        device_type: "Raspberry Pi".to_string(),
        architecture: "armv7l".to_string(),
    };

    match platform {
        PlatformType::EdgeDevice {
            device_type,
            architecture,
        } => {
            assert_eq!(device_type, "Raspberry Pi");
            assert_eq!(architecture, "armv7l");
        }
        _ => panic!("Expected EdgeDevice platform type"),
    }
}

#[test]
fn test_platform_type_mcu_development() {
    let platform = PlatformType::MCUDevelopment {
        platform: "Arduino".to_string(),
        tool: "arduino-cli".to_string(),
    };

    match platform {
        PlatformType::MCUDevelopment { platform, tool } => {
            assert_eq!(platform, "Arduino");
            assert_eq!(tool, "arduino-cli");
        }
        _ => panic!("Expected MCUDevelopment platform type"),
    }
}

#[test]
fn test_platform_type_biological_computing() {
    let platform = PlatformType::BiologicalComputing {
        platform: "DNA Computing".to_string(),
        simulation: true,
    };

    match platform {
        PlatformType::BiologicalComputing {
            platform,
            simulation,
        } => {
            assert_eq!(platform, "DNA Computing");
            assert!(simulation);
        }
        _ => panic!("Expected BiologicalComputing platform type"),
    }
}

#[test]
fn test_platform_type_quantum() {
    let platform = PlatformType::Quantum {
        framework: "Qiskit".to_string(),
        simulator: true,
    };

    match platform {
        PlatformType::Quantum {
            framework,
            simulator,
        } => {
            assert_eq!(framework, "Qiskit");
            assert!(simulator);
        }
        _ => panic!("Expected Quantum platform type"),
    }
}

#[test]
fn test_platform_type_neuromorphic() {
    let platform = PlatformType::NeuromorphicComputing {
        platform: "Intel Loihi".to_string(),
        hardware: false,
    };

    match platform {
        PlatformType::NeuromorphicComputing { platform, hardware } => {
            assert_eq!(platform, "Intel Loihi");
            assert!(!hardware);
        }
        _ => panic!("Expected NeuromorphicComputing platform type"),
    }
}

// ============================================================================
// SubstrateCapabilities Tests
// ============================================================================

#[test]
fn test_substrate_capabilities_empty() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.total_platforms(), 0);
    assert!(!capabilities.has_containers());
    assert!(!capabilities.has_gpu());
    assert!(capabilities.language_runtimes.is_empty());
}

#[test]
fn test_substrate_capabilities_with_containers() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![PlatformType::Docker, PlatformType::Podman],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.total_platforms(), 2);
    assert!(capabilities.has_containers());
    assert!(!capabilities.has_gpu());
}

#[test]
fn test_substrate_capabilities_with_gpu() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        }],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.total_platforms(), 1);
    assert!(!capabilities.has_containers());
    assert!(capabilities.has_gpu());
}

#[test]
fn test_substrate_capabilities_with_languages() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![],
        container_platforms: vec![],
        language_runtimes: vec![
            PlatformType::Language {
                name: "Python".to_string(),
                command: "python3".to_string(),
            },
            PlatformType::Language {
                name: "Node.js".to_string(),
                command: "node".to_string(),
            },
        ],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.total_platforms(), 2);
    assert!(!capabilities.language_runtimes.is_empty());
}

#[test]
fn test_substrate_capabilities_comprehensive() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![PlatformType::Linux {
            distribution: "Ubuntu".to_string(),
            architecture: "x86_64".to_string(),
        }],
        container_platforms: vec![PlatformType::Docker, PlatformType::Podman],
        language_runtimes: vec![
            PlatformType::Language {
                name: "Python".to_string(),
                command: "python3".to_string(),
            },
            PlatformType::Language {
                name: "Node.js".to_string(),
                command: "node".to_string(),
            },
        ],
        gpu_platforms: vec![PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        }],
        specialized_platforms: vec![PlatformType::Quantum {
            framework: "Qiskit".to_string(),
            simulator: true,
        }],
        experimental_platforms: vec![],
    };

    assert_eq!(capabilities.total_platforms(), 7);
    assert!(capabilities.has_containers());
    assert!(capabilities.has_gpu());
    assert!(!capabilities.language_runtimes.is_empty());
}

#[test]
fn test_substrate_capabilities_clone() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![PlatformType::Docker],
        container_platforms: vec![],
        language_runtimes: vec![],
        gpu_platforms: vec![],
        specialized_platforms: vec![],
        experimental_platforms: vec![],
    };

    let cloned = capabilities.clone();
    assert_eq!(cloned.total_platforms(), 1);
}

#[test]
fn test_total_platforms_calculation() {
    let capabilities = SubstrateCapabilities {
        traditional_platforms: vec![PlatformType::Docker; 3],
        container_platforms: vec![PlatformType::Podman; 2],
        language_runtimes: vec![PlatformType::Containerd; 1],
        gpu_platforms: vec![PlatformType::Docker; 4],
        specialized_platforms: vec![PlatformType::Docker; 5],
        experimental_platforms: vec![PlatformType::Docker; 1],
    };

    // 3 + 2 + 1 + 4 + 5 + 1 = 16
    assert_eq!(capabilities.total_platforms(), 16);
}

// ============================================================================
// Platform Type Collections Tests
// ============================================================================

#[test]
fn test_multiple_linux_distributions() {
    let platforms = vec![
        PlatformType::Linux {
            distribution: "Ubuntu".to_string(),
            architecture: "x86_64".to_string(),
        },
        PlatformType::Linux {
            distribution: "Fedora".to_string(),
            architecture: "x86_64".to_string(),
        },
        PlatformType::Linux {
            distribution: "Arch".to_string(),
            architecture: "x86_64".to_string(),
        },
    ];

    assert_eq!(platforms.len(), 3);
}

#[test]
fn test_multiple_container_platforms() {
    let platforms = vec![
        PlatformType::Docker,
        PlatformType::Podman,
        PlatformType::Containerd,
    ];

    assert_eq!(platforms.len(), 3);
}

#[test]
fn test_multiple_language_runtimes() {
    let languages = vec![
        PlatformType::Language {
            name: "Python".to_string(),
            command: "python3".to_string(),
        },
        PlatformType::Language {
            name: "Node.js".to_string(),
            command: "node".to_string(),
        },
        PlatformType::Language {
            name: "Ruby".to_string(),
            command: "ruby".to_string(),
        },
        PlatformType::Language {
            name: "Go".to_string(),
            command: "go".to_string(),
        },
    ];

    assert_eq!(languages.len(), 4);
}

#[test]
fn test_multiple_gpu_platforms() {
    let gpus = vec![
        PlatformType::GPU {
            vendor: "NVIDIA".to_string(),
            framework: "CUDA".to_string(),
        },
        PlatformType::GPU {
            vendor: "AMD".to_string(),
            framework: "ROCm".to_string(),
        },
        PlatformType::GPU {
            vendor: "Intel".to_string(),
            framework: "oneAPI".to_string(),
        },
    ];

    assert_eq!(gpus.len(), 3);
}

#[test]
fn test_specialized_platforms_collection() {
    let specialized = vec![
        PlatformType::Quantum {
            framework: "Qiskit".to_string(),
            simulator: true,
        },
        PlatformType::NeuromorphicComputing {
            platform: "Intel Loihi".to_string(),
            hardware: false,
        },
        PlatformType::BiologicalComputing {
            platform: "DNA Computing".to_string(),
            simulation: true,
        },
    ];

    assert_eq!(specialized.len(), 3);
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_empty_platform_strings() {
    let platform = PlatformType::Linux {
        distribution: "".to_string(),
        architecture: "".to_string(),
    };

    match platform {
        PlatformType::Linux {
            distribution,
            architecture,
        } => {
            assert!(distribution.is_empty());
            assert!(architecture.is_empty());
        }
        _ => panic!("Expected Linux platform"),
    }
}

#[test]
fn test_long_platform_strings() {
    let long_string = "x".repeat(1000);
    let platform = PlatformType::Linux {
        distribution: long_string.clone(),
        architecture: long_string.clone(),
    };

    match platform {
        PlatformType::Linux {
            distribution,
            architecture,
        } => {
            assert_eq!(distribution.len(), 1000);
            assert_eq!(architecture.len(), 1000);
        }
        _ => panic!("Expected Linux platform"),
    }
}

#[test]
fn test_quantum_simulator_vs_hardware() {
    let simulator = PlatformType::Quantum {
        framework: "Qiskit".to_string(),
        simulator: true,
    };

    let hardware = PlatformType::Quantum {
        framework: "IBM Quantum".to_string(),
        simulator: false,
    };

    match simulator {
        PlatformType::Quantum {
            framework: _,
            simulator,
        } => assert!(simulator),
        _ => panic!("Expected Quantum platform"),
    }

    match hardware {
        PlatformType::Quantum {
            framework: _,
            simulator,
        } => assert!(!simulator),
        _ => panic!("Expected Quantum platform"),
    }
}

#[test]
fn test_neuromorphic_hardware_vs_simulation() {
    let simulation = PlatformType::NeuromorphicComputing {
        platform: "BrainScaleS".to_string(),
        hardware: false,
    };

    let hardware = PlatformType::NeuromorphicComputing {
        platform: "Intel Loihi 2".to_string(),
        hardware: true,
    };

    match simulation {
        PlatformType::NeuromorphicComputing {
            platform: _,
            hardware,
        } => assert!(!hardware),
        _ => panic!("Expected NeuromorphicComputing platform"),
    }

    match hardware {
        PlatformType::NeuromorphicComputing {
            platform: _,
            hardware,
        } => assert!(hardware),
        _ => panic!("Expected NeuromorphicComputing platform"),
    }
}

#[test]
fn test_biological_computing_simulation_flag() {
    let real = PlatformType::BiologicalComputing {
        platform: "Wetware Lab".to_string(),
        simulation: false,
    };

    let simulated = PlatformType::BiologicalComputing {
        platform: "Digital DNA".to_string(),
        simulation: true,
    };

    match real {
        PlatformType::BiologicalComputing {
            platform: _,
            simulation,
        } => assert!(!simulation),
        _ => panic!("Expected BiologicalComputing platform"),
    }

    match simulated {
        PlatformType::BiologicalComputing {
            platform: _,
            simulation,
        } => assert!(simulation),
        _ => panic!("Expected BiologicalComputing platform"),
    }
}

// ============================================================================
// Architecture Variants Tests
// ============================================================================

#[test]
fn test_various_architectures() {
    let architectures = vec!["x86_64", "arm64", "armv7l", "aarch64", "riscv64"];

    for arch in architectures {
        let platform = PlatformType::Linux {
            distribution: "Generic".to_string(),
            architecture: arch.to_string(),
        };

        match platform {
            PlatformType::Linux {
                distribution: _,
                architecture,
            } => assert_eq!(architecture, arch),
            _ => panic!("Expected Linux platform"),
        }
    }
}

#[test]
fn test_cross_platform_support() {
    let platforms = vec![
        PlatformType::Linux {
            distribution: "Ubuntu".to_string(),
            architecture: "x86_64".to_string(),
        },
        PlatformType::Windows {
            version: "11".to_string(),
            architecture: "x86_64".to_string(),
        },
        PlatformType::MacOS {
            version: "14".to_string(),
            architecture: "arm64".to_string(),
        },
    ];

    assert_eq!(platforms.len(), 3);
}
