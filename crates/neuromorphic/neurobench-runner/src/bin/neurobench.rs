// SPDX-License-Identifier: AGPL-3.0-only
//! `NeuroBench` CLI Runner
//!
//! Run `NeuroBench` benchmarks on Akida `NPU` hardware.
//!
//! ## Usage
//!
//! ```bash
//! # Run all benchmarks
//! neurobench --device 0000:a1:00.0 --all
//!
//! # Run specific benchmark
//! neurobench --device 0000:a1:00.0 --benchmark dvs-gesture
//!
//! # Run with synthetic data (no dataset required)
//! neurobench --device 0000:a1:00.0 --benchmark keyword-fscil --synthetic
//!
//! # List available benchmarks
//! neurobench --list
//! ```

use neurobench_runner::{
    Benchmark, BenchmarkConfig, BenchmarkResult, Harness, HarnessConfig, Result,
};
use std::env;
use tracing::{Level, error, info};
use tracing_subscriber::FmtSubscriber;

fn main() {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber");

    if let Err(e) = run() {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Parse arguments
    let mut device_id = "0000:a1:00.0".to_string();
    let mut benchmark_name: Option<String> = None;
    let mut run_all = false;
    let mut synthetic = false;
    let mut iterations = 1000;
    let mut data_dir = "data/neurobench".to_string();
    let mut models_dir = "models/akida".to_string();
    let mut show_list = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--device" | "-d" => {
                i += 1;
                if i < args.len() {
                    device_id.clone_from(&args[i]);
                }
            }
            "--benchmark" | "-b" => {
                i += 1;
                if i < args.len() {
                    benchmark_name = Some(args[i].clone());
                }
            }
            "--all" | "-a" => {
                run_all = true;
            }
            "--synthetic" | "-s" => {
                synthetic = true;
            }
            "--iterations" | "-n" => {
                i += 1;
                if i < args.len() {
                    iterations = args[i].parse().unwrap_or(1000);
                }
            }
            "--data-dir" => {
                i += 1;
                if i < args.len() {
                    data_dir.clone_from(&args[i]);
                }
            }
            "--models-dir" => {
                i += 1;
                if i < args.len() {
                    models_dir.clone_from(&args[i]);
                }
            }
            "--list" | "-l" => {
                show_list = true;
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // List benchmarks
    if show_list {
        list_benchmarks();
        return Ok(());
    }

    // Determine which benchmarks to run
    let benchmarks: Vec<Benchmark> = if run_all {
        vec![
            Benchmark::DvsGesture,
            Benchmark::KeywordFscil,
            Benchmark::ChaoticFunction,
            Benchmark::NhpMotor,
            Benchmark::EventCamera,
        ]
    } else if let Some(name) = benchmark_name {
        vec![parse_benchmark(&name)?]
    } else {
        eprintln!("No benchmark specified. Use --benchmark <name> or --all");
        print_help();
        std::process::exit(1);
    };

    // Initialize harness
    info!("Initializing NeuroBench harness...");
    let config = HarnessConfig {
        device_id,
        data_dir,
        models_dir,
        ..Default::default()
    };
    let mut harness = Harness::with_config(config)?;
    info!("Device: {}", harness.device_info());

    // Configure benchmarks
    let bench_config = BenchmarkConfig {
        num_iterations: iterations,
        warmup_iterations: 100,
        quantized: true,
        measure_power: true,
        seed: 42,
    };

    // Run benchmarks
    let mut results: Vec<BenchmarkResult> = Vec::new();

    for benchmark in &benchmarks {
        info!("\n{}", "=".repeat(60));
        info!("Running: {}", benchmark.description());
        info!("{}", "=".repeat(60));

        let result = if synthetic {
            harness.run_synthetic(*benchmark, &bench_config)?
        } else {
            harness.run(*benchmark, &bench_config)?
        };

        result.print_summary();
        results.push(result);
    }

    // Print summary table
    if results.len() > 1 {
        print_summary_table(&results);
    }

    Ok(())
}

fn parse_benchmark(name: &str) -> Result<Benchmark> {
    match name.to_lowercase().as_str() {
        "dvs-gesture" | "dvsgesture" | "dvs_gesture" => Ok(Benchmark::DvsGesture),
        "keyword-fscil" | "keywordfscil" | "keyword_fscil" | "kws" => Ok(Benchmark::KeywordFscil),
        "chaotic" | "chaotic-function" | "lorenz" => Ok(Benchmark::ChaoticFunction),
        "nhp-motor" | "nhpmotor" | "nhp_motor" | "motor" => Ok(Benchmark::NhpMotor),
        "event-camera" | "eventcamera" | "event_camera" => Ok(Benchmark::EventCamera),
        _ => Err(neurobench_runner::Error::BenchmarkFailed(format!(
            "Unknown benchmark: {name}. Use --list to see available benchmarks."
        ))),
    }
}

fn list_benchmarks() {
    println!("\nAvailable NeuroBench Benchmarks:");
    println!("{}", "=".repeat(60));

    for benchmark in [
        Benchmark::DvsGesture,
        Benchmark::KeywordFscil,
        Benchmark::ChaoticFunction,
        Benchmark::NhpMotor,
        Benchmark::EventCamera,
    ] {
        println!("\n  {benchmark:?}");
        println!("    {}", benchmark.description());
        println!("    Classes: {}", benchmark.num_classes());
        println!("    Input shape: {:?}", benchmark.input_shape());
    }

    println!("\n{}", "=".repeat(60));
}

fn print_summary_table(results: &[BenchmarkResult]) {
    println!("\n{}", "=".repeat(80));
    println!("SUMMARY");
    println!("{}", "=".repeat(80));
    println!(
        "{:<20} {:>10} {:>12} {:>12} {:>12}",
        "Benchmark", "Accuracy", "Throughput", "Mean (ms)", "Power (mW)"
    );
    println!("{}", "-".repeat(80));

    for result in results {
        let power = result
            .mean_power_mw
            .map_or_else(|| "-".to_string(), |p| format!("{p:.1}"));

        println!(
            "{:<20} {:>9.1}% {:>10.1}/s {:>10.3}ms {:>12}",
            format!("{:?}", result.benchmark),
            result.accuracy * 100.0,
            result.throughput,
            result.mean_latency.as_secs_f64() * 1000.0,
            power
        );
    }

    println!("{}", "=".repeat(80));
}

fn print_help() {
    println!(
        r"
NeuroBench CLI Runner

USAGE:
    neurobench [OPTIONS]

OPTIONS:
    -d, --device <ADDR>      PCIe address of Akida device (default: 0000:a1:00.0)
    -b, --benchmark <NAME>   Run specific benchmark
    -a, --all                Run all benchmarks
    -s, --synthetic          Use synthetic data (no dataset required)
    -n, --iterations <N>     Number of iterations (default: 1000)
    --data-dir <PATH>        Path to dataset directory (default: data/neurobench)
    --models-dir <PATH>      Path to model directory (default: models/akida)
    -l, --list               List available benchmarks
    -h, --help               Show this help message

BENCHMARKS:
    dvs-gesture      DVS Gesture recognition (11 classes)
    keyword-fscil    Few-shot keyword spotting (35 classes)
    chaotic          Chaotic function prediction (regression)
    nhp-motor        Neural prosthetics motor decoding
    event-camera     Event camera object detection

EXAMPLES:
    # Run DVS Gesture benchmark
    neurobench --device 0000:a1:00.0 --benchmark dvs-gesture

    # Run all benchmarks with synthetic data
    neurobench --all --synthetic

    # Run keyword spotting with custom data directory
    neurobench -b keyword-fscil --data-dir /data/speech_commands
"
    );
}
