//! Unified Scheduler Demo Binary
//!
//! Demonstrates automatic hardware selection

use barracuda::scheduler::UnifiedScheduler;
use barracuda::unified_math::{DType, MathOp, TensorDescriptor};
use barracuda::error::Result;

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
    
    // Test different operation sizes
    test_operation(&scheduler, "Tiny ReLU", vec![10, 10], MathOp::ReLU);
    test_operation(&scheduler, "Small ReLU", vec![100, 100], MathOp::ReLU);
    test_operation(&scheduler, "Medium ReLU", vec![1000, 1000], MathOp::ReLU);
    test_operation(&scheduler, "Large ReLU", vec![4096, 4096], MathOp::ReLU);
    
    println!();
    
    // Test matrix operations
    let matmul = MathOp::MatMul { transpose_a: false, transpose_b: false };
    test_matmul(&scheduler, "Tiny MatMul", vec![10, 10], matmul.clone());
    test_matmul(&scheduler, "Small MatMul", vec![100, 100], matmul.clone());
    test_matmul(&scheduler, "Medium MatMul", vec![1000, 1000], matmul.clone());
    test_matmul(&scheduler, "Large MatMul", vec![4096, 4096], matmul);
    
    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ Scheduler automatically picks the best hardware!");
    println!("   • Tiny/Small → CPU (avoid transfer overhead)");
    println!("   • Large → GPU/TPU (parallel advantage)");
    println!("   • Always works → CPU fallback guaranteed");
    
    Ok(())
}

fn test_operation(scheduler: &UnifiedScheduler, name: &str, shape: Vec<usize>, op: MathOp) {
    let desc = TensorDescriptor::new(shape.clone(), DType::F32);
    let shape_str = format_shape(&desc.shape);
    let exec = scheduler.select_executor(&op, &[desc]);
    let score = exec.score_operation(&op, &[TensorDescriptor::new(shape, DType::F32)]);
    
    println!("📊 {} [{}]", name, shape_str);
    println!("   → Selected: {} (score: {:.2})", exec.name(), score);
}

fn test_matmul(scheduler: &UnifiedScheduler, name: &str, shape: Vec<usize>, op: MathOp) {
    let desc = TensorDescriptor::new(shape.clone(), DType::F32);
    let exec = scheduler.select_executor(&op, &[desc.clone(), desc.clone()]);
    let score = exec.score_operation(&op, &[desc.clone(), desc]);
    
    println!("🔢 {} [{}x{}]", name, shape[0], shape[1]);
    println!("   → Selected: {} (score: {:.2})", exec.name(), score);
}

fn format_shape(shape: &[usize]) -> String {
    shape.iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("x")
}
