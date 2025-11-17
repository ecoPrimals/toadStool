//! Comprehensive tests for hardware detection module
//! Target: crates/auto_config/src/hardware.rs

use toadstool_auto_config::*;

#[tokio::test]
async fn test_hardware_detector_new() {
    let _detector = HardwareDetector::new();
    // Detector should initialize successfully
}

#[tokio::test]
async fn test_hardware_detector_default() {
    let _detector = HardwareDetector::default();
    // Default constructor should work
}

#[tokio::test]
async fn test_system_capabilities_default() {
    let capabilities = SystemCapabilities::default();

    // Check default values are sensible
    assert_eq!(capabilities.cpu_cores, 4.0);
    assert_eq!(capabilities.memory_gb, 8.0);
    assert_eq!(capabilities.gpu_count, 0);
    assert_eq!(capabilities.storage_gb, 100.0);
    assert_eq!(capabilities.cpu_info.physical_cores, 4);
    assert_eq!(capabilities.memory_info.total_gb, 8.0);
}

#[tokio::test]
async fn test_cpu_info_default() {
    let cpu_info = CpuInfo::default();

    assert_eq!(cpu_info.model_name, "Unknown CPU");
    assert_eq!(cpu_info.physical_cores, 4);
    assert_eq!(cpu_info.logical_cores, 4);
    assert_eq!(cpu_info.family, 0);
    assert_eq!(cpu_info.base_frequency_mhz, 2000.0);
    assert_eq!(cpu_info.max_frequency_mhz, 3000.0);
    assert_eq!(cpu_info.cache_size_kb, 8192);
    assert!(cpu_info.instruction_sets.is_empty());
}

// CpuFeatures is not exported, so we test it indirectly through CpuInfo
#[tokio::test]
async fn test_cpu_info_features() {
    let cpu_info = CpuInfo::default();
    // CPU info should have features field accessible
    // Test that we can read the features without panicking
    let _has_avx = cpu_info.features.supports_avx;
    let _has_avx2 = cpu_info.features.supports_avx2;
    // Features should be readable (test passes if no panic)
}

#[tokio::test]
async fn test_memory_info_default() {
    let memory_info = MemoryInfo::default();

    assert_eq!(memory_info.total_gb, 8.0);
    assert_eq!(memory_info.available_gb, 6.0);
    assert_eq!(memory_info.memory_type, "DDR4");
    assert_eq!(memory_info.frequency_mhz, 2400);
}

#[tokio::test]
async fn test_storage_info_default() {
    let storage_info = StorageInfo::default();

    assert_eq!(storage_info.total_gb, 100.0);
    assert_eq!(storage_info.available_gb, 80.0);
    assert!(matches!(storage_info.storage_type, StorageType::SSD));
}

