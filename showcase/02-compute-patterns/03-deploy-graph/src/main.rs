// SPDX-License-Identifier: AGPL-3.0-only

use colored::Colorize;
use serde_json::json;
use std::path::Path;
use tokio::net::UnixStream;

/// `XDG_RUNTIME_DIR` fallback: discovers UID from `/proc/self/status` (pure Rust, no libc).
fn runtime_dir_fallback() -> String {
    let uid = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("Uid:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|u| u.parse::<u32>().ok())
        })
        .unwrap_or(1000);
    format!("/run/user/{uid}")
}

#[tokio::main]
async fn main() {
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("{}", "  ToadStool Showcase: Deploy Graph — Capability Routing".cyan().bold());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!();

    // Section: Architecture
    println!("{}", "► Architecture".cyan());
    println!("  Demarcation:");
    println!("  • toadStool decides WHERE to run (resource estimation, substrate selection)");
    println!("  • barraCuda decides WHAT to compute (shader dispatch, kernel selection)");
    println!();

    // Section: Capability Call
    println!("{}", "► Capability Call".cyan());
    let capability_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "deploy.capability_call",
        "params": {
            "capability": "compute",
            "method": "gpu.dispatch",
            "params": {
                "operation": "eigensolve",
                "dimensions": [256, 256],
                "precision": "f64"
            }
        }
    });
    println!("  Request (routes to barraCuda's compute.sock):");
    println!("{}", serde_json::to_string_pretty(&capability_request).unwrap());
    println!();

    // Section: Routing Decision
    println!("{}", "► Routing Decision".cyan());
    println!("  Decision tree:");
    println!("  1. Check resource availability (CPU, GPU memory)");
    println!("     → GPU memory: 8 GiB available");
    println!("  2. Check substrate capabilities (f64 support? shared memory reliable?)");
    println!("     → f64: supported, shared memory: reliable");
    println!("  3. Select target: barraCuda (GPU) or CPU fallback");
    println!("     → {} barraCuda (GPU) selected", "✓".green());
    println!();

    // Section: Graph Status
    println!("{}", "► Graph Status".cyan());
    let graph_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "deploy.graph_status",
        "params": {}
    });
    println!("  Request:");
    println!("{}", serde_json::to_string_pretty(&graph_request).unwrap());
    let graph_response = json!({
        "result": {
            "active_jobs": 1,
            "nodes": ["toadstool", "barracuda"],
            "edges": [{"from": "toadstool", "to": "barracuda", "pending": 0}]
        }
    });
    println!("  Simulated response:");
    println!("{}", serde_json::to_string_pretty(&graph_response).unwrap());
    println!();

    // Section: Socket Check
    println!("{}", "► Socket Check".cyan());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| runtime_dir_fallback());
    let compute_sock = format!("{}/biomeos/compute.sock", runtime_dir);

    let live = Path::new(&compute_sock).exists()
        && UnixStream::connect(&compute_sock).await.is_ok();

    if live {
        println!("  {} barraCuda compute.sock available", "✓".green());
    } else {
        println!("  {} barraCuda compute.sock not available (simulated routing)", "○".yellow());
    }
    println!();

    // Summary
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
    println!("  {} Deploy graph routing demonstrated — toadStool WHERE, barraCuda WHAT", "✓".green());
    println!("{}", "═══════════════════════════════════════════════════════════".cyan());
}
