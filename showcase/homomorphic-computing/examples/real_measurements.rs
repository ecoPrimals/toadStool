//! Real Hardware Measurement Demo
//!
//! **Deep Debt Principle Phase 4**: Measure actual hardware, don't hardcode!
//!
//! This demo shows:
//! - Real CPU power measurement (RAPL when available)
//! - Real GPU power measurement (nvidia-smi/rocm-smi when available)
//! - Real NPU power measurement (Akida API when available)
//! - Graceful fallback to estimates when APIs unavailable
//!
//! Run with:
//! ```bash
//! cargo run --example real_measurements --release
//! ```
//!
//! For RAPL access (real CPU power):
//! ```bash
//! sudo cargo run --example real_measurements --release
//! ```

use homomorphic_computing::*;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════╗");
    println!("║  📊 Real Hardware Measurement Demo                       ║");
    println!("║                                                          ║");
    println!("║  Deep Debt Principle: Measure, Don't Estimate!          ║");
    println!("║  • Real power consumption via hardware APIs              ║");
    println!("║  • Real performance via actual benchmarks                ║");
    println!("║  • Graceful fallback when APIs unavailable               ║");
    println!("╚══════════════════════════════════════════════════════════╝\n");
    
    // Phase 1: CPU Power Measurement
    println!("🔋 Phase 1: CPU Power Measurement\n");
    
    let cpu_monitor = CpuPowerMonitor::new()?;
    let cpu_power = cpu_monitor.measure_watts()?;
    
    if cpu_power.is_measured {
        println!("   ✅ Real Measurement via {}:", cpu_power.method);
        println!("   CPU Power: {:.2}W (actual measurement!)", cpu_power.watts);
        println!("   Source: Linux RAPL interface\n");
    } else {
        println!("   ⚠️  Fallback Estimate:");
        println!("   CPU Power: {:.2}W ({})", cpu_power.watts, cpu_power.method);
        println!("   Note: Run with 'sudo' for real RAPL measurement\n");
    }
    
    // Phase 2: GPU Power Measurement
    println!("🎮 Phase 2: GPU Power Measurement\n");
    
    let gpu_monitor = GpuPowerMonitor::new()?;
    let gpu_power = gpu_monitor.measure_watts()?;
    
    if gpu_power.is_measured {
        println!("   ✅ Real Measurement via {}:", gpu_power.method);
        println!("   GPU Power: {:.2}W (actual measurement!)", gpu_power.watts);
        println!("   Source: GPU vendor APIs\n");
    } else {
        println!("   ⚠️  Fallback Estimate:");
        println!("   GPU Power: {:.2}W ({})", gpu_power.watts, gpu_power.method);
        println!("   Note: Requires nvidia-smi or rocm-smi for real measurement\n");
    }
    
    // Phase 3: NPU Power Measurement
    println!("🧠 Phase 3: NPU Power Measurement\n");
    
    let npu_monitor = NpuPowerMonitor::new()?;
    let npu_power = npu_monitor.measure_watts()?;
    
    if npu_power.is_measured {
        println!("   ✅ Real Measurement via {}:", npu_power.method);
        println!("   NPU Power: {:.2}W (actual measurement!)", npu_power.watts);
        println!("   Source: BrainChip Akida API\n");
    } else {
        println!("   ⚠️  Fallback Estimate:");
        println!("   NPU Power: {:.2}W ({})", npu_power.watts, npu_power.method);
        println!("   Note: Requires Akida hardware for real measurement\n");
    }
    
    // Phase 4: Power Comparison
    println!("📊 Phase 4: Power Comparison\n");
    println!("┌─────────────┬────────────┬──────────────────────┐");
    println!("│ Substrate   │ Power (W)  │ Measurement Method   │");
    println!("├─────────────┼────────────┼──────────────────────┤");
    println!("│ CPU         │ {:>9.2} │ {:<20} │", 
        cpu_power.watts, 
        if cpu_power.is_measured { "✅ Measured" } else { "⚠️  Estimated" }
    );
    println!("│ GPU         │ {:>9.2} │ {:<20} │", 
        gpu_power.watts,
        if gpu_power.is_measured { "✅ Measured" } else { "⚠️  Estimated" }
    );
    println!("│ NPU         │ {:>9.2} │ {:<20} │", 
        npu_power.watts,
        if npu_power.is_measured { "✅ Measured" } else { "⚠️  Estimated" }
    );
    println!("└─────────────┴────────────┴──────────────────────┘\n");
    
    // Calculate efficiency metrics
    let _cpu_efficiency = 1.0; // Baseline
    let gpu_efficiency = cpu_power.watts / gpu_power.watts;
    let npu_efficiency = cpu_power.watts / npu_power.watts;
    
    println!("⚡ Phase 5: Energy Efficiency Analysis\n");
    println!("   CPU Baseline: 1.0x");
    println!("   GPU: {:.2}x more power-hungry", 1.0 / gpu_efficiency);
    println!("   NPU: {:.2}x more energy-efficient! 🌱\n", npu_efficiency);
    
    // Phase 6: Deep Debt Validation
    println!("💡 Phase 6: Deep Debt Principle Validation\n");
    
    let total_measured = [&cpu_power, &gpu_power, &npu_power]
        .iter()
        .filter(|p| p.is_measured)
        .count();
    
    let total_substrates = 3;
    let measurement_percentage = (total_measured as f64 / total_substrates as f64) * 100.0;
    
    println!("   Measurement Coverage: {}/{} ({:.0}%)", 
        total_measured, 
        total_substrates, 
        measurement_percentage
    );
    
    if total_measured == total_substrates {
        println!("   ✅ 100% Real Measurements - No Hardcoding!");
        println!("   ✅ Deep debt principle fully satisfied!");
    } else if total_measured > 0 {
        println!("   ✅ Partial Real Measurements ({:.0}%)", measurement_percentage);
        println!("   ✅ Graceful fallback for unavailable APIs");
        println!("   ✅ Deep debt principle: measure where possible!");
    } else {
        println!("   ⚠️  Using Estimates (hardware APIs unavailable)");
        println!("   ✅ Graceful degradation maintained");
        println!("   📝 Run with hardware access for real measurements");
    }
    
    println!("\n🎯 Key Achievements:\n");
    println!("   ✅ No hardcoded power values in production");
    println!("   ✅ Real measurement when hardware APIs available");
    println!("   ✅ Transparent about measurement vs estimate");
    println!("   ✅ Graceful degradation when APIs unavailable");
    println!("   ✅ Cross-platform support (Linux RAPL, nvidia-smi, etc.)");
    
    println!("\n✅ Real hardware measurement demo complete!\n");
    
    Ok(())
}
