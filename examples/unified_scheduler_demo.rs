//! Unified Scheduler Demo
//!
//! Demonstrates automatic hardware selection across CPU, GPU, TPU, and NPU
//!
//! **What This Shows:**
//! - Automatic hardware discovery
//! - Smart operation routing
//! - Size-based hardware selection
//! - Performance characteristics

use barracuda::scheduler::UnifiedScheduler;
use barracuda::unified_math::{DType, MathOp, TensorDescriptor};
use barracuda::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🦈 BarraCUDA Unified Scheduler Demo");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    // Create scheduler (auto-discovers all hardware)
    let scheduler = UnifiedScheduler::new().await?;
    
    // Print what we found
    scheduler.print_summary();
    
    println!("\n🎯 Testing Automatic Hardware Selection\n");
    
    // Test 1: Small operation (should prefer CPU)
    println!("📊 Test 1: Small ReLU [10x10]");
    let small_desc = TensorDescriptor::new(vec![10, 10], DType::F32);
    let small_op = MathOp::ReLU;
    let small_exec = scheduler.select_executor(&small_op, &[small_desc]);
    println!("   → Selected: {} ({:?})", small_exec.name(), small_exec.hardware_type());
    println!("   → Reason: Too small for GPU transfer overhead\n");
    
    // Test 2: Medium operation
    println!("📊 Test 2: Medium Matrix Multiply [1000x1000]");
    let medium_desc = TensorDescriptor::new(vec![1000, 1000], DType::F32);
    let medium_op = MathOp::MatMul { transpose_a: false, transpose_b: false };
    let medium_exec = scheduler.select_executor(&medium_op, &[medium_desc.clone(), medium_desc]);
    println!("   → Selected: {} ({:?})", medium_exec.name(), medium_exec.hardware_type());
    println!("   → Reason: Good balance point for GPU\n");
    
    // Test 3: Large operation (should prefer GPU/TPU)
    println!("📊 Test 3: Large Matrix Multiply [4096x4096]");
    let large_desc = TensorDescriptor::new(vec![4096, 4096], DType::F32);
    let large_op = MathOp::MatMul { transpose_a: false, transpose_b: false };
    let large_exec = scheduler.select_executor(&large_op, &[large_desc.clone(), large_desc]);
    println!("   → Selected: {} ({:?})", large_exec.name(), large_exec.hardware_type());
    println!("   → Reason: GPU/TPU excels at large parallel operations\n");
    
    // Test 4: Convolution (should prefer GPU)
    println!("📊 Test 4: Convolution [256, 256, 3]");
    let conv_desc = TensorDescriptor::new(vec![1, 3, 256, 256], DType::F32);
    let conv_op = MathOp::Conv2D {
        stride: (1, 1),
        padding: (1, 1),
        dilation: (1, 1),
        groups: 1,
    };
    let conv_exec = scheduler.select_executor(&conv_op, &[conv_desc]);
    println!("   → Selected: {} ({:?})", conv_exec.name(), conv_exec.hardware_type());
    println!("   → Reason: GPU optimized for convolutions\n");
    
    // Test 5: Reduction (depends on size)
    println!("📊 Test 5: Large Reduction [10M elements]");
    let reduce_desc = TensorDescriptor::new(vec![10_000_000], DType::F32);
    let reduce_op = MathOp::ReduceSum { dim: None, keepdim: false };
    let reduce_exec = scheduler.select_executor(&reduce_op, &[reduce_desc]);
    println!("   → Selected: {} ({:?})", reduce_exec.name(), reduce_exec.hardware_type());
    println!("   → Reason: Tree reduction efficient on GPU\n");
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ Scheduler automatically picks the best hardware!");
    println!("   • Small ops → CPU (avoid transfer overhead)");
    println!("   • Large ops → GPU/TPU (parallel advantage)");
    println!("   • Always works → CPU fallback guaranteed");
    
    Ok(())
}
