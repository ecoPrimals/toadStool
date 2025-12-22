//! Query detailed board capabilities

use akida_detection_demo::{detect_all_boards, substrate_integration};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    println!("Akida Board Capabilities Report\n");
    println!("================================\n");
    
    let mesh = detect_all_boards().await?;
    
    if mesh.boards.is_empty() {
        println!("No Akida boards detected.\n");
        return Ok(());
    }
    
    for board in &mesh.boards {
        println!("Board {}: {}", board.index, board.chip_name);
        println!("{}", "─".repeat(50));
        
        println!("\nHardware:");
        println!("  Chip: {}", board.chip_name);
        println!("  Manufacturer: BrainChip");
        println!("  NPUs: {} neural processing units", board.npu_count);
        println!("  Neurons/NPU: ~1,024");
        println!("  Synapses/NPU: ~10,000");
        println!("  Total neurons: ~{:,}", board.npu_count * 1024);
        println!("  Total synapses: ~{:,}", board.npu_count * 10_000);
        
        println!("\nMemory:");
        println!("  On-chip SRAM: {} MB", board.memory_bytes / 1_048_576);
        println!("  Model storage: Up to 9 MB");
        println!("  Input buffer: ~512 KB");
        println!("  Output buffer: ~256 KB");
        
        println!("\nPCIe Interface:");
        println!("  Bus address: {}", board.pcie_address);
        println!("  Device: {}", board.device_path.display());
        println!("  Generation: PCIe Gen{}", board.pcie_generation);
        println!("  Lanes: x{}", board.pcie_lanes);
        println!("  Bandwidth: {:.1} GB/s", board.bandwidth_gbps());
        
        println!("\nPower & Thermal:");
        println!("  Current power: {:.1}W", board.power_watts);
        println!("  TDP: 10W (peak)");
        println!("  Idle power: ~0.1-0.3W");
        println!("  Temperature: {:.1}°C", board.temperature_celsius);
        
        println!("\nWorkload Compatibility:");
        let workloads = [
            ("Classification", true),
            ("Pattern matching", true),
            ("Intent recognition", true),
            ("K-mer filtering", true),
            ("Motion detection", true),
            ("Anomaly detection", true),
            ("Event processing", true),
            ("Matrix operations", false),
            ("Ray tracing", false),
        ];
        
        for (name, compatible) in &workloads {
            let status = if *compatible { "✓" } else { "✗" };
            println!("  {} {}", status, name);
        }
        
        println!("\nPerformance Characteristics:");
        println!("  Inference latency: <1ms (typical)");
        println!("  Power efficiency: 1000x vs GPU (for compatible workloads)");
        println!("  Event-driven: Processes spikes as they arrive");
        println!("  Batch processing: Not required (online learning)");
        
        println!("\nHealth:");
        println!("  Status: {:?}", board.health);
        
        if let Some(node) = &board.node_name {
            println!("\nLocation:");
            println!("  Node: {} (remote)", node);
        } else {
            println!("\nLocation:");
            println!("  Node: Local");
        }
        
        println!("\n");
    }
    
    // Show compatible workload types
    println!("Workload Routing Guidelines:");
    println!("{}", "─".repeat(50));
    println!("\nRoute to Akida when:");
    println!("  • Workload is classification or pattern matching");
    println!("  • Low latency is critical (<1ms required)");
    println!("  • Power efficiency is prioritized");
    println!("  • Input is event-driven or streaming");
    println!("  • Model is small (<9MB)");
    println!("\nRoute to GPU when:");
    println!("  • Workload requires matrix operations");
    println!("  • Model is large (>10MB)");
    println!("  • Throughput matters more than latency");
    println!("  • Workload is batch-oriented");
    
    Ok(())
}

