//! Hardware Discovery Tool for ToadStool Universal Compute
//!
//! **Purpose**: Detect and validate all compute substrates
//! - CPUs (dual socket NUMA)
//! - GPUs (NVIDIA, AMD via WebGPU)
//! - NPUs (BrainChip Akida)
//!
//! **Deep Debt**: Validate foundation before building more!

use colored::*;
use serde::{Deserialize, Serialize};
use sysinfo::System;

#[derive(Debug, Serialize, Deserialize)]
struct HardwareInventory {
    cpus: Vec<CpuInfo>,
    gpus: Vec<GpuInfo>,
    npus: Vec<NpuInfo>,
    total_substrates: usize,
    validation_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CpuInfo {
    name: String,
    socket_id: usize,
    cores: usize,
    threads: usize,
    frequency_mhz: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GpuInfo {
    name: String,
    vendor: String,
    backend: String,
    device_type: String,
    memory_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NpuInfo {
    name: String,
    pci_address: String,
    device_id: u32,
    status: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "  ToadStool Universal Compute - Hardware Discovery".bright_white().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    // Detect all hardware
    let mut inventory = HardwareInventory {
        cpus: Vec::new(),
        gpus: Vec::new(),
        npus: Vec::new(),
        total_substrates: 0,
        validation_ready: false,
    };

    // 1. CPU Detection
    println!("{}", "🔍 Detecting CPUs...".bright_yellow().bold());
    detect_cpus(&mut inventory)?;
    println!();

    // 2. GPU Detection (via WebGPU/wgpu)
    println!("{}", "🔍 Detecting GPUs (via WebGPU)...".bright_yellow().bold());
    detect_gpus(&mut inventory).await?;
    println!();

    // 3. NPU Detection (BrainChip Akida)
    println!("{}", "🔍 Detecting NPUs (BrainChip Akida)...".bright_yellow().bold());
    detect_npus(&mut inventory).await?;
    println!();

    // Calculate totals
    inventory.total_substrates = inventory.cpus.len() + inventory.gpus.len() + inventory.npus.len();
    inventory.validation_ready = inventory.total_substrates >= 1; // Need at least 1 substrate

    // Display Summary
    display_summary(&inventory);

    // Export to JSON
    export_inventory(&inventory)?;

    // Validation readiness check
    check_validation_readiness(&inventory);

    Ok(())
}

/// Detect CPU configuration (dual socket EPYC)
fn detect_cpus(inventory: &mut HardwareInventory) -> Result<(), Box<dyn std::error::Error>> {
    let mut sys = System::new_all();
    sys.refresh_all();

    // Get total CPU info
    let total_cpus = sys.cpus().len();
    let cpu_name = sys.cpus().first().map(|c| c.brand()).unwrap_or("Unknown");

    // Detect dual socket configuration
    // EPYC 7452 has 32 cores per socket, 2 threads per core = 64 threads per socket
    let threads_per_socket = total_cpus / 2; // Assume dual socket if even count >= 64
    let cores_per_socket = threads_per_socket / 2; // Assume 2 threads per core

    if total_cpus >= 64 {
        // Likely dual socket
        for socket in 0..2 {
            let cpu_info = CpuInfo {
                name: cpu_name.to_string(),
                socket_id: socket,
                cores: cores_per_socket,
                threads: threads_per_socket,
                frequency_mhz: sys.cpus().first().map(|c| c.frequency() as f32).unwrap_or(0.0),
            };
            println!("  {} CPU Socket {}: {} ({} cores, {} threads)",
                "✅".green(),
                socket,
                cpu_name.bright_white(),
                cores_per_socket.to_string().bright_cyan(),
                threads_per_socket.to_string().bright_cyan()
            );
            inventory.cpus.push(cpu_info);
        }
    } else {
        // Single socket or unknown
        let cpu_info = CpuInfo {
            name: cpu_name.to_string(),
            socket_id: 0,
            cores: total_cpus / 2,
            threads: total_cpus,
            frequency_mhz: sys.cpus().first().map(|c| c.frequency() as f32).unwrap_or(0.0),
        };
        println!("  {} CPU: {} ({} cores, {} threads)",
            "✅".green(),
            cpu_name.bright_white(),
            (total_cpus / 2).to_string().bright_cyan(),
            total_cpus.to_string().bright_cyan()
        );
        inventory.cpus.push(cpu_info);
    }

    Ok(())
}

/// Detect GPUs via WebGPU (wgpu)
async fn detect_gpus(inventory: &mut HardwareInventory) -> Result<(), Box<dyn std::error::Error>> {
    // Create wgpu instance
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Enumerate adapters
    let adapters = instance.enumerate_adapters(wgpu::Backends::all());

    for (idx, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        
        // Skip software renderers for now
        if info.device_type == wgpu::DeviceType::Cpu {
            continue;
        }

        let gpu_info = GpuInfo {
            name: info.name.clone(),
            vendor: format!("{:?}", info.vendor),
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            memory_mb: None, // wgpu doesn't expose memory directly
        };

        println!("  {} GPU {}: {} ({})",
            "✅".green(),
            idx,
            info.name.bright_white(),
            format!("{:?}", info.backend).bright_cyan()
        );
        println!("     Backend: {}, Type: {:?}",
            format!("{:?}", info.backend).yellow(),
            info.device_type
        );

        inventory.gpus.push(gpu_info);
    }

    if inventory.gpus.is_empty() {
        println!("  {} No discrete GPUs detected via WebGPU", "⚠️".yellow());
    }

    Ok(())
}

/// Detect NPUs (BrainChip Akida)
async fn detect_npus(inventory: &mut HardwareInventory) -> Result<(), Box<dyn std::error::Error>> {
    // Try to detect Akida NPUs
    match detect_akida_npus().await {
        Ok(npus) => {
            for (idx, npu) in npus.iter().enumerate() {
                println!("  {} NPU {}: BrainChip Akida (PCI: {})",
                    "✅".green(),
                    idx,
                    npu.pci_address.bright_white()
                );
                inventory.npus.push(npu.clone());
            }
        }
        Err(e) => {
            println!("  {} NPU detection: {} ({})",
                "⚠️".yellow(),
                "Not available".bright_white(),
                e.to_string().dimmed()
            );
        }
    }

    Ok(())
}

/// Detect Akida NPUs via PCIe scan
async fn detect_akida_npus() -> Result<Vec<NpuInfo>, Box<dyn std::error::Error>> {
    let mut npus = Vec::new();

    // Try to read lspci output for BrainChip devices
    let output = std::process::Command::new("lspci")
        .args(&["-d", "1e7c::", "-v"]) // BrainChip vendor ID: 1e7c
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for (idx, line) in stdout.lines().enumerate() {
                if line.contains("Brainchip") || line.contains("AKD1000") {
                    // Extract PCI address (e.g., "a1:00.0")
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let pci_addr = parts.first().unwrap_or(&"unknown").to_string();
                    
                    npus.push(NpuInfo {
                        name: "BrainChip Akida AKD1000".to_string(),
                        pci_address: pci_addr,
                        device_id: idx as u32,
                        status: "Detected".to_string(),
                    });
                }
            }
        }
        _ => {
            return Err("lspci command failed or BrainChip devices not found".into());
        }
    }

    if npus.is_empty() {
        return Err("No BrainChip Akida devices detected".into());
    }

    Ok(npus)
}

/// Display comprehensive summary
fn display_summary(inventory: &HardwareInventory) {
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "  HARDWARE INVENTORY SUMMARY".bright_white().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    println!("  {} CPUs: {}", "📊".bright_white(), inventory.cpus.len().to_string().bright_green().bold());
    for cpu in &inventory.cpus {
        println!("     • Socket {}: {} cores, {} threads",
            cpu.socket_id,
            cpu.cores.to_string().bright_cyan(),
            cpu.threads.to_string().bright_cyan()
        );
    }
    println!();

    println!("  {} GPUs: {}", "🎮".bright_white(), inventory.gpus.len().to_string().bright_green().bold());
    for gpu in &inventory.gpus {
        println!("     • {} ({})", gpu.name, gpu.backend);
    }
    if inventory.gpus.is_empty() {
        println!("     {} No GPUs detected", "⚠️".yellow());
    }
    println!();

    println!("  {} NPUs: {}", "🧠".bright_white(), inventory.npus.len().to_string().bright_green().bold());
    for npu in &inventory.npus {
        println!("     • {} (PCI: {})", npu.name, npu.pci_address);
    }
    if inventory.npus.is_empty() {
        println!("     {} No NPUs detected", "⚠️".yellow());
    }
    println!();

    println!("{}", "───────────────────────────────────────────────────────────────".dimmed());
    println!("  {} Total Substrates: {}",
        "🔢".bright_white(),
        inventory.total_substrates.to_string().bright_green().bold()
    );
    println!("{}", "───────────────────────────────────────────────────────────────".dimmed());
}

/// Export inventory to JSON
fn export_inventory(inventory: &HardwareInventory) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(inventory)?;
    std::fs::write("hardware_inventory.json", json)?;
    println!();
    println!("  {} Hardware inventory exported to: {}",
        "💾".bright_white(),
        "hardware_inventory.json".bright_cyan()
    );
    Ok(())
}

