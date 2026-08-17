// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage tests for under-covered GPU runtime modules (S155)
//!
//! Tests type construction, config builders, scheduler, coordinator, strategy,
//! memory pool, tracker, unified memory, distributed types, engine, and frameworks
//! without requiring real GPU hardware.

#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use toadstool::{RuntimeEngine, WorkloadType};
use toadstool_runtime_gpu::ParallelComputeFramework;
use toadstool_runtime_gpu::config::{
    AllocationStrategy, CachingConfig, CompilationConfig, DeviceSelectionStrategy, ExecutionConfig,
    GpuDiscoveryConfig, LoadBalancingAlgorithm, LoadBalancingConfig, MonitoringConfig,
    OptimizationLevel, RecursionConfig, RecursiveSchedulingStrategy, ResourceConfig,
    UniversalGpuConfig, UniversalIrFormat,
};
use toadstool_runtime_gpu::coordinator::ComputeResourceCoordinator;
use toadstool_runtime_gpu::distributed::{
    DistributedStats, JobStatus, JobTracker, PartitionStrategy, RemoteTowerEndpoint, TowerManager,
};
use toadstool_runtime_gpu::engine::UniversalGpuEngine;
use toadstool_runtime_gpu::frameworks::WebGpuFramework;
use toadstool_runtime_gpu::memory_pool::{MemoryPool, PoolStatistics};
use toadstool_runtime_gpu::scheduler::{SchedulingPolicy, UniversalComputeScheduler};
use toadstool_runtime_gpu::strategy::{BackendSelectionStrategy, EvolutionMetrics};
use toadstool_runtime_gpu::types::{
    AccessPattern, ComputeEngineStatistics, ComputeWorkload, DataType, DeviceCapabilities,
    DeviceId, DeviceInfo, DeviceRequirements, DeviceType, DeviceUsage, GpuFramework, KernelFormat,
    PerformanceCharacteristics, ResourceAllocation, ResourcePool, SessionStatus,
    UniversalComputeDevice,
};
use toadstool_runtime_gpu::unified_memory::{
    BackendStrategy, BackendType, BufferId, BufferIdGenerator, MemoryFlags, SyncState,
    UnifiedMemoryCapabilities, UnifiedMemoryConfig, UnifiedMemoryStats,
};
use toadstool_runtime_gpu::universal::UniversalComputeResource;

// ============================================================================
// Types - construction, Display/Debug, defaults
// ============================================================================

#[test]
fn types_device_id_construction() {
    let id = DeviceId::new(GpuFramework::WebGpu, 0, "test-uuid".to_string());
    assert_eq!(id.framework, GpuFramework::WebGpu);
    assert_eq!(id.device_index, 0);
    assert_eq!(id.uuid, "test-uuid");
}

#[test]
fn types_device_requirements_minimal() {
    let reqs = DeviceRequirements::minimal();
    assert!(reqs.min_memory_bytes.is_some());
    assert!(reqs.min_compute_units.is_some());
    assert!(
        reqs.required_data_types
            .iter()
            .any(|t| matches!(t, DataType::Float32))
    );
}

#[test]
fn types_device_requirements_high_performance() {
    let reqs = DeviceRequirements::high_performance();
    assert!(reqs.min_memory_bytes.unwrap() > 64 * 1024 * 1024);
    assert!(reqs.min_compute_capability.is_some());
}

#[test]
fn types_device_usage_default() {
    let usage = DeviceUsage::default();
    assert_eq!(usage.gpu_utilization_percent, 0.0);
    assert_eq!(usage.memory_used_bytes, 0);
}

#[test]
fn types_resource_allocation() {
    let alloc = ResourceAllocation {
        memory_bytes: 1024,
        compute_units: 2,
        priority: 1,
    };
    assert_eq!(alloc.memory_bytes, 1024);
}

#[test]
fn types_resource_pool() {
    let pool = ResourcePool {
        total_memory: 0,
        allocated_memory: 0,
        total_compute_units: 0,
        allocated_compute_units: 0,
        allocation_queue: vec![],
    };
    assert_eq!(pool.total_memory, 0);
}

#[test]
fn types_session_status_variants() {
    let _ = SessionStatus::Initializing;
    let _ = SessionStatus::Running;
    let _ = SessionStatus::Completed;
    let _ = SessionStatus::Failed("err".to_string());
}

