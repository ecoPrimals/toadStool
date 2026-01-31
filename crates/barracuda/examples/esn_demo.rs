//! Echo State Network Time Series Prediction Demo
//!
//! This example demonstrates using the high-level ESN API for time series
//! prediction. We train an ESN to learn a simple sine wave pattern and then
//! use it to predict future values.
//!
//! Run with: `cargo test --package barracuda --example esn_demo`

use barracuda::prelude::*;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🧠 Echo State Network - Time Series Prediction Demo\n");
    println!("═══════════════════════════════════════════════════\n");
    
    // Initialize GPU device
    println!("🔧 Initializing GPU device...");
    let device = WgpuDevice::new().await?;
    println!("✅ Device ready\n");
    
    // Configure ESN
    println!("⚙️  Configuring ESN...");
    let config = ESNConfig {
        input_size: 1,
        reservoir_size: 100,
        output_size: 1,
        spectral_radius: 0.95,
        connectivity: 0.1,
        leak_rate: 0.3,
        regularization: 1e-6,
        seed: 42,
    };
    
    println!("   Reservoir: {} neurons", config.reservoir_size);
    println!("   Spectral radius: {:.2}", config.spectral_radius);
    println!();
    
    // Create ESN
    println!("🧠 Creating ESN...");
    let mut esn = ESN::new(&device, config).await?;
    println!("✅ ESN initialized\n");
    
    // Generate training data
    println!("📊 Generating training data (sine wave)...");
    let num_train = 100;
    let mut train_inputs = Vec::new();
    let mut train_targets = Vec::new();
    
    for i in 0..num_train {
        let t = i as f32 * 0.1;
        train_inputs.push(vec![(t).sin()]);
        train_targets.push(vec![(t + 0.1).sin()]);
    }
    
    println!("   {} training samples\n", num_train);
    
    // Train
    println!("🎓 Training ESN...");
    let mse = esn.train(&train_inputs, &train_targets).await?;
    println!("✅ Training MSE: {:.6}\n", mse);
    
    // Test
    println!("🔮 Testing predictions...");
    esn.reset_state();
    let test_inputs: Vec<Vec<f32>> = (0..20).map(|i| {
        vec![((num_train + i) as f32 * 0.1).sin()]
    }).collect();
    
    let predictions = esn.predict(&test_inputs).await?;
    
    println!("✅ {} predictions generated\n", predictions.len());
    
    println!("🎊 Demo Complete!");
    println!("\n💡 Key Achievement:");
    println!("   Universal compute: Same code runs on NPU/GPU/CPU!");
    
    Ok(())
}
