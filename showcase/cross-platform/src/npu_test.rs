//! Direct NPU Test via VFIO
//!
//! Tests the Akida NPU through the pure Rust VFIO backend.

use akida_driver::{select_backend, BackendSelection};

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Akida NPU Direct Test via VFIO                               ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Test NPU 1
    println!("Testing NPU #1 (0000:a1:00.0)...");
    match select_backend(BackendSelection::Vfio, "0000:a1:00.0") {
        Ok(mut backend) => {
            info!("NPU #1 initialized via VFIO");
            println!("  Backend type: {:?}", backend.backend_type());
            println!("  Ready: {}", backend.is_ready());

            // Try a simple inference
            let test_input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
            match backend.infer(&test_input) {
                Ok(output) => {
                    println!("  Inference output: {} values", output.len());
                    if !output.is_empty() {
                        println!("  First 5: {:?}", &output[..output.len().min(5)]);
                    }
                }
                Err(e) => println!("  Inference error: {}", e),
            }

            // Power measurement
            match backend.measure_power() {
                Ok(power) => println!("  Power: {:.2} W", power),
                Err(e) => println!("  Power measurement: {}", e),
            }
        }
        Err(e) => {
            println!("  ✗ Failed to initialize: {}", e);
        }
    }

    println!();

    // Test NPU 2
    println!("Testing NPU #2 (0000:e2:00.0)...");
    match select_backend(BackendSelection::Vfio, "0000:e2:00.0") {
        Ok(mut backend) => {
            info!("NPU #2 initialized via VFIO");
            println!("  Backend type: {:?}", backend.backend_type());
            println!("  Ready: {}", backend.is_ready());

            // Try inference
            let test_input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
            match backend.infer(&test_input) {
                Ok(output) => {
                    println!("  Inference output: {} values", output.len());
                }
                Err(e) => println!("  Inference error: {}", e),
            }
        }
        Err(e) => {
            println!("  ✗ Failed to initialize: {}", e);
        }
    }

    println!();
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  NPU Test Complete                                            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    Ok(())
}
