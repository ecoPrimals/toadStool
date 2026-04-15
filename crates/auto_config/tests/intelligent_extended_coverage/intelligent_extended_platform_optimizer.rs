// SPDX-License-Identifier: AGPL-3.0-or-later
use toadstool_auto_config::hardware::{
    CpuInfo, MemoryInfo, NetworkInfo, StorageInfo, SystemCapabilities,
};
use toadstool_auto_config::intelligent::PlatformOptimizer;

#[test]
fn test_platform_optimizer_new() {
    let optimizer = PlatformOptimizer::new();
    let _ = optimizer;
}

#[test]
fn test_platform_optimizer_default() {
    let optimizer = PlatformOptimizer::default();
    let _ = optimizer;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_platform_optimizer_optimize_linux() {
    let optimizer = PlatformOptimizer::new();

    let hardware = SystemCapabilities {
        cpu_info: CpuInfo::default(),
        cpu_cores: 8.0,
        memory_info: MemoryInfo::default(),
        memory_gb: 16.0,
        gpu_info: vec![toadstool_auto_config::hardware::GpuInfo {
            name: "Test GPU".to_string(),
            vendor: "Test Vendor".to_string(),
            memory_gb: 8.0,
            driver_version: "1.0".to_string(),
            compute_capability: "8.0".to_string(),
            supports_cuda: true,
            supports_opencl: true,
        }],
        gpu_count: 1,
        gpu_memory_gb: Some(8.0),
        storage_info: StorageInfo::default(),
        storage_gb: 512.0,
        network_info: NetworkInfo::default(),
        performance_class: toadstool_auto_config::hardware::PerformanceClass::Mainstream,
    };

    let result = optimizer.optimize_for_platform(&hardware);

    assert!(result.is_ok());

    if let Ok(config) = result {
        assert!(!config.platform_name.is_empty());
        if config.platform_name == "linux" {
            assert!(config.supports_containers());
            assert!(config.supports_sandboxing());
            assert!(!config.optimizations.is_empty());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_platform_optimizer_high_core_optimization() {
    let optimizer = PlatformOptimizer::new();

    let hardware = SystemCapabilities {
        cpu_info: CpuInfo::default(),
        cpu_cores: 16.0,
        memory_info: MemoryInfo::default(),
        memory_gb: 16.0,
        gpu_info: Vec::new(),
        gpu_count: 0,
        gpu_memory_gb: None,
        storage_info: StorageInfo::default(),
        storage_gb: 512.0,
        network_info: NetworkInfo::default(),
        performance_class: toadstool_auto_config::hardware::PerformanceClass::Mainstream,
    };

    let result = optimizer.optimize_for_platform(&hardware);

    assert!(result.is_ok());

    if let Ok(config) = result {
        let has_parallel_compilation = config
            .optimizations
            .iter()
            .any(|opt| opt.optimization_type == "parallel_compilation");
        assert!(
            has_parallel_compilation,
            "High-core systems should get parallel compilation optimization"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_platform_optimizer_high_memory_optimization() {
    let optimizer = PlatformOptimizer::new();

    let hardware = SystemCapabilities {
        cpu_info: CpuInfo::default(),
        cpu_cores: 8.0,
        memory_info: MemoryInfo::default(),
        memory_gb: 32.0,
        gpu_info: Vec::new(),
        gpu_count: 0,
        gpu_memory_gb: None,
        storage_info: StorageInfo::default(),
        storage_gb: 512.0,
        network_info: NetworkInfo::default(),
        performance_class: toadstool_auto_config::hardware::PerformanceClass::Mainstream,
    };

    let result = optimizer.optimize_for_platform(&hardware);

    assert!(result.is_ok());

    if let Ok(config) = result {
        let has_large_buffer = config
            .optimizations
            .iter()
            .any(|opt| opt.optimization_type == "large_buffer");
        assert!(
            has_large_buffer,
            "High-memory systems should get large buffer optimization"
        );
    }
}
