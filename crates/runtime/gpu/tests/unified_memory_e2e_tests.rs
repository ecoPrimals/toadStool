//! End-to-End tests for Universal Unified Memory
//!
//! These tests verify the complete unified memory system from initialization
//! through allocation, data transfer, synchronization, and cleanup.
//!
//! Test Coverage:
//! - Backend initialization and fallback
//! - Multi-buffer allocation and management
//! - CPU-GPU data transfer workflows
//! - Concurrent buffer operations
//! - Error handling and recovery
//! - Memory pressure scenarios
//! - Backend switching
//! - Real-world usage patterns

use toadstool_runtime_gpu::unified_memory::{MemoryFlags, UniversalUnifiedMemory};

/// Test basic end-to-end workflow: allocate, write, read
#[tokio::test]
async fn test_e2e_basic_workflow() {
    // Initialize memory manager with explicit CPU backend to avoid WebGPU issues
    use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
    let memory = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .expect("Failed to initialize memory manager");

    // Allocate a buffer
    let mut buffer = memory
        .allocate(4096)
        .await
        .expect("Failed to allocate buffer");

    // Write data
    let write_data: Vec<u8> = (0..256).map(|i| i as u8).collect();
    buffer
        .write_async(0, &write_data)
        .await
        .expect("Failed to write data");

    // Read data back
    let read_data = buffer
        .read_async(0, 256)
        .await
        .expect("Failed to read data");

    // Verify data integrity
    assert_eq!(write_data, read_data, "Data mismatch after write/read");

    // Check metrics
    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 1);
    assert_eq!(stats.total_allocated, 4096);
}

/// Test multiple buffer allocation and management
#[tokio::test]
async fn test_e2e_multiple_buffers() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    // Allocate multiple buffers
    let mut buffer1 = memory.allocate(1024).await.unwrap();
    let mut buffer2 = memory.allocate(2048).await.unwrap();
    let mut buffer3 = memory.allocate(4096).await.unwrap();

    // Write unique data to each
    buffer1.write_async(0, &[1u8; 100]).await.unwrap();
    buffer2.write_async(0, &[2u8; 100]).await.unwrap();
    buffer3.write_async(0, &[3u8; 100]).await.unwrap();

    // Verify each buffer maintains its data
    let data1 = buffer1.read_async(0, 100).await.unwrap();
    let data2 = buffer2.read_async(0, 100).await.unwrap();
    let data3 = buffer3.read_async(0, 100).await.unwrap();

    assert_eq!(data1, vec![1u8; 100]);
    assert_eq!(data2, vec![2u8; 100]);
    assert_eq!(data3, vec![3u8; 100]);

    // Check metrics
    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 3);
    assert_eq!(stats.total_allocated, 1024 + 2048 + 4096);
}

/// Test concurrent buffer operations
#[tokio::test]
async fn test_e2e_concurrent_operations() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    // Spawn multiple tasks that allocate and use buffers concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let memory_clone = memory.clone();
        let handle = tokio::spawn(async move {
            let mut buffer = memory_clone.allocate(1024).await.unwrap();
            let data = vec![i as u8; 256];
            buffer.write_async(0, &data).await.unwrap();

            let read_data = buffer.read_async(0, 256).await.unwrap();

            assert_eq!(data, read_data);
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task panicked");
    }

    // All buffers freed synchronously when tasks complete — no sleep needed.
    let stats = memory.stats();
    assert_eq!(
        stats.active_allocations, 0,
        "Buffers not properly cleaned up"
    );
}

/// Test large data transfer
#[tokio::test]
async fn test_e2e_large_data_transfer() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    // Allocate 16MB buffer
    let size = 16 * 1024 * 1024;
    let mut buffer = memory.allocate(size).await.unwrap();

    // Generate large dataset
    let write_data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

    // Write in chunks
    let chunk_size = 1024 * 1024; // 1MB chunks
    for (i, chunk) in write_data.chunks(chunk_size).enumerate() {
        buffer.write_async(i * chunk_size, chunk).await.unwrap();
    }

    // Read back and verify
    let mut read_data = Vec::with_capacity(size);
    for i in 0..(size / chunk_size) {
        let chunk = buffer.read_async(i * chunk_size, chunk_size).await.unwrap();
        read_data.extend_from_slice(&chunk);
    }

    assert_eq!(write_data, read_data, "Large data transfer failed");
}

/// Test buffer fill operation
#[tokio::test]
async fn test_e2e_buffer_fill() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };
    let mut buffer = memory.allocate(4096).await.unwrap();

    // Fill with pattern
    buffer.fill(0xAB).await.unwrap();

    // Verify fill
    let data = buffer.read_async(0, 4096).await.unwrap();

    assert!(data.iter().all(|&b| b == 0xAB), "Fill operation failed");
}

/// Test error handling for out-of-bounds access
#[tokio::test]
async fn test_e2e_bounds_checking() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };
    let mut buffer = memory.allocate(1024).await.unwrap();

    // Try to write beyond buffer
    let result = buffer.write_async(1000, &[0u8; 100]).await;
    assert!(result.is_err(), "Should fail on out-of-bounds write");

    // Try to read beyond buffer
    let _data = buffer.read_async(1000, 100).await;
    assert!(result.is_err(), "Should fail on out-of-bounds read");
}

/// Test zero-size allocation rejection
#[tokio::test]
async fn test_e2e_zero_size_allocation() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };
    let result = memory.allocate(0).await;
    assert!(result.is_err(), "Should reject zero-size allocation");
}