/// Check if system is ready for validation
fn check_validation_readiness(inventory: &HardwareInventory) {
    println!();
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!("{}", "  VALIDATION READINESS".bright_white().bold());
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
    println!();

    if inventory.total_substrates >= 6 {
        println!("  {} Status: {} (6+ substrates detected!)",
            "✅".green(),
            "EXCELLENT".bright_green().bold()
        );
        println!("     Heterogeneous validation ready:");
        println!("     • {} CPUs for reference baseline", inventory.cpus.len());
        println!("     • {} GPUs for cross-vendor comparison", inventory.gpus.len());
        println!("     • {} NPUs for neuromorphic validation", inventory.npus.len());
        println!();
        println!("  {} {}",
            "🚀".bright_white(),
            "Ready to validate \"same math on any chip\"!".bright_green().bold()
        );
    } else if inventory.total_substrates >= 3 {
        println!("  {} Status: {} ({} substrates)",
            "✅".green(),
            "GOOD".bright_green().bold(),
            inventory.total_substrates
        );
        println!("     Cross-substrate validation possible");
    } else if inventory.total_substrates >= 1 {
        println!("  {} Status: {} ({} substrate)",
            "⚠️".yellow(),
            "LIMITED".yellow().bold(),
            inventory.total_substrates
        );
        println!("     Single substrate - limited validation");
    } else {
        println!("  {} Status: {} (no substrates)",
            "❌".red(),
            "NOT READY".red().bold()
        );
    }

    println!();
    println!("{}", "═══════════════════════════════════════════════════════════════".bright_cyan());
}
