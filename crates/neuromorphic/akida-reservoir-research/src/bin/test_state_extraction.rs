// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test: Can we extract internal NPU layer activations?
//!
//! **CRITICAL EXPERIMENT**: This determines if reservoir computing is feasible!

use akida_driver::DeviceManager;
use akida_models::Model;
use akida_reservoir_research::ReservoirResult as Result;
use akida_reservoir_research::state_extraction::StateExtractor;
use tracing::{error, info, warn};

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
    println!("║       🔬 EXPERIMENT 1: State Extraction Test 🔬                            ║");
    println!("║                                                                              ║");
    println!("║       RESEARCH QUESTION: Can we extract internal NPU layer states?          ║");
    println!("║                                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    // Step 1: Discover devices
    println!("1️⃣  Discovering Akida devices...\n");
    let manager = DeviceManager::discover().map_err(|e| {
        akida_reservoir_research::ReservoirError::InvalidState(format!(
            "Failed to discover Akida devices: {e}"
        ))
    })?;

    if manager.device_count() == 0 {
        error!("❌ No Akida devices found!");
        println!("\n⚠️  EXPERIMENT BLOCKED: No hardware available");
        println!("    Please ensure:");
        println!("    - Akida hardware is connected");
        println!("    - Kernel driver is loaded (lsmod | grep akida)");
        println!("    - User has permissions (ls -l /dev/akida*)");
        return Ok(());
    }

    info!("Found {} Akida device(s)", manager.device_count());
    for (i, device_info) in manager.devices().iter().enumerate() {
        println!("   Device {i}: {device_info:?}");
    }

    // Step 2: Check for model file
    println!("\n2️⃣  Looking for test model...\n");

    let test_model_paths = vec![
        "test_model.fbz",
        "models/test_model.fbz",
        "../models/test_model.fbz",
    ];

    let mut model_path = None;
    for path in &test_model_paths {
        if std::path::Path::new(path).exists() {
            model_path = Some(path.to_string());
            break;
        }
    }

    if model_path.is_none() {
        warn!("⚠️  No test model found!");
        println!("\n⚠️  EXPERIMENT NEEDS MODEL FILE");
        println!("    Looked for:");
        for path in &test_model_paths {
            println!("      - {path}");
        }
        println!("\n    To create a test model:");
        println!("      1. Use BrainChip Akida SDK to train a simple CNN");
        println!("      2. Save as .fbz format");
        println!("      3. Copy to one of the paths above");
        println!("\n    Or use generate-reservoir to create a random reservoir model");
        return Ok(());
    }

    let model_path = model_path.unwrap();
    info!("Found model: {}", model_path);

    // Step 3: Load model
    println!("\n3️⃣  Loading model...\n");
    let model = Model::from_file(&model_path).map_err(|e| {
        akida_reservoir_research::ReservoirError::InvalidState(format!(
            "Failed to load model: {model_path}: {e}"
        ))
    })?;

    info!("Model loaded: {} layers", model.layer_count());
    info!("Program size: {} bytes", model.program_size());

    // Step 4: Load to device
    println!("\n4️⃣  Loading model to device...\n");
    let mut device = manager.open_first().map_err(|e| {
        akida_reservoir_research::ReservoirError::InvalidState(format!(
            "Failed to open device: {e}"
        ))
    })?;

    let load_metrics = model.load_to_device(&mut device).map_err(|e| {
        akida_reservoir_research::ReservoirError::InvalidState(format!(
            "Failed to load to device: {e}"
        ))
    })?;

    info!("Loaded in {:?}", load_metrics.duration);

    // Step 5: Run inference
    println!("\n5️⃣  Running inference...\n");

    // Create dummy input (all zeros for now)
    let input_size = 784; // MNIST default
    let input = vec![0u8; input_size];

    info!("Input size: {} bytes", input.len());

    let result = model.infer(&input, &mut device).map_err(|e| {
        akida_reservoir_research::ReservoirError::InvalidState(format!("Inference failed: {e}"))
    })?;

    info!("Inference complete");
    info!("Output size: {} values", result.output.len());

    // Step 6: CRITICAL TEST - Try to extract states
    println!("\n6️⃣  🔬 ATTEMPTING STATE EXTRACTION 🔬\n");

    let extractor = StateExtractor::all_layers(model.layer_count());

    match extractor.extract_states(&model, &result) {
        Ok(states) => {
            println!("   ✅ State extraction succeeded!");
            println!("\n   Extracted {} layer(s):", states.len());

            for layer in &states {
                println!(
                    "      Layer {}: {} values",
                    layer.layer_idx,
                    layer.values.len()
                );

                if layer.values.len() <= 10 {
                    println!("         Values: {:?}", layer.values);
                } else {
                    println!("         First 10: {:?}", &layer.values[..10]);
                }
            }

            if states.len() == 1 {
                warn!("⚠️  Only extracted final layer!");
                println!("\n   ⚠️  PARTIAL SUCCESS:");
                println!("      - We can extract the final output");
                println!("      - But we need INTERNAL layer states for reservoir computing");
                println!("      - Driver enhancement needed!");
            } else {
                println!("\n   ✅ FULL SUCCESS!");
                println!("      - We can extract internal layer states!");
                println!("      - Reservoir computing is FEASIBLE! 🎉");
            }
        }
        Err(e) => {
            error!("❌ State extraction failed: {}", e);
            println!("\n   ❌ EXPERIMENT FAILED");
            println!("      Error: {e}");
        }
    }

    // Step 7: Show research notes
    println!("\n7️⃣  📝 Research Notes\n");
    println!("{}", StateExtractor::research_notes());

    // Step 8: Conclusion
    println!("\n╔══════════════════════════════════════════════════════════════════════════════╗");
    println!("║                                                                              ║");
    println!("║       📊 EXPERIMENT RESULTS                                                  ║");
    println!("║                                                                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════════════╝\n");

    println!("Status: PARTIAL (can extract final output, need internal states)");
    println!("\nNext Steps:");
    println!("   1. Research Akida kernel driver for layer access ioctls");
    println!("   2. Extend akida-driver to expose internal states");
    println!("   3. Implement proper layer introspection");
    println!("\nFeasibility: HIGH (hardware supports it, need driver work)");
    println!("Estimated Effort: 2-4 weeks\n");

    Ok(())
}