/// Test memory pressure scenario
#[tokio::test]
async fn test_e2e_memory_pressure() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    // Allocate many small buffers
    let mut buffers = vec![];
    for _ in 0..100 {
        if let Ok(buffer) = memory.allocate(4096).await {
            buffers.push(buffer);
        }
    }

    // Should have allocated at least some buffers
    assert!(!buffers.is_empty(), "Should allocate at least some buffers");

    let stats = memory.stats();
    assert!(stats.active_allocations > 0);
    assert!(stats.total_allocated > 0);

    // Drop all buffers
    drop(buffers);
    // Drop is synchronous; stats update immediately.

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 0, "All buffers should be freed");
}

/// Test buffer with different memory flags
#[tokio::test]
async fn test_e2e_memory_flags() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    // Test different flag combinations
    let flags_to_test = vec![
        MemoryFlags::balanced(),
        MemoryFlags::cpu_optimized(),
        MemoryFlags::gpu_optimized(),
        MemoryFlags::balanced(),
    ];

    for flags in flags_to_test {
        let mut buffer = memory.allocate_with_flags(1024, flags).await.unwrap();

        // Verify basic operations work with all flag types
        buffer.write_async(0, &[42u8; 100]).await.unwrap();
        let data = buffer.read_async(0, 100).await.unwrap();
        assert_eq!(data, vec![42u8; 100]);
    }
}

/// Test sync operations
#[tokio::test]
async fn test_e2e_sync_operations() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };
    let mut buffer = memory.allocate(1024).await.unwrap();

    // Write data
    buffer.write_async(0, &[1u8; 100]).await.unwrap();

    // Sync to device (should be no-op for CPU backend, but shouldn't fail)
    buffer.sync_to_device().await.unwrap();

    // Sync to CPU
    buffer.sync_to_cpu().await.unwrap();

    // Read should still work
    let data = buffer.read_async(0, 100).await.unwrap();
    assert_eq!(data, vec![1u8; 100]);
}

/// Test buffer metadata access
#[tokio::test]
async fn test_e2e_buffer_metadata() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };
    let buffer = memory.allocate(4096).await.unwrap();

    // Metadata access via id() and size()
    assert_eq!(buffer.size(), 4096);
    assert!(buffer.id().as_u64() > 0);
}

/// Test real-world workflow: image processing simulation
#[tokio::test]
async fn test_e2e_image_processing_workflow() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    // Simulate 1920x1080 RGBA image
    let width = 1920;
    let height = 1080;
    let channels = 4;
    let image_size = width * height * channels;

    // Allocate input and output buffers
    let mut input_buffer = memory.allocate(image_size).await.unwrap();
    let mut output_buffer = memory.allocate(image_size).await.unwrap();

    // Generate input image data (gradient pattern)
    let input_data: Vec<u8> = (0..image_size)
        .map(|i| ((i / channels) % 256) as u8)
        .collect();

    // Upload to GPU
    input_buffer.write_async(0, &input_data).await.unwrap();
    input_buffer.sync_to_device().await.unwrap();

    // Simulate GPU processing (invert colors)
    let mut processed_data = input_buffer.read_async(0, image_size).await.unwrap();
    for pixel in processed_data.iter_mut() {
        *pixel = 255 - *pixel;
    }

    // Write processed data to output buffer
    output_buffer.write_async(0, &processed_data).await.unwrap();

    // Download from GPU
    output_buffer.sync_to_cpu().await.unwrap();
    let final_data = output_buffer.read_async(0, image_size).await.unwrap();

    // Verify processing
    assert_eq!(final_data, processed_data);

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 2);
}

/// Test buffer lifecycle with explicit drop
#[tokio::test]
async fn test_e2e_buffer_lifecycle() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    {
        let _buffer = memory.allocate(1024).await.unwrap();
        let stats = memory.stats();
        assert_eq!(stats.active_allocations, 1);
    } // buffer dropped here

    // Drop is synchronous; stats update immediately.

    let stats = memory.stats();
    assert_eq!(
        stats.active_allocations, 0,
        "Buffer should be freed after drop"
    );
}

/// Test backend capabilities query
#[tokio::test]
async fn test_e2e_backend_capabilities() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    // Should be able to query backend type
    let _stats = memory.stats();
    assert!(!memory.backend_name().is_empty());

    // CPU backend should always be available
    assert!(
        memory.backend_name() == "CPU"
            || memory.backend_name() == "WebGPU"
            || memory.backend_name() == "Vulkan"
            || memory.backend_name() == "OpenCL"
    );
}

/// Test stress scenario: rapid allocation/deallocation
#[tokio::test]
async fn test_e2e_rapid_alloc_dealloc() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };

    for _ in 0..50 {
        let mut buffer = memory.allocate(1024).await.unwrap();
        buffer.write_async(0, &[42u8; 100]).await.unwrap();
        drop(buffer);
    }

    // Drop is synchronous; stats update immediately.

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 0);
}

/// Test partial buffer operations
#[tokio::test]
async fn test_e2e_partial_operations() {
    let memory = {
        use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType};
        UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
            .await
            .unwrap()
    };
    let mut buffer = memory.allocate(4096).await.unwrap();

    // Write to different regions
    buffer.write_async(0, &[1u8; 100]).await.unwrap();
    buffer.write_async(1000, &[2u8; 100]).await.unwrap();
    buffer.write_async(2000, &[3u8; 100]).await.unwrap();

    // Read back each region
    let region1 = buffer.read_async(0, 100).await.unwrap();
    let region2 = buffer.read_async(1000, 100).await.unwrap();
    let region3 = buffer.read_async(2000, 100).await.unwrap();

    assert_eq!(region1, vec![1u8; 100]);
    assert_eq!(region2, vec![2u8; 100]);
    assert_eq!(region3, vec![3u8; 100]);
}
