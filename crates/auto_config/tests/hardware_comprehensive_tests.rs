//! Comprehensive tests for hardware detection module
//!
//! These tests ensure complete coverage of the hardware detection system,
//! including all data structures, enums, and helper functions.

use toadstool_auto_config::hardware::*;

// ============================================================================
// Default Values Tests
// ============================================================================

#[test]
fn test_hardware_detector_default() {
    let _detector = HardwareDetector::default();
    let _new_detector = HardwareDetector::new();

    // Both constructors should work
    // Note: HardwareDetector doesn't implement Debug, so we can't compare directly
}

#[test]
fn test_system_capabilities_default() {
    let capabilities = SystemCapabilities::default();

    assert_eq!(capabilities.cpu_cores, 4.0);
    assert_eq!(capabilities.memory_gb, 8.0);
    assert_eq!(capabilities.storage_gb, 100.0);
    assert_eq!(capabilities.gpu_count, 0);
    assert!(capabilities.gpu_memory_gb.is_none());
    assert!(capabilities.gpu_info.is_empty());
}

#[test]
fn test_cpu_info_default() {
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

#[test]
fn test_cpu_features_default() {
    let features = CpuFeatures::default();

    assert!(!features.supports_avx);
    assert!(!features.supports_avx2);
    assert!(!features.supports_sse4_1);
    assert!(!features.supports_sse4_2);
    assert!(!features.supports_neon);
}

#[test]
fn test_memory_info_default() {
    let memory_info = MemoryInfo::default();

    assert_eq!(memory_info.total_gb, 8.0);
    assert_eq!(memory_info.available_gb, 6.0);
    assert_eq!(memory_info.memory_type, "DDR4");
    assert_eq!(memory_info.frequency_mhz, 2400);
}

#[test]
fn test_storage_info_default() {
    let storage_info = StorageInfo::default();

    assert_eq!(storage_info.total_gb, 100.0);
    assert_eq!(storage_info.available_gb, 80.0);
}

#[test]
fn test_network_info_default() {
    let network_info = NetworkInfo::default();

    assert!(network_info.interfaces.is_empty());
}

// ============================================================================
// Clone Tests
// ============================================================================

#[test]
fn test_system_capabilities_clone() {
    let original = SystemCapabilities::default();
    let cloned = original.clone();

    assert_eq!(original.cpu_cores, cloned.cpu_cores);
    assert_eq!(original.memory_gb, cloned.memory_gb);
    assert_eq!(original.gpu_count, cloned.gpu_count);
}

#[test]
fn test_cpu_info_clone() {
    let original = CpuInfo::default();
    let cloned = original.clone();

    assert_eq!(original.model_name, cloned.model_name);
    assert_eq!(original.physical_cores, cloned.physical_cores);
    assert_eq!(original.logical_cores, cloned.logical_cores);
}

#[test]
fn test_cpu_features_clone() {
    let original = CpuFeatures {
        supports_avx: true,
        supports_avx2: true,
        ..Default::default()
    };

    let cloned = original.clone();

    assert_eq!(original.supports_avx, cloned.supports_avx);
    assert_eq!(original.supports_avx2, cloned.supports_avx2);
}

#[test]
fn test_memory_info_clone() {
    let original = MemoryInfo::default();
    let cloned = original.clone();

    assert_eq!(original.total_gb, cloned.total_gb);
    assert_eq!(original.available_gb, cloned.available_gb);
    assert_eq!(original.memory_type, cloned.memory_type);
}

#[test]
fn test_storage_info_clone() {
    let original = StorageInfo::default();
    let cloned = original.clone();

    assert_eq!(original.total_gb, cloned.total_gb);
    assert_eq!(original.available_gb, cloned.available_gb);
}

#[test]
fn test_network_info_clone() {
    let original = NetworkInfo::default();
    let cloned = original.clone();

    assert_eq!(original.interfaces.len(), cloned.interfaces.len());
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_system_capabilities_serialization() {
    let capabilities = SystemCapabilities::default();

    let json = serde_json::to_string(&capabilities).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: SystemCapabilities = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(capabilities.cpu_cores, deserialized.cpu_cores);
}

#[test]
fn test_cpu_info_serialization() {
    let cpu_info = CpuInfo::default();

    let json = serde_json::to_string(&cpu_info).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: CpuInfo = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(cpu_info.model_name, deserialized.model_name);
}

#[test]
fn test_cpu_features_serialization() {
    let features = CpuFeatures::default();

    let json = serde_json::to_string(&features).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: CpuFeatures = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(features.supports_avx, deserialized.supports_avx);
}

#[test]
fn test_memory_info_serialization() {
    let memory_info = MemoryInfo::default();

    let json = serde_json::to_string(&memory_info).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: MemoryInfo = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(memory_info.total_gb, deserialized.total_gb);
}

#[test]
fn test_storage_info_serialization() {
    let storage_info = StorageInfo::default();

    let json = serde_json::to_string(&storage_info).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: StorageInfo = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(storage_info.total_gb, deserialized.total_gb);
}

#[test]
fn test_network_info_serialization() {
    let network_info = NetworkInfo::default();

    let json = serde_json::to_string(&network_info).expect("Should serialize");
    assert!(!json.is_empty());

    let _deserialized: NetworkInfo = serde_json::from_str(&json).expect("Should deserialize");
}

// ============================================================================
// Debug Tests
// ============================================================================

#[test]
fn test_system_capabilities_debug() {
    let capabilities = SystemCapabilities::default();
    let debug_str = format!("{:?}", capabilities);

    assert!(debug_str.contains("SystemCapabilities"));
    assert!(debug_str.contains("cpu_cores"));
    assert!(debug_str.contains("memory_gb"));
}

#[test]
fn test_cpu_info_debug() {
    let cpu_info = CpuInfo::default();
    let debug_str = format!("{:?}", cpu_info);

    assert!(debug_str.contains("CpuInfo"));
    assert!(debug_str.contains("model_name"));
}

#[test]
fn test_cpu_features_debug() {
    let features = CpuFeatures::default();
    let debug_str = format!("{:?}", features);

    assert!(debug_str.contains("CpuFeatures"));
    assert!(debug_str.contains("supports_avx"));
}

#[test]
fn test_memory_info_debug() {
    let memory_info = MemoryInfo::default();
    let debug_str = format!("{:?}", memory_info);

    assert!(debug_str.contains("MemoryInfo"));
    assert!(debug_str.contains("total_gb"));
}

#[test]
fn test_storage_info_debug() {
    let storage_info = StorageInfo::default();
    let debug_str = format!("{:?}", storage_info);

    assert!(debug_str.contains("StorageInfo"));
}

#[test]
fn test_network_info_debug() {
    let network_info = NetworkInfo::default();
    let debug_str = format!("{:?}", network_info);

    assert!(debug_str.contains("NetworkInfo"));
}

// ============================================================================
// Enum Tests
// ============================================================================

#[test]
fn test_storage_type_variants() {
    let _ = StorageType::HDD;
    let _ = StorageType::SSD;
    let _ = StorageType::NVME;
    let _ = StorageType::Unknown;
}

#[test]
fn test_storage_type_serialization() {
    let ssd = StorageType::SSD;
    let json = serde_json::to_string(&ssd).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: StorageType = serde_json::from_str(&json).expect("Should deserialize");
    let deserialized_str = format!("{:?}", deserialized);
    assert!(deserialized_str.contains("SSD"));
}

#[test]
fn test_network_interface_type_variants() {
    let _ = NetworkInterfaceType::Ethernet;
    let _ = NetworkInterfaceType::WiFi;
    let _ = NetworkInterfaceType::Loopback;
    let _ = NetworkInterfaceType::Unknown;
}

#[test]
fn test_network_interface_type_serialization() {
    let ethernet = NetworkInterfaceType::Ethernet;
    let json = serde_json::to_string(&ethernet).expect("Should serialize");
    assert!(!json.is_empty());
}

#[test]
fn test_performance_class_variants() {
    let _ = PerformanceClass::LowEnd;
    let _ = PerformanceClass::Budget;
    let _ = PerformanceClass::Mainstream;
    let _ = PerformanceClass::HighEnd;
}

#[test]
fn test_performance_class_serialization() {
    let high_end = PerformanceClass::HighEnd;
    let json = serde_json::to_string(&high_end).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: PerformanceClass = serde_json::from_str(&json).expect("Should deserialize");
    let deserialized_str = format!("{:?}", deserialized);
    assert!(deserialized_str.contains("HighEnd"));
}

// ============================================================================
// GPU Info Tests
// ============================================================================

#[test]
fn test_gpu_info_creation() {
    let gpu = GpuInfo {
        name: "NVIDIA RTX 4090".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 24.0,
        driver_version: "535.54".to_string(),
        compute_capability: "8.9".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    assert_eq!(gpu.name, "NVIDIA RTX 4090");
    assert_eq!(gpu.vendor, "NVIDIA");
    assert_eq!(gpu.memory_gb, 24.0);
    assert!(gpu.supports_cuda);
    assert!(gpu.supports_opencl);
}

#[test]
fn test_gpu_info_serialization() {
    let gpu = GpuInfo {
        name: "AMD Radeon RX 7900 XTX".to_string(),
        vendor: "AMD".to_string(),
        memory_gb: 24.0,
        driver_version: "23.10".to_string(),
        compute_capability: "RDNA3".to_string(),
        supports_cuda: false,
        supports_opencl: true,
    };

    let json = serde_json::to_string(&gpu).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: GpuInfo = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(gpu.name, deserialized.name);
    assert_eq!(gpu.vendor, deserialized.vendor);
}

#[test]
fn test_gpu_info_clone() {
    let original = GpuInfo {
        name: "Intel UHD Graphics".to_string(),
        vendor: "Intel".to_string(),
        memory_gb: 2.0,
        driver_version: "30.0.101.1191".to_string(),
        compute_capability: "Gen12".to_string(),
        supports_cuda: false,
        supports_opencl: true,
    };

    let cloned = original.clone();

    assert_eq!(original.name, cloned.name);
    assert_eq!(original.vendor, cloned.vendor);
    assert_eq!(original.memory_gb, cloned.memory_gb);
}

// ============================================================================
// Network Interface Tests
// ============================================================================

#[test]
fn test_network_interface_creation() {
    let interface = NetworkInterface {
        name: "eth0".to_string(),
        interface_type: NetworkInterfaceType::Ethernet,
        speed_mbps: 1000,
        is_wireless: false,
    };

    assert_eq!(interface.name, "eth0");
    assert_eq!(interface.speed_mbps, 1000);
    assert!(!interface.is_wireless);
}

#[test]
fn test_network_interface_wireless() {
    let interface = NetworkInterface {
        name: "wlan0".to_string(),
        interface_type: NetworkInterfaceType::WiFi,
        speed_mbps: 867,
        is_wireless: true,
    };

    assert_eq!(interface.name, "wlan0");
    assert_eq!(interface.speed_mbps, 867);
    assert!(interface.is_wireless);
}

#[test]
fn test_network_interface_serialization() {
    let interface = NetworkInterface {
        name: "lo".to_string(),
        interface_type: NetworkInterfaceType::Loopback,
        speed_mbps: 0,
        is_wireless: false,
    };

    let json = serde_json::to_string(&interface).expect("Should serialize");
    assert!(!json.is_empty());

    let deserialized: NetworkInterface = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(interface.name, deserialized.name);
}

// ============================================================================
// Performance Score Tests (via public API only)
// ============================================================================
// Note: Score calculation methods are private, so we test them indirectly
// through scan_system() which uses them internally.

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_hardware_detector_creation() {
    let _detector = HardwareDetector::new();
    // Constructor should work (no panic)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_scan_succeeds() {
    let mut detector = HardwareDetector::new();
    let result = detector.scan_system().await;

    assert!(result.is_ok(), "System scan should succeed");

    let capabilities = result.unwrap();
    assert!(capabilities.cpu_cores > 0.0, "Should detect CPU cores");
    assert!(capabilities.memory_gb > 0.0, "Should detect memory");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_scan_produces_valid_capabilities() {
    let mut detector = HardwareDetector::new();
    let capabilities = detector.scan_system().await.unwrap();

    // Validate CPU
    assert!(capabilities.cpu_cores >= 1.0, "Should have at least 1 core");
    assert!(
        !capabilities.cpu_info.model_name.is_empty(),
        "Should have CPU name"
    );

    // Validate memory
    assert!(
        capabilities.memory_gb >= 1.0,
        "Should have at least 1GB memory"
    );

    // Validate storage
    assert!(
        capabilities.storage_gb >= 1.0,
        "Should have at least 1GB storage"
    );

    // Validate performance class
    let _class = capabilities.performance_class;
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_empty_gpu_info_vec() {
    let capabilities = SystemCapabilities {
        gpu_info: vec![],
        gpu_count: 0,
        gpu_memory_gb: None,
        ..Default::default()
    };

    assert!(capabilities.gpu_info.is_empty());
    assert_eq!(capabilities.gpu_count, 0);
    assert!(capabilities.gpu_memory_gb.is_none());
}

#[test]
fn test_multiple_gpus() {
    let gpu1 = GpuInfo {
        name: "GPU 1".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 12.0,
        driver_version: "535.54".to_string(),
        compute_capability: "8.6".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    let gpu2 = GpuInfo {
        name: "GPU 2".to_string(),
        vendor: "NVIDIA".to_string(),
        memory_gb: 8.0,
        driver_version: "535.54".to_string(),
        compute_capability: "7.5".to_string(),
        supports_cuda: true,
        supports_opencl: true,
    };

    let capabilities = SystemCapabilities {
        gpu_info: vec![gpu1, gpu2],
        gpu_count: 2,
        gpu_memory_gb: Some(12.0),
        ..Default::default()
    };

    assert_eq!(capabilities.gpu_count, 2);
    assert_eq!(capabilities.gpu_memory_gb, Some(12.0));
}

#[test]
fn test_cpu_features_all_enabled() {
    let features = CpuFeatures {
        supports_avx: true,
        supports_avx2: true,
        supports_sse4_1: true,
        supports_sse4_2: true,
        supports_neon: true,
        ..Default::default()
    };

    assert!(features.supports_avx);
    assert!(features.supports_avx2);
    assert!(features.supports_sse4_1);
    assert!(features.supports_sse4_2);
    assert!(features.supports_neon);
}

#[test]
fn test_zero_available_storage() {
    let storage = StorageInfo {
        total_gb: 100.0,
        available_gb: 0.0,
        storage_type: StorageType::SSD,
    };

    assert_eq!(storage.available_gb, 0.0);
    assert!(storage.total_gb > storage.available_gb);
}

#[test]
fn test_very_high_memory() {
    let memory = MemoryInfo {
        total_gb: 128.0,
        available_gb: 100.0,
        memory_type: "DDR5".to_string(),
        frequency_mhz: 6000,
    };

    assert_eq!(memory.total_gb, 128.0);
    assert_eq!(memory.frequency_mhz, 6000);
}

// ============================================================================
// Performance Class Tests (via public API)
// ============================================================================
// Note: classify_performance is private, so we test performance classification
// indirectly through scan_system() which returns a performance_class field.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_scan_includes_performance_class() {
    let mut detector = HardwareDetector::new();
    let capabilities = detector.scan_system().await.unwrap();

    // Performance class should be set
    let class_str = format!("{:?}", capabilities.performance_class);
    assert!(
        class_str.contains("LowEnd")
            || class_str.contains("Budget")
            || class_str.contains("Mainstream")
            || class_str.contains("HighEnd"),
        "Performance class should be one of the valid variants"
    );
}
