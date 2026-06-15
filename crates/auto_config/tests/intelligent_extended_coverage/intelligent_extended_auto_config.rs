// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::SystemTime;

use toadstool_auto_config::ecosystem::{DiscoveredServices, DiscoverySummary};
use toadstool_auto_config::hardware::{
    CpuInfo, MemoryInfo, NetworkInfo, StorageInfo, SystemCapabilities,
};
use toadstool_auto_config::intelligent::{
    IntelligentAutoConfig, PlatformConfig, PlatformOptimization, UsageHints,
};

#[test]
fn test_intelligent_autoconfig_new() {
    let config = IntelligentAutoConfig::new();
    let _ = config;
}

#[test]
fn test_intelligent_autoconfig_default() {
    let config = IntelligentAutoConfig::default();
    let _ = config;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_autoconfig_scan_system() {
    let mut config = IntelligentAutoConfig::new();

    let result = config.scan_system().await;

    assert!(result.is_ok());

    if let Ok(caps) = result {
        assert!(caps.cpu_cores > 0.0);
        assert!(caps.memory_gb > 0.0);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_autoconfig_generate_optimal_config_high_end() {
    let mut config = IntelligentAutoConfig::new();

    let hardware = SystemCapabilities {
        cpu_info: CpuInfo::default(),
        cpu_cores: 16.0,
        memory_info: MemoryInfo::default(),
        memory_gb: 32.0,
        gpu_info: vec![
            toadstool_auto_config::hardware::GpuInfo {
                name: "Test GPU 1".to_string(),
                vendor: "Test Vendor".to_string(),
                memory_gb: 16.0,
                driver_version: "1.0".to_string(),
                compute_capability: "8.0".to_string(),
                supports_cuda: true,
            },
            toadstool_auto_config::hardware::GpuInfo {
                name: "Test GPU 2".to_string(),
                vendor: "Test Vendor".to_string(),
                memory_gb: 16.0,
                driver_version: "1.0".to_string(),
                compute_capability: "8.0".to_string(),
                supports_cuda: true,
            },
        ],
        gpu_count: 2,
        gpu_memory_gb: Some(16.0),
        storage_info: StorageInfo::default(),
        storage_gb: 1000.0,
        network_info: NetworkInfo::default(),
        performance_class: toadstool_auto_config::hardware::PerformanceClass::Mainstream,
    };

    let platform = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: vec![PlatformOptimization {
            optimization_type: "containers".to_string(),
            description: "Container support".to_string(),
            performance_gain: 0.2,
        }],
    };

    let ecosystem = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary {
            total_services_found: 0,
            discovery_methods_used: Vec::new(),
            services_by_type: HashMap::new(),
            discovery_errors: Vec::new(),
        },
        discovery_timestamp: SystemTime::now(),
    };

    let usage_hints = UsageHints {
        predicted_workload_types: vec!["development".to_string()],
        expected_cpu_usage: 0.5,
        expected_memory_usage: 0.6,
        prefers_gpu: true,
        prefers_containers: true,
    };

    let result = config
        .config_generator
        .generate_optimal_config(&hardware, &platform, &ecosystem, &usage_hints)
        .await;

    assert!(result.is_ok());

    if let Ok(generated_config) = result {
        assert!(
            generated_config.runtime.max_concurrent_executions >= 8,
            "High-end system should get at least 8 concurrent executions"
        );

        assert!(
            generated_config.runtime.gpu.is_some(),
            "High-end system with GPUs should have GPU config"
        );

        assert!(
            generated_config.runtime.resource_limits.max_cpu_usage > 10.0,
            "High-end system should have high CPU limits"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_autoconfig_generate_optimal_config_low_end() {
    let mut config = IntelligentAutoConfig::new();

    let hardware = SystemCapabilities {
        cpu_info: CpuInfo::default(),
        cpu_cores: 2.0,
        memory_info: MemoryInfo::default(),
        memory_gb: 4.0,
        gpu_info: Vec::new(),
        gpu_count: 0,
        gpu_memory_gb: None,
        storage_info: StorageInfo::default(),
        storage_gb: 128.0,
        network_info: NetworkInfo::default(),
        performance_class: toadstool_auto_config::hardware::PerformanceClass::Mainstream,
    };

    let platform = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: Vec::new(),
    };

    let ecosystem = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary {
            total_services_found: 0,
            discovery_methods_used: Vec::new(),
            services_by_type: HashMap::new(),
            discovery_errors: Vec::new(),
        },
        discovery_timestamp: SystemTime::now(),
    };

    let usage_hints = UsageHints::default();

    let result = config
        .config_generator
        .generate_optimal_config(&hardware, &platform, &ecosystem, &usage_hints)
        .await;

    assert!(result.is_ok());

    if let Ok(generated_config) = result {
        assert!(
            generated_config.runtime.max_concurrent_executions <= 4,
            "Low-end system should get conservative concurrent executions"
        );

        assert!(
            generated_config.runtime.gpu.is_none(),
            "Low-end system without GPUs should not have GPU config"
        );

        assert!(
            generated_config.runtime.resource_limits.max_cpu_usage <= 2.0,
            "Low-end system should have conservative CPU limits"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_config_validation_valid() {
    let mut config = IntelligentAutoConfig::new();

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
        }],
        gpu_count: 1,
        gpu_memory_gb: Some(8.0),
        storage_info: StorageInfo::default(),
        storage_gb: 512.0,
        network_info: NetworkInfo::default(),
        performance_class: toadstool_auto_config::hardware::PerformanceClass::Mainstream,
    };

    let platform = PlatformConfig {
        platform_name: "linux".to_string(),
        supported_features: HashSet::new(),
        optimizations: Vec::new(),
    };

    let ecosystem = DiscoveredServices {
        discovered_services: HashMap::new(),
        discovery_summary: DiscoverySummary {
            total_services_found: 0,
            discovery_methods_used: Vec::new(),
            services_by_type: HashMap::new(),
            discovery_errors: Vec::new(),
        },
        discovery_timestamp: SystemTime::now(),
    };

    let usage_hints = UsageHints::default();

    let result = config
        .config_generator
        .generate_optimal_config(&hardware, &platform, &ecosystem, &usage_hints)
        .await;

    assert!(result.is_ok());
}
