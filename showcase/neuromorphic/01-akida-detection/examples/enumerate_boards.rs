//! Multi-board enumeration and topology example

use akida_detection_demo::detect_all_boards;
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    println!("Enumerating all Akida boards across mesh...\n");

    let mesh = detect_all_boards().await?;

    if mesh.boards.is_empty() {
        println!("No Akida boards detected.\n");
        return Ok(());
    }

    // Group by node
    let local = mesh.local_boards();
    let remote = mesh.remote_boards();

    println!("Local boards ({}): ", local.len());
    for board in &local {
        println!("  akida{}: {}", board.index, board.pcie_address);
    }
    println!();

    if !remote.is_empty() {
        println!("Remote boards ({}):", remote.len());
        for board in &remote {
            let node = board.node_name.as_ref().unwrap();
            println!("  {}/akida{}: {}", node, board.index, board.pcie_address);
        }
        println!();
    }

    // Show topology
    println!("Board topology:");

    // Strandgate boards
    let strandgate_boards: Vec<_> = mesh
        .boards
        .iter()
        .filter(|b| b.is_local()) // Assuming we're running on Strandgate
        .collect();

    if !strandgate_boards.is_empty() {
        println!("  Strandgate (PCIe lanes: 128)");
        for (i, board) in strandgate_boards.iter().enumerate() {
            let tree_char = if i == strandgate_boards.len() - 1 {
                "└"
            } else {
                "├"
            };
            println!(
                "    {}── Slot {}: akida{} (PCIe Gen{} x{})",
                tree_char,
                i + 1,
                board.index,
                board.pcie_generation,
                board.pcie_lanes
            );
        }
        println!();
    }

    // Remote boards (Southgate, etc.)
    for board in &remote {
        let node = board.node_name.as_ref().unwrap();
        println!("  {} (PCIe lanes: 24)", node);
        println!(
            "    └── Slot 1: akida{} (PCIe Gen{} x{})",
            board.index, board.pcie_generation, board.pcie_lanes
        );
        println!();
    }

    // Workload distribution recommendations
    println!("Optimal workload distribution:");
    println!(
        "  - Dense neuromorphic compute → Strandgate ({} boards, low latency)",
        local.len()
    );
    println!(
        "  - Real-time classification → Southgate ({} boards, near GPU)",
        remote.len()
    );
    println!(
        "  - Fault tolerance → All {} boards (automatic failover)",
        mesh.boards.len()
    );

    Ok(())
}