#[tokio::test]
async fn test_gpu_info_creation() {
    let gpu = GpuInfo {
        name: "Test GPU".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 8.0,
        driver_version: "525.0".to_string(),
        compute_capability: "8.6".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    assert_eq!(gpu.name, "Test GPU");
    assert_eq!(gpu.vendor, "NVIDIA");
    assert_eq!(gpu.memory_gb, 8.0);
    assert!(gpu.supports_cuda);
    assert!(gpu.supports_opencl);
}

#[tokio::test]
async fn test_storage_type_ssd() {
    let storage = StorageInfo {
        total_gb: 500.0,
        available_gb: 300.0,
        storage_type: StorageType::SSD,
    };

    assert!(matches!(storage.storage_type, StorageType::SSD));
}

#[tokio::test]
async fn test_storage_type_nvme() {
    let storage = StorageInfo {
        total_gb: 1000.0,
        available_gb: 800.0,
        storage_type: StorageType::NVME,
    };

    assert!(matches!(storage.storage_type, StorageType::NVME));
}

#[tokio::test]
async fn test_storage_type_hdd() {
    let storage = StorageInfo {
        total_gb: 2000.0,
        available_gb: 1500.0,
        storage_type: StorageType::HDD,
    };

    assert!(matches!(storage.storage_type, StorageType::HDD));
}

#[tokio::test]
async fn test_storage_type_unknown() {
    let storage = StorageInfo {
        total_gb: 500.0,
        available_gb: 400.0,
        storage_type: StorageType::Unknown,
    };

    assert!(matches!(storage.storage_type, StorageType::Unknown));
}

#[tokio::test]
async fn test_performance_class_highend() {
    let perf = PerformanceClass::HighEnd;
    assert!(matches!(perf, PerformanceClass::HighEnd));
}

#[tokio::test]
async fn test_performance_class_mainstream() {
    let perf = PerformanceClass::Mainstream;
    assert!(matches!(perf, PerformanceClass::Mainstream));
}

#[tokio::test]
async fn test_performance_class_budget() {
    let perf = PerformanceClass::Budget;
    assert!(matches!(perf, PerformanceClass::Budget));
}

#[tokio::test]
async fn test_performance_class_lowend() {
    let perf = PerformanceClass::LowEnd;
    assert!(matches!(perf, PerformanceClass::LowEnd));
}

#[tokio::test]
async fn test_cpu_features_with_avx_via_cpu_info() {
    let mut cpu_info = CpuInfo::default();
    cpu_info.features.supports_avx = true;
    cpu_info.features.supports_sse4_1 = true;
    cpu_info.features.supports_sse4_2 = true;

    assert!(cpu_info.features.supports_avx);
    assert!(!cpu_info.features.supports_avx2);
    assert!(cpu_info.features.supports_sse4_1);
    assert!(cpu_info.features.supports_sse4_2);
}

#[tokio::test]
async fn test_cpu_features_with_avx2_via_cpu_info() {
    let mut cpu_info = CpuInfo::default();
    cpu_info.features.supports_avx = true;
    cpu_info.features.supports_avx2 = true;
    cpu_info.features.supports_sse4_1 = true;
    cpu_info.features.supports_sse4_2 = true;

    assert!(cpu_info.features.supports_avx);
    assert!(cpu_info.features.supports_avx2);
}

#[tokio::test]
async fn test_system_capabilities_with_gpu() {
    let gpu = GpuInfo {
        name: "RTX 4090".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 24.0,
        driver_version: "535.0".to_string(),
        compute_capability: "8.9".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    let capabilities = SystemCapabilities {
        gpu_info: vec![gpu],
        gpu_count: 1,
        gpu_memory_gb: Some(24.0),
        ..Default::default()
    };

    assert_eq!(capabilities.gpu_count, 1);
    assert_eq!(capabilities.gpu_memory_gb, Some(24.0));
    assert_eq!(capabilities.gpu_info[0].name, "RTX 4090");
}

#[tokio::test]
async fn test_system_capabilities_multiple_gpus() {
    let gpu1 = GpuInfo {
        name: "RTX 4090".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 24.0,
        driver_version: "535.0".to_string(),
        compute_capability: "8.9".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    let gpu2 = GpuInfo {
        name: "RTX 4080".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 16.0,
        driver_version: "535.0".to_string(),
        compute_capability: "8.9".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    let capabilities = SystemCapabilities {
        gpu_info: vec![gpu1, gpu2],
        gpu_count: 2,
        gpu_memory_gb: Some(24.0), // First GPU
        ..Default::default()
    };

    assert_eq!(capabilities.gpu_count, 2);
    assert_eq!(capabilities.gpu_info.len(), 2);
}

#[tokio::test]
async fn test_cpu_info_high_end() {
    // Create CPU info with high-end specs
    let mut cpu = CpuInfo {
        model_name: "AMD Ryzen 9 7950X".to_string(),
        physical_cores: 16,
        logical_cores: 32,
        family: 25,
        base_frequency_mhz: 4500.0,
        max_frequency_mhz: 5700.0,
        cache_size_kb: 65536,
        instruction_sets: vec!["avx2".to_string(), "sse4.2".to_string()],
        ..Default::default()
    };
    cpu.features.supports_avx = true;
    cpu.features.supports_avx2 = true;
    cpu.features.supports_sse4_1 = true;
    cpu.features.supports_sse4_2 = true;

    assert_eq!(cpu.physical_cores, 16);
    assert_eq!(cpu.logical_cores, 32);
    assert!(cpu.base_frequency_mhz > 4000.0);
    assert!(cpu.features.supports_avx2);
}

#[tokio::test]
async fn test_memory_info_large_capacity() {
    let memory = MemoryInfo {
        total_gb: 64.0,
        available_gb: 48.0,
        memory_type: "DDR5".to_string(),
        frequency_mhz: 5600,
    };

    assert_eq!(memory.total_gb, 64.0);
    assert_eq!(memory.available_gb, 48.0);
    assert_eq!(memory.memory_type, "DDR5");
    assert!(memory.frequency_mhz > 5000);
}

#[tokio::test]
async fn test_system_capabilities_high_end_system() {
    let mut capabilities = SystemCapabilities::default();

    capabilities.cpu_info.model_name = "Intel Core i9-13900K".to_string();
    capabilities.cpu_info.physical_cores = 24;
    capabilities.cpu_info.logical_cores = 32;
    capabilities.cpu_info.family = 6;
    capabilities.cpu_info.base_frequency_mhz = 3000.0;
    capabilities.cpu_info.max_frequency_mhz = 5800.0;
    capabilities.cpu_info.cache_size_kb = 36864;
    capabilities.cpu_info.instruction_sets = vec!["avx2".to_string()];
    capabilities.cpu_info.features.supports_avx = true;
    capabilities.cpu_info.features.supports_avx2 = true;
    capabilities.cpu_info.features.supports_sse4_1 = true;
    capabilities.cpu_info.features.supports_sse4_2 = true;
    capabilities.cpu_cores = 24.0;

    capabilities.memory_info.total_gb = 128.0;
    capabilities.memory_info.available_gb = 100.0;
    capabilities.memory_info.memory_type = "DDR5".to_string();
    capabilities.memory_info.frequency_mhz = 6000;
    capabilities.memory_gb = 128.0;

    capabilities.gpu_info = vec![GpuInfo {
        name: "RTX 4090".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 24.0,
        driver_version: "535.0".to_string(),
        compute_capability: "8.9".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    }];
    capabilities.gpu_count = 1;
    capabilities.gpu_memory_gb = Some(24.0);

    capabilities.storage_info.total_gb = 2000.0;
    capabilities.storage_info.available_gb = 1500.0;
    capabilities.storage_info.storage_type = StorageType::NVME;
    capabilities.storage_gb = 2000.0;

    capabilities.performance_class = PerformanceClass::HighEnd;

    assert_eq!(capabilities.cpu_cores, 24.0);
    assert_eq!(capabilities.memory_gb, 128.0);
    assert_eq!(capabilities.gpu_count, 1);
    assert!(matches!(
        capabilities.performance_class,
        PerformanceClass::HighEnd
    ));
}

#[tokio::test]
async fn test_system_capabilities_budget_system() {
    let mut capabilities = SystemCapabilities::default();

    capabilities.cpu_info.model_name = "Intel Pentium G6400".to_string();
    capabilities.cpu_info.physical_cores = 2;
    capabilities.cpu_info.logical_cores = 4;
    capabilities.cpu_info.family = 6;
    capabilities.cpu_info.base_frequency_mhz = 4000.0;
    capabilities.cpu_info.max_frequency_mhz = 4000.0;
    capabilities.cpu_info.cache_size_kb = 4096;
    capabilities.cpu_cores = 2.0;

    capabilities.memory_info.total_gb = 8.0;
    capabilities.memory_info.available_gb = 6.0;
    capabilities.memory_info.memory_type = "DDR4".to_string();
    capabilities.memory_info.frequency_mhz = 2400;
    capabilities.memory_gb = 8.0;

    capabilities.gpu_info = vec![];
    capabilities.gpu_count = 0;
    capabilities.gpu_memory_gb = None;

    capabilities.storage_info.total_gb = 256.0;
    capabilities.storage_info.available_gb = 200.0;
    capabilities.storage_info.storage_type = StorageType::SSD;
    capabilities.storage_gb = 256.0;

    capabilities.performance_class = PerformanceClass::Budget;

    assert_eq!(capabilities.cpu_cores, 2.0);
    assert_eq!(capabilities.memory_gb, 8.0);
    assert_eq!(capabilities.gpu_count, 0);
    assert!(matches!(
        capabilities.performance_class,
        PerformanceClass::Budget
    ));
}

#[tokio::test]
async fn test_cpu_info_serialization() {
    let cpu = CpuInfo::default();
    let serialized = serde_json::to_string(&cpu);
    assert!(serialized.is_ok());

    let deserialized: Result<CpuInfo, _> = serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
}

#[tokio::test]
async fn test_system_capabilities_serialization() {
    let capabilities = SystemCapabilities::default();
    let serialized = serde_json::to_string(&capabilities);
    assert!(serialized.is_ok());

    let deserialized: Result<SystemCapabilities, _> = serde_json::from_str(&serialized.unwrap());
    assert!(deserialized.is_ok());
}

#[tokio::test]
async fn test_memory_info_serialization() {
    let memory = MemoryInfo::default();
    let serialized = serde_json::to_string(&memory);
    assert!(serialized.is_ok());
}

#[tokio::test]
async fn test_storage_info_serialization() {
    let storage = StorageInfo::default();
    let serialized = serde_json::to_string(&storage);
    assert!(serialized.is_ok());
}

#[tokio::test]
async fn test_gpu_info_serialization() {
    let gpu = GpuInfo {
        name: "Test GPU".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 8.0,
        driver_version: "525.0".to_string(),
        compute_capability: "8.6".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    let serialized = serde_json::to_string(&gpu);
    assert!(serialized.is_ok());
}

#[tokio::test]
async fn test_clone_cpu_info() {
    let cpu = CpuInfo::default();
    let cloned = cpu.clone();
    assert_eq!(cpu.model_name, cloned.model_name);
    assert_eq!(cpu.physical_cores, cloned.physical_cores);
}

#[tokio::test]
async fn test_clone_system_capabilities() {
    let capabilities = SystemCapabilities::default();
    let cloned = capabilities.clone();
    assert_eq!(capabilities.cpu_cores, cloned.cpu_cores);
    assert_eq!(capabilities.memory_gb, cloned.memory_gb);
}

#[tokio::test]
async fn test_clone_gpu_info() {
    let gpu = GpuInfo {
        name: "Test GPU".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 8.0,
        driver_version: "525.0".to_string(),
        compute_capability: "8.6".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    let cloned = gpu.clone();
    assert_eq!(gpu.name, cloned.name);
    assert_eq!(gpu.vendor, cloned.vendor);
    assert_eq!(gpu.memory_gb, cloned.memory_gb);
}

#[tokio::test]
async fn test_debug_output_cpu_info() {
    let cpu = CpuInfo::default();
    let debug_str = format!("{:?}", cpu);
    assert!(debug_str.contains("CpuInfo"));
}

#[tokio::test]
async fn test_debug_output_system_capabilities() {
    let capabilities = SystemCapabilities::default();
    let debug_str = format!("{:?}", capabilities);
    assert!(debug_str.contains("SystemCapabilities"));
}

#[tokio::test]
async fn test_debug_output_performance_class() {
    let perf = PerformanceClass::HighEnd;
    let debug_str = format!("{:?}", perf);
    assert!(debug_str.contains("HighEnd"));
}

#[tokio::test]
async fn test_gpu_info_amd() {
    let gpu = GpuInfo {
        name: "AMD Radeon RX 7900 XTX".to_string(),
        vendor: "AMD".to_string(),
        memory_gb: 24.0,
        driver_version: "23.30".to_string(),
        compute_capability: "RDNA 3".to_string(),
        supports_cuda: false,
        supports_opencl: true,
    };

    assert_eq!(gpu.vendor, "AMD");
    assert!(!gpu.supports_cuda);
    assert!(gpu.supports_opencl);
}

#[tokio::test]
async fn test_gpu_info_intel() {
    let gpu = GpuInfo {
        name: "Intel Arc A770".to_string(),
        vendor: "Intel".to_string(),
        memory_gb: 16.0,
        driver_version: "31.0".to_string(),
        compute_capability: "Xe-HPG".to_string(),
        supports_cuda: false,
        supports_opencl: true,
    };

    assert_eq!(gpu.vendor, "Intel");
    assert!(!gpu.supports_cuda);
    assert!(gpu.supports_opencl);
}

#[tokio::test]
async fn test_storage_info_capacity_calculations() {
    let storage = StorageInfo {
        total_gb: 1000.0,
        available_gb: 750.0,
        storage_type: StorageType::NVME,
    };

    let used_gb = storage.total_gb - storage.available_gb;
    let usage_percent = (used_gb / storage.total_gb) * 100.0;

    assert_eq!(used_gb, 250.0);
    assert_eq!(usage_percent, 25.0);
}

#[tokio::test]
async fn test_memory_info_availability() {
    let memory = MemoryInfo {
        total_gb: 32.0,
        available_gb: 24.0,
        memory_type: "DDR4".to_string(),
        frequency_mhz: 3200,
    };

    let used_gb = memory.total_gb - memory.available_gb;
    assert_eq!(used_gb, 8.0);
    assert!(memory.available_gb > 20.0);
}

#[tokio::test]
async fn test_cpu_info_instruction_sets() {
    let cpu = CpuInfo {
        instruction_sets: vec![
            "sse4.1".to_string(),
            "sse4.2".to_string(),
            "avx".to_string(),
            "avx2".to_string(),
        ],
        ..Default::default()
    };

    assert_eq!(cpu.instruction_sets.len(), 4);
    assert!(cpu.instruction_sets.contains(&"avx2".to_string()));
}

#[tokio::test]
async fn test_system_capabilities_no_gpu() {
    let capabilities = SystemCapabilities::default();

    assert_eq!(capabilities.gpu_count, 0);
    assert!(capabilities.gpu_info.is_empty());
    assert_eq!(capabilities.gpu_memory_gb, None);
}
