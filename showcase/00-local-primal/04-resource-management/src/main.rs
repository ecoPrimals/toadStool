// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;
use toadstool_sysmon::{cpu_count, disk_usage, load_average, memory_info};

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Resource Management".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section 1: System Resources
    println!("{}", "► System Resources".cyan());
    let cpu_cores = cpu_count();
    let mem = memory_info().unwrap_or(toadstool_sysmon::MemoryInfo {
        total: 0,
        available: 0,
        used: 0,
        swap_total: 0,
        swap_free: 0,
    });
    let load = load_average().unwrap_or(toadstool_sysmon::LoadAverage {
        one: 0.0,
        five: 0.0,
        fifteen: 0.0,
    });
    let disks = disk_usage().unwrap_or_default();

    println!("  CPU cores: {}", cpu_cores);
    println!("  Memory: {} total, {} available, {} used", format_bytes(mem.total), format_bytes(mem.available), format_bytes(mem.used));
    println!("  Load average: 1m={:.2} 5m={:.2} 15m={:.2}", load.one, load.five, load.fifteen);
    if let Some(d) = disks.first() {
        println!("  Disk ({}): {} total, {} available", d.mount_point, format_bytes(d.total_space), format_bytes(d.available_space));
    } else {
        println!("  Disk: (no mounts)");
    }
    println!();

    // Section 2: Resource Estimation
    println!("{}", "► Resource Estimation".cyan());
    let workload = json!({
        "cpu_cores": 4,
        "memory_gb": 8,
        "gpu_memory_mb": 2048,
        "estimated_duration_secs": 120
    });
    println!("{}", serde_json::to_string_pretty(&workload).unwrap());
    println!();

    // Section 3: Availability Check
    let req_cpu = 4;
    let req_mem_gb = 8u64;
    let req_mem_bytes = req_mem_gb * 1024 * 1024 * 1024;

    let cpu_ok = cpu_cores >= req_cpu;
    let mem_ok = mem.available >= req_mem_bytes;

    println!("{}", "► Availability Check".cyan());
    println!("  CPU cores (need {}): {}", req_cpu, if cpu_ok { "✓ pass".green() } else { "✗ fail".red() });
    println!("  Memory (need {} GB): {}", req_mem_gb, if mem_ok { "✓ pass".green() } else { "✗ fail".red() });
    println!("  GPU memory: {} (requires wgpu probe)", "—".yellow());
    println!();

    // Section 4: Optimization Suggestions
    println!("{}", "► Optimization Suggestions".cyan());
    let mut suggestions = Vec::new();

    if load.one > 2.0 {
        suggestions.push("Consider deferring workload — system under load".to_string());
    }
    let mem_pct = if mem.total > 0 { (mem.used as f64 / mem.total as f64) * 100.0 } else { 0.0 };
    if mem_pct > 80.0 {
        suggestions.push("Memory pressure detected — reduce batch size".to_string());
    }
    if cpu_cores >= req_cpu {
        suggestions.push("CPU allocation: nominal".to_string());
    }
    suggestions.push("GPU memory: requires wgpu probe (run 02-hardware-discovery for details)".to_string());

    for s in &suggestions {
        println!("  • {}", s);
    }
    println!();

    // Summary
    let passed = [cpu_ok, mem_ok].iter().filter(|&&x| x).count();
    let total = 2;
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!(
        "  Resource assessment complete — {}/{} checks passed",
        passed,
        total
    );
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
