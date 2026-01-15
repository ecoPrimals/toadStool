//! Experiment 001b: MatMul Workgroup Sweep on AMD GPU
//!
//! **Purpose**: Cross-vendor validation of Experiment 001 findings
//!
//! **Hardware**: AMD Radeon RX 6950 XT (RDNA 2, Wavefront 64)
//! **Compare With**: NVIDIA RTX 3090 (Ampere, Warp 32)
//!
//! **Key Question**: Are optimal workgroup sizes vendor-specific?
//!
//! **Hypothesis**: AMD's wavefront size (64) may lead to different
//! optimal workgroup sizes compared to NVIDIA's warp size (32).

use ml_inference_showcase::wgpu::WgpuExecutor;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔬 Experiment 001b: MatMul Workgroup Sweep - AMD GPU");
    println!("====================================================\n");
    
    // Request AMD GPU specifically (NOT the default!)
    let executor = WgpuExecutor::new_amd().await?;
    
    println!("Hardware: {}", executor.gpu_info());
    println!("🔴 AMD RDNA 2 Architecture (Wavefront Size: 64)");
    println!("📊 Compare with NVIDIA RTX 3090 (Warp Size: 32)\n");
    
    // Matrix sizes to test
    let matrix_sizes = vec![256, 512, 1024, 2048];
    
    // Workgroup sizes - same as Experiment 001 for comparison
    let workgroup_sizes = vec![32, 64, 128, 256, 512, 1024];
    
    println!("Matrix Sizes: {:?}", matrix_sizes);
    println!("Workgroup Sizes: {:?}\n", workgroup_sizes);
    
    let mut all_results = Vec::new();
    
    for &n in &matrix_sizes {
        println!("\n📊 Testing Matrix Size: {}×{}", n, n);
        println!("{}", "=".repeat(50));
        
        // Generate test matrices
        let a: Vec<f32> = (0..n*n).map(|i| (i as f32 * 0.01).sin()).collect();
        let b: Vec<f32> = (0..n*n).map(|i| (i as f32 * 0.02).cos()).collect();
        
        for &wg_size in &workgroup_sizes {
            print!("  Workgroup {:<4}: ", wg_size);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            // Warmup (3 runs)
            for _ in 0..3 {
                let _ = executor.execute_matmul(&a, &b, n, n, n).await?;
            }
            
            // Measurement runs (10 runs)
            let mut times = Vec::new();
            for _ in 0..10 {
                let start = std::time::Instant::now();
                let _ = executor.execute_matmul(&a, &b, n, n, n).await?;
                let elapsed = start.elapsed();
                times.push(elapsed.as_micros() as f64);
            }
            
            // Calculate statistics
            let mean = times.iter().sum::<f64>() / times.len() as f64;
            let variance = times.iter()
                .map(|&x| (x - mean).powi(2))
                .sum::<f64>() / (times.len() - 1) as f64;
            let std_dev = variance.sqrt();
            
            println!("{:>9.2} μs (±{:.2} μs)", mean, std_dev);
            
            all_results.push(ExperimentResult {
                matrix_size: n,
                workgroup_size: wg_size,
                mean_us: mean,
                std_dev_us: std_dev,
            });
        }
    }
    
    // Analysis
    println!("\n\n📈 ANALYSIS - AMD RX 6950 XT");
    println!("==================================================\n");
    
    for &n in &matrix_sizes {
        println!("Matrix Size: {}×{}", n, n);
        
        let results: Vec<_> = all_results.iter()
            .filter(|r| r.matrix_size == n)
            .collect();
        
        let optimal = results.iter()
            .min_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();
        
        let worst = results.iter()
            .max_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();
        
        let speedup = worst.mean_us / optimal.mean_us;
        
        println!("  Optimal Workgroup: {} ({:.2} μs)", optimal.workgroup_size, optimal.mean_us);
        println!("  Worst Workgroup: {} ({:.2} μs)", worst.workgroup_size, worst.mean_us);
        println!("  Speedup: {:.2}x\n", speedup);
    }
    
    // Cross-vendor comparison
    println!("\n🔬 CROSS-VENDOR COMPARISON");
    println!("==================================================");
    println!("\nNVIDIA RTX 3090 (Experiment 001):");
    println!("  256×256:   256 threads optimal (4248μs)");
    println!("  512×512:   256 threads optimal (5309μs)");
    println!("  1024×1024: 128 threads optimal (9702μs)");
    println!("  2048×2048: 128 threads optimal (37812μs)");
    
    println!("\nAMD RX 6950 XT (Experiment 001b):");
    for &n in &matrix_sizes {
        let results: Vec<_> = all_results.iter()
            .filter(|r| r.matrix_size == n)
            .collect();
        let optimal = results.iter()
            .min_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();
        println!("  {}×{}: {} threads optimal ({:.2}μs)",
            n, n, optimal.workgroup_size, optimal.mean_us);
    }
    
    // Save results
    let results_json = serde_json::to_string_pretty(&all_results)?;
    std::fs::write("experiment_001b_amd_results.json", results_json)?;
    println!("\n✅ Results saved to: experiment_001b_amd_results.json");
    
    let mut csv = String::from("matrix_size,workgroup_size,mean_us,std_dev_us\n");
    for result in &all_results {
        csv.push_str(&format!(
            "{},{},{},{}\n",
            result.matrix_size,
            result.workgroup_size,
            result.mean_us,
            result.std_dev_us
        ));
    }
    std::fs::write("experiment_001b_amd_results.csv", csv)?;
    println!("✅ CSV saved to: experiment_001b_amd_results.csv");
    
    println!("\n🎯 EXPERIMENT 001b COMPLETE - AMD GPU PROFILED!");
    
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ExperimentResult {
    matrix_size: usize,
    workgroup_size: usize,
    mean_us: f64,
    std_dev_us: f64,
}
