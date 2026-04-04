// SPDX-License-Identifier: AGPL-3.0-only
//! Example: Parse Akida .fbz model file
//!
//! Demonstrates parsing of Akida model files with pure Rust.

use akida_models::prelude::*;

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("akida_models=debug")
        .init();

    println!("🧠 Akida Model Parser - Pure Rust\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Parse model file
    let model_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: cargo run --example parse_fbz -- <path_to_model.fbz>");
        eprintln!("Example: cargo run --example parse_fbz -- /path/to/model.fbz");
        std::process::exit(1);
    });

    println!("📂 Loading model: {model_path}\n");

    let model = Model::from_file(&model_path)?;

    println!("✅ Model loaded successfully!\n");

    // Display model info
    println!("📊 Model Information:");
    println!("   Version:      {}", model.version());
    println!("   Layers:       {}", model.layer_count());
    #[expect(
        clippy::cast_precision_loss,
        reason = "precision loss acceptable for this conversion"
    )] // KB display; precision loss acceptable
    let program_size_kb = model.program_size() as f32 / 1024.0;
    println!(
        "   Program size: {} bytes ({program_size_kb:.2} KB)\n",
        model.program_size()
    );

    // Display layers
    println!("🏗️  Model Architecture:");
    println!("┌────────────────────────────────────────────────┐");

    for (i, layer) in model.layers().iter().enumerate() {
        println!(
            "│  Layer {i}: {:20} {:15} │",
            layer.name,
            format!("({})", layer.layer_type)
        );
    }

    println!("└────────────────────────────────────────────────┘\n");

    // Display weight information
    println!("⚖️  Weight Data:");
    println!("   Weight blocks:  {}", model.weights().len());
    println!("   Total weights:  ~{}\n", model.total_weight_count());

    if !model.weights().is_empty() {
        for (i, weight) in model.weights().iter().enumerate() {
            println!(
                "   Block {i}: {} bytes ({}-bit quantization)",
                weight.data.len(),
                weight.quantization.bits
            );
        }
        println!();
    }

    println!("✨ Parse complete! Pure Rust FlatBuffers parsing working!\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}