#[test]
fn types_kernel_format_display() {
    let fmt = KernelFormat::Wgsl;
    let s = format!("{fmt:?}");
    assert!(!s.is_empty());
}

#[test]
fn types_access_pattern() {
    let _ = AccessPattern::ReadOnly;
    let _ = AccessPattern::WriteOnly;
    let _ = AccessPattern::ReadWrite;
}

#[test]
fn types_compute_engine_statistics() {
    let stats = ComputeEngineStatistics {
        total_devices: 0,
        active_sessions: 0,
        frameworks_available: 0,
        recursive_sessions: 0,
        max_recursion_depth: 0,
    };
    assert_eq!(stats.total_devices, 0);
}

// ============================================================================
// Config - builders, validation, defaults
// ============================================================================

#[test]
fn config_universal_gpu_config_default() {
    let config = UniversalGpuConfig::default();
    assert!(!config.discovery.enabled_frameworks.is_empty());
    assert!(config.resources.max_memory_usage_percent > 0.0);
}

#[test]
fn config_universal_gpu_config_serialization() {
    let config = UniversalGpuConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let _: UniversalGpuConfig = serde_json::from_str(&json).unwrap();
}

#[test]
fn config_gpu_discovery_config_default() {
    let config = GpuDiscoveryConfig::default();
    assert!(!config.enabled_frameworks.is_empty());
    assert!(config.auto_fallback);
}

#[test]
fn config_resource_config_default() {
    let config = ResourceConfig::default();
    assert_eq!(config.max_memory_usage_percent, 80.0);
}

#[test]
fn config_compilation_config_default() {
    let config = CompilationConfig::default();
    assert!(config.jit_enabled);
}

#[test]
fn config_execution_config_default() {
    let config = ExecutionConfig::default();
    assert!(config.retry_enabled);
}

#[test]
fn config_monitoring_config_default() {
    let config = MonitoringConfig::default();
    assert!(config.profiling_enabled);
}

#[test]
fn config_recursion_config_default() {
    let config = RecursionConfig::default();
    assert!(config.recursive_enabled);
    assert!(config.max_recursion_depth > 0);
}

#[test]
fn config_load_balancing_config_default() {
    let config = LoadBalancingConfig::default();
    assert!(config.enabled);
}

#[test]
fn config_caching_config_default() {
    let config = CachingConfig::default();
    assert!(config.enabled);
}

#[test]
fn config_allocation_strategy_variants() {
    let _ = AllocationStrategy::OnDemand;
    let _ = AllocationStrategy::Pooled;
    let _ = AllocationStrategy::Adaptive;
    let _ = AllocationStrategy::Unified;
}

#[test]
fn config_device_selection_strategy_variants() {
    let _ = DeviceSelectionStrategy::Optimal;
    let _ = DeviceSelectionStrategy::RoundRobin;
    let _ = DeviceSelectionStrategy::MaxMemory;
    let _ = DeviceSelectionStrategy::MaxCompute;
    let _ = DeviceSelectionStrategy::LoadBalance;
}

#[test]
fn config_device_selection_strategy_specific() {
    let id = DeviceId::new(GpuFramework::Cuda, 0, "cuda-0".to_string());
    let _ = DeviceSelectionStrategy::Specific(id);
}

#[test]
fn config_load_balancing_algorithm_variants() {
    let _ = LoadBalancingAlgorithm::RoundRobin;
    let _ = LoadBalancingAlgorithm::WeightedRoundRobin;
    let _ = LoadBalancingAlgorithm::LeastConnections;
}

#[test]
fn config_optimization_level_variants() {
    let _ = OptimizationLevel::None;
    let _ = OptimizationLevel::Basic;
    let _ = OptimizationLevel::Adaptive;
    let _ = OptimizationLevel::Aggressive;
}

#[test]
fn config_universal_ir_format_variants() {
    let _ = UniversalIrFormat::Spirv;
    let _ = UniversalIrFormat::Llvm;
    let _ = UniversalIrFormat::Wasm;
    let _ = UniversalIrFormat::Custom("custom".to_string());
}

#[test]
fn config_recursive_scheduling_strategy_variants() {
    let _ = RecursiveSchedulingStrategy::Cooperative;
    let _ = RecursiveSchedulingStrategy::Preemptive;
    let _ = RecursiveSchedulingStrategy::Isolated;
}

// ============================================================================
// Scheduler - without real GPU
// ============================================================================

