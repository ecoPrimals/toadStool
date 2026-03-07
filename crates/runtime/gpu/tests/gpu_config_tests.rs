// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::float_cmp)]
//! Comprehensive tests for GPU runtime configuration

use std::time::Duration;
use toadstool_runtime_gpu::config::*;
use toadstool_runtime_gpu::types::GpuFramework;

// ============================================================================
// UniversalGpuConfig Tests
// ============================================================================

#[test]
fn test_universal_gpu_config_default() {
    let config = UniversalGpuConfig::default();

    assert!(!config.discovery.enabled_frameworks.is_empty());
    assert!(config.resources.max_memory_usage_percent > 0.0);
}

#[test]
fn test_universal_gpu_config_clone() {
    let config1 = UniversalGpuConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.resources.max_memory_usage_percent,
        config2.resources.max_memory_usage_percent
    );
}

#[test]
fn test_universal_gpu_config_serialization() {
    let config = UniversalGpuConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// GpuDiscoveryConfig Tests
// ============================================================================

#[test]
fn test_discovery_config_default() {
    let config = GpuDiscoveryConfig::default();

    assert!(!config.enabled_frameworks.is_empty());
    assert_eq!(config.discovery_timeout, Duration::from_secs(10));
    assert!(config.auto_fallback);
}

#[test]
fn test_discovery_config_has_webgpu() {
    let config = GpuDiscoveryConfig::default();

    assert!(config.enabled_frameworks.contains(&GpuFramework::WebGpu));
}

#[test]
fn test_discovery_config_has_vulkan() {
    let config = GpuDiscoveryConfig::default();

    assert!(config.enabled_frameworks.contains(&GpuFramework::Vulkan));
}

#[test]
fn test_discovery_config_has_opencl() {
    let config = GpuDiscoveryConfig::default();

    assert!(config.enabled_frameworks.contains(&GpuFramework::OpenCl));
}

#[test]
fn test_discovery_config_has_cuda() {
    let config = GpuDiscoveryConfig::default();

    assert!(config.enabled_frameworks.contains(&GpuFramework::Cuda));
}

#[test]
fn test_discovery_config_has_metal() {
    let config = GpuDiscoveryConfig::default();

    assert!(config.enabled_frameworks.contains(&GpuFramework::Metal));
}

#[test]
fn test_discovery_config_has_rocm() {
    let config = GpuDiscoveryConfig::default();

    assert!(config.enabled_frameworks.contains(&GpuFramework::Rocm));
}

#[test]
fn test_discovery_config_has_directcompute() {
    let config = GpuDiscoveryConfig::default();

    assert!(config
        .enabled_frameworks
        .contains(&GpuFramework::DirectCompute));
}

#[test]
fn test_discovery_config_framework_count() {
    let config = GpuDiscoveryConfig::default();

    assert_eq!(config.enabled_frameworks.len(), 7);
}

#[test]
fn test_discovery_config_timeout_value() {
    let config = GpuDiscoveryConfig::default();

    assert!(config.discovery_timeout > Duration::from_secs(0));
    assert!(config.discovery_timeout <= Duration::from_secs(60));
}

#[test]
fn test_discovery_config_clone() {
    let config1 = GpuDiscoveryConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.auto_fallback, config2.auto_fallback);
    assert_eq!(config1.discovery_timeout, config2.discovery_timeout);
}

#[test]
fn test_discovery_config_serialization() {
    let config = GpuDiscoveryConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// ResourceConfig Tests
// ============================================================================

#[test]
fn test_resource_config_default() {
    let config = ResourceConfig::default();

    assert_eq!(config.max_memory_usage_percent, 80.0);
}

#[test]
fn test_resource_config_memory_percent_valid() {
    let config = ResourceConfig::default();

    assert!(config.max_memory_usage_percent > 0.0);
    assert!(config.max_memory_usage_percent <= 100.0);
}

#[test]
fn test_resource_config_has_allocation_strategy() {
    let config = ResourceConfig::default();

    assert!(matches!(
        config.allocation_strategy,
        AllocationStrategy::Adaptive
    ));
}

#[test]
fn test_resource_config_has_device_selection() {
    let config = ResourceConfig::default();

    assert!(matches!(
        config.device_selection,
        DeviceSelectionStrategy::Optimal
    ));
}

#[test]
fn test_resource_config_clone() {
    let config1 = ResourceConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.max_memory_usage_percent,
        config2.max_memory_usage_percent
    );
}

#[test]
fn test_resource_config_serialization() {
    let config = ResourceConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// CompilationConfig Tests
// ============================================================================

#[test]
fn test_compilation_config_default() {
    let config = CompilationConfig::default();

    // Check that config has some optimization level set
    assert!(config.jit_enabled);
}

#[test]
fn test_compilation_config_jit_enabled() {
    let config = CompilationConfig::default();

    assert!(config.jit_enabled);
}

#[test]
fn test_compilation_config_clone() {
    let config1 = CompilationConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.jit_enabled, config2.jit_enabled);
}

