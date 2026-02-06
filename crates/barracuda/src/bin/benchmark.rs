//! BarraCUDA Benchmark Binary
//!
//! Run comprehensive benchmarks comparing BarraCUDA vs CUDA

use barracuda::benchmarks::{BenchmarkConfig, BenchmarkSuite};
use barracuda::error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    println!("🦈 BarraCUDA Benchmark Suite");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    
    // Configure benchmarks
    let config = BenchmarkConfig {
        warmup_iterations: 10,
        measurement_iterations: 100,
        compare_cuda: true,
        ..Default::default()
    };
    
    // Run benchmark suite
    let mut suite = BenchmarkSuite::new(config);
    suite.run_all().await?;
    
    // Print summary
    let summary = suite.summary();
    println!();
    println!("{}", summary);
    
    // TODO: Save results to file
    // let report = ReportGenerator::new(suite.results().to_vec());
    // report.save_to_file("benchmark_results.md")?;
    
    Ok(())
}
