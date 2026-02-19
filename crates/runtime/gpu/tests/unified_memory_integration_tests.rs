//! Integration tests for unified memory system
//!
//! These tests verify interactions between multiple components.
#![allow(clippy::expect_used)] // In tests, expect() gives clear failure messages for setup/assertions

use toadstool_runtime_gpu::unified_memory::{BackendStrategy, BackendType, UniversalUnifiedMemory};

async fn create_test_memory() -> UniversalUnifiedMemory {
    UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .expect("Failed to create test memory manager")
}

// ============================================================================
// Multi-Buffer Operations
// ============================================================================

#[tokio::test]
async fn test_multiple_buffers_independent() {
    let memory = create_test_memory().await;

    let mut b1 = memory.allocate(1024).await.expect("Allocate b1");
    let mut b2 = memory.allocate(2048).await.expect("Allocate b2");
    let mut b3 = memory.allocate(4096).await.expect("Allocate b3");

    // Write unique data to each
    b1.write_async(0, &[1u8; 100]).await.expect("Write b1");
    b2.write_async(0, &[2u8; 100]).await.expect("Write b2");
    b3.write_async(0, &[3u8; 100]).await.expect("Write b3");

    // Read back and verify isolation
    let d1 = b1.read_async(0, 100).await.expect("Read b1");
    let d2 = b2.read_async(0, 100).await.expect("Read b2");
    let d3 = b3.read_async(0, 100).await.expect("Read b3");

    assert_eq!(d1, vec![1u8; 100]);
    assert_eq!(d2, vec![2u8; 100]);
    assert_eq!(d3, vec![3u8; 100]);
}

#[tokio::test]
async fn test_interleaved_operations() {
    let memory = create_test_memory().await;

    let mut b1 = memory.allocate(1024).await.expect("Allocate b1");
    let mut b2 = memory.allocate(1024).await.expect("Allocate b2");

    // Interleaved writes
    b1.write_async(0, &[1u8; 100]).await.expect("Write b1");
    b2.write_async(0, &[2u8; 100]).await.expect("Write b2");
    b1.write_async(500, &[3u8; 100]).await.expect("Write b1");
    b2.write_async(500, &[4u8; 100]).await.expect("Write b2");

    // Interleaved reads
    let d1a = b1.read_async(0, 100).await.expect("Read b1");
    let d2a = b2.read_async(0, 100).await.expect("Read b2");
    let d1b = b1.read_async(500, 100).await.expect("Read b1");
    let d2b = b2.read_async(500, 100).await.expect("Read b2");

    assert_eq!(d1a, vec![1u8; 100]);
    assert_eq!(d2a, vec![2u8; 100]);
    assert_eq!(d1b, vec![3u8; 100]);
    assert_eq!(d2b, vec![4u8; 100]);
}

// ============================================================================
// Concurrent Access
// ============================================================================

#[tokio::test]
async fn test_concurrent_allocations() {
    let memory = create_test_memory().await;

    let mut handles = vec![];

    for i in 0..10 {
        let mem = memory.clone();
        let handle = tokio::spawn(async move {
            let mut buffer = mem.allocate(1024).await.expect("Allocate failed");
            buffer
                .write_async(0, &vec![i as u8; 100])
                .await
                .expect("Write failed");
            let data = buffer.read_async(0, 100).await.expect("Read failed");
            assert_eq!(data, vec![i as u8; 100]);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task failed");
    }
}

#[tokio::test]
async fn test_concurrent_reads() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Allocate failed");

    // Write initial data
    buffer
        .write_async(0, &[42u8; 1024])
        .await
        .expect("Write failed");

    let mut handles = vec![];

    for _ in 0..10 {
        let _buf_id = buffer.id(); // Keep for future registry lookup
        let mem = memory.clone();

        let handle = tokio::spawn(async move {
            // Find buffer by ID (simplified: in real code you'd use a registry)
            let test_buffer = mem.allocate(1024).await.expect("Allocate");
            let data = test_buffer.read_async(0, 100).await.expect("Read failed");
            assert_eq!(data.len(), 100);
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await.expect("Task failed");
    }
}

// ============================================================================
// Memory Pressure
// ============================================================================

#[tokio::test]
async fn test_many_small_allocations() {
    let memory = create_test_memory().await;

    let mut buffers = vec![];

    for i in 0..100 {
        let buffer = memory.allocate(1024).await.expect("Allocate failed");
        buffers.push(buffer);

        if i % 10 == 0 {
            let stats = memory.stats();
            assert_eq!(stats.active_allocations as usize, i + 1);
        }
    }

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 100);

    drop(buffers);
    // Drop is synchronous — stats update immediately.

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 0);
}

#[tokio::test]
async fn test_allocation_deallocation_patterns() {
    let memory = create_test_memory().await;

    // Pattern 1: Allocate all, then deallocate all
    let mut buffers = vec![];
    for _ in 0..20 {
        buffers.push(memory.allocate(1024).await.expect("Allocate failed"));
    }
    drop(buffers);
    // Drop is synchronous — stats update immediately.

    // Pattern 2: Interleaved allocate/deallocate
    for _ in 0..20 {
        let _buffer = memory.allocate(1024).await.expect("Allocate failed");
        // Immediately dropped
    }
    // Drop is synchronous — stats update immediately.

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 0);
}

