// SPDX-License-Identifier: AGPL-3.0-only
#![allow(clippy::cast_possible_truncation)]
//! Universal Compute Demo
//!
//! Demonstrates the capability-based universal compute system:
//! - Automatic GPU vs CPU selection
//! - Intelligent scheduling
//! - Hardware-agnostic workloads
//!
//! Run with: cargo run --example `universal_compute_demo` --features full

use std::sync::Arc;
use toadstool_runtime_gpu::{
    cpu_resource::CpuComputeResource,
    scheduler::{SchedulingPolicy, UniversalComputeScheduler},
    universal::{
        ComputeBuffer, ComputeRequirements, Operation, OptimizationHints, Precision,
        UniversalComputeResource, UniversalKernel, UniversalWorkload,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🌍 Universal Compute Demo - ToadStool Runtime\n");
    println!("{}", "=".repeat(60));

    // Create scheduler with capability-based policy
    let scheduler = UniversalComputeScheduler::new(SchedulingPolicy::CapabilityMatch);

    // Register CPU as compute resource
    let cpu_resource = Arc::new(CpuComputeResource::new()?);
    scheduler
        .register_resource(Arc::clone(&cpu_resource) as Arc<dyn UniversalComputeResource>)
        .await;

    println!("\n✅ Registered compute resources:");
    for resource in scheduler.list_resources().await {
        println!("   - {resource}");
    }

    println!("\n{}", "=".repeat(60));

    // Demo 1: Small workload (should select CPU)
    println!("\n📊 Demo 1: Small Workload (Low Parallelism)");
    println!("{}", "-".repeat(60));

    let small_workload = create_workload(
        "small-compute",
        8,    // 8 threads
        1024, // 1 KB
        vec![Operation::GeneralCompute],
    );

    println!("\nWorkload requirements:");
    println!(
        "  - Parallel threads: {}",
        small_workload.requirements.min_parallel_threads
    );
    println!(
        "  - Memory: {} KB",
        small_workload.requirements.memory_bytes / 1024
    );
    println!(
        "  - Operations: {:?}",
        small_workload.requirements.operations
    );

    let resource = scheduler
        .select_resource(&small_workload.requirements)
        .await?;

    println!("\n🎯 Scheduler selected: {}", resource.resource_id());
    println!("   Reason: Low overhead for small workload");

    let mut context = resource.create_context().await?;
    let result = context.execute(&small_workload).await?;

    println!("\n✅ Execution complete:");
    println!("   - Time: {:?}", result.metrics.execution_time);
    println!("   - Memory used: {} KB", result.metrics.memory_used / 1024);
    println!("   - Outputs: {} buffers", result.outputs.len());

    context.close().await?;

    println!("\n{}", "=".repeat(60));

    // Demo 2: Branch-heavy workload (should prefer CPU)
    println!("\n📊 Demo 2: Branch-Heavy Workload");
    println!("{}", "-".repeat(60));

    let branching_workload = create_workload(
        "branch-heavy",
        16,   // 16 threads
        4096, // 4 KB
        vec![Operation::BranchHeavy],
    );

    println!("\nWorkload requirements:");
    println!(
        "  - Parallel threads: {}",
        branching_workload.requirements.min_parallel_threads
    );
    println!(
        "  - Memory: {} KB",
        branching_workload.requirements.memory_bytes / 1024
    );
    println!(
        "  - Operations: {:?}",
        branching_workload.requirements.operations
    );

    let resource = scheduler
        .select_resource(&branching_workload.requirements)
        .await?;

    println!("\n🎯 Scheduler selected: {}", resource.resource_id());
    println!("   Reason: CPU has high branching efficiency");

    let mut context = resource.create_context().await?;
    let result = context.execute(&branching_workload).await?;

    println!("\n✅ Execution complete:");
    println!("   - Time: {:?}", result.metrics.execution_time);
    println!("   - Memory used: {} KB", result.metrics.memory_used / 1024);
    println!(
        "   - Energy: {:.2} J",
        result.metrics.energy_joules.unwrap_or(0.0)
    );

    context.close().await?;

    println!("\n{}", "=".repeat(60));

    // Demo 3: Capability matching
    println!("\n📊 Demo 3: Capability Scoring");
    println!("{}", "-".repeat(60));

    let test_requirements = ComputeRequirements {
        min_parallel_threads: 1024,
        memory_bytes: 1024 * 1024,
        precision: Precision::Fp32,
        operations: vec![Operation::MatrixMultiply],
        ..Default::default()
    };

    println!("\nTest requirements:");
    println!(
        "  - Parallel threads: {}",
        test_requirements.min_parallel_threads
    );
    println!(
        "  - Memory: {} MB",
        test_requirements.memory_bytes / (1024 * 1024)
    );
    println!("  - Precision: {:?}", test_requirements.precision);
    println!("  - Operations: {:?}", test_requirements.operations);

    println!("\n🎯 Capability scores:");

    // Score CPU
    let cpu_score = cpu_resource.score_workload(&test_requirements);
    let cpu_can = cpu_resource.can_execute(&test_requirements);

    println!("   - CPU: {cpu_score:.2} (can execute: {cpu_can})");

    if !cpu_can {
        println!(
            "     Reason: CPU cannot handle {} threads",
            test_requirements.min_parallel_threads
        );
    }

    println!("\n{}", "=".repeat(60));

    // Summary
    println!("\n🎉 Demo Complete!\n");
    println!("Key Takeaways:");
    println!("  ✅ Hardware-agnostic workloads");
    println!("  ✅ Automatic resource selection");
    println!("  ✅ CPU as first-class compute resource");
    println!("  ✅ Capability-based scheduling");
    println!("  ✅ Future-proof architecture\n");

    println!("Next Steps:");
    println!("  - Add GPU resources to see GPU vs CPU selection");
    println!("  - Try different scheduling policies");
    println!("  - Implement custom operations");
    println!("  - Add TPU/NPU/Quantum resources!\n");

    println!("{}", "=".repeat(60));

    Ok(())
}

/// Helper to create a workload
fn create_workload(
    id: &str,
    threads: u64,
    memory: u64,
    operations: Vec<Operation>,
) -> UniversalWorkload {
    use toadstool_runtime_gpu::types::DataType;

    UniversalWorkload {
        id: id.to_string(),
        requirements: ComputeRequirements {
            estimated_operations: Some(1_000_000),
            min_parallel_threads: threads,
            memory_bytes: memory,
            precision: Precision::Fp32,
            operations,
            max_execution_time: None,
            preferred_access_pattern: None,
        },
        kernel: UniversalKernel::Operation {
            operation: Operation::GeneralCompute,
            parameters: std::collections::HashMap::new(),
        },
        inputs: vec![ComputeBuffer {
            name: "input".to_string(),
            data: vec![1u8; memory as usize].into(),
            element_type: DataType::UInt8,
        }],
        output_size: memory as usize,
        hints: OptimizationHints {
            low_latency: false,
            energy_efficient: false,
            approximate: false,
            priority: 5,
        },
    }
}
