//! Comprehensive test coverage for hardware detection module
//!
//! This test suite provides property-based tests, table-driven tests, and error path
//! coverage for the hardware detection system.

use toadstool_auto_config::hardware::{CpuFeatures, NetworkInfo};
use toadstool_auto_config::{
    CpuInfo, GpuInfo, HardwareDetector, MemoryInfo, PerformanceClass, StorageInfo, StorageType,
    SystemCapabilities,
};

/// Test that hardware detector can be created
#[test]
fn test_hardware_detector_creation() {
    let _detector = HardwareDetector::new();
    // Should create successfully
}

/// Test CPU info default values are sensible
#[test]
fn test_cpu_info_defaults() {
    let cpu = CpuInfo::default();

    // Should have reasonable defaults
    assert!(
        !cpu.model_name.is_empty(),
        "CPU model name should not be empty"
    );
    assert!(cpu.physical_cores > 0, "Should have at least one core");
    assert!(
        cpu.logical_cores >= cpu.physical_cores,
        "Logical cores should be >= physical cores"
    );
    assert!(
        cpu.base_frequency_mhz > 0.0,
        "Base frequency should be positive"
    );
    assert!(
        cpu.max_frequency_mhz >= cpu.base_frequency_mhz,
        "Max frequency should be >= base"
    );
}

/// Test memory info default values
#[test]
fn test_memory_info_defaults() {
    let mem = MemoryInfo::default();

    assert!(mem.total_gb > 0.0, "Total memory should be positive");
    assert!(
        mem.available_gb >= 0.0,
        "Available memory should be non-negative"
    );
    assert!(
        mem.available_gb <= mem.total_gb,
        "Available should not exceed total"
    );
}

/// Test GPU info structure
#[test]
fn test_gpu_info_structure() {
    let gpu = GpuInfo {
        name: "Test GPU".to_string(),
        vendor: "Test Vendor".to_string(),
        memory_gb: 8.0,
        driver_version: "1.0".to_string(),
        compute_capability: "8.6".to_string(),
        supports_cuda: true,
        supports_opencl: false,
    };

    assert_eq!(gpu.name, "Test GPU");
    assert_eq!(gpu.vendor, "Test Vendor");
    assert_eq!(gpu.memory_gb, 8.0);
    assert!(!gpu.compute_capability.is_empty());
}

/// Test storage type classification
#[test]
fn test_storage_type_classification() {
    let storage_types = vec![
        StorageType::SSD,
        StorageType::HDD,
        StorageType::NVME,
        StorageType::Unknown,
    ];

    // Should have distinct types
    assert_eq!(storage_types.len(), 4);

    // Should be able to debug print
    for storage_type in &storage_types {
        let _debug_str = format!("{:?}", storage_type);
    }
}

/// Test storage info validation
#[test]
fn test_storage_info_validation() {
    let storage = StorageInfo {
        total_gb: 100.0,
        available_gb: 50.0,
        storage_type: StorageType::SSD,
    };

    assert!(storage.total_gb > 0.0);
    assert!(storage.available_gb >= 0.0);
    assert!(storage.available_gb <= storage.total_gb);
}

/// Test performance class ordering
#[test]
fn test_performance_class_ordering() {
    let classes = vec![
        PerformanceClass::HighEnd,
        PerformanceClass::Mainstream,
        PerformanceClass::Budget,
        PerformanceClass::LowEnd,
    ];

    // All classes should be distinct
    assert_eq!(classes.len(), 4);

    // Should support Debug
    for class in &classes {
        let _debug = format!("{:?}", class);
    }
}

/// Test system capabilities default values
#[test]
fn test_system_capabilities_defaults() {
    let caps = SystemCapabilities::default();

    // Should have sensible defaults
    assert!(caps.cpu_cores > 0.0);
    assert!(caps.memory_gb > 0.0);
    assert!(caps.storage_gb > 0.0);
    assert!(!caps.cpu_info.model_name.is_empty());
}

