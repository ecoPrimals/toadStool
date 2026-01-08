//! Pure Rust GPU Demo using wgpu
//! 
//! No FFI, no unsafe - just modern idiomatic Rust!

use anyhow::Result;
use ml_inference_showcase::wgpu_executor::WgpuExecutor;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║  Pure Rust GPU Demo - wgpu (WebGPU)                     ║");
    println!("║  No FFI, No Unsafe - Modern Idiomatic Rust!             ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    
    // Create executor (pure Rust!)
    println!("🚀 Initializing pure Rust GPU executor...");
    let executor = WgpuExecutor::new().await?;
    println!("✓ GPU: {}", executor.gpu_info());
    println!();
    
    // Test ReLU
    println!("─────────────────────────────────────────────────────────");
    println!("TEST 1: ReLU Activation");
    println!("─────────────────────────────────────────────────────────");
    
    let input = vec![-2.0, -1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0];
    println!("Input:  {:?}", input);
    
    let start = Instant::now();
    let output = executor.execute_relu(&input).await?;
    let elapsed = start.elapsed();
    
    println!("Output: {:?}", output);
    println!("Time:   {:.3} ms", elapsed.as_secs_f64() * 1000.0);
    
    // Verify correctness
    let expected: Vec<f32> = input.iter().map(|&x| x.max(0.0)).collect();
    let max_diff: f32 = output.iter().zip(expected.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    
    println!("Max difference: {:.6}", max_diff);
    println!("Correctness: {}", if max_diff < 1e-5 { "✅ PASS" } else { "❌ FAIL" });
    println!();
    
    // Test Matrix Multiplication
    println!("─────────────────────────────────────────────────────────");
    println!("TEST 2: Matrix Multiplication");
    println!("─────────────────────────────────────────────────────────");
    
    // 2x3 * 3x2 = 2x2
    let a = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    let b = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
    
    println!("A (2x3): {:?}", a);
    println!("B (3x2): {:?}", b);
    
    let start = Instant::now();
    let c = executor.execute_matmul(&a, &b, 2, 3, 2).await?;
    let elapsed = start.elapsed();
    
    println!("C (2x2): {:?}", c);
    println!("Time:    {:.3} ms", elapsed.as_secs_f64() * 1000.0);
    
    // Expected: [[22, 28], [49, 64]]
    let expected = vec![22.0, 28.0, 49.0, 64.0];
    let max_diff: f32 = c.iter().zip(expected.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f32::max);
    
    println!("Expected: {:?}", expected);
    println!("Max difference: {:.6}", max_diff);
    println!("Correctness: {}", if max_diff < 1e-3 { "✅ PASS" } else { "❌ FAIL" });
    println!();
    
    // Larger benchmark
    println!("─────────────────────────────────────────────────────────");
    println!("BENCHMARK: Large Vector ReLU");
    println!("─────────────────────────────────────────────────────────");
    
    let sizes = vec![1_000, 10_000, 100_000, 1_000_000];
    
    for size in sizes {
        let input: Vec<f32> = (0..size).map(|i| (i as f32 - size as f32 / 2.0) / 100.0).collect();
        
        let start = Instant::now();
        let output = executor.execute_relu(&input).await?;
        let elapsed = start.elapsed();
        
        let throughput = size as f64 / elapsed.as_secs_f64() / 1_000_000.0;
        
        println!("  Size: {:>10} elements | Time: {:>8.3} ms | Throughput: {:>8.2} M elem/s",
            size, elapsed.as_secs_f64() * 1000.0, throughput);
        
        // Quick correctness check
        let correct = output.iter().zip(input.iter())
            .all(|(out, inp)| (out - inp.max(0.0)).abs() < 1e-4);
        if !correct {
            println!("    ⚠️  Correctness issue detected!");
        }
    }
    
    println!();
    println!("═══════════════════════════════════════════════════════════");
    println!("🎉 Pure Rust GPU Demo Complete!");
    println!("═══════════════════════════════════════════════════════════");
    println!();
    println!("Key Achievements:");
    println!("  ✅ Zero FFI - Pure Rust implementation");
    println!("  ✅ Zero Unsafe - Type-safe GPU programming");
    println!("  ✅ Cross-Platform - Vulkan, Metal, DX12, WebGPU");
    println!("  ✅ Future-Proof - WebGPU standard");
    println!("  ✅ Idiomatic - Modern Rust patterns");
    println!();
    println!("This is the future of GPU computing in Rust! 🦀");
    println!();
    
    Ok(())
}

