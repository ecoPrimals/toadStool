// SPDX-License-Identifier: AGPL-3.0-or-later
//! Debug test to isolate GPU safety issue

use toadstool_runtime_gpu::unified_memory::*;

#[tokio::test]
async fn test_minimal_allocation() {
    eprintln!("=== Test starting ===");

    // Create memory manager
    eprintln!("Creating memory manager...");
    let memory = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .expect("Failed to create memory manager");

    eprintln!("Memory manager created, backend: {}", memory.backend_name());

    // Allocate buffer
    eprintln!("Allocating 1024 bytes...");
    let buffer = memory.allocate(1024).await.expect("Failed to allocate");

    eprintln!(
        "Buffer allocated: id={}, size={}",
        buffer.id(),
        buffer.size()
    );

    // Try to get device pointer (should be safe)
    let dev_ptr = buffer.device_ptr();
    eprintln!("Device pointer: {:#x}", dev_ptr as usize);

    eprintln!("=== Allocation successful ===");
}

#[tokio::test]
async fn test_write_simple() {
    eprintln!("=== Test write starting ===");

    let memory = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .expect("Failed to create memory");

    eprintln!("Allocating buffer...");
    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    eprintln!("Buffer allocated, attempting write...");

    // Write single byte
    let data = vec![42u8];
    eprintln!("Writing 1 byte...");
    buffer.write_async(0, &data).await.expect("Write failed");

    eprintln!("=== Write successful ===");
}

#[tokio::test]
async fn test_read_simple() {
    eprintln!("=== Test read starting ===");

    let memory = UniversalUnifiedMemory::with_strategy(BackendStrategy::Specific(BackendType::Cpu))
        .await
        .expect("Failed to create memory");

    let mut buffer = memory.allocate(1024).await.expect("Failed to allocate");

    // Write then read
    eprintln!("Writing data...");
    buffer.write_async(0, &[42u8]).await.expect("Write failed");

    eprintln!("Reading data...");
    let result = buffer.read_async(0, 1).await.expect("Read failed");

    eprintln!("Read result: {:?}", result);
    assert_eq!(result, vec![42u8]);

    eprintln!("=== Read successful ===");
}