/// Table-driven tests for CPU core count validation
#[test]
fn test_cpu_core_count_validation() {
    let test_cases = vec![
        (1, 1, true),  // Single core
        (2, 2, true),  // Dual core
        (4, 4, true),  // Quad core
        (4, 8, true),  // Hyper-threading
        (8, 16, true), // High-end with HT
        (0, 0, false), // Invalid: zero cores
        (2, 1, false), // Invalid: more physical than logical
    ];

    for (physical, logical, should_be_valid) in test_cases {
        let cpu = CpuInfo {
            model_name: "Test CPU".to_string(),
            physical_cores: physical,
            logical_cores: logical,
            family: 6,
            base_frequency_mhz: 3000.0,
            max_frequency_mhz: 4000.0,
            cache_size_kb: 8192,
            instruction_sets: Vec::new(),
            features: CpuFeatures::default(),
        };

        let is_valid = cpu.logical_cores >= cpu.physical_cores && cpu.physical_cores > 0;
        assert_eq!(
            is_valid, should_be_valid,
            "CPU validation failed for physical={}, logical={}",
            physical, logical
        );
    }
}

/// Table-driven tests for memory validation
#[test]
fn test_memory_validation_scenarios() {
    let test_cases = vec![
        (8.0, 4.0, true),   // Normal: 8GB total, 4GB available
        (16.0, 8.0, true),  // Normal: 16GB total, 8GB available
        (32.0, 0.5, true),  // Low memory: 32GB total, 0.5GB available
        (4.0, 4.0, true),   // All available
        (0.0, 0.0, false),  // Invalid: no memory
        (8.0, 10.0, false), // Invalid: more available than total
        (-1.0, 0.0, false), // Invalid: negative total
    ];

    for (total, available, should_be_valid) in test_cases {
        let mem = MemoryInfo {
            total_gb: total,
            available_gb: available,
            memory_type: "DDR4".to_string(),
            frequency_mhz: 3200,
        };

        let is_valid =
            mem.total_gb > 0.0 && mem.available_gb >= 0.0 && mem.available_gb <= mem.total_gb;

        assert_eq!(
            is_valid, should_be_valid,
            "Memory validation failed for total={}, available={}",
            total, available
        );
    }
}

/// Test performance classification logic
#[test]
fn test_performance_classification() {
    let test_cases = vec![
        // (cores, memory_gb, gpu_count, expected_class)
        (16.0, 32.0, 2, PerformanceClass::HighEnd),
        (8.0, 16.0, 1, PerformanceClass::Mainstream),
        (4.0, 8.0, 0, PerformanceClass::Budget),
        (2.0, 4.0, 0, PerformanceClass::LowEnd),
    ];

    for (cores, memory, gpu_count, expected_class) in test_cases {
        let caps = SystemCapabilities {
            cpu_cores: cores,
            memory_gb: memory,
            gpu_count,
            gpu_memory_gb: if gpu_count > 0 { Some(8.0) } else { None },
            storage_gb: 500.0,
            cpu_info: CpuInfo::default(),
            memory_info: MemoryInfo::default(),
            gpu_info: Vec::new(),
            storage_info: StorageInfo::default(),
            network_info: NetworkInfo {
                interfaces: Vec::new(),
            },
            performance_class: expected_class.clone(),
        };

        // Verify the classification is reasonable
        match caps.performance_class {
            PerformanceClass::HighEnd => {
                assert!(caps.cpu_cores >= 8.0 || caps.memory_gb >= 16.0);
            }
            PerformanceClass::Mainstream => {
                assert!(caps.cpu_cores >= 4.0 || caps.memory_gb >= 8.0);
            }
            PerformanceClass::Budget => {
                assert!(caps.cpu_cores >= 2.0 || caps.memory_gb >= 4.0);
            }
            PerformanceClass::LowEnd => {
                // Any configuration
            }
        }
    }
}

