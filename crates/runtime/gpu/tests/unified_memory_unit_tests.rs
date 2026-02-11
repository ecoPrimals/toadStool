//! Unit tests for unified memory buffer operations
//!
//! These tests verify individual buffer operations with correct API usage.
#![allow(clippy::expect_used)] // In tests, expect() gives clear failure messages for setup/assertions

use toadstool_runtime_gpu::unified_memory::{
    BackendStrategy, BackendType, MemoryFlags, UniversalUnifiedMemory,
};

// Helper function to create CPU backend for stable testing
async fn create_test_memory() -> UniversalUnifiedMemory {
    UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .expect("Failed to create test memory manager")
}

// ============================================================================
// Basic Buffer Operations
// ============================================================================

#[tokio::test]
async fn test_buffer_creation() {
    let memory = create_test_memory().await;
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    assert_eq!(buffer.size(), 1024);
    assert!(buffer.id().as_u64() > 0);
}

#[tokio::test]
async fn test_buffer_write_read_basic() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    let data = vec![42u8; 100];
    buffer.write_async(0, &data).await.expect("Failed to write");

    let result = buffer.read_async(0, 100).await.expect("Failed to read");
    assert_eq!(data, result);
}

#[tokio::test]
async fn test_buffer_write_read_offset() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    // Write at different offsets
    buffer
        .write_async(0, &[1u8; 100])
        .await
        .expect("Write at 0");
    buffer
        .write_async(500, &[2u8; 100])
        .await
        .expect("Write at 500");

    // Read back
    let data1 = buffer.read_async(0, 100).await.expect("Read at 0");
    let data2 = buffer.read_async(500, 100).await.expect("Read at 500");

    assert_eq!(data1, vec![1u8; 100]);
    assert_eq!(data2, vec![2u8; 100]);
}

#[tokio::test]
async fn test_buffer_write_patterns() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(4096).await.expect("Failed to allocate");

    // Write various patterns
    let patterns = vec![
        vec![0u8; 100],
        vec![255u8; 100],
        (0..100).map(|i| i as u8).collect::<Vec<_>>(),
        vec![0xAA; 100],
    ];

    for (i, pattern) in patterns.iter().enumerate() {
        let offset = i * 200;
        buffer
            .write_async(offset, pattern)
            .await
            .expect("Write pattern");

        let result = buffer
            .read_async(offset, pattern.len())
            .await
            .expect("Read pattern");
        assert_eq!(&result, pattern, "Pattern {} mismatch", i);
    }
}

// ============================================================================
// Bounds Checking
// ============================================================================

#[tokio::test]
async fn test_buffer_write_out_of_bounds() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    // Try to write beyond buffer
    let result = buffer.write_async(1000, &[0u8; 100]).await;
    assert!(result.is_err(), "Should fail on out-of-bounds write");
}

#[tokio::test]
async fn test_buffer_read_out_of_bounds() {
    let memory = create_test_memory().await;
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    // Try to read beyond buffer
    let result = buffer.read_async(1000, 100).await;
    assert!(result.is_err(), "Should fail on out-of-bounds read");
}

#[tokio::test]
async fn test_buffer_write_at_boundary() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    // Write exactly at boundary (should succeed)
    let result = buffer.write_async(1024 - 100, &[0u8; 100]).await;
    assert!(result.is_ok(), "Should succeed at boundary");

    // Write one byte over (should fail)
    let result = buffer.write_async(1024 - 100, &[0u8; 101]).await;
    assert!(result.is_err(), "Should fail one byte over");
}

#[tokio::test]
async fn test_zero_size_read() {
    let memory = create_test_memory().await;
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    let result = buffer
        .read_async(0, 0)
        .await
        .expect("Zero read should work");
    assert_eq!(result.len(), 0);
}

#[tokio::test]
async fn test_zero_size_write() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    let result = buffer.write_async(0, &[]).await;
    assert!(result.is_ok(), "Zero write should work");
}

// ============================================================================
// Fill Operations
// ============================================================================

#[tokio::test]
async fn test_buffer_fill() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    buffer.fill(0xFF).await.expect("Fill failed");

    let data = buffer.read_async(0, 1024).await.expect("Read failed");
    assert!(data.iter().all(|&b| b == 0xFF));
}

