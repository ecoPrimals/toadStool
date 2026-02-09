//! ToadStool Hardware Discovery Example
//!
//! Demonstrates pure Rust hardware discovery on a fresh system
//! No scripts, no sudo, no manual setup

use anyhow::Result;
use toadstool_core::{HardwareManager, HardwareType};

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    println!("=== ToadStool Hardware Discovery ===\n");
    println!("Deep Debt: Pure Rust, no scripts, no sudo\n");
    
    // Discover all hardware
    println!("[1/3] Discovering hardware...");
    let hw = HardwareManager::discover()?;
    
    println!("Found {} device(s)\n", hw.devices().len());
    
    // Show all devices
    println!("[2/3] Device inventory:");
    for device in hw.devices() {
        println!("\n  Device: {}", device.name);
        println!("  Type: {:?}", device.hardware_type);
        
        if let Some(ref addr) = device.pcie_address {
            println!("  PCIe: {}", addr);
        }
        
        if let Some(ref vendor) = device.vendor_id {
            println!("  Vendor: 0x{}", vendor);
        }
        
        println!("  Kernel driver: {}", if device.driver_available { "✓" } else { "✗" });
        println!("  Userspace: {}", if device.userspace_capable { "✓" } else { "✗" });
    }
    
    // Show hardware summary
    println!("\n[3/3] Hardware summary:");
    println!("  GPUs: {}", hw.devices_by_type(HardwareType::Gpu).len());
    println!("  NPUs: {}", hw.devices_by_type(HardwareType::Npu).len());
    println!("  CPUs: {}", hw.devices_by_type(HardwareType::Cpu).len());
    
    // Check what BarraCUDA can use
    println!("\n=== BarraCUDA Capabilities ===");
    if hw.has_gpu() {
        println!("✓ GPU compute available (via WGPU)");
    }
    if hw.has_npu() {
        println!("✓ NPU compute available");
        let npus = hw.devices_by_type(HardwareType::Npu);
        for npu in npus {
            let mode = if npu.driver_available {
                "kernel driver (high performance)"
            } else if npu.userspace_capable {
                "userspace driver (no kernel module)"
            } else {
                "unavailable (need permissions)"
            };
            println!("  → {}: {}", npu.name, mode);
        }
    }
    println!("✓ CPU compute always available");
    
    println!("\n=== Status ===");
    println!("ToadStool ready! No setup required.");
    println!("BarraCUDA can run math on {} device(s).", hw.devices().len());
    
    Ok(())
}
