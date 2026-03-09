// SPDX-License-Identifier: AGPL-3.0-only
//! Generate reservoir models with random weights
//!
//! Creates .fbz models suitable for reservoir computing with different seeds.

use akida_reservoir_research::reservoir::{ReservoirConfig, ReservoirGenerator};
use akida_reservoir_research::ReservoirResult as Result;
use tracing::info;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Parse args manually (keeping it simple for now)
    let args: Vec<String> = std::env::args().collect();
    let seed = args
        .get(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(42);

    println!("╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                              ║");
    println!("║       🎲 Reservoir Generator 🎲                                             ║");
    println!("║                                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    info!("Configuration:");
    info!("  Seed: {}", seed);
    info!("  Size: 1000 neurons");
    info!("  Input: 784 dimensions");
    info!("  Output: 10 dimensions");
    info!("  Spectral radius: 0.9");

    // Create configuration
    let config = ReservoirConfig {
        seed,
        ..Default::default()
    };

    // Generate reservoir
    println!("\n🎲 Generating reservoir weights...\n");
    let generator = ReservoirGenerator::new(config);
    let (w_in, w_res) = generator.generate_weights()?;

    info!("W_in shape: {:?}", w_in.shape());
    info!("W_res shape: {:?}", w_res.shape());

    // Statistics
    println!("\n📊 Reservoir Statistics:\n");

    let w_in_mean = w_in.mean().unwrap_or(0.0);
    let w_in_std = w_in.std(0.0);
    println!("   W_in (Input weights):");
    println!("      Mean: {w_in_mean:.6}");
    println!("      Std:  {w_in_std:.6}");
    println!(
        "      Min:  {:.6}",
        w_in.iter().copied().fold(f32::INFINITY, f32::min)
    );
    println!(
        "      Max:  {:.6}",
        w_in.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    );

    let w_res_mean = w_res.mean().unwrap_or(0.0);
    let w_res_std = w_res.std(0.0);
    println!("\n   W_res (Reservoir weights):");
    println!("      Mean: {w_res_mean:.6}");
    println!("      Std:  {w_res_std:.6}");
    println!(
        "      Min:  {:.6}",
        w_res.iter().copied().fold(f32::INFINITY, f32::min)
    );
    println!(
        "      Max:  {:.6}",
        w_res.iter().copied().fold(f32::NEG_INFINITY, f32::max)
    );

    // Check echo state property (approximate)
    let frobenius_norm: f32 = w_res.iter().map(|&x| x * x).sum::<f32>().sqrt();
    println!("\n   Echo State Property:");
    println!("      Frobenius norm: {frobenius_norm:.6}");
    println!("      Target spectral radius: 0.9");

    if frobenius_norm < 1.5 {
        println!("      ✅ Likely satisfies echo state property");
    } else {
        println!("      ⚠️  May not satisfy echo state property (norm too large)");
    }

    // Save
    println!("\n💾 Would save to: reservoir_seed{seed}_size1000.npy");
    println!("\n   ⚠️  NOTE: .fbz conversion not yet implemented!");
    println!("   To create actual .fbz file:");
    println!("      1. Export weights to NumPy format (.npy)");
    println!("      2. Use BrainChip MetaTF to create Keras model");
    println!("      3. Load weights into model");
    println!("      4. Compile to Akida format (.fbz)");

    println!("\n✅ Reservoir generation complete!\n");

    Ok(())
}