#[tokio::test]
async fn test_buffer_zero() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    // Fill with non-zero first
    buffer.fill(0xAA).await.expect("Fill failed");

    // Zero it
    buffer.zero().await.expect("Zero failed");

    let data = buffer.read_async(0, 1024).await.expect("Read failed");
    assert!(data.iter().all(|&b| b == 0));
}

#[tokio::test]
async fn test_buffer_fill_patterns() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    let patterns = [0x00, 0xFF, 0xAA, 0x55, 0x01, 0xFE];

    for &pattern in &patterns {
        buffer.fill(pattern).await.expect("Fill failed");

        let data = buffer.read_async(0, 1024).await.expect("Read failed");
        assert!(
            data.iter().all(|&b| b == pattern),
            "Fill pattern 0x{:02X} failed",
            pattern
        );
    }
}

// ============================================================================
// Sync State Operations
// ============================================================================

#[tokio::test]
async fn test_sync_state_initial() {
    let memory = create_test_memory().await;
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    use toadstool_runtime_gpu::unified_memory::SyncState;
    assert_eq!(buffer.sync_state(), SyncState::Synced);
}

#[tokio::test]
async fn test_sync_state_after_write() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    buffer
        .write_async(0, &[1u8; 100])
        .await
        .expect("Write failed");

    use toadstool_runtime_gpu::unified_memory::SyncState;
    assert_eq!(buffer.sync_state(), SyncState::CpuModified);
}

#[tokio::test]
async fn test_sync_to_device() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    buffer
        .write_async(0, &[1u8; 100])
        .await
        .expect("Write failed");

    buffer.sync_to_device().await.expect("Sync failed");

    use toadstool_runtime_gpu::unified_memory::SyncState;
    assert_eq!(buffer.sync_state(), SyncState::Synced);
}

#[tokio::test]
async fn test_mark_gpu_modified() {
    let memory = create_test_memory().await;
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    buffer.mark_gpu_modified();

    use toadstool_runtime_gpu::unified_memory::SyncState;
    assert_eq!(buffer.sync_state(), SyncState::GpuModified);
}

#[tokio::test]
async fn test_sync_to_cpu() {
    let memory = create_test_memory().await;
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    buffer.mark_gpu_modified();
    buffer.sync_to_cpu().await.expect("Sync failed");

    use toadstool_runtime_gpu::unified_memory::SyncState;
    assert_eq!(buffer.sync_state(), SyncState::Synced);
}

// ============================================================================
// Memory Flags
// ============================================================================

#[tokio::test]
async fn test_allocate_with_balanced_flags() {
    let memory = create_test_memory().await;
    let buffer = memory
        .allocate_with_flags(1024, MemoryFlags::balanced())
        .await
        .expect("Failed to allocate");

    assert_eq!(buffer.size(), 1024);
}

#[tokio::test]
async fn test_allocate_with_cpu_optimized_flags() {
    let memory = create_test_memory().await;
    let buffer = memory
        .allocate_with_flags(1024, MemoryFlags::cpu_optimized())
        .await
        .expect("Failed to allocate");

    assert_eq!(buffer.size(), 1024);
}

#[tokio::test]
async fn test_allocate_with_gpu_optimized_flags() {
    let memory = create_test_memory().await;
    let buffer = memory
        .allocate_with_flags(1024, MemoryFlags::gpu_optimized())
        .await
        .expect("Failed to allocate");

    assert_eq!(buffer.size(), 1024);
}

// ============================================================================
// Statistics and Metrics
// ============================================================================

#[tokio::test]
async fn test_stats_after_allocation() {
    let memory = create_test_memory().await;

    let initial_stats = memory.stats();
    assert_eq!(initial_stats.active_allocations, 0);

    let _buffer = memory.allocate(4096).await.expect("Failed to allocate");

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 1);
    assert_eq!(stats.total_allocated, 4096);
}

