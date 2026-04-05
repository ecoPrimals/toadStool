// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal Unified Memory Demo

#![allow(
    clippy::cast_possible_truncation,
    reason = "truncation acceptable for this conversion"
)]
#![allow(clippy::cast_sign_loss)]
//!
//! Demonstrates vendor-agnostic zero-copy GPU compute using ToadStool's
//! unified memory system.
//!
//! # What This Shows
//!
//! - Automatic backend selection (sovereignty-first)
//! - Zero-copy CPU/GPU data sharing
//! - Async-native memory operations
//! - Smart synchronization
//! - Performance metrics
//!
//! # Run
//!
//! ```bash
//! cargo run --example unified_memory_demo
//! ```

use toadstool_runtime_gpu::unified_memory::{MemoryFlags, UniversalUnifiedMemory};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("🍄 ToadStool Universal Unified Memory Demo\n");

    // ═══════════════════════════════════════════════════════════════
    // STEP 1: Initialize Unified Memory (automatic backend selection)
    // ═══════════════════════════════════════════════════════════════

    println!("📦 Initializing unified memory...");
    let memory = UniversalUnifiedMemory::new().await?;

    println!("✅ Backend: {}", memory.backend_name());
    println!("   Type: {:?}", memory.backend_type());

    let caps = memory.capabilities();
    println!("\n🔍 Capabilities:");
    println!(
        "   Max allocation: {} MB",
        caps.max_allocation_size / 1024 / 1024
    );
    println!("   Zero-copy: {}", caps.zero_copy);
    println!("   Coherent: {}", caps.coherent);
    println!("   CPU fast: {}", caps.cpu_fast_access);
    println!("   GPU fast: {}", caps.gpu_fast_access);
    println!("   Alignment: {} bytes", caps.alignment_requirement);

    // ═══════════════════════════════════════════════════════════════
    // STEP 2: Allocate Unified Buffer
    // ═══════════════════════════════════════════════════════════════

    println!("\n📦 Allocating 1MB unified buffer...");
    let mut buffer = memory.allocate(1024 * 1024).await?;

    println!("✅ Buffer ID: {}", buffer.id());
    println!("   Size: {} bytes", buffer.size());
    println!("   Sync state: {:?}", buffer.sync_state());

    // ═══════════════════════════════════════════════════════════════
    // STEP 3: Write Data from CPU
    // ═══════════════════════════════════════════════════════════════

    println!("\n✍️  Writing data from CPU...");

    // Create test data
    let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

    // Write to buffer
    buffer.write_async(0, &data).await?;

    println!("✅ Wrote {} bytes", data.len());
    println!("   Sync state: {:?}", buffer.sync_state());

    // ═══════════════════════════════════════════════════════════════
    // STEP 4: Synchronize to GPU
    // ═══════════════════════════════════════════════════════════════

    println!("\n🔄 Synchronizing to GPU...");
    buffer.sync_to_device().await?;

    println!("✅ Synced to device");
    println!("   Sync state: {:?}", buffer.sync_state());
    println!("   Device pointer: {:p}", buffer.device_ptr());

    // ═══════════════════════════════════════════════════════════════
    // STEP 5: Simulate GPU Kernel Execution
    // ═══════════════════════════════════════════════════════════════

    println!("\n🚀 Simulating GPU kernel execution...");

    // In a real scenario, you would:
    // 1. Get device pointer: buffer.device_ptr()
    // 2. Pass to GPU kernel
    // 3. Execute kernel
    // 4. Mark buffer as modified

    // For demo, we just mark it as modified
    buffer.mark_gpu_modified();

    println!("✅ GPU kernel completed (simulated)");
    println!("   Sync state: {:?}", buffer.sync_state());

    // ═══════════════════════════════════════════════════════════════
    // STEP 6: Synchronize Back to CPU
    // ═══════════════════════════════════════════════════════════════

    println!("\n🔄 Synchronizing back to CPU...");
    buffer.sync_to_cpu().await?;

    println!("✅ Synced to CPU");
    println!("   Sync state: {:?}", buffer.sync_state());

    // ═══════════════════════════════════════════════════════════════
    // STEP 7: Read Data from CPU
    // ═══════════════════════════════════════════════════════════════

    println!("\n📖 Reading data from CPU...");

    let result = buffer.read_async(0, 1024).await?;

    println!("✅ Read {} bytes", result.len());
    println!("   First 16 bytes: {:?}", &result[..16]);
    println!("   Last 16 bytes: {:?}", &result[result.len() - 16..]);

    // Verify data integrity
    let matches = result == data;
    println!(
        "   Data integrity: {}",
        if matches { "✅ PASS" } else { "❌ FAIL" }
    );

    // ═══════════════════════════════════════════════════════════════
    // STEP 8: Demonstrate Fill Operations
    // ═══════════════════════════════════════════════════════════════

    println!("\n🎨 Testing fill operations...");

    // Fill with pattern
    buffer.fill(0xAA).await?;
    let filled = buffer.read_async(0, 16).await?;
    println!("✅ Filled with 0xAA: {:?}", &filled[..8]);

    // Zero buffer
    buffer.zero().await?;
    let zeroed = buffer.read_async(0, 16).await?;
    println!("✅ Zeroed: {:?}", &zeroed[..8]);

    // ═══════════════════════════════════════════════════════════════
    // STEP 9: Performance Metrics
    // ═══════════════════════════════════════════════════════════════

    println!("\n📊 Performance Metrics:");

    let stats = memory.stats();
    println!("   Total allocated: {} bytes", stats.total_allocated);
    println!("   Peak allocated: {} bytes", stats.peak_allocated);
    println!("   Allocations: {}", stats.allocation_count);
    println!("   Deallocations: {}", stats.deallocation_count);
    println!("   Active: {}", stats.active_allocations);
    println!("   CPU→GPU syncs: {}", stats.cpu_to_gpu_syncs);
    println!("   GPU→CPU syncs: {}", stats.gpu_to_cpu_syncs);
    println!("   Bytes synced: {}", stats.bytes_synced);

    // ═══════════════════════════════════════════════════════════════
    // STEP 10: Multiple Buffers
    // ═══════════════════════════════════════════════════════════════

    println!("\n📦 Testing multiple buffers...");

    let buffer2 = memory.allocate(4096).await?;
    let buffer3 = memory.allocate(8192).await?;

    println!("✅ Created 3 buffers total");
    println!("   Active allocations: {}", memory.active_allocations());
    println!("   Total allocated: {} bytes", memory.total_allocated());

    drop(buffer2);
    drop(buffer3);

    println!("✅ Dropped 2 buffers");
    println!("   Active allocations: {}", memory.active_allocations());

    // ═══════════════════════════════════════════════════════════════
    // STEP 11: Different Memory Flags
    // ═══════════════════════════════════════════════════════════════

    println!("\n🎯 Testing different memory flags...");

    // CPU-optimized
    let cpu_buffer = memory
        .allocate_with_flags(4096, MemoryFlags::cpu_optimized())
        .await?;
    println!("✅ CPU-optimized buffer: {}", cpu_buffer.id());

    // GPU-optimized
    let gpu_buffer = memory
        .allocate_with_flags(4096, MemoryFlags::gpu_optimized())
        .await?;
    println!("✅ GPU-optimized buffer: {}", gpu_buffer.id());

    // Balanced
    let balanced_buffer = memory
        .allocate_with_flags(4096, MemoryFlags::balanced())
        .await?;
    println!("✅ Balanced buffer: {}", balanced_buffer.id());

    // ═══════════════════════════════════════════════════════════════
    // FINAL STATS
    // ═══════════════════════════════════════════════════════════════

    println!("\n📊 Final Statistics:");
    let final_stats = memory.stats();
    println!("   Backend: {}", final_stats.backend);
    println!("   Total allocations: {}", final_stats.allocation_count);
    println!("   Active buffers: {}", final_stats.active_allocations);
    println!("   Peak memory: {} KB", final_stats.peak_allocated / 1024);

    println!("\n✅ Demo complete!");
    println!("\n💡 Key Takeaways:");
    println!("   • Vendor-agnostic: Works on Intel, AMD, NVIDIA");
    println!("   • Zero-copy: No data duplication");
    println!("   • Async-native: Fully concurrent");
    println!("   • Type-safe: No unwraps, comprehensive error handling");
    println!("   • Sovereignty-first: Prioritizes pure Rust backends");

    Ok(())
}
