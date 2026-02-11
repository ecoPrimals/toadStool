//! Basic Akida detection example
//!
//! Demonstrates PCIe bus scanning and board discovery.

use akida_detection_demo::{detect_all_boards, substrate_integration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Scanning PCIe bus for Akida devices...\n");

    // Detect all Akida boards
    let mesh = detect_all_boards().await?;

    if mesh.boards.is_empty() {
        println!("No Akida boards detected.");
        println!("\nNote: This demo requires BrainChip Akida PCIe boards to be installed.");
        println!("Expected deployment: 2x on Strandgate, 1x on Southgate");
        return Ok(());
    }

    println!("Found {} Akida board(s):\n", mesh.boards.len());

    // Display each board
    for board in &mesh.boards {
        println!(
            "  Board {}: {} @ {}",
            board.index, board.chip_name, board.pcie_address
        );

        if let Some(node) = &board.node_name {
            println!("    Location: {} (remote)", node);
        } else {
            println!("    Location: Local");
        }

        println!("    NPUs: {}", board.npu_count);
        println!("    Memory: {} MB", board.memory_bytes / 1_048_576);
        println!("    Power: {:.1}W (current)", board.power_watts);
        println!("    Temperature: {:.1}°C", board.temperature_celsius);
        println!(
            "    PCIe: Gen{} x{} ({:.1} GB/s bandwidth)",
            board.pcie_generation,
            board.pcie_lanes,
            board.bandwidth_gbps()
        );
        println!("    Health: {:?}", board.health);
        println!();
    }

    // Summary
    println!("Total neuromorphic compute capacity:");
    println!("  - {} NPUs", mesh.total_npus);
    println!("  - {} MB total SRAM", mesh.total_memory_bytes / 1_048_576);
    println!("  - {:.1}W total power consumption", mesh.total_power_watts);
    println!(
        "  - {} independent boards for redundancy\n",
        mesh.boards.len()
    );

    // Register with UniversalSubstrate
    println!("Registering with UniversalSubstrate...");
    substrate_integration::register_with_substrate(&mesh).await?;
    println!("✓ All boards registered successfully\n");

    Ok(())
}