#[tokio::test]
async fn scheduler_default_creation() {
    let scheduler = UniversalComputeScheduler::default();
    let list = scheduler.list_resources().await;
    assert!(list.is_empty());
}

#[tokio::test]
async fn scheduler_new_with_policy() {
    let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::Performance);
    let list = scheduler.list_resources().await;
    assert!(list.is_empty());
}

#[tokio::test]
async fn scheduler_select_resource_no_resources_fails() {
    let scheduler = UniversalComputeScheduler::default();
    let reqs = toadstool_runtime_gpu::universal::ComputeRequirements::default();
    let result = scheduler.select_resource(&reqs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn scheduler_with_cpu_resource() {
    let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::CapabilityMatch);
    let cpu = toadstool_runtime_gpu::cpu_resource::CpuComputeResource::new().expect("CPU");
    scheduler
        .register_resource(Arc::new(
            toadstool_runtime_gpu::compute_dispatch::UniversalComputeResourceDispatch::Cpu(cpu),
        ))
        .await;

    let list = scheduler.list_resources().await;
    assert!(!list.is_empty());
    assert!(list[0].contains("CPU"));
}

#[tokio::test]
async fn scheduler_select_resource_with_cpu() {
    let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::CapabilityMatch);
    let cpu = toadstool_runtime_gpu::cpu_resource::CpuComputeResource::new().expect("CPU");
    scheduler
        .register_resource(Arc::new(
            toadstool_runtime_gpu::compute_dispatch::UniversalComputeResourceDispatch::Cpu(cpu),
        ))
        .await;

    let reqs = toadstool_runtime_gpu::universal::ComputeRequirements::default();
    let result = scheduler.select_resource(&reqs).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn scheduler_record_performance() {
    let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::Performance);
    let cpu = toadstool_runtime_gpu::cpu_resource::CpuComputeResource::new().expect("CPU");
    let id = cpu.resource_id().to_string();
    scheduler
        .register_resource(Arc::new(
            toadstool_runtime_gpu::compute_dispatch::UniversalComputeResourceDispatch::Cpu(cpu),
        ))
        .await;

    let reqs = toadstool_runtime_gpu::universal::ComputeRequirements::default();
    scheduler
        .record_performance(&id, &reqs, Duration::from_millis(50))
        .await;
}

// ============================================================================
// Coordinator - without real GPU
// ============================================================================

#[tokio::test]
async fn coordinator_creation() {
    let config = ResourceConfig::default();
    let coordinator = ComputeResourceCoordinator::new(config);
    let _ = coordinator;
}