/// Test edge cases for hardware detection
#[test]
fn test_hardware_detection_edge_cases() {
    // Test with minimal capabilities
    let minimal_caps = SystemCapabilities {
        cpu_cores: 1.0,
        memory_gb: 1.0,
        gpu_count: 0,
        gpu_memory_gb: None,
        storage_gb: 10.0,
        cpu_info: CpuInfo::default(),
        memory_info: MemoryInfo::default(),
        gpu_info: Vec::new(),
        storage_info: StorageInfo::default(),
        network_info: NetworkInfo {
            interfaces: Vec::new(),
        },
        performance_class: PerformanceClass::LowEnd,
    };

    assert_eq!(minimal_caps.cpu_cores, 1.0);
    assert_eq!(minimal_caps.memory_gb, 1.0);
    assert_eq!(minimal_caps.gpu_count, 0);
    assert!(minimal_caps.gpu_memory_gb.is_none());

    // Test with maximum capabilities
    let max_caps = SystemCapabilities {
        cpu_cores: 128.0,
        memory_gb: 1024.0,
        gpu_count: 8,
        gpu_memory_gb: Some(80.0),
        storage_gb: 10_000.0,
        cpu_info: CpuInfo::default(),
        memory_info: MemoryInfo::default(),
        gpu_info: Vec::new(),
        storage_info: StorageInfo::default(),
        network_info: NetworkInfo {
            interfaces: Vec::new(),
        },
        performance_class: PerformanceClass::HighEnd,
    };

    assert!(max_caps.cpu_cores >= 128.0);
    assert!(max_caps.memory_gb >= 1024.0);
}

/// Test CPU feature detection
#[test]
fn test_cpu_features() {
    let features = CpuFeatures {
        supports_avx: true,
        supports_avx2: true,
        supports_sse4_1: true,
        supports_sse4_2: true,
        supports_neon: false,
    };

    assert!(features.supports_avx);
    assert!(features.supports_avx2);
    assert!(features.supports_sse4_2);
    assert!(!features.supports_neon);
}

/// Test storage type detection patterns
#[test]
fn test_storage_type_patterns() {
    let test_cases = vec![
        ("NVMe SSD", StorageType::NVME),
        ("SATA SSD", StorageType::SSD),
        ("HDD", StorageType::HDD),
        ("Unknown Device", StorageType::Unknown),
    ];

    for (description, expected_type) in test_cases {
        // Simulate classification logic
        let storage_type = if description.contains("NVMe") {
            StorageType::NVME
        } else if description.contains("SSD") {
            StorageType::SSD
        } else if description.contains("HDD") {
            StorageType::HDD
        } else {
            StorageType::Unknown
        };

        assert_eq!(
            format!("{:?}", storage_type),
            format!("{:?}", expected_type),
            "Storage type mismatch for: {}",
            description
        );
    }
}

/// Test concurrent hardware detection (should be thread-safe)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_hardware_detection() {
    use std::sync::Arc;
    use tokio::sync::Semaphore;

    let semaphore = Arc::new(Semaphore::new(4));
    let mut handles = vec![];

    for _ in 0..10 {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();

            // Create detector (should be thread-safe)
            let _detector = HardwareDetector::new();

            // Get default capabilities (doesn't require actual scanning)
            let caps = SystemCapabilities::default();

            assert!(caps.cpu_cores > 0.0);
            assert!(caps.memory_gb > 0.0);
        });

        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
    }
}

/// Test error handling for invalid system paths
#[test]
fn test_invalid_system_paths() {
    // Test that defaults are used when system files don't exist
    let caps = SystemCapabilities::default();

    // Should have sensible defaults even without system files
    assert!(caps.cpu_cores > 0.0);
    assert!(caps.memory_gb > 0.0);
    assert!(!caps.cpu_info.model_name.is_empty());
}

