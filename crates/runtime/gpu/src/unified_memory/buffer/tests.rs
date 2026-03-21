// SPDX-License-Identifier: AGPL-3.0-only
//! Unit tests for [`super::UnifiedBuffer`].

use crate::unified_memory::manager::UniversalUnifiedMemory;
use crate::unified_memory::types::{BackendStrategy, BackendType, SyncState};

#[tokio::test]
async fn test_buffer_write_read() {
    eprintln!("=== Original test_buffer_write_read starting ===");

    // DEEP DEBT FIX: Force CPU backend until WebGPU Drop is fixed
    let memory = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .unwrap();
    eprintln!("Memory created, backend: {}", memory.backend_name());

    let mut buffer = memory.allocate(4096).await.unwrap();
    eprintln!("Buffer allocated: {}", buffer.size());

    // Write data
    let data = vec![42u8; 1024];
    eprintln!("Writing {} bytes...", data.len());
    buffer.write_async(0, &data).await.unwrap();
    eprintln!("Write complete");

    // Read back
    eprintln!("Reading back...");
    let result = buffer.read_async(0, 1024).await.unwrap();
    eprintln!("Read complete");

    assert_eq!(data.as_slice(), result.as_ref());
    eprintln!("=== Test passed ===");
}

#[tokio::test]
async fn test_buffer_bounds_checking() {
    let memory = UniversalUnifiedMemory::new().await.unwrap();
    let mut buffer = memory.allocate(1024).await.unwrap();

    // Write beyond bounds should fail
    let data = vec![0u8; 2048];
    let result = buffer.write_async(0, &data).await;
    assert!(result.is_err());

    // Read beyond bounds should fail
    let result = buffer.read_async(0, 2048).await;
    assert!(result.is_err());

    // Write with offset beyond bounds should fail
    let data = vec![0u8; 512];
    let result = buffer.write_async(1024, &data).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_buffer_sync_state() {
    // DEEP DEBT FIX: Force CPU backend until WebGPU Drop is fixed
    let memory = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .unwrap();
    let mut buffer = memory.allocate(1024).await.unwrap();

    // Initially synced
    assert_eq!(buffer.sync_state(), SyncState::Synced);

    // After write, CPU modified
    let data = vec![42u8; 512];
    buffer.write_async(0, &data).await.unwrap();
    assert_eq!(buffer.sync_state(), SyncState::CpuModified);

    // After sync to device, synced again
    buffer.sync_to_device().await.unwrap();
    assert_eq!(buffer.sync_state(), SyncState::Synced);

    // Mark GPU modified
    buffer.mark_gpu_modified();
    assert_eq!(buffer.sync_state(), SyncState::GpuModified);

    // Sync back to CPU
    buffer.sync_to_cpu().await.unwrap();
    assert_eq!(buffer.sync_state(), SyncState::Synced);
}

#[tokio::test]
async fn test_buffer_fill() {
    // DEEP DEBT FIX: Force CPU backend until WebGPU Drop is fixed
    let memory = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .unwrap();
    let mut buffer = memory.allocate(1024).await.unwrap();

    // Fill with value
    buffer.fill(0xFF).await.unwrap();

    // Read back
    let result = buffer.read_async(0, 1024).await.unwrap();
    assert!(result.iter().all(|&b| b == 0xFF));

    // Zero buffer
    buffer.zero().await.unwrap();
    let result = buffer.read_async(0, 1024).await.unwrap();
    assert!(result.iter().all(|&b| b == 0));
}
