//! Experiment 001: Workgroup Size Sweep for MatMul
//!
//! **Hypothesis**: Different workgroup sizes will have different performance
//! characteristics on WebGPU, and the optimal size may differ from CUDA's typical 256.
//!
//! **Variables**:
//! - Workgroup sizes: 32, 64, 128, 256, 512, 1024
//! - Matrix sizes: 256×256, 512×512, 1024×1024, 2048×2048
//!
//! **Controls**:
//! - Same GPU
//! - Same data (random but reproducible)
//! - Same algorithm (naive MatMul)
//!
//! **Measurements**:
//! - Execution time
//! - Statistical significance (10 runs)
//!
//! **Expected Outcome**: Optimal workgroup size per matrix size

use ml_inference_showcase::wgpu::WgpuExecutor;
use serde_json::json;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔬 Experiment 001: Workgroup Size Sweep for MatMul");
    println!("==================================================\n");
    
    // Initialize GPU
    let executor = WgpuExecutor::new().await?;
    
    println!("Hardware: {}", executor.get_adapter_info().name);
    println!("Backend: {:?}\n", executor.get_adapter_info().backend);
    
    // Matrix sizes to test
    let matrix_sizes = vec![256, 512, 1024, 2048];
    
    // Workgroup sizes to test (powers of 2)
    let workgroup_sizes = vec![32, 64, 128, 256, 512, 1024];
    
    println!("Matrix Sizes: {:?}", matrix_sizes);
    println!("Workgroup Sizes: {:?}\n", workgroup_sizes);
    
    // Results storage
    let mut all_results = Vec::new();
    
    // Run experiments
    for size in &matrix_sizes {
        println!("\n📊 Testing Matrix Size: {}×{}", size, size);
        println!("{}", "=".repeat(50));
        
        // Generate test data (reproducible)
        let a: Vec<f32> = (0..*size * *size).map(|i| (i as f32).sin()).collect();
        let b: Vec<f32> = (0..*size * *size).map(|i| (i as f32).cos()).collect();
        
        for &wg_size in &workgroup_sizes {
            print!("  Workgroup {:<4}: ", wg_size);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            // Warmup (3 runs)
            for _ in 0..3 {
                let _ = executor.execute_matmul(&a, &b, *size, *size, *size).await?;
            }
            
            // Measurement runs (10 runs)
            let mut times = Vec::new();
            for _ in 0..10 {
                let start = std::time::Instant::now();
                let _ = executor.execute_matmul(&a, &b, *size, *size, *size).await?;
                let elapsed = start.elapsed();
                times.push(elapsed.as_micros() as f64);
            }
            
            // Calculate statistics
            let mean = times.iter().sum::<f64>() / times.len() as f64;
            let variance = times.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (times.len() - 1) as f64;
            let std_dev = variance.sqrt();
            
            let mut sorted = times.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = sorted[sorted.len() / 2];
            
            println!("{:>8.2} μs (±{:.2} μs)", mean, std_dev);
            
            // Store result
            all_results.push(ExperimentResult {
                matrix_size: *size,
                workgroup_size: wg_size,
                mean_us: mean,
                median_us: median,
                std_dev_us: std_dev,
                min_us: *sorted.first().unwrap(),
                max_us: *sorted.last().unwrap(),
            });
        }
    }
    
    // Analysis
    println!("\n\n📈 ANALYSIS");
    println!("==================================================\n");
    
    for size in &matrix_sizes {
        println!("Matrix Size: {}×{}", size, size);
        
        let results: Vec<_> = all_results.iter()
            .filter(|r| r.matrix_size == *size)
            .collect();
        
        // Find optimal workgroup size
        let optimal = results.iter()
            .min_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();
        
        println!("  Optimal Workgroup: {} ({:.2} μs)", optimal.workgroup_size, optimal.mean_us);
        
        // Find worst
        let worst = results.iter()
            .max_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();
        
        let speedup = worst.mean_us / optimal.mean_us;
        println!("  Worst Workgroup: {} ({:.2} μs)", worst.workgroup_size, worst.mean_us);
        println!("  Speedup: {:.2}x\n", speedup);
    }
    
    // Save results to JSON
    let results_json = serde_json::to_string_pretty(&all_results)?;
    std::fs::write("experiment_001_results.json", results_json)?;
    println!("✅ Results saved to: experiment_001_results.json");
    
    // Generate CSV for easy plotting
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
    std::fs::write("experiment_001_results.csv", csv)?;
    println!("✅ CSV saved to: experiment_001_results.csv");
    
    println!("\n🎯 EXPERIMENT COMPLETE!");
    
    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ExperimentResult {
    matrix_size: usize,
    workgroup_size: usize,
    mean_us: f64,
    median_us: f64,
    std_dev_us: f64,
    min_us: f64,
    max_us: f64,
}
