// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use std::time::Duration;
use toadstool_sysmon::{
    cpu_brand, cpu_count, disk_usage, load_average, memory_info, network_stats, per_cpu_usage,
};

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Hardware Discovery".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    let mut substrate_count = 0;

    // CPU Substrate
    println!("{}", "► CPU Substrate".cyan());
    println!("  Cores: {}", cpu_count());
    match cpu_brand() {
        Ok(brand) => println!("  Brand: {}", brand),
        Err(e) => println!("  Brand: (error: {})", e),
    }
    match per_cpu_usage(Duration::from_millis(100)) {
        Ok(usages) => {
            println!("  Per-CPU usage (first snapshot):");
            for (i, u) in usages.iter().take(8).enumerate() {
                println!("    CPU {}: {:.1}%", i, u);
            }
            if usages.len() > 8 {
                println!("    ... ({} more)", usages.len() - 8);
            }
        }
        Err(e) => println!("  Per-CPU usage: (error: {})", e),
    }
    substrate_count += 1;
    println!();

    // Memory Substrate
    println!("{}", "► Memory Substrate".cyan());
    match memory_info() {
        Ok(mem) => {
            println!("  Total:     {}", format_bytes(mem.total));
            println!("  Available: {}", format_bytes(mem.available));
            println!("  Used:      {}", format_bytes(mem.used));
        }
        Err(e) => println!("  (error: {})", e),
    }
    substrate_count += 1;
    println!();

    // Disk Substrate
    println!("{}", "► Disk Substrate".cyan());
    match disk_usage() {
        Ok(disks) => {
            for d in &disks {
                let used = d.total_space.saturating_sub(d.available_space);
                println!(
                    "  {}  total: {}  used: {}  available: {}",
                    d.mount_point,
                    format_bytes(d.total_space),
                    format_bytes(used),
                    format_bytes(d.available_space)
                );
            }
            if disks.is_empty() {
                println!("  (no real mounts found)");
            } else {
                substrate_count += 1;
            }
        }
        Err(e) => println!("  (error: {})", e),
    }
    println!();

    // Network Substrate
    println!("{}", "► Network Substrate".cyan());
    match network_stats() {
        Ok(ifaces) => {
            for iface in &ifaces {
                println!(
                    "  {}  rx: {}  tx: {}",
                    iface.name,
                    format_bytes(iface.received),
                    format_bytes(iface.transmitted)
                );
            }
            if ifaces.is_empty() {
                println!("  (no non-loopback interfaces)");
            } else {
                substrate_count += 1;
            }
        }
        Err(e) => println!("  (error: {})", e),
    }
    println!();

    // Load Average
    println!("{}", "► Load Average".cyan());
    match load_average() {
        Ok(la) => {
            println!("  1 min:  {:.2}", la.one);
            println!("  5 min:  {:.2}", la.five);
            println!("  15 min: {:.2}", la.fifteen);
        }
        Err(e) => println!("  (error: {})", e),
    }
    substrate_count += 1;
    println!();

    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!(
        "  {} Substrates discovered: {}",
        "✓".green(),
        substrate_count
    );
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
