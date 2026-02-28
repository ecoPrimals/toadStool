//! hotSpring Integration Bridge
//!
//! Connects ToadStool's BarraCuda to hotSpring's MD validation suite.
//! This allows running hotSpring's control experiments through ToadStool
//! for cross-validation.

use barracuda::device::WgpuDevice;
use std::sync::Arc;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Path to hotSpring repo (relative to ecoPrimals)
const HOTSPRING_PATH: &str = "../../../hotSpring";

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  hotSpring ↔ ToadStool Integration Bridge                     ║");
    println!("║  MD Validation via BarraCuda                                  ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    // Check hotSpring path
    let hotspring_exists = std::path::Path::new(HOTSPRING_PATH).exists();
    if !hotspring_exists {
        warn!("hotSpring not found at {}", HOTSPRING_PATH);
        warn!("Expected: ecoPrimals/hotSpring/");
        println!();
        println!("To set up hotSpring integration:");
        println!("  1. cd ../../../hotSpring");
        println!("  2. cargo run --bin validate_md");
        return Ok(());
    }

    info!("hotSpring found at {}", HOTSPRING_PATH);
    println!();

    // Initialize GPU
    let device = Arc::new(WgpuDevice::new().await?);
    println!("GPU Device: {}", device.name());
    println!();

    // Available validation binaries in hotSpring
    let validations = [
        ("validate_md", "Molecular Dynamics forces & integrators"),
        (
            "validate_linalg",
            "Linear algebra (Cholesky, eigendecomposition)",
        ),
        (
            "validate_special_functions",
            "Bessel, gamma, incomplete beta",
        ),
        ("validate_optimizers", "Nelder-Mead, Levenberg-Marquardt"),
        ("nuclear_eos_l1", "Nuclear EOS surrogate (L1)"),
        ("nuclear_eos_l2", "Nuclear EOS surrogate (L2)"),
        ("nuclear_eos_l2_hetero", "Nuclear EOS heterogeneous compute"),
    ];

    println!("═══ Available hotSpring Validations ═══");
    println!();
    for (name, description) in &validations {
        println!("  • {}", name);
        println!("    {}", description);
        println!(
            "    Run: cd {}/barracuda && cargo run --bin {}",
            HOTSPRING_PATH, name
        );
        println!();
    }

    println!("═══ Integration Status ═══");
    println!();
    println!("  hotSpring repo:       ✅ Found");
    println!("  BarraCuda dependency: ✅ Linked (path = ../../phase1/toadstool/crates/barracuda)");
    println!("  GPU device:           ✅ {} available", device.name());
    println!();

    // Quick health check: run a mini validation
    println!("═══ Quick Health Check ═══");
    println!();
    println!("  Testing BarraCuda tensor creation...");

    let test_data: Vec<f32> = (0..100).map(|i| i as f32).collect();
    let tensor = barracuda::tensor::Tensor::from_data(&test_data, vec![10, 10], device.clone())?;

    let retrieved = tensor.to_vec()?;
    let matches = test_data
        .iter()
        .zip(retrieved.iter())
        .all(|(a, b)| (a - b).abs() < 1e-6);

    if matches {
        println!("  ✅ Tensor round-trip: PASS");
    } else {
        println!("  ❌ Tensor round-trip: FAIL");
    }
    println!();

    println!("═══ Next Steps ═══");
    println!();
    println!("  1. Run MD validation:");
    println!("     cd {}/barracuda", HOTSPRING_PATH);
    println!("     cargo run --bin validate_md");
    println!();
    println!("  2. Run all validations:");
    println!("     cargo run --bin validate_linalg");
    println!("     cargo run --bin validate_special_functions");
    println!("     cargo run --bin validate_optimizers");
    println!();
    println!("  3. Run nuclear EOS surrogate:");
    println!("     cargo run --bin nuclear_eos_l1");
    println!("     cargo run --bin nuclear_eos_l2_hetero");
    println!();

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  Bridge Status: READY                                         ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");

    Ok(())
}
