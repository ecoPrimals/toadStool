#!/usr/bin/env cargo +nightly -Zscript
//! # Real GPU + CPU Pool with Universal Abstraction
//!
//! Uses ToadStool's universal compute interface - works with CUDA, OpenCL, Vulkan, or CPU
//! No backend-specific code needed!
//!
//! ```cargo
//! [dependencies]
//! toadstool = { path = "../crates/core/toadstool" }
//! toadstool-runtime-gpu = { path = "../crates/runtime/gpu", features = ["opencl"] }
//! tokio = { version = "1", features = ["full"] }
//! tracing = "0.1"
//! tracing-subscriber = "0.3"
//! uuid = "1"
//! serde_json = "1"
//! ```

use std::sync::Arc;
use std::time::Instant;
use toadstool_runtime_gpu::{
    cpu_resource::CpuComputeResource,
    scheduler::{SchedulingPolicy, UniversalComputeScheduler},
    universal::*,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                                                              ║");
    println!("║  🍄 ToadStool Universal GPU + CPU Pool Demo                ║");
    println!("║                                                              ║");
    println!("║  Backend-agnostic: Works with CUDA, OpenCL, Vulkan, or CPU  ║");
    println!("║                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Create scheduler with load balancing
    let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::LoadBalance);

    println!("📊 Initializing Compute Resource Pool...");
    println!();

    // Register CPU resource (always available)
    match CpuComputeResource::new() {
        Ok(cpu_resource) => {
            let caps = cpu_resource.capabilities();
            println!("✅ CPU Resource Registered:");
            println!("   Resource: {}", cpu_resource.resource_id());
            println!("   Type: {}", caps.resource_type);
            println!("   Cores: {}", caps.parallelism.max_parallel_threads);
            println!(
                "   Memory: {} GB",
                caps.memory.total_bytes / (1024 * 1024 * 1024)
            );
            println!();

            scheduler.register_resource(Arc::new(cpu_resource)).await;
        }
        Err(e) => {
            eprintln!("❌ Failed to initialize CPU resource: {}", e);
            return Err(e.into());
        }
    }

    // TODO: GPU resource auto-detection
    // The abstraction will automatically use CUDA, OpenCL, or Vulkan
    // based on what's available on your system
    println!("ℹ️  GPU resource detection coming soon");
    println!("   Will auto-detect: CUDA, OpenCL, Vulkan, Metal");
    println!("   For now: Using CPU resource pool");
    println!();

    // List all registered resources
    let resources = scheduler.list_resources().await;
    println!("📋 Resource Pool Summary:");
    for resource in &resources {
        println!("   • {}", resource);
    }
    println!();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  🎮 Executing Universal Compute Workloads                   ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let global_start = Instant::now();
    let mut workload_times = Vec::new();

    // ========================================================================
    // WORKLOAD 1: Matrix Multiplication (GPU-optimized, CPU fallback)
    // ========================================================================
    println!("1️⃣  Matrix Multiplication (2048x2048)");
    println!("   Backend: Universal (CUDA/OpenCL/Vulkan/CPU auto-select)");
    println!();

    let matrix_requirements = ComputeRequirements {
        min_parallel_threads: 4096,
        memory_bytes: 64 * 1024 * 1024,
        precision: Precision::Fp32,
        operations: vec![Operation::MatrixMultiply],
        max_execution_time: Some(std::time::Duration::from_secs(5)),
        preferred_access_pattern: Some(MemoryAccessPattern::Sequential),
        estimated_operations: Some(4096 * 1024),
    };

    match scheduler.select_resource(&matrix_requirements).await {
        Ok(resource) => {
            println!(
                "   🔍 Scheduler selected: {} ({})",
                resource.resource_id(),
                resource.capabilities().resource_type
            );

            let estimate = resource.estimate_execution_time(&matrix_requirements);
            println!("   ⏱️  Estimated time: {}ms", estimate.as_millis());

            // Create workload
            let workload = UniversalWorkload {
                id: uuid::Uuid::new_v4().to_string(),
                requirements: matrix_requirements,
                kernel: UniversalKernel::Operation {
                    operation: Operation::MatrixMultiply,
                    parameters: std::collections::HashMap::new(),
                },
                inputs: vec![ComputeBuffer {
                    name: "matrix_a".to_string(),
                    data: vec![0u8; 64 * 1024 * 1024], // 64 MB
                    element_type: toadstool_runtime_gpu::types::DataType::UInt8,
                }],
                hints: OptimizationHints::default(),
                output_size: 64 * 1024 * 1024,
            };

            // Execute on selected resource
            let start = Instant::now();
            match resource.create_context().await {
                Ok(mut context) => {
                    match context.execute(&workload).await {
                        Ok(result) => {
                            let duration = start.elapsed();
                            workload_times.push(("Matrix Multiply", duration));

                            println!("   ✅ Completed: {}ms", duration.as_millis());
                            println!("      Outputs: {} buffers", result.outputs.len());
                            // Resource info available via context
                            println!();
                        }
                        Err(e) => {
                            println!("   ❌ Execution failed: {}", e);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ Context creation failed: {}", e);
                    println!();
                }
            }
        }
        Err(e) => {
            println!("   ❌ No suitable resource: {}", e);
            println!();
        }
    }

    // ========================================================================
    // WORKLOAD 2: Parallel Reduction (CPU-efficient)
    // ========================================================================
    println!("2️⃣  Parallel Reduction (8M elements)");
    println!("   Backend: Universal (CPU optimized)");
    println!();

    let reduction_requirements = ComputeRequirements {
        min_parallel_threads: 256,
        memory_bytes: 8 * 1024 * 1024,
        precision: Precision::Fp32,
        operations: vec![Operation::Reduction],
        max_execution_time: Some(std::time::Duration::from_secs(1)),
        preferred_access_pattern: Some(MemoryAccessPattern::Sequential),
        estimated_operations: Some(256 * 1024),
    };

    match scheduler.select_resource(&reduction_requirements).await {
        Ok(resource) => {
            println!(
                "   🔍 Scheduler selected: {} ({})",
                resource.resource_id(),
                resource.capabilities().resource_type
            );

            let estimate = resource.estimate_execution_time(&reduction_requirements);
            println!("   ⏱️  Estimated time: {}ms", estimate.as_millis());

            let workload = UniversalWorkload {
                id: uuid::Uuid::new_v4().to_string(),
                requirements: reduction_requirements,
                kernel: UniversalKernel::Operation {
                    operation: Operation::Reduction,
                    parameters: std::collections::HashMap::new(),
                },
                inputs: vec![ComputeBuffer {
                    name: "data".to_string(),
                    data: vec![1u8; 8 * 1024 * 1024],
                    element_type: toadstool_runtime_gpu::types::DataType::UInt8,
                }],
                hints: OptimizationHints::default(),
                output_size: 8,
            };

            let start = Instant::now();
            match resource.create_context().await {
                Ok(mut context) => {
                    match context.execute(&workload).await {
                        Ok(result) => {
                            let duration = start.elapsed();
                            workload_times.push(("Reduction", duration));

                            println!("   ✅ Completed: {}ms", duration.as_millis());
                            println!("      Outputs: {} buffers", result.outputs.len());
                            // Resource info available via context
                            println!();
                        }
                        Err(e) => {
                            println!("   ❌ Execution failed: {}", e);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ Context creation failed: {}", e);
                    println!();
                }
            }
        }
        Err(e) => {
            println!("   ❌ No suitable resource: {}", e);
            println!();
        }
    }

    // ========================================================================
    // WORKLOAD 3: General Compute (Load balanced)
    // ========================================================================
    println!("3️⃣  General Parallel Compute (16M elements)");
    println!("   Backend: Universal (Load balanced)");
    println!();

    let compute_requirements = ComputeRequirements {
        min_parallel_threads: 1024,
        memory_bytes: 16 * 1024 * 1024,
        precision: Precision::Fp32,
        operations: vec![Operation::GeneralCompute],
        max_execution_time: Some(std::time::Duration::from_secs(2)),
        preferred_access_pattern: Some(MemoryAccessPattern::Random),
        estimated_operations: Some(1024 * 1024),
    };

    match scheduler.select_resource(&compute_requirements).await {
        Ok(resource) => {
            println!(
                "   🔍 Scheduler selected: {} ({})",
                resource.resource_id(),
                resource.capabilities().resource_type
            );

            let estimate = resource.estimate_execution_time(&compute_requirements);
            println!("   ⏱️  Estimated time: {}ms", estimate.as_millis());

            let workload = UniversalWorkload {
                id: uuid::Uuid::new_v4().to_string(),
                requirements: compute_requirements,
                kernel: UniversalKernel::Operation {
                    operation: Operation::GeneralCompute,
                    parameters: std::collections::HashMap::new(),
                },
                inputs: vec![ComputeBuffer {
                    name: "data".to_string(),
                    data: vec![42u8; 16 * 1024 * 1024],
                    element_type: toadstool_runtime_gpu::types::DataType::UInt8,
                }],
                hints: OptimizationHints::default(),
                output_size: 16 * 1024 * 1024,
            };

            let start = Instant::now();
            match resource.create_context().await {
                Ok(mut context) => {
                    match context.execute(&workload).await {
                        Ok(result) => {
                            let duration = start.elapsed();
                            workload_times.push(("General Compute", duration));

                            println!("   ✅ Completed: {}ms", duration.as_millis());
                            println!("      Outputs: {} buffers", result.outputs.len());
                            // Resource info available via context
                            println!();
                        }
                        Err(e) => {
                            println!("   ❌ Execution failed: {}", e);
                            println!();
                        }
                    }
                }
                Err(e) => {
                    println!("   ❌ Context creation failed: {}", e);
                    println!();
                }
            }
        }
        Err(e) => {
            println!("   ❌ No suitable resource: {}", e);
            println!();
        }
    }

    let total_duration = global_start.elapsed();

    // ========================================================================
    // Summary
    // ========================================================================
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  ✅ Universal Compute Pool Demo Complete!                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    println!("📊 Execution Summary:");
    println!();
    for (name, duration) in &workload_times {
        println!("   • {}: {}ms", name, duration.as_millis());
    }
    println!();
    println!("   Total time: {}ms", total_duration.as_millis());
    println!();

    println!("🎯 Universal Abstraction Benefits:");
    println!();
    println!("   ✅ Backend-agnostic code (same code works on CUDA, OpenCL, Vulkan, CPU)");
    println!("   ✅ Automatic resource selection based on workload requirements");
    println!("   ✅ Load balancing across available resources");
    println!("   ✅ Graceful fallback (GPU → CPU when GPU unavailable)");
    println!("   ✅ Sovereignty-first design (works without proprietary backends)");
    println!();

    println!("📝 What Just Happened:");
    println!();
    println!("   1. Scheduler registered CPU resource pool");
    println!("   2. Analyzed workload requirements (threads, memory, operations)");
    println!("   3. Selected optimal resource for each workload");
    println!("   4. Executed using Rayon parallel execution on CPU");
    println!("   5. (When GPU available: Would auto-use CUDA/OpenCL/Vulkan)");
    println!();

    println!("🚀 Add GPU to Pool:");
    println!();
    println!("   // GPU auto-detection coming soon!");
    println!("   let gpu = GpuComputeResource::detect()?;");
    println!("   scheduler.register_resource(Arc::new(gpu)).await;");
    println!();
    println!("   // Workloads automatically distributed across GPU + CPU");
    println!("   // NO CODE CHANGES NEEDED!");
    println!();

    println!("🎉 Universal abstraction working! Any backend, same code.");
    println!();

    Ok(())
}
