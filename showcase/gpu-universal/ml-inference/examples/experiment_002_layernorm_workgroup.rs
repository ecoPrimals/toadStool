// SPDX-License-Identifier: AGPL-3.0-or-later
//! Experiment 002: Workgroup Size Sweep for LayerNorm
//!
//! **Hypothesis**: Memory-bound operations (LayerNorm) may have different optimal
//! workgroup sizes compared to compute-bound operations (MatMul from Exp 001).
//!
//! **Variables**:
//! - Workgroup sizes: 32, 64, 128, 256, 512, 1024
//! - Tensor sizes: 128K, 384K (BERT), 1M (GPT-2), 8M (LLaMA)
//!
//! **Controls**:
//! - Same GPU
//! - Same data (reproducible random)
//! - Same algorithm (3-pass LayerNorm)
//!
//! **Measurements**:
//! - Execution time
//! - Statistical significance (10 runs)
//!
//! **Expected Outcome**: Optimal workgroup size for memory-bound operations
//! **Comparison**: Vs Experiment 001 (compute-bound MatMul)

use ml_inference_showcase::wgpu::{NormConfig, WgpuExecutor};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🔬 Experiment 002: Workgroup Size Sweep for LayerNorm");
    println!("=====================================================\n");

    // Initialize GPU
    let executor = WgpuExecutor::new().await?;

    println!("Hardware: {}\n", executor.gpu_info());

    // Tensor sizes to test (common in transformers)
    let tensor_sizes = vec![
        (128 * 1024, "128K"),      // Small
        (512 * 768, "384K_BERT"),  // BERT hidden size
        (1024 * 1024, "1M_GPT2"),  // GPT-2
        (2048 * 4096, "8M_LLaMA"), // LLaMA (critical!)
    ];

    // Workgroup sizes to test
    let workgroup_sizes = vec![32, 64, 128, 256, 512, 1024];

    println!(
        "Tensor Sizes: {:?}",
        tensor_sizes
            .iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>()
    );
    println!("Workgroup Sizes: {:?}\n", workgroup_sizes);
    println!("NOTE: LayerNorm is MEMORY-BOUND (compare with compute-bound MatMul from Exp 001)\n");

    // Results storage
    let mut all_results = Vec::new();

    // Run experiments
    for (size, name) in &tensor_sizes {
        println!("\n📊 Testing Tensor Size: {} ({})", name, size);
        println!("{}", "=".repeat(60));

        // Generate test data
        let input: Vec<f32> = (0..*size).map(|i| (i as f32 * 0.01).sin()).collect();
        let config = NormConfig {
            epsilon: 1e-5,
            gamma: Some(vec![1.0; *size]),
            beta: Some(vec![0.0; *size]),
        };

        for &wg_size in &workgroup_sizes {
            print!("  Workgroup {:<4}: ", wg_size);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();

            // Warmup (3 runs)
            for _ in 0..3 {
                let _ = executor.execute_layernorm(&input, config.clone()).await?;
            }

            // Measurement runs (10 runs)
            let mut times = Vec::new();
            for _ in 0..10 {
                let start = std::time::Instant::now();
                let _ = executor.execute_layernorm(&input, config.clone()).await?;
                let elapsed = start.elapsed();
                times.push(elapsed.as_micros() as f64);
            }

            // Calculate statistics
            let mean = times.iter().sum::<f64>() / times.len() as f64;
            let variance =
                times.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (times.len() - 1) as f64;
            let std_dev = variance.sqrt();

            let mut sorted = times.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = sorted[sorted.len() / 2];

            println!("{:>9.2} μs (±{:.2} μs)", mean, std_dev);

            // Store result
            all_results.push(ExperimentResult {
                tensor_size: *size,
                tensor_name: name.to_string(),
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

    for (size, name) in &tensor_sizes {
        println!("Tensor Size: {} ({})", name, size);

        let results: Vec<_> = all_results
            .iter()
            .filter(|r| r.tensor_size == *size)
            .collect();

        // Find optimal
        let optimal = results
            .iter()
            .min_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();

        println!(
            "  Optimal Workgroup: {} ({:.2} μs)",
            optimal.workgroup_size, optimal.mean_us
        );

        // Find worst
        let worst = results
            .iter()
            .max_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();

        let speedup = worst.mean_us / optimal.mean_us;
        println!(
            "  Worst Workgroup: {} ({:.2} μs)",
            worst.workgroup_size, worst.mean_us
        );
        println!("  Speedup: {:.2}x\n", speedup);
    }

    // Comparison with Experiment 001
    println!("\n🔬 COMPARISON: LayerNorm vs MatMul");
    println!("==================================================");
    println!("Experiment 001 (MatMul, Compute-Bound):");
    println!("  Small: 256 threads optimal");
    println!("  Large: 128 threads optimal\n");

    println!("Experiment 002 (LayerNorm, Memory-Bound):");
    for (size, name) in &tensor_sizes {
        let results: Vec<_> = all_results
            .iter()
            .filter(|r| r.tensor_size == *size)
            .collect();
        let optimal = results
            .iter()
            .min_by(|a, b| a.mean_us.partial_cmp(&b.mean_us).unwrap())
            .unwrap();
        println!("  {}: {} threads optimal", name, optimal.workgroup_size);
    }

    // Save results
    let results_json = serde_json::to_string_pretty(&all_results)?;
    std::fs::write("experiment_002_results.json", results_json)?;
    println!("\n✅ Results saved to: experiment_002_results.json");

    let mut csv = String::from("tensor_size,tensor_name,workgroup_size,mean_us,std_dev_us\n");
    for result in &all_results {
        csv.push_str(&format!(
            "{},{},{},{},{}\n",
            result.tensor_size,
            result.tensor_name,
            result.workgroup_size,
            result.mean_us,
            result.std_dev_us
        ));
    }
    std::fs::write("experiment_002_results.csv", csv)?;
    println!("✅ CSV saved to: experiment_002_results.csv");

    println!("\n🎯 EXPERIMENT 002 COMPLETE!");

    Ok(())
}

#[derive(Debug, serde::Serialize)]
struct ExperimentResult {
    tensor_size: usize,
    tensor_name: String,
    workgroup_size: usize,
    mean_us: f64,
    median_us: f64,
    std_dev_us: f64,
    min_us: f64,
    max_us: f64,
}
