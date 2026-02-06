//! Comprehensive Auto-Tensor Demo
//! 
//! Demonstrates all scheduler-aware operations:
//! - Binary ops: add, sub, mul, div
//! - Activations: relu, sigmoid, tanh
//! - Linear algebra: matmul
//! - Convolution: conv2d

use barracuda::auto_tensor::AutoContext;
use barracuda::error::Result;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  🦈 Comprehensive Auto-Tensor Demo                          ║");
    println!("║  All operations with automatic hardware selection           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");
    
    println!("🔧 Initializing AutoContext...");
    let ctx = AutoContext::new().await?;
    
    println!("\n═══════════════════════════════════════════════════════════════");
    println!("🧮 Binary Operations");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    // Small binary ops (should prefer CPU)
    println!("━━━ Small Tensors [1000 elements] ━━━\n");
    let a_small = ctx.randn(vec![1000])?;
    let b_small = ctx.randn(vec![1000])?;
    
    let start = Instant::now();
    let _add_result = ctx.add(&a_small, &b_small)?;
    println!("  Add: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _sub_result = ctx.sub(&a_small, &b_small)?;
    println!("  Sub: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _mul_result = ctx.mul(&a_small, &b_small)?;
    println!("  Mul: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _div_result = ctx.div(&a_small, &b_small)?;
    println!("  Div: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    // Large binary ops (should prefer GPU)
    println!("━━━ Large Tensors [1M elements] ━━━\n");
    let a_large = ctx.randn(vec![1000, 1000])?;
    let b_large = ctx.randn(vec![1000, 1000])?;
    
    let start = Instant::now();
    let _add_result = ctx.add(&a_large, &b_large)?;
    println!("  Add: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _sub_result = ctx.sub(&a_large, &b_large)?;
    println!("  Sub: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _mul_result = ctx.mul(&a_large, &b_large)?;
    println!("  Mul: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _div_result = ctx.div(&a_large, &b_large)?;
    println!("  Div: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("🎯 Activation Functions");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    // Small activations
    println!("━━━ Small Tensors [1000 elements] ━━━\n");
    let x_small = ctx.randn(vec![1000])?;
    
    let start = Instant::now();
    let _relu = ctx.relu(&x_small)?;
    println!("  ReLU: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _sigmoid = ctx.sigmoid(&x_small)?;
    println!("  Sigmoid: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _tanh = ctx.tanh(&x_small)?;
    println!("  Tanh: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    // Large activations
    println!("━━━ Large Tensors [1M elements] ━━━\n");
    let x_large = ctx.randn(vec![1000, 1000])?;
    
    let start = Instant::now();
    let _relu = ctx.relu(&x_large)?;
    println!("  ReLU: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _sigmoid = ctx.sigmoid(&x_large)?;
    println!("  Sigmoid: {:.3} ms", start.elapsed().as_secs_f64() * 1000.0);
    
    let start = Instant::now();
    let _tanh = ctx.tanh(&x_large)?;
    println!("  Tanh: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("🔢 Linear Algebra");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    println!("━━━ Small MatMul (64×64) ━━━\n");
    let m_small = ctx.randn(vec![64, 64])?;
    let n_small = ctx.randn(vec![64, 64])?;
    
    let start = Instant::now();
    let _result = ctx.matmul(&m_small, &n_small)?;
    println!("  MatMul: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    println!("━━━ Large MatMul (1024×1024) ━━━\n");
    let m_large = ctx.randn(vec![1024, 1024])?;
    let n_large = ctx.randn(vec![1024, 1024])?;
    
    let start = Instant::now();
    let _result = ctx.matmul(&m_large, &n_large)?;
    println!("  MatMul: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("🖼️  Convolution");
    println!("═══════════════════════════════════════════════════════════════\n");
    
    println!("━━━ Small Conv2D (28×28 * 3×3) ━━━\n");
    let img_small = ctx.randn(vec![28, 28])?;
    let kernel_small = ctx.randn(vec![3, 3])?;
    
    let start = Instant::now();
    let _result = ctx.conv2d(&img_small, &kernel_small)?;
    println!("  Conv2D: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    println!("━━━ Large Conv2D (224×224 * 7×7) ━━━\n");
    let img_large = ctx.randn(vec![224, 224])?;
    let kernel_large = ctx.randn(vec![7, 7])?;
    
    let start = Instant::now();
    let _result = ctx.conv2d(&img_large, &kernel_large)?;
    println!("  Conv2D: {:.3} ms\n", start.elapsed().as_secs_f64() * 1000.0);
    
    println!("═══════════════════════════════════════════════════════════════");
    println!("🎉 Complete!");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("✅ All operations executed with automatic hardware selection");
    println!("✅ Zero manual device management required");
    println!("✅ Scheduler made intelligent routing decisions");
    println!();
    
    Ok(())
}
