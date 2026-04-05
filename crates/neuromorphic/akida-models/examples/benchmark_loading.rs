// SPDX-License-Identifier: AGPL-3.0-or-later
//! Benchmark: Model loading performance
//!
//! Compares Rust loading performance across different scenarios.

use akida_driver::DeviceManager;
use akida_models::Model;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("warn") // Quiet for benchmarking
        .init();

    println!("🏃 Akida Loading Benchmark\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Get model path
    let model_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example benchmark_loading -- <path_to_model.fbz>");
        eprintln!("Example: cargo run --example benchmark_loading -- /path/to/model.fbz");
        std::process::exit(1);
    });

    println!("📂 Model: {model_path}\n");

    // Parse model once
    println!("1️⃣  Parsing model...");
    let parse_start = Instant::now();
    let model = Model::from_file(&model_path)?;
    let parse_time = parse_start.elapsed();
    println!("   ✅ Parsed in {parse_time:?}\n");

    // Discover devices
    println!("2️⃣  Discovering devices...");
    let manager = DeviceManager::discover()?;

    if manager.device_count() == 0 {
        println!("❌ No devices found!");
        return Ok(());
    }

    println!("   ✅ Found {} device(s)\n", manager.device_count());

    // Benchmark: Cold load (first time)
    println!("3️⃣  Cold load (first time)...");
    let mut device = manager.open(0)?;

    let cold_start = Instant::now();
    let cold_metrics = model.load_to_device(&mut device)?;
    let cold_total = cold_start.elapsed();

    let cold_duration = cold_metrics.duration;
    println!("   Transfer: {cold_duration:?}");
    println!("   Total:    {cold_total:?}\n");

    drop(device);

    // Benchmark: Warm loads (repeated)
    println!("4️⃣  Warm loads (10 iterations)...");
    let mut warm_times = Vec::new();

    for i in 0..10 {
        let mut device = manager.open(0)?;

        let start = Instant::now();
        let _ = model.load_to_device(&mut device)?;
        let elapsed = start.elapsed();

        warm_times.push(elapsed);

        if i == 0 || i == 9 {
            println!("   Run {}: {elapsed:?}", i + 1);
        } else if i == 1 {
            println!("   ...");
        }

        drop(device);
    }

    // Calculate statistics
    let warm_min = warm_times.iter().min().unwrap();
    let warm_max = warm_times.iter().max().unwrap();
    let warm_avg = warm_times.iter().sum::<std::time::Duration>()
        / u32::try_from(warm_times.len()).expect("warm_times non-empty");

    println!("\n   Min:     {warm_min:?}");
    println!("   Max:     {warm_max:?}");
    println!("   Average: {warm_avg:?}\n");

    // Results summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    println!("📊 BENCHMARK RESULTS\n");

    println!("Parse Performance:");
    println!("   Time:       {parse_time:?}");
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // MB/s display; precision loss acceptable
    let parse_throughput = (model.program_size() as f64 / 1_048_576.0) / parse_time.as_secs_f64();
    println!("   Throughput: {parse_throughput:.2} MB/s\n");

    println!("Load Performance (Cold):");
    println!("   Transfer:   {cold_duration:?}");
    println!("   Total:      {cold_total:?}");
    let cold_throughput = cold_metrics.throughput_mbps;
    println!("   Throughput: {cold_throughput:.2} MB/s\n");

    println!("Load Performance (Warm, N=10):");
    println!("   Min:        {warm_min:?}");
    println!("   Avg:        {warm_avg:?}");
    println!("   Max:        {warm_max:?}");

    // Calculate improvement
    let speedup = cold_total.as_secs_f64() / warm_avg.as_secs_f64();
    println!("   Speedup:    {speedup:.2}x vs cold\n");

    // Model info
    println!("Model Statistics:");
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // KB display; precision loss acceptable
    let program_size_kb = model.program_size() as f64 / 1024.0;
    println!(
        "   Size:       {} bytes ({program_size_kb:.2} KB)",
        model.program_size()
    );
    println!("   Layers:     {}", model.layer_count());
    println!("   Weights:    {} blocks", model.weights().len());

    println!("\n🎉 Benchmark complete!\n");

    Ok(())
}
