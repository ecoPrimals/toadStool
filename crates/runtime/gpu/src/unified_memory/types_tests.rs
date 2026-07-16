// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from types.rs (S335).

use super::types::*;

#[test]
fn test_buffer_id_generator() {
    let id_generator = BufferIdGenerator::new();
    let id1 = id_generator.next();
    let id2 = id_generator.next();
    let id3 = id_generator.next();

    assert_ne!(id1, id2);
    assert_ne!(id2, id3);
    assert_eq!(id1.as_u64(), 1);
    assert_eq!(id2.as_u64(), 2);
    assert_eq!(id3.as_u64(), 3);
}

#[test]
fn test_memory_flags() {
    let balanced = MemoryFlags::balanced();
    assert!(!balanced.prefer_cpu);
    assert!(!balanced.prefer_gpu);
    assert!(balanced.coherent);

    let cpu = MemoryFlags::cpu_optimized();
    assert!(cpu.prefer_cpu);
    assert!(!cpu.prefer_gpu);
    assert!(cpu.cached);

    let gpu = MemoryFlags::gpu_optimized();
    assert!(!gpu.prefer_cpu);
    assert!(gpu.prefer_gpu);
    assert!(!gpu.coherent);
}

#[test]
fn test_sync_state() {
    let state = SyncState::default();
    assert_eq!(state, SyncState::Synced);

    let modified = SyncState::CpuModified;
    assert_ne!(modified, SyncState::Synced);
}

#[test]
fn test_backend_type_display() {
    assert_eq!(BackendType::Vulkan.to_string(), "Vulkan");
    assert_eq!(BackendType::OpenCL.to_string(), "OpenCL");
    assert_eq!(BackendType::WebGpu.to_string(), "WebGPU");
    assert_eq!(BackendType::Cpu.to_string(), "CPU");
}

#[test]
fn test_unified_memory_capabilities() {
    let caps = UnifiedMemoryCapabilities {
        backend_type: BackendType::Vulkan,
        max_allocation_size: 1024 * 1024 * 1024,
        zero_copy: true,
        coherent: true,
        cpu_fast_access: true,
        gpu_fast_access: true,
        alignment_requirement: 64,
    };

    assert!(caps.is_truly_unified());
    assert!(!caps.needs_explicit_sync());
}

#[test]
fn test_buffer_metadata() {
    let id = BufferId::new(1);
    let mut metadata = UnifiedBufferMetadata::new(id, 4096, MemoryFlags::default());

    assert_eq!(metadata.id, id);
    assert_eq!(metadata.size, 4096);
    assert_eq!(metadata.access_count, 0);

    metadata.record_access();
    assert_eq!(metadata.access_count, 1);

    metadata.record_access();
    assert_eq!(metadata.access_count, 2);
}

#[test]
fn test_unified_memory_stats() {
    let mut stats = UnifiedMemoryStats::new("Vulkan".to_string());

    stats.total_allocated = 1024;
    stats.update_peak(1024);
    assert_eq!(stats.peak_allocated, 1024);

    stats.total_allocated = 2048;
    stats.update_peak(2048);
    assert_eq!(stats.peak_allocated, 2048);

    stats.total_allocated = 1024;
    stats.update_peak(1024);
    assert_eq!(stats.peak_allocated, 2048); // Peak stays at max

    stats.calculate_pool_hit_rate(80, 20);
    assert!((stats.pool_hit_rate - 0.8).abs() < 0.01);
}