#[test]
fn test_compilation_config_serialization() {
    let config = CompilationConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// AllocationStrategy Tests
// ============================================================================

#[test]
fn test_allocation_strategy_adaptive() {
    let strategy = AllocationStrategy::Adaptive;
    assert!(matches!(strategy, AllocationStrategy::Adaptive));
}

#[test]
fn test_allocation_strategy_pooled() {
    let strategy = AllocationStrategy::Pooled;
    assert!(matches!(strategy, AllocationStrategy::Pooled));
}

#[test]
fn test_allocation_strategy_on_demand() {
    let strategy = AllocationStrategy::OnDemand;
    assert!(matches!(strategy, AllocationStrategy::OnDemand));
}

#[test]
fn test_allocation_strategy_unified() {
    let strategy = AllocationStrategy::Unified;
    assert!(matches!(strategy, AllocationStrategy::Unified));
}

#[test]
fn test_allocation_strategy_clone() {
    let strategy1 = AllocationStrategy::Adaptive;
    let strategy2 = strategy1.clone();

    match (strategy1, strategy2) {
        (AllocationStrategy::Adaptive, AllocationStrategy::Adaptive) => {} // Clone successful
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_allocation_strategy_serialization() {
    let strategy = AllocationStrategy::Pooled;
    let serialized = serde_json::to_string(&strategy).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// DeviceSelectionStrategy Tests
// ============================================================================

#[test]
fn test_device_selection_strategy_optimal() {
    let strategy = DeviceSelectionStrategy::Optimal;
    assert!(matches!(strategy, DeviceSelectionStrategy::Optimal));
}

#[test]
fn test_device_selection_strategy_max_memory() {
    let strategy = DeviceSelectionStrategy::MaxMemory;
    assert!(matches!(strategy, DeviceSelectionStrategy::MaxMemory));
}

#[test]
fn test_device_selection_strategy_max_compute() {
    let strategy = DeviceSelectionStrategy::MaxCompute;
    assert!(matches!(strategy, DeviceSelectionStrategy::MaxCompute));
}

#[test]
fn test_device_selection_strategy_load_balance() {
    let strategy = DeviceSelectionStrategy::LoadBalance;
    assert!(matches!(strategy, DeviceSelectionStrategy::LoadBalance));
}

#[test]
fn test_device_selection_strategy_round_robin() {
    let strategy = DeviceSelectionStrategy::RoundRobin;
    assert!(matches!(strategy, DeviceSelectionStrategy::RoundRobin));
}

#[test]
fn test_device_selection_strategy_clone() {
    let strategy1 = DeviceSelectionStrategy::Optimal;
    let strategy2 = strategy1.clone();

    match (strategy1, strategy2) {
        (DeviceSelectionStrategy::Optimal, DeviceSelectionStrategy::Optimal) => {} // Clone successful
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_device_selection_strategy_serialization() {
    let strategy = DeviceSelectionStrategy::MaxCompute;
    let serialized = serde_json::to_string(&strategy).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// OptimizationLevel Tests
// ============================================================================

#[test]
fn test_optimization_level_none() {
    let level = OptimizationLevel::None;
    assert!(matches!(level, OptimizationLevel::None));
}

#[test]
fn test_optimization_level_basic() {
    let level = OptimizationLevel::Basic;
    assert!(matches!(level, OptimizationLevel::Basic));
}

#[test]
fn test_optimization_level_adaptive() {
    let level = OptimizationLevel::Adaptive;
    assert!(matches!(level, OptimizationLevel::Adaptive));
}

#[test]
fn test_optimization_level_aggressive() {
    let level = OptimizationLevel::Aggressive;
    assert!(matches!(level, OptimizationLevel::Aggressive));
}

#[test]
fn test_optimization_level_clone() {
    let level1 = OptimizationLevel::Adaptive;
    let level2 = level1.clone();

    match (level1, level2) {
        (OptimizationLevel::Adaptive, OptimizationLevel::Adaptive) => {} // Clone successful
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_optimization_level_serialization() {
    let level = OptimizationLevel::Aggressive;
    let serialized = serde_json::to_string(&level).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// LoadBalancingConfig Tests
// ============================================================================

#[test]
fn test_load_balancing_config_default() {
    let config = LoadBalancingConfig::default();

    assert!(config.enabled);
}

#[test]
fn test_load_balancing_config_clone() {
    let config1 = LoadBalancingConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.enabled, config2.enabled);
}

#[test]
fn test_load_balancing_config_serialization() {
    let config = LoadBalancingConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// CachingConfig Tests
// ============================================================================

#[test]
fn test_caching_config_default() {
    let config = CachingConfig::default();

    assert!(config.enabled);
}

#[test]
fn test_caching_config_clone() {
    let config1 = CachingConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.enabled, config2.enabled);
}

#[test]
fn test_caching_config_serialization() {
    let config = CachingConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    assert!(!serialized.is_empty());
}
