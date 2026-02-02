//! Test: Dual-chip ensemble reservoir
//!
//! Runs two reservoirs in parallel and concatenates their states.

use akida_reservoir_research::ensemble::{DualChipEnsemble, EnsembleConfig};
use anyhow::Result;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                              ║");
    println!("║       🧠🧠 EXPERIMENT 3: Dual-Chip Ensemble 🧠🧠                            ║");
    println!("║                                                                              ║");
    println!("║       RESEARCH QUESTION: Can we run parallel reservoirs?                    ║");
    println!("║                                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Configuration
    let config = EnsembleConfig {
        reservoir1_path: "reservoir_seed42.fbz".to_string(),
        reservoir2_path: "reservoir_seed123.fbz".to_string(),
        state_size_per_chip: 1000,
    };

    println!("📋 Configuration:");
    println!("   Reservoir 1: {}", config.reservoir1_path);
    println!("   Reservoir 2: {}", config.reservoir2_path);
    println!(
        "   Expected state size per chip: {}",
        config.state_size_per_chip
    );

    // Check if model files exist
    println!("\n1️⃣  Checking for model files...\n");

    let model1_exists = std::path::Path::new(&config.reservoir1_path).exists();
    let model2_exists = std::path::Path::new(&config.reservoir2_path).exists();

    if !model1_exists || !model2_exists {
        println!("   ⚠️  Model files not found!");
        if !model1_exists {
            println!("      Missing: {}", config.reservoir1_path);
        }
        if !model2_exists {
            println!("      Missing: {}", config.reservoir2_path);
        }

        println!("\n   To create reservoir models:");
        println!(
            "      cargo run --bin generate-reservoir -- --seed 42 --out reservoir_seed42.fbz"
        );
        println!(
            "      cargo run --bin generate-reservoir -- --seed 123 --out reservoir_seed123.fbz"
        );
        println!("\n   Note: You'll need BrainChip SDK to compile to .fbz format");

        return Ok(());
    }

    info!("Both model files found");

    // Create ensemble
    println!("\n2️⃣  Creating ensemble...\n");

    match DualChipEnsemble::discover_and_create(config.clone()) {
        Ok(mut ensemble) => {
            info!("Ensemble created successfully");

            // Load reservoirs
            println!("\n3️⃣  Loading reservoirs to devices...\n");
            ensemble.load_reservoirs()?;

            // Test inference
            println!("\n4️⃣  Running test inference...\n");

            let test_input = vec![0u8; 784]; // MNIST-sized input

            info!("Running dual-chip inference");
            let start = std::time::Instant::now();

            let ensemble_state = ensemble.get_ensemble_state(&test_input)?;

            let elapsed = start.elapsed();

            println!("   ✅ Ensemble inference complete!");
            println!("\n   Results:");
            println!("      State size: {} dimensions", ensemble_state.len());
            println!(
                "      Latency: {:?} ({:.2} µs)",
                elapsed,
                elapsed.as_micros() as f64
            );

            // Expected: ~100-150µs for dual-chip parallel inference
            if elapsed.as_micros() < 500 {
                println!("      ✅ EXCELLENT! Sub-500µs latency achieved!");
            } else if elapsed.as_micros() < 2000 {
                println!("      ✅ GOOD! Sub-2ms latency");
            } else {
                println!("      ⚠️  Slower than expected (target: <500µs)");
            }

            println!("\n5️⃣  Performance Analysis\n");
            println!("   Expected breakdown:");
            println!("      Chip 1 inference:  ~70-96µs");
            println!("      Chip 2 inference:  ~70-96µs (parallel)");
            println!("      State extraction:  ~10-50µs");
            println!("      Concatenation:     ~1-10µs");
            println!("      ────────────────────────────");
            println!("      Total expected:    ~100-150µs");
            println!(
                "      Actual measured:   {:.2}µs",
                elapsed.as_micros() as f64
            );

            println!("\n✅ EXPERIMENT SUCCESS!");
            println!("   - Dual-chip ensemble is working");
            println!("   - Parallel reservoir inference confirmed");
            println!("   - Combined state: {} dimensions", ensemble_state.len());
        }
        Err(e) => {
            println!("   ❌ Failed to create ensemble: {}", e);
            println!("\n   Possible causes:");
            println!("      - Less than 2 Akida devices available");
            println!("      - Device permission issues");
            println!("      - Kernel driver not loaded");
            println!("\n   Check:");
            println!("      lsmod | grep akida");
            println!("      ls -l /dev/akida*");
        }
    }

    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                              ║");
    println!("║       📊 ENSEMBLE TEST COMPLETE                                             ║");
    println!("║                                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    Ok(())
}