#[tokio::test]
async fn coordinator_select_device_empty_fails() {
    let coordinator = ComputeResourceCoordinator::new(ResourceConfig::default());
    let reqs = DeviceRequirements::minimal();
    let result = coordinator.select_device(&[], &reqs).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn coordinator_initialize_device_pool() {
    let coordinator = ComputeResourceCoordinator::new(ResourceConfig::default());
    let device = make_test_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
    let result = coordinator.initialize_device_pool(&device).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn coordinator_allocate_and_release() {
    let coordinator = ComputeResourceCoordinator::new(ResourceConfig::default());
    let device = make_test_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
    coordinator.initialize_device_pool(&device).await.unwrap();

    let alloc = coordinator
        .allocate_resources(&device.id, &DeviceRequirements::minimal())
        .await
        .unwrap();
    assert_eq!(alloc.memory_bytes, 64 * 1024 * 1024);

    let result = coordinator.release_resources(&device.id, &alloc).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn coordinator_get_pool_stats() {
    let coordinator = ComputeResourceCoordinator::new(ResourceConfig::default());
    let device = make_test_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
    coordinator.initialize_device_pool(&device).await.unwrap();

    let stats = coordinator.get_pool_stats(&device.id).await;
    assert!(stats.is_some());
    let s = stats.unwrap();
    assert_eq!(s.total_memory, 8 * 1024 * 1024 * 1024);
}

#[tokio::test]
async fn coordinator_select_device_with_devices() {
    let coordinator = ComputeResourceCoordinator::new(ResourceConfig::default());
    let device = make_test_device("gpu-0", 8 * 1024 * 1024 * 1024, 16);
    coordinator.initialize_device_pool(&device).await.unwrap();

    let result = coordinator
        .select_device(
            std::slice::from_ref(&device.id),
            &DeviceRequirements::minimal(),
        )
        .await;
    assert!(result.is_ok());
}

fn make_test_device(id: &str, total_memory: u64, compute_units: u32) -> UniversalComputeDevice {
    use std::sync::RwLock;
    UniversalComputeDevice {
        id: DeviceId::new(GpuFramework::Cuda, 0, id.to_string()),
        info: DeviceInfo {
            name: format!("Device {id}"),
            vendor: "Test".to_string(),
            device_type: DeviceType::DiscreteGpu,
            driver_version: "1.0".to_string(),
            architecture: "test".to_string(),
            physical_location: None,
        },
        capabilities: DeviceCapabilities {
            compute_capability: "7.0".to_string(),
            total_memory_bytes: total_memory,
            memory_bandwidth_gbps: 100.0,
            compute_units,
            max_work_group_size: (256, 256, 256),
            supported_data_types: vec![],
            extensions: HashMap::new(),
            performance: PerformanceCharacteristics {
                peak_gflops_fp32: 1000.0,
                peak_gflops_fp64: Some(500.0),
                peak_gflops_fp16: Some(2000.0),
                peak_memory_bandwidth_utilization: 0.8,
                typical_power_watts: 100.0,
                max_power_watts: 200.0,
            },
        },
        usage: Arc::new(RwLock::new(DeviceUsage::default())),
        framework_handle: None,
    }
}

// ============================================================================
// Strategy - backend selection logic
// ============================================================================

#[test]
fn strategy_backend_selection_default() {
    let strategy = BackendSelectionStrategy::default();
    assert!(matches!(strategy, BackendSelectionStrategy::Automatic));
}

#[test]
fn strategy_automatic_prefers_webgpu() {
    let strategy = BackendSelectionStrategy::Automatic;
    let available = vec![GpuFramework::WebGpu, GpuFramework::Cuda];
    let selected = strategy.select_framework(None, &available);
    assert_eq!(selected, Some(GpuFramework::WebGpu));
}

#[test]
fn strategy_sovereign_only_requires_webgpu() {
    let strategy = BackendSelectionStrategy::SovereignOnly;
    let available = vec![GpuFramework::Cuda];
    let selected = strategy.select_framework(None, &available);
    assert_eq!(selected, None);

    let available = vec![GpuFramework::WebGpu];
    let selected = strategy.select_framework(None, &available);
    assert_eq!(selected, Some(GpuFramework::WebGpu));
}

#[test]
fn strategy_pragmatic_prefers_cuda_for_python() {
    let strategy = BackendSelectionStrategy::Pragmatic;
    let available = vec![GpuFramework::WebGpu, GpuFramework::Cuda];
    let selected = strategy.select_framework(Some(&WorkloadType::Python), &available);
    assert_eq!(selected, Some(GpuFramework::Cuda));
}

#[test]
fn strategy_specific_framework() {
    let strategy = BackendSelectionStrategy::Specific(GpuFramework::Metal);
    let available = vec![GpuFramework::Metal, GpuFramework::WebGpu];
    let selected = strategy.select_framework(None, &available);
    assert_eq!(selected, Some(GpuFramework::Metal));
}

#[test]
fn strategy_evolution_metrics_default() {
    let metrics = EvolutionMetrics::default();
    assert!(!metrics.ready_to_drop_cuda());
}

#[test]
fn strategy_evolution_metrics_ready_to_drop() {
    let metrics = EvolutionMetrics {
        webgpu_ai_coverage: 0.9,
        webgpu_performance_ratio: 0.98,
        pytorch_webgpu_ready: true,
        tensorflow_webgpu_ready: false,
        burn_adoption_rate: 0.15,
        cuda_usage_percentage: 0.2,
        webgpu_usage_percentage: 0.8,
    };
    assert!(metrics.ready_to_drop_cuda());
}

// ============================================================================
// Memory pool
// ============================================================================

#[tokio::test]
async fn memory_pool_default() {
    let pool = MemoryPool::new();
    let stats = pool.statistics().await;
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.cache_hits, 0);
}

#[tokio::test]
async fn memory_pool_with_capacity() {
    let pool = MemoryPool::with_capacity(8);
    let _ = pool;
}

#[tokio::test]
async fn memory_pool_hit_rate_empty() {
    let pool = MemoryPool::new();
    let rate = pool.hit_rate().await;
    assert_eq!(rate, 0.0);
}

#[tokio::test]
async fn memory_pool_clear() {
    let pool = MemoryPool::new();
    pool.clear().await;
}

#[test]
fn memory_pool_statistics_default() {
    let stats = PoolStatistics::default();
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.cache_hits, 0);
}

// ============================================================================
// Unified memory types
// ============================================================================

#[test]
fn unified_memory_buffer_id() {
    let id = BufferId::new(42);
    assert_eq!(id.as_u64(), 42);
    assert!(format!("{id}").contains("Buffer"));
}

#[test]
fn unified_memory_buffer_id_generator() {
    let id_generator = BufferIdGenerator::new();
    let id1 = id_generator.next();
    let id2 = id_generator.next();
    assert_ne!(id1, id2);
}

#[test]
fn unified_memory_memory_flags() {
    let balanced = MemoryFlags::balanced();
    assert!(balanced.coherent);

    let cpu = MemoryFlags::cpu_optimized();
    assert!(cpu.prefer_cpu);

    let gpu = MemoryFlags::gpu_optimized();
    assert!(gpu.prefer_gpu);
}

#[test]
fn unified_memory_sync_state() {
    let state = SyncState::default();
    assert_eq!(state, SyncState::Synced);
}

#[test]
fn unified_memory_backend_type_display() {
    assert_eq!(BackendType::Cpu.to_string(), "CPU");
    assert_eq!(BackendType::WebGpu.to_string(), "WebGPU");
}

#[test]
fn unified_memory_backend_strategy_default() {
    let strategy = BackendStrategy::default();
    assert!(matches!(strategy, BackendStrategy::Automatic));
}

#[test]
fn unified_memory_config_default() {
    let config = UnifiedMemoryConfig::default();
    assert!(config.enable_metrics);
}

#[test]
fn unified_memory_stats() {
    let mut stats = UnifiedMemoryStats::new("CPU".to_string());
    stats.update_peak(1024);
    assert_eq!(stats.peak_allocated, 1024);
}

#[test]
fn unified_memory_capabilities() {
    let caps = UnifiedMemoryCapabilities {
        backend_type: BackendType::Cpu,
        max_allocation_size: 1024 * 1024 * 1024,
        zero_copy: true,
        coherent: true,
        cpu_fast_access: true,
        gpu_fast_access: false,
        alignment_requirement: 64,
    };
    assert!(caps.is_truly_unified());
}

// ============================================================================
// Unified memory manager (CPU backend - no GPU needed)
// ============================================================================

#[tokio::test]
async fn unified_memory_manager_cpu_init() {
    let result = toadstool_runtime_gpu::unified_memory::UniversalUnifiedMemory::with_strategy(
        BackendStrategy::Specific(BackendType::Cpu),
    )
    .await;
    assert!(result.is_ok());
    let memory = result.unwrap();
    assert_eq!(memory.backend_type(), BackendType::Cpu);
}

#[tokio::test]
async fn unified_memory_manager_allocate() {
    let memory = toadstool_runtime_gpu::unified_memory::UniversalUnifiedMemory::with_strategy(
        BackendStrategy::Specific(BackendType::Cpu),
    )
    .await
    .unwrap();

    let buffer = memory.allocate(4096).await.unwrap();
    assert_eq!(buffer.size(), 4096);
}

#[tokio::test]
async fn unified_memory_manager_zero_size_fails() {
    let memory = toadstool_runtime_gpu::unified_memory::UniversalUnifiedMemory::with_strategy(
        BackendStrategy::Specific(BackendType::Cpu),
    )
    .await
    .unwrap();

    let result = memory.allocate(0).await;
    assert!(result.is_err());
}

// ============================================================================
// Distributed - job tracker, tower manager
// ============================================================================

#[tokio::test]
async fn distributed_job_tracker_creation() {
    let tracker = JobTracker::new();
    let jobs = tracker.all_jobs().await;
    assert!(jobs.is_empty());
}

#[tokio::test]
async fn distributed_job_tracker_register_and_get() {
    use toadstool_runtime_gpu::distributed::DistributedJobState;
    use toadstool_runtime_gpu::universal::{
        ComputeRequirements, KernelLanguage, OptimizationHints, UniversalKernel, UniversalWorkload,
    };

    let tracker = JobTracker::new();
    let job = DistributedJobState {
        job_id: "job-1".to_string(),
        workload: UniversalWorkload {
            id: "w1".to_string(),
            requirements: ComputeRequirements::default(),
            kernel: UniversalKernel::Source {
                language: KernelLanguage::Wgsl,
                code: "fn main() {}".to_string(),
                entry_point: "main".to_string(),
            },
            inputs: vec![],
            output_size: 1024,
            hints: OptimizationHints::default(),
        },
        status: JobStatus::Pending,
        assigned_tower: None,
        result: None,
        created_at: Instant::now(),
        completed_at: None,
    };
    tracker.register_job(job).await;
    let retrieved = tracker.get_job("job-1").await;
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn distributed_tower_manager_creation() {
    let manager = TowerManager::new("local-tower".to_string());
    assert_eq!(manager.local_tower_id(), "local-tower");
    assert_eq!(manager.tower_count().await, 1);
}

#[tokio::test]
async fn distributed_tower_manager_register_tower() {
    let manager = TowerManager::new("local".to_string());
    let endpoint = RemoteTowerEndpoint {
        tower_id: "remote-1".to_string(),
        address: "http://192.168.1.10:8084".to_string(),
        gpu_capabilities: None,
        last_seen: Instant::now(),
        latency_ms: 10,
    };
    manager.register_tower(endpoint).await;
    assert_eq!(manager.tower_count().await, 2);
}

#[test]
fn distributed_stats_empty() {
    let stats = DistributedStats::empty();
    assert_eq!(stats.total_towers, 0);
    assert_eq!(stats.total_jobs, 0);
}

#[test]
fn distributed_job_status_variants() {
    let _ = JobStatus::Pending;
    let _ = JobStatus::Scheduled;
    let _ = JobStatus::Running;
    let _ = JobStatus::Completed;
    let _ = JobStatus::Failed;
}

#[test]
fn distributed_partition_strategy_variants() {
    let _ = PartitionStrategy::Single;
    let _ = PartitionStrategy::DataParallel { chunk_size: 1024 };
    let _ = PartitionStrategy::Redundant { replicas: 3 };
    let _ = PartitionStrategy::Pipeline {
        stages: vec!["s1".to_string()],
    };
}

// ============================================================================
// Engine - without real GPU
// ============================================================================

#[tokio::test]
async fn engine_default_creation() {
    let engine = UniversalGpuEngine::default();
    let stats = engine.get_statistics().await;
    assert_eq!(stats.total_devices, 0);
    assert_eq!(stats.active_sessions, 0);
}

#[test]
fn engine_get_capabilities() {
    let engine = UniversalGpuEngine::default();
    let caps = engine.get_capabilities();
    assert!(caps.supported_workloads.contains(&WorkloadType::Gpu));
}

#[test]
fn engine_supports_workload() {
    let engine = UniversalGpuEngine::default();
    assert!(engine.supports_workload(&WorkloadType::Gpu));
    assert!(!engine.supports_workload(&WorkloadType::Native));
}

#[tokio::test]
async fn engine_with_config_and_strategy() {
    let mut config = UniversalGpuConfig::default();
    config.discovery.enabled_frameworks = vec![GpuFramework::Metal];
    let strategy = BackendSelectionStrategy::default();
    let result = UniversalGpuEngine::with_config_and_strategy(config, strategy).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn engine_execute_workload_no_devices() {
    let engine = UniversalGpuEngine::default();
    let workload = ComputeWorkload {
        name: "test".to_string(),
        kernel_source: "void main() {}".to_string(),
        kernel_format: KernelFormat::OpenClC,
        inputs: vec![],
        requirements: DeviceRequirements::minimal(),
        parent_session: None,
        recursive_workloads: vec![],
        priority: 1,
    };
    let result = engine.execute_workload(workload).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn engine_shutdown_empty() {
    let mut engine = UniversalGpuEngine::default();
    let result: Result<(), _> = engine.shutdown().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn engine_evolution_metrics() {
    let engine = UniversalGpuEngine::default();
    let metrics = engine.get_evolution_metrics().await;
    assert!(metrics.webgpu_ai_coverage >= 0.0);
}

// ============================================================================
// Frameworks
// ============================================================================

#[test]
fn frameworks_webgpu_new() {
    let result = WebGpuFramework::new();
    assert!(result.is_ok());
}

#[test]
fn frameworks_webgpu_framework_type() {
    let framework = WebGpuFramework::new().unwrap();
    assert_eq!(framework.framework_type(), GpuFramework::WebGpu);
}
