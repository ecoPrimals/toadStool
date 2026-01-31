//! Capability-Based Substrate Selection Demo
//!
//! This example demonstrates runtime substrate discovery and selection
//! based on workload characteristics.
//!
//! **Deep Debt Principle**: No hardcoding, runtime capability discovery
//!
//! Run with:
//! ```bash
//! cargo run --example capability_selection --release
//! ```

use homomorphic_computing::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  🔍 Capability-Based Substrate Selection Demo            ║");
    println!("║                                                          ║");
    println!("║  Deep Debt Principle: Runtime Discovery                 ║");
    println!("║  • No hardcoded substrate choices                        ║");
    println!("║  • Auto-detect available hardware                        ║");
    println!("║  • Select based on workload characteristics              ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    // Phase 1: Substrate Discovery
    println!("🔍 Phase 1: Discovering Available Substrates...\n");
    
    let selector = SubstrateSelector::detect().await?;
    
    println!("✅ Discovery complete!");
    println!("   Available substrates: {}", selector.available_count());
    println!("   Names: {:?}\n", selector.available_names());
    
    // Phase 2: Workload-Based Selection
    println!("🎯 Phase 2: Workload-Based Selection\n");
    
    // Scenario 1: Edge Deployment
    println!("📱 Scenario 1: Edge Deployment (Power-Constrained)");
    println!("   Power budget: 5W");
    println!("   Latency target: <50ms");
    println!("   Continuous operation: Yes");
    
    let edge_hints = WorkloadHints::edge_deployment();
    let edge_substrate = selector.select(&edge_hints)?;
    
    println!("   → Selected: {} ⭐", edge_substrate.name());
    println!("   → Rationale: Best energy efficiency for 24/7 operation\n");
    
    // Scenario 2: Batch Processing
    println!("📊 Scenario 2: Batch Processing (High Throughput)");
    println!("   Power budget: Unlimited");
    println!("   Batch size: 1000");
    println!("   Throughput priority: Yes");
    
    let batch_hints = WorkloadHints::batch_processing();
    let batch_substrate = selector.select(&batch_hints)?;
    
    println!("   → Selected: {} ⭐", batch_substrate.name());
    println!("   → Rationale: Highest throughput for large batches\n");
    
    // Scenario 3: Real-Time Streaming
    println!("🌊 Scenario 3: Real-Time Streaming");
    println!("   Power budget: 10W");
    println!("   Latency target: <10ms");
    println!("   Batch size: 1");
    println!("   Continuous operation: Yes");
    
    let streaming_hints = WorkloadHints::streaming();
    let streaming_substrate = selector.select(&streaming_hints)?;
    
    println!("   → Selected: {} ⭐", streaming_substrate.name());
    println!("   → Rationale: Low latency + energy efficient streaming\n");
    
    // Phase 3: Custom Workload
    println!("⚙️  Phase 3: Custom Workload Configuration\n");
    
    let custom_hints = WorkloadHints {
        power_budget_watts: Some(3.0),  // Very constrained
        throughput_priority: false,
        latency_ms_target: Some(5.0),   // Low latency
        batch_size: Some(10),
        continuous_operation: true,
    };
    
    println!("🎚️  Custom Hints:");
    println!("   Power budget: 3W (very constrained)");
    println!("   Latency target: <5ms");
    println!("   Batch size: 10");
    println!("   Continuous: Yes");
    
    let custom_substrate = selector.select(&custom_hints)?;
    
    println!("   → Selected: {} ⭐", custom_substrate.name());
    println!("   → Rationale: Meets all constraints efficiently\n");
    
    // Phase 4: Comparison
    println!("📊 Phase 4: Selection Comparison\n");
    println!("┌─────────────────────┬──────────────────────────┐");
    println!("│ Workload Type       │ Selected Substrate       │");
    println!("├─────────────────────┼──────────────────────────┤");
    println!("│ Edge Deployment     │ {:<24} │", edge_substrate.name());
    println!("│ Batch Processing    │ {:<24} │", batch_substrate.name());
    println!("│ Real-Time Streaming │ {:<24} │", streaming_substrate.name());
    println!("│ Custom (3W budget)  │ {:<24} │", custom_substrate.name());
    println!("└─────────────────────┴──────────────────────────┘\n");
    
    // Phase 5: Capability-Based Insights
    println!("💡 Phase 5: Capability-Based Selection Insights\n");
    println!("   ✅ No hardcoded substrate choices");
    println!("   ✅ Runtime hardware discovery");
    println!("   ✅ Automatic workload matching");
    println!("   ✅ Power-aware selection");
    println!("   ✅ Performance-aware selection");
    println!("   ✅ Primal self-knowledge only\n");
    
    println!("🎯 Deep Debt Principle Validated:");
    println!("   Capability-based design allows:");
    println!("   • Adapting to available hardware");
    println!("   • Optimizing for workload characteristics");
    println!("   • Operating without external dependencies");
    println!("   • Evolving substrate implementations transparently\n");
    
    println!("✅ Capability-based selection demo complete!\n");
    
    Ok(())
}
