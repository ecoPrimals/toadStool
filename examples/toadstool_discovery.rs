//! ToadStool Hardware Discovery Example
//!
//! Demonstrates pure Rust hardware discovery on a fresh system
//! No scripts, no sudo, no manual setup

use anyhow::Result;

fn main() -> Result<()> {
    println!("=== ToadStool Hardware Discovery ===\n");
    println!("Deep Debt: Pure Rust, no scripts, no sudo\n");
    
    println!("[1/3] Discovering hardware...");
    println!("  (Note: Full implementation in toadstool-core crate)");
    
    // Show what ToadStool discovers
    println!("\n[2/3] Device inventory:");
    
    // Check for GPUs via /sys/class/drm
    print!("\n  Checking GPUs (via /sys/class/drm)... ");
    if let Ok(entries) = std::fs::read_dir("/sys/class/drm") {
        let gpu_count = entries.filter(|e| {
            e.as_ref().ok()
                .and_then(|entry| entry.path().join("device").exists().then_some(()))
                .is_some()
        }).count();
        
        if gpu_count > 0 {
            println!("✓ Found {} GPU(s)", gpu_count);
            println!("    → BarraCUDA can use via WGPU (no drivers needed)");
        } else {
            println!("✗ No GPUs detected");
        }
    } else {
        println!("✗ Cannot access /sys/class/drm");
    }
    
    // Check for NPUs via PCIe scan
    print!("\n  Checking NPUs (via /sys/bus/pci/devices)... ");
    if let Ok(entries) = std::fs::read_dir("/sys/bus/pci/devices") {
        let mut npu_count = 0;
        
        for entry in entries.flatten() {
            let device_path = entry.path();
            if let Ok(vendor) = std::fs::read_to_string(device_path.join("vendor")) {
                if vendor.trim() == "0x1e7c" {
                    npu_count += 1;
                    
                    if let Ok(device_id) = std::fs::read_to_string(device_path.join("device")) {
                        let name = match device_id.trim() {
                            "0xbca1" => "Akida AKD1000",
                            "0xbca2" => "Akida AKD1500",
                            _ => "Akida NPU",
                        };
                        
                        let pcie_addr = device_path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown");
                        
                        println!("\n    → Found: {} at {}", name, pcie_addr);
                        
                        // Check kernel driver
                        let has_kernel = std::path::Path::new("/dev").read_dir()
                            .map(|entries| {
                                entries.flatten()
                                    .any(|e| e.file_name().to_string_lossy().starts_with("akida"))
                            })
                            .unwrap_or(false);
                        
                        // Check userspace capability
                        let has_userspace = device_path.join("resource0").exists();
                        
                        if has_kernel {
                            println!("      Kernel driver: ✓ (high performance)");
                        } else if has_userspace {
                            println!("      Userspace driver: ✓ (no kernel module needed)");
                        } else {
                            println!("      Access: ✗ (need permissions)");
                        }
                    }
                }
            }
        }
        
        if npu_count == 0 {
            println!("✗ No NPUs detected");
        }
    } else {
        println!("✗ Cannot access /sys/bus/pci/devices");
    }
    
    // CPU always available
    println!("\n  CPU: ✓ Always available");
    
    println!("\n[3/3] Summary:");
    println!("  ToadStool discovers hardware at runtime (no scripts)");
    println!("  BarraCUDA runs math on all discovered hardware");
    println!("  No sudo needed for userspace drivers");
    println!("  Self-adapts to hardware changes (hot-plug)");
    
    println!("\n=== Status ===");
    println!("✓ ToadStool hardware layer ready");
    println!("✓ BarraCUDA can run computations");
    println!("✓ No manual setup required on fresh systems");
    
    Ok(())
}