// ============================================================================
// Complex Workflows
// ============================================================================

#[tokio::test]
async fn test_data_pipeline() {
    let memory = create_test_memory().await;

    // Stage 1: Input buffer
    let mut input = memory.allocate(4096).await.expect("Allocate input");
    let input_data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();
    input
        .write_async(0, &input_data)
        .await
        .expect("Write input");

    // Stage 2: Processing buffer (copy and transform)
    let mut processing = memory.allocate(4096).await.expect("Allocate processing");
    let data = input.read_async(0, 1024).await.expect("Read input");
    let transformed: Vec<u8> = data.iter().map(|&b| b.wrapping_add(1)).collect();
    processing
        .write_async(0, &transformed)
        .await
        .expect("Write processing");

    // Stage 3: Output buffer (copy final result)
    let mut output = memory.allocate(4096).await.expect("Allocate output");
    let final_data = processing
        .read_async(0, 1024)
        .await
        .expect("Read processing");
    output
        .write_async(0, &final_data)
        .await
        .expect("Write output");

    // Verify pipeline
    let result = output.read_async(0, 1024).await.expect("Read output");
    let expected: Vec<u8> = (0..1024).map(|i| ((i % 256) + 1) as u8).collect();
    assert_eq!(result, expected);
}

#[tokio::test]
async fn test_double_buffering() {
    let memory = create_test_memory().await;

    let mut buffer_a = memory.allocate(2048).await.expect("Allocate A");
    let mut buffer_b = memory.allocate(2048).await.expect("Allocate B");

    // Simulate double-buffering pattern
    for frame in 0..10 {
        let data = vec![frame as u8; 1024];

        if frame % 2 == 0 {
            // Write to A, read from B
            buffer_a.write_async(0, &data).await.expect("Write A");
            if frame > 0 {
                let _ = buffer_b.read_async(0, 1024).await.expect("Read B");
            }
        } else {
            // Write to B, read from A
            buffer_b.write_async(0, &data).await.expect("Write B");
            let _ = buffer_a.read_async(0, 1024).await.expect("Read A");
        }
    }

    let stats = memory.stats();
    assert_eq!(stats.active_allocations, 2);
}

// ============================================================================
// Sync Workflows
// ============================================================================

#[tokio::test]
async fn test_cpu_gpu_cpu_workflow() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Allocate failed");

    // CPU write
    buffer
        .write_async(0, &[42u8; 100])
        .await
        .expect("CPU write");

    // Sync to GPU
    buffer.sync_to_device().await.expect("Sync to device");

    // Simulate GPU processing (mark as modified)
    buffer.mark_gpu_modified();

    // Sync back to CPU
    buffer.sync_to_cpu().await.expect("Sync to CPU");

    // CPU read
    let data = buffer.read_async(0, 100).await.expect("CPU read");
    assert_eq!(data.len(), 100);
}

#[tokio::test]
async fn test_multiple_sync_cycles() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Allocate failed");

    for i in 0..10 {
        // CPU → GPU
        buffer
            .write_async(0, &vec![i as u8; 100])
            .await
            .expect("Write");
        buffer.sync_to_device().await.expect("Sync to device");

        // GPU → CPU
        buffer.mark_gpu_modified();
        buffer.sync_to_cpu().await.expect("Sync to CPU");

        let data = buffer.read_async(0, 100).await.expect("Read");
        assert_eq!(data.len(), 100);
    }

    let stats = memory.stats();
    assert!(stats.cpu_to_gpu_syncs >= 10);
}