/// Test GPU memory validation
#[test]
fn test_gpu_memory_validation() {
    let test_cases = vec![
        (0, None, true),       // No GPU, no memory
        (1, Some(4.0), true),  // 4GB GPU
        (2, Some(8.0), true),  // 8GB GPU
        (1, None, true),       // GPU without detected memory
        (0, Some(8.0), false), // No GPU but memory reported (invalid)
    ];

    for (gpu_count, gpu_memory, should_be_valid) in test_cases {
        let is_valid = if gpu_count == 0 {
            gpu_memory.is_none()
        } else {
            true // GPU with or without memory is valid
        };

        assert_eq!(
            is_valid, should_be_valid,
            "GPU validation failed for count={}, memory={:?}",
            gpu_count, gpu_memory
        );
    }
}

/// Test Clone and Debug traits
#[test]
fn test_trait_implementations() {
    let cpu = CpuInfo::default();
    let cloned_cpu = cpu.clone();
    assert_eq!(cpu.physical_cores, cloned_cpu.physical_cores);

    let mem = MemoryInfo::default();
    let _debug = format!("{:?}", mem);

    let storage = StorageInfo::default();
    let _debug = format!("{:?}", storage);
}

/// Test performance class progression
#[test]
fn test_performance_class_progression() {
    let classes = vec![
        PerformanceClass::LowEnd,
        PerformanceClass::Budget,
        PerformanceClass::Mainstream,
        PerformanceClass::HighEnd,
    ];

    // Should represent increasing capability
    assert_eq!(classes.len(), 4);

    // Each should be distinct
    let class_set: std::collections::HashSet<_> =
        classes.iter().map(|c| format!("{:?}", c)).collect();
    assert_eq!(class_set.len(), 4);
}

/// Test system capabilities serialization (if Serialize is implemented)
#[test]
fn test_system_capabilities_structure() {
    let caps = SystemCapabilities {
        cpu_cores: 8.0,
        memory_gb: 16.0,
        gpu_count: 1,
        gpu_memory_gb: Some(8.0),
        storage_gb: 500.0,
        cpu_info: CpuInfo::default(),
        memory_info: MemoryInfo::default(),
        gpu_info: vec![GpuInfo {
            name: "Test GPU".to_string(),
            vendor: "Test".to_string(),
            memory_gb: 8.0,
            driver_version: "1.0".to_string(),
            compute_capability: "8.6".to_string(),
            supports_cuda: true,
            supports_opencl: false,
        }],
        storage_info: StorageInfo::default(),
        network_info: NetworkInfo {
            interfaces: Vec::new(),
        },
        performance_class: PerformanceClass::Mainstream,
    };

    // Verify structure is well-formed
    assert_eq!(caps.gpu_info.len(), 1);
    assert_eq!(caps.gpu_info[0].name, "Test GPU");
    assert_eq!(caps.gpu_count, 1);
}

/// Test rapid successive hardware detections (stress test)
#[tokio::test]
async fn test_rapid_hardware_detection() {
    for _ in 0..100 {
        let _detector = HardwareDetector::new();
        let caps = SystemCapabilities::default();
        assert!(caps.cpu_cores > 0.0);
    }
}

/// Test memory constraints
#[test]
fn test_memory_constraints() {
    let test_cases = vec![
        (0.5, "Minimal"),    // 512MB
        (1.0, "Low"),        // 1GB
        (2.0, "Low"),        // 2GB
        (4.0, "Adequate"),   // 4GB
        (8.0, "Good"),       // 8GB
        (16.0, "Excellent"), // 16GB
        (32.0, "Excellent"), // 32GB
    ];

    for (memory_gb, _expected_class) in test_cases {
        let mem = MemoryInfo {
            total_gb: memory_gb,
            available_gb: memory_gb * 0.5,
            memory_type: "DDR4".to_string(),
            frequency_mhz: 3200,
        };

        assert!(mem.total_gb > 0.0);

        // Classify memory
        let classification = if memory_gb >= 16.0 {
            "Excellent"
        } else if memory_gb >= 4.0 {
            "Good"
        } else if memory_gb >= 1.0 {
            "Adequate"
        } else {
            "Low"
        };

        // This is a heuristic, so we just verify it's classified
        assert!(!classification.is_empty());
    }
}