#[tokio::test]
async fn test_stats_after_deallocation() {
    let memory = create_test_memory().await;

    {
        let _buffer = memory.allocate(4096).await.expect("Failed to allocate");
        let stats = memory.stats();
        assert_eq!(stats.active_allocations, 1);
    } // buffer dropped here

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 0);
    assert_eq!(stats.total_allocated, 0);
}

#[tokio::test]
async fn test_stats_multiple_allocations() {
    let memory = create_test_memory().await;

    let _b1 = memory.allocate(1024).await.expect("Failed");
    let _b2 = memory.allocate(2048).await.expect("Failed");
    let _b3 = memory.allocate(4096).await.expect("Failed");

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 3);
    assert_eq!(stats.total_allocated, 1024 + 2048 + 4096);
}

#[tokio::test]
async fn test_peak_allocated() {
    let memory = create_test_memory().await;

    {
        let _large = memory.allocate(8192).await.expect("Failed");
        let stats = memory.stats();
        assert!(stats.peak_allocated >= 8192);
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 0);
    assert!(stats.peak_allocated >= 8192, "Peak should remain");
}

// ============================================================================
// Backend Information
// ============================================================================

#[tokio::test]
async fn test_backend_name() {
    let memory = create_test_memory().await;
    let name = memory.backend_name();

    assert!(!name.is_empty());
    assert_eq!(name, "CPU"); // We forced CPU backend
}

#[tokio::test]
async fn test_automatic_backend_selection() {
    let memory = UniversalUnifiedMemory::new()
        .await
        .expect("Failed to create");
    let name = memory.backend_name();

    assert!(!name.is_empty());
    // Should be one of the supported backends
    assert!(
        name == "CPU" || name == "WebGPU" || name == "Vulkan" || name == "OpenCL",
        "Unknown backend: {}",
        name
    );
}

// ============================================================================
// Large Allocations
// ============================================================================

#[tokio::test]
async fn test_large_allocation() {
    let memory = create_test_memory().await;

    // 16 MB allocation
    let size = 16 * 1024 * 1024;
    let buffer = memory
        .allocate(size)
        .await
        .expect("Large allocation failed");

    assert_eq!(buffer.size(), size);
}

#[tokio::test]
async fn test_large_data_transfer() {
    let memory = create_test_memory().await;

    // 8 MB buffer
    let size = 8 * 1024 * 1024;
    let mut buffer = memory.allocate(size).await.expect("Failed to allocate");

    // Write 1 MB chunks
    let chunk_size = 1024 * 1024;
    for i in 0..(size / chunk_size) {
        let data = vec![i as u8; chunk_size];
        buffer
            .write_async(i * chunk_size, &data)
            .await
            .expect("Write failed");
    }

    // Read back and verify
    for i in 0..(size / chunk_size) {
        let data = buffer
            .read_async(i * chunk_size, chunk_size)
            .await
            .expect("Read failed");
        assert!(data.iter().all(|&b| b == i as u8), "Chunk {} mismatch", i);
    }
}

// ============================================================================
// Error Conditions
// ============================================================================

#[tokio::test]
async fn test_zero_size_allocation() {
    let memory = create_test_memory().await;
    let result = memory.allocate(0).await;

    assert!(result.is_err(), "Should reject zero-size allocation");
}

#[tokio::test]
async fn test_invalid_offset() {
    let memory = create_test_memory().await;
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    let result = buffer.read_async(1024, 1).await;
    assert!(result.is_err(), "Should fail with invalid offset");
}

// ============================================================================
// Buffer Lifecycle
// ============================================================================

#[tokio::test]
async fn test_buffer_drop_cleanup() {
    let memory = create_test_memory().await;

    for _ in 0..10 {
        let _buffer = memory.allocate(1024).await.expect("Failed to allocate");
    }

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 0, "All buffers should be freed");
}

#[tokio::test]
async fn test_buffer_reallocation_cycles() {
    let memory = create_test_memory().await;

    for _ in 0..20 {
        let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");
        buffer
            .write_async(0, &[42u8; 100])
            .await
            .expect("Write failed");
        let data = buffer.read_async(0, 100).await.expect("Read failed");
        assert_eq!(data, vec![42u8; 100]);
    }

    let stats = memory.stats();
    assert_eq!(stats.deallocation_count, 20);
}