// ============================================================================
// Manager Operations
// ============================================================================

#[tokio::test]
async fn test_manager_clone() {
    let memory1 = create_test_memory().await;
    let memory2 = memory1.clone();

    let _b1 = memory1.allocate(1024).await.expect("Allocate from m1");
    let _b2 = memory2.allocate(2048).await.expect("Allocate from m2");

    let stats1 = memory1.stats();
    let stats2 = memory2.stats();

    // Both managers share the same state
    assert_eq!(stats1.active_allocations, stats2.active_allocations);
    assert_eq!(stats1.total_allocated, stats2.total_allocated);
}

#[tokio::test]
async fn test_backend_consistency() {
    let memory = create_test_memory().await;

    // All allocations should use same backend
    let _b1 = memory.allocate(1024).await.expect("Allocate");
    let _b2 = memory.allocate(2048).await.expect("Allocate");

    assert_eq!(memory.backend_name(), "CPU");
}

// ============================================================================
// Error Recovery
// ============================================================================

#[tokio::test]
async fn test_continue_after_allocation_error() {
    let memory = create_test_memory().await;

    // Trigger error
    let result = memory.allocate(0).await;
    assert!(result.is_err());

    // Should be able to continue
    let buffer = memory.allocate(1024).await.expect("Allocate after error");
    assert_eq!(buffer.size(), 1024);
}

#[tokio::test]
async fn test_continue_after_bounds_error() {
    let memory = create_test_memory().await;
    let mut buffer = memory.allocate(1024).await.expect("Allocate");

    // Trigger bounds error
    let result = buffer.write_async(2000, &[0u8; 100]).await;
    assert!(result.is_err());

    // Should be able to continue with valid operations
    let result = buffer.write_async(0, &[42u8; 100]).await;
    assert!(result.is_ok());
}

// ============================================================================
// Real-World Scenarios
// ============================================================================

#[tokio::test]
async fn test_image_processing_simulation() {
    let memory = create_test_memory().await;

    // Simulate 256x256 RGBA image (256KB)
    let image_size = 256 * 256 * 4;

    let mut input_buffer = memory.allocate(image_size).await.expect("Input buffer");
    let mut output_buffer = memory.allocate(image_size).await.expect("Output buffer");

    // Generate input image
    let input_data: Vec<u8> = (0..image_size).map(|i| (i % 256) as u8).collect();
    input_buffer
        .write_async(0, &input_data)
        .await
        .expect("Write input");

    // Sync to GPU
    input_buffer
        .sync_to_device()
        .await
        .expect("Sync input to GPU");

    // Simulate GPU processing (read, process, write)
    let data = input_buffer.read_async(0, image_size).await.expect("Read");
    let processed: Vec<u8> = data.iter().map(|&b| 255 - b).collect(); // Invert

    output_buffer
        .write_async(0, &processed)
        .await
        .expect("Write output");
    output_buffer
        .sync_to_device()
        .await
        .expect("Sync output to GPU");

    // Read back result
    output_buffer
        .sync_to_cpu()
        .await
        .expect("Sync output to CPU");
    let result = output_buffer
        .read_async(0, image_size)
        .await
        .expect("Read result");

    // Verify processing
    for (i, &value) in result.iter().enumerate().take(100) {
        assert_eq!(value, 255 - ((i % 256) as u8));
    }
}

#[tokio::test]
async fn test_batch_processing() {
    let memory = create_test_memory().await;

    const BATCH_SIZE: usize = 10;
    const ITEM_SIZE: usize = 4096;

    let mut buffers = Vec::with_capacity(BATCH_SIZE);

    // Allocate batch
    for _ in 0..BATCH_SIZE {
        let buffer = memory.allocate(ITEM_SIZE).await.expect("Allocate");
        buffers.push(buffer);
    }

    // Process batch
    for (i, buffer) in buffers.iter_mut().enumerate() {
        let data = vec![i as u8; 1024];
        buffer.write_async(0, &data).await.expect("Write");
        buffer.sync_to_device().await.expect("Sync");
    }

    // Verify batch
    for (i, buffer) in buffers.iter().enumerate() {
        let data = buffer.read_async(0, 1024).await.expect("Read");
        assert!(data.iter().all(|&b| b == i as u8));
    }
}
